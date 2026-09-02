use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use indexmap::IndexMap;

use super::store::{CACHE_FILE, read_cache};
use super::{VNode, commit, fence_token};

fn scratch(name: &str) -> PathBuf {
  let nanos = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("time")
    .as_nanos();
  let dir = std::env::temp_dir().join(format!("dkcache-{name}-{nanos}"));
  fs::create_dir_all(&dir).expect("dir");
  dir
}

fn host(id: &str, text: &str) -> VNode {
  VNode::host(id, text)
}

fn frame(id: &str, children: Vec<VNode>) -> VNode {
  VNode::frame(id, children)
}

fn apply(dir: &std::path::Path, entries: &[(&str, Vec<VNode>)]) {
  let mut files = IndexMap::new();
  for (name, nodes) in entries {
    files.insert(
      dir.join(name).to_str().expect("utf8").to_string(),
      nodes.clone(),
    );
  }
  commit(dir.to_str().expect("utf8"), &files).expect("commit");
}

fn read(dir: &std::path::Path, name: &str) -> String {
  fs::read_to_string(dir.join(name)).expect("read")
}

#[test]
fn a_create_missing_file() {
  let dir = scratch("a");
  apply(&dir, &[("out.ts", vec![host("one", "alpha")])]);
  assert_eq!(read(&dir, "out.ts"), "alpha");
}

#[test]
fn b_noop_when_unchanged() {
  let dir = scratch("b");
  apply(&dir, &[("out.ts", vec![host("one", "alpha")])]);
  let path = dir.join("out.ts");
  let before = fs::metadata(&path)
    .expect("meta")
    .modified()
    .expect("mtime");
  apply(&dir, &[("out.ts", vec![host("one", "alpha")])]);
  let after = fs::metadata(&path)
    .expect("meta")
    .modified()
    .expect("mtime");
  assert_eq!(before, after);
}

#[test]
fn c_replace_one_region() {
  let dir = scratch("c");
  apply(
    &dir,
    &[(
      "out.ts",
      vec![host("a", "keep"), host("b", "old"), host("c", "tail")],
    )],
  );
  apply(
    &dir,
    &[(
      "out.ts",
      vec![host("a", "keep"), host("b", "new"), host("c", "tail")],
    )],
  );
  assert_eq!(read(&dir, "out.ts"), "keepnewtail");
}

#[test]
fn d_cut_region() {
  let dir = scratch("d");
  apply(
    &dir,
    &[(
      "out.ts",
      vec![host("a", "keep"), host("b", "gone"), host("c", "tail")],
    )],
  );
  apply(
    &dir,
    &[("out.ts", vec![host("a", "keep"), host("c", "tail")])],
  );
  assert_eq!(read(&dir, "out.ts"), "keeptail");
}

#[test]
fn e_insert_in_order() {
  let dir = scratch("e");
  apply(
    &dir,
    &[("out.ts", vec![host("a", "one"), host("c", "three")])],
  );
  apply(
    &dir,
    &[(
      "out.ts",
      vec![host("a", "one"), host("b", "two"), host("c", "three")],
    )],
  );
  assert_eq!(read(&dir, "out.ts"), "onetwothree");
}

#[test]
fn f_empty_deletes_file() {
  let dir = scratch("f");
  apply(&dir, &[("out.ts", vec![host("a", "x")])]);
  apply(&dir, &[]);
  assert!(!dir.join("out.ts").exists());
}

#[test]
fn j_replace_template_chunk() {
  let dir = scratch("j");
  apply(
    &dir,
    &[("out.ts", vec![host("lit0", "class "), host("e1", "User")])],
  );
  apply(
    &dir,
    &[(
      "out.ts",
      vec![host("lit0", "export class "), host("e1", "User")],
    )],
  );
  assert_eq!(read(&dir, "out.ts"), "export class User");
}

#[test]
fn k_preserves_unmarked() {
  let dir = scratch("k");
  apply(
    &dir,
    &[("out.ts", vec![host("a", "gen1"), host("b", "gen2")])],
  );
  let generated = read(&dir, "out.ts");
  let at = generated.find("gen2").expect("b");
  let mut edited = String::new();
  edited.push_str(&generated[..at]);
  edited.push_str("// user note\n");
  edited.push_str(&generated[at..]);
  fs::write(dir.join("out.ts"), edited).expect("edit");
  apply(
    &dir,
    &[("out.ts", vec![host("a", "gen1"), host("b", "gen2")])],
  );
  let text = read(&dir, "out.ts");
  assert!(text.contains("// user note"), "{text}");
  assert_eq!(text, "gen1// user note\ngen2");
}

#[test]
fn python_output_has_no_hash_fences() {
  let dir = scratch("py");
  apply(&dir, &[("out.py", vec![host("one", "x = 1\n")])]);
  let text = read(&dir, "out.py");
  assert_eq!(text, "x = 1\n");
  assert!(!text.contains("# @dk"), "{text}");
  assert!(!text.contains("/*"), "{text}");
  let parsed = std::process::Command::new("python3")
    .args(["-c", "import ast, sys; ast.parse(open(sys.argv[1]).read())"])
    .arg(dir.join("out.py"))
    .status();
  match parsed {
    Ok(status) => assert!(status.success(), "{text}"),
    Err(_) => {}
  }
}

#[test]
fn fence_token_is_16_hex_and_differs_by_identity() {
  let event = fence_token("6:5::model=Event");
  let venue = fence_token("6:5::model=Venue");
  assert_eq!(event.len(), 16);
  assert_eq!(venue.len(), 16);
  assert!(event.chars().all(|ch| ch.is_ascii_hexdigit()));
  assert_ne!(event, venue);
}

#[test]
fn migrates_legacy_begin_end_path_ids() {
  let dir = scratch("legacy");
  let identity = "6:5::model=Event";
  let token = fence_token(identity);
  let old = format!(
    "/*@dk:begin /abs/types.ts::{identity}@*/\nexport type Event\n/*@dk:end /abs/types.ts::{identity}@*/\n"
  );
  fs::write(dir.join("out.ts"), old).expect("seed");
  apply(
    &dir,
    &[("out.ts", vec![host(&token, "export type Event\n")])],
  );
  let text = read(&dir, "out.ts");
  assert_eq!(text, "export type Event\n");
  assert!(!text.contains("@dk"), "{text}");
}

#[test]
fn migrates_hashed_block_fences() {
  let dir = scratch("hashed");
  fs::write(dir.join("out.ts"), "/*@dk^one@*/\nalpha/*@dk$one@*/\n").expect("seed");
  apply(&dir, &[("out.ts", vec![host("one", "alpha")])]);
  assert_eq!(read(&dir, "out.ts"), "alpha");
}

#[test]
fn sequential_match_after_inserted_line() {
  let dir = scratch("insert-line");
  apply(
    &dir,
    &[("out.ts", vec![host("a", "alpha"), host("b", "beta")])],
  );
  fs::write(dir.join("out.ts"), "\nalphabeta").expect("edit");
  apply(
    &dir,
    &[("out.ts", vec![host("a", "ALPHA"), host("b", "beta")])],
  );
  assert_eq!(read(&dir, "out.ts"), "\nALPHAbeta");
}

#[test]
fn collision_identical_hosts_pair_in_order() {
  let dir = scratch("collision");
  apply(&dir, &[("out.ts", vec![host("a", "x"), host("b", "x")])]);
  apply(&dir, &[("out.ts", vec![host("a", "x"), host("b", "y")])]);
  assert_eq!(read(&dir, "out.ts"), "xy");
}

#[test]
fn nested_inner_update_keeps_sibling_literals() {
  let dir = scratch("nested");
  apply(
    &dir,
    &[(
      "out.ts",
      vec![frame(
        "f",
        vec![host("pre", "pre "), host("x", "old"), host("post", " post")],
      )],
    )],
  );
  assert_eq!(read(&dir, "out.ts"), "pre old post");
  apply(
    &dir,
    &[(
      "out.ts",
      vec![frame(
        "f",
        vec![host("pre", "pre "), host("x", "new"), host("post", " post")],
      )],
    )],
  );
  assert_eq!(read(&dir, "out.ts"), "pre new post");
}

#[test]
fn nested_unmarked_between_hosts_survives() {
  let dir = scratch("nested-unmarked");
  apply(
    &dir,
    &[(
      "out.ts",
      vec![frame(
        "f",
        vec![host("pre", "pre "), host("x", "old"), host("post", " post")],
      )],
    )],
  );
  fs::write(dir.join("out.ts"), "pre old /* wow */ post").expect("edit");
  apply(
    &dir,
    &[(
      "out.ts",
      vec![frame(
        "f",
        vec![host("pre", "pre "), host("x", "new"), host("post", " post")],
      )],
    )],
  );
  assert_eq!(read(&dir, "out.ts"), "pre new /* wow */ post");
}

#[test]
fn unmarked_inside_unmounted_frame_is_removed() {
  let dir = scratch("unmount-inner");
  apply(
    &dir,
    &[(
      "out.ts",
      vec![
        frame("keep", vec![host("a", "keep")]),
        frame("gone", vec![host("b", "go"), host("c", "ne")]),
      ],
    )],
  );
  fs::write(dir.join("out.ts"), "keepgo /* inside */ne").expect("edit");
  apply(
    &dir,
    &[("out.ts", vec![frame("keep", vec![host("a", "keep")])])],
  );
  let text = read(&dir, "out.ts");
  assert_eq!(text, "keep");
  assert!(!text.contains("inside"), "{text}");
}

#[test]
fn scoped_match_does_not_cross_frames() {
  let dir = scratch("scoped");
  apply(
    &dir,
    &[(
      "out.ts",
      vec![
        frame("left", vec![host("t", "x")]),
        frame("right", vec![host("t2", "x")]),
      ],
    )],
  );
  assert_eq!(read(&dir, "out.ts"), "xx");
  apply(
    &dir,
    &[(
      "out.ts",
      vec![
        frame("left", vec![host("t", "x")]),
        frame("right", vec![host("t2", "y")]),
      ],
    )],
  );
  assert_eq!(read(&dir, "out.ts"), "xy");
}

#[test]
fn subdir_cache_and_root_index() {
  let dir = scratch("subdir");
  let nested = dir.join("gen");
  fs::create_dir_all(&nested).expect("nested");
  let mut files = IndexMap::new();
  files.insert(
    nested.join("out.ts").to_str().expect("utf8").to_string(),
    vec![host("one", "alpha")],
  );
  commit(dir.to_str().expect("utf8"), &files).expect("commit");
  assert_eq!(
    fs::read_to_string(nested.join("out.ts")).expect("read"),
    "alpha"
  );
  assert!(nested.join(CACHE_FILE).exists());
  let root = read_cache(&dir.join(CACHE_FILE)).expect("root cache");
  assert!(
    root.dirs.iter().any(|item| item == "gen"),
    "{:?}",
    root.dirs
  );
}

#[test]
fn vanished_subdir_is_unmounted() {
  let dir = scratch("vanish-dir");
  let nested = dir.join("gen");
  fs::create_dir_all(&nested).expect("nested");
  let mut files = IndexMap::new();
  files.insert(
    nested.join("out.ts").to_str().expect("utf8").to_string(),
    vec![host("one", "alpha")],
  );
  commit(dir.to_str().expect("utf8"), &files).expect("first");
  commit(dir.to_str().expect("utf8"), &IndexMap::new()).expect("second");
  assert!(!nested.join("out.ts").exists());
  assert!(!nested.join(CACHE_FILE).exists());
}
