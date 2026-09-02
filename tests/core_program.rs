use std::cell::Cell;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::{fs, panic};

use datuma_k::core::format_datuma_tree;
use datuma_k::core::modes::ProgramParseMode;
use datuma_k::core::state::DatumaState;
use datuma_k::core::value::{CoreOperator, CoreValue};
use datuma_k::core::{ParseFile, ParseStack, parse_stack};

const SAMPLE_NAMES: &[&str] = &[
  "main",
  "literals_ops",
  "collections_ops",
  "control_flow",
  "calls_members",
  "ident_assigns",
  "mixed_script",
  "control_edges",
  "numeric_edges",
  "call_literal_edges",
  "grouped_exprs",
  "chain_and_merge",
  "control_more",
  "invoked_ops",
  "call_bare_edges",
  "string_num_more",
  "accessors",
  "jumps",
];

fn core_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/core")
}

fn sample_path(name: &str) -> PathBuf {
  core_dir().join(format!("{name}.dk"))
}

fn lock_path() -> PathBuf {
  core_dir().join(".lock.json")
}

/// Every runnable fixture: the top level plus `errors/`, but never `operators/`,
/// whose files are parse-level accept/reject cases rather than programs.
fn discover_fixtures() -> Vec<(String, PathBuf)> {
  let mut found: Vec<(String, PathBuf)> = [core_dir(), core_dir().join("errors")]
    .iter()
    .flat_map(|dir| {
      fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "dk"))
        .map(|path| {
          let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_else(|| panic!("bad fixture name {}", path.display()))
            .to_string();
          (name, path)
        })
        .collect::<Vec<_>>()
    })
    .collect();
  found.sort();
  for pair in found.windows(2) {
    assert_ne!(
      pair[0].0, pair[1].0,
      "duplicate fixture name across folders"
    );
  }
  assert!(!found.is_empty(), "no fixtures found");
  found
}

fn format_trace_char(ch: char) -> String {
  match ch {
    '\n' => "'\\n'".to_string(),
    '\r' => "'\\r'".to_string(),
    '\t' => "'\\t'".to_string(),
    ' ' => "' '".to_string(),
    _ => format!("{ch:?}"),
  }
}

async fn parse_traced(path: &Path) -> String {
  let mut file = ParseFile::open(path.to_str().unwrap())
    .await
    .unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
  let current = Rc::new(Cell::new('\0'));
  let lines = Rc::new(Cell::new(Vec::<String>::new()));
  let mut stack = ParseStack::with_root(Box::new(ProgramParseMode::new()));
  {
    let current_for_change = Rc::clone(&current);
    let lines = Rc::clone(&lines);
    stack.on_change(move |stack| {
      let mut buf = lines.take();
      buf.push(format!(
        "{} -> {}",
        format_trace_char(current_for_change.get()),
        stack.path()
      ));
      lines.set(buf);
    });
  }
  let current_for_input = Rc::clone(&current);
  let mut on_input = move |ch: char| current_for_input.set(ch);
  parse_stack(&mut stack, &mut file, Some(&mut on_input))
    .await
    .unwrap_or_else(|e| panic!("parse error in {}: {e}", path.display()));
  stack.dismiss_resolved();
  let state = stack
    .into_root()
    .into_datuma_state()
    .expect("program state");
  let mut report = String::from("# stack changes\n");
  for line in lines.take() {
    report.push_str(&line);
    report.push('\n');
  }
  report.push('\n');
  report.push_str("# tree\n");
  report.push_str(&format_datuma_tree(&state));
  report
}

async fn parse_fixture_path(path: &Path) -> DatumaState {
  let mut file = ParseFile::open(path.to_str().unwrap())
    .await
    .unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
  let mut stack = ParseStack::with_root(Box::new(ProgramParseMode::new()));
  parse_stack(&mut stack, &mut file, None)
    .await
    .unwrap_or_else(|e| panic!("parse error in {}: {e}", path.display()));
  stack.dismiss_resolved();
  stack
    .into_root()
    .into_datuma_state()
    .expect("program state")
}

/// One serial test owns every trace file and the lock: `cargo test` runs the
/// tests in a binary on parallel threads, so spreading the read-modify-write of
/// `.lock.json` across per-fixture tests would race.
#[tokio::test]
async fn traces_match_lock() {
  let lock_file = lock_path();
  let mut lock: BTreeMap<String, bool> = match fs::read_to_string(&lock_file) {
    Ok(text) => {
      serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", lock_file.display()))
    }
    Err(_) => BTreeMap::new(),
  };
  let dir = core_dir().join(".output");
  fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("mkdir {}: {e}", dir.display()));

  let mut changed = Vec::new();
  for (name, source) in discover_fixtures() {
    let report = parse_traced(&source).await;
    let key = format!("{name}.trace");
    let trace = dir.join(&key);
    if lock.get(&key).copied().unwrap_or(false) {
      let expected = fs::read_to_string(&trace).unwrap_or_else(|e| {
        panic!(
          "{key} is locked but {} is missing ({e}); set it false to regenerate",
          trace.display()
        )
      });
      if report != expected {
        let actual = dir.join(format!("{key}.actual"));
        fs::write(&actual, &report).unwrap_or_else(|e| panic!("write {}: {e}", actual.display()));
        changed.push(key);
      }
    } else {
      fs::write(&trace, &report).unwrap_or_else(|e| panic!("write {}: {e}", trace.display()));
      lock.insert(key, false);
    }
  }

  let mut encoded = serde_json::to_string_pretty(&lock).expect("encode lock");
  encoded.push('\n');
  fs::write(&lock_file, encoded).unwrap_or_else(|e| panic!("write {}: {e}", lock_file.display()));
  assert!(
    changed.is_empty(),
    "locked traces changed: {changed:?}; compare the .actual files, then either fix the parser or set those keys false to accept"
  );
}

fn value_kind(state: &DatumaState) -> &str {
  state.value.as_ref().expect("value").kind()
}

fn value_kind_opt(state: &DatumaState) -> Option<&str> {
  state.value.as_ref().map(|v| v.kind())
}

async fn run_sample(name: &str) -> DatumaState {
  parse_fixture_path(&sample_path(name)).await
}

fn collect_kinds(state: &DatumaState, kinds: &mut HashSet<&'static str>) {
  if let Some(value) = &state.value {
    kinds.insert(value.kind());
  }
  for child in &state.children {
    collect_kinds(child, kinds);
  }
}

fn collect_operators(state: &DatumaState, ops: &mut HashSet<CoreOperator>) {
  if let Some(value) = &state.value {
    if let Some(core) = value.as_any().downcast_ref::<CoreValue>() {
      if let CoreValue::Operator(op) = core {
        ops.insert(*op);
      }
    }
  }
  for child in &state.children {
    collect_operators(child, ops);
  }
}

fn count_kind(state: &DatumaState, kind: &str) -> usize {
  let mut n = if state.value.as_ref().is_some_and(|v| v.kind() == kind) {
    1
  } else {
    0
  };
  for child in &state.children {
    n += count_kind(child, kind);
  }
  n
}

fn find_kind<'a>(state: &'a DatumaState, kind: &str) -> Option<&'a DatumaState> {
  if state.value.as_ref().is_some_and(|v| v.kind() == kind) {
    Some(state)
  } else {
    state.children.iter().find_map(|c| find_kind(c, kind))
  }
}

fn collect_kinds_nodes<'a>(state: &'a DatumaState, kind: &str) -> Vec<&'a DatumaState> {
  let mut out = Vec::new();
  collect_kinds_nodes_rec(state, kind, &mut out);
  out
}

fn collect_kinds_nodes_rec<'a>(state: &'a DatumaState, kind: &str, out: &mut Vec<&'a DatumaState>) {
  if state.value.as_ref().is_some_and(|v| v.kind() == kind) {
    out.push(state);
  }
  for child in &state.children {
    collect_kinds_nodes_rec(child, kind, out);
  }
}

fn find_invoked<'a>(state: &'a DatumaState, name: &str) -> Option<&'a DatumaState> {
  if state
    .value
    .as_ref()
    .and_then(|v| v.as_any().downcast_ref::<CoreValue>())
    .is_some_and(|v| matches!(v, CoreValue::InvokedFunction(n) if n == name))
  {
    Some(state)
  } else {
    state.children.iter().find_map(|c| find_invoked(c, name))
  }
}

fn find_core_boolean(state: &DatumaState, expected: bool) -> Option<bool> {
  if state
    .value
    .as_ref()
    .and_then(|v| v.as_any().downcast_ref::<CoreValue>())
    .is_some_and(|v| matches!(v, CoreValue::Boolean(b) if *b == expected))
  {
    Some(expected)
  } else {
    state
      .children
      .iter()
      .find_map(|c| find_core_boolean(c, expected))
  }
}

fn count_operator(state: &DatumaState, op: CoreOperator) -> usize {
  let mut n = if state
    .value
    .as_ref()
    .and_then(|v| v.as_any().downcast_ref::<CoreValue>())
    .is_some_and(|v| matches!(v, CoreValue::Operator(o) if *o == op))
  {
    1
  } else {
    0
  };
  for child in &state.children {
    n += count_operator(child, op);
  }
  n
}

#[test]
fn all_core_samples_have_sources() {
  for name in SAMPLE_NAMES {
    assert!(
      Path::new(&sample_path(name)).is_file(),
      "missing sample {name}"
    );
  }
}

#[tokio::test]
async fn main_program_shape() {
  let program = run_sample("main").await;
  assert_eq!(value_kind(&program), "program");
  assert_eq!(count_kind(&program, "function_def"), 2);
  assert_eq!(count_kind(&program, "if"), 1);
  assert_eq!(count_kind(&program, "else"), 0);
  assert!(collect_kinds_nodes(&program, "if").iter().any(|node| {
    node.children.iter().skip(1).any(|child| {
      child
        .value
        .as_ref()
        .is_some_and(|value| value.kind() != "program" && value.kind() != "else")
    })
  }));
  assert!(count_kind(&program, "grouped") >= 1);
  assert!(count_kind(&program, "invoked_function") >= 3);
  let abs = program
    .children
    .iter()
    .find(|c| {
      c.value
        .as_ref()
        .and_then(|v| v.as_any().downcast_ref::<CoreValue>())
        .is_some_and(|v| matches!(v, CoreValue::FunctionDef(name) if name == "abs"))
    })
    .expect("abs fn");
  assert!(find_kind(abs, "if").is_some());
}

#[tokio::test]
async fn literals_ops_shape() {
  let program = run_sample("literals_ops").await;
  for kind in ["integer", "float", "double", "string", "boolean", "null"] {
    assert!(count_kind(&program, kind) >= 1, "missing kind {kind}");
  }
  let mut ops = HashSet::new();
  collect_operators(&program, &mut ops);
  for op in [
    CoreOperator::Add,
    CoreOperator::Mul,
    CoreOperator::Div,
    CoreOperator::Mod,
    CoreOperator::Pow,
    CoreOperator::LessEqual,
    CoreOperator::NotEqual,
    CoreOperator::Equal,
    CoreOperator::And,
    CoreOperator::Or,
    CoreOperator::Xor,
  ] {
    assert!(ops.contains(&op), "missing operator {op:?}");
  }
}

#[tokio::test]
async fn collections_ops_shape() {
  let program = run_sample("collections_ops").await;
  assert!(count_kind(&program, "array") >= 1);
  assert!(count_kind(&program, "dict") >= 1);
  let mut ops = HashSet::new();
  collect_operators(&program, &mut ops);
  for op in [
    CoreOperator::Add,
    CoreOperator::Sub,
    CoreOperator::SymmetricDiff,
    CoreOperator::Intersect,
    CoreOperator::LeftDiff,
    CoreOperator::RightDiff,
  ] {
    assert!(ops.contains(&op), "missing collection operator {op:?}");
  }
}

#[tokio::test]
async fn control_flow_shape() {
  let program = run_sample("control_flow").await;
  assert_eq!(count_kind(&program, "function_def"), 1);
  assert_eq!(count_kind(&program, "for"), 3);
  let if_node = find_kind(&program, "if").expect("if");
  assert!(
    if_node.children.iter().any(|c| value_kind(c) == "elseif"),
    "elseif under if"
  );
  let elseif = if_node
    .children
    .iter()
    .find(|c| value_kind(c) == "elseif")
    .expect("elseif");
  assert!(
    elseif.children.iter().any(|c| value_kind(c) == "else"),
    "else under elseif"
  );
  assert!(count_kind(&program, "invoked_function") >= 1);
  assert!(count_kind(&program, "array") >= 1);
}

#[tokio::test]
async fn calls_members_shape() {
  let program = run_sample("calls_members").await;
  assert_eq!(count_kind(&program, "function_def"), 2);
  assert!(count_kind(&program, "invoked_function") >= 4);
  assert!(count_kind(&program, "grouped") >= 2);
  let mut ops = HashSet::new();
  collect_operators(&program, &mut ops);
  assert!(ops.contains(&CoreOperator::Add));
  assert!(ops.contains(&CoreOperator::Equal));
  assert!(ops.contains(&CoreOperator::Dot));
}

#[tokio::test]
async fn ident_assigns_shape() {
  let program = run_sample("ident_assigns").await;
  let mut ops = HashSet::new();
  collect_operators(&program, &mut ops);
  for op in [
    CoreOperator::Assign,
    CoreOperator::AddAssign,
    CoreOperator::SubAssign,
    CoreOperator::MulAssign,
    CoreOperator::DivAssign,
    CoreOperator::ModAssign,
    CoreOperator::PowAssign,
    CoreOperator::AndAssign,
    CoreOperator::OrAssign,
    CoreOperator::XorAssign,
    CoreOperator::AndAndAssign,
    CoreOperator::OrOrAssign,
    CoreOperator::RightDiffAssign,
    CoreOperator::LeftDiffAssign,
  ] {
    assert!(ops.contains(&op), "missing assign operator {op:?}");
  }
}

#[tokio::test]
async fn mixed_script_shape() {
  let program = run_sample("mixed_script").await;
  assert_eq!(count_kind(&program, "function_def"), 2);
  assert!(count_kind(&program, "dict") >= 1);
  assert!(count_kind(&program, "array") >= 1);
  assert!(count_kind(&program, "for") >= 1);
  assert!(count_kind(&program, "if") >= 1);
  assert!(count_kind(&program, "string") >= 1);
  assert!(count_kind(&program, "double") >= 1 || count_kind(&program, "float") >= 1);
  assert!(count_kind(&program, "invoked_function") >= 2);
}

#[tokio::test]
async fn control_edges_shape() {
  let program = run_sample("control_edges").await;
  assert!(count_kind(&program, "if") >= 2);
  assert!(count_kind(&program, "for") >= 3);
  assert!(count_kind(&program, "elseif") >= 1);
  let mut saw_bare = false;
  let mut saw_elseif_no_else = false;
  for if_node in collect_kinds_nodes(&program, "if") {
    let has_elseif = if_node.children.iter().any(|c| value_kind(c) == "elseif");
    let has_else = if_node.children.iter().any(|c| value_kind(c) == "else");
    if !has_elseif && !has_else {
      saw_bare = true;
    } else if has_elseif {
      let elseif = if_node
        .children
        .iter()
        .find(|c| value_kind(c) == "elseif")
        .expect("elseif");
      if !elseif.children.iter().any(|c| value_kind(c) == "else") {
        saw_elseif_no_else = true;
      }
    }
  }
  assert!(saw_bare, "expected an if without else/elseif");
  assert!(
    saw_elseif_no_else,
    "expected elseif chain without final else"
  );
  let tick = find_invoked(&program, "tick").expect("tick call");
  assert!(tick.children.iter().any(|c| {
    c.value
      .as_ref()
      .and_then(|v| v.as_any().downcast_ref::<CoreValue>())
      .is_some_and(|v| matches!(v, CoreValue::Boolean(true)))
  }));
}

#[tokio::test]
async fn numeric_edges_shape() {
  let program = run_sample("numeric_edges").await;
  assert!(count_kind(&program, "float") >= 1);
  assert!(count_kind(&program, "double") >= 1);
  let mut ops = HashSet::new();
  collect_operators(&program, &mut ops);
  for op in [
    CoreOperator::Increment,
    CoreOperator::Decrement,
    CoreOperator::Gt,
    CoreOperator::GreaterEqual,
    CoreOperator::BitAnd,
    CoreOperator::BitOr,
    CoreOperator::Xor,
  ] {
    assert!(ops.contains(&op), "missing numeric edge operator {op:?}");
  }
}

#[tokio::test]
async fn call_literal_edges_shape() {
  let program = run_sample("call_literal_edges").await;
  let blank = program
    .children
    .iter()
    .find(|c| {
      c.value
        .as_ref()
        .and_then(|v| v.as_any().downcast_ref::<CoreValue>())
        .is_some_and(|v| matches!(v, CoreValue::FunctionDef(name) if name == "blank"))
    })
    .expect("blank fn");
  assert!(
    blank
      .children
      .first()
      .is_some_and(|params| params.value.is_none() && params.children.is_empty()),
    "blank has empty params"
  );
  assert!(count_kind(&program, "invoked_function") >= 4);
  assert!(count_kind(&program, "dict") >= 1);
  assert!(count_kind(&program, "string") >= 1);
  assert!(count_kind(&program, "boolean") >= 1);
  assert!(count_kind(&program, "null") >= 1);
  let true_lit = find_core_boolean(&program, true).expect("TRUE literal");
  assert!(true_lit);
  assert!(
    find_core_boolean(&program, false).is_some(),
    "FALSE literal"
  );
}

#[tokio::test]
async fn grouped_exprs_shape() {
  let program = run_sample("grouped_exprs").await;
  assert!(count_kind(&program, "grouped") >= 6);
  assert!(
    collect_kinds_nodes(&program, "grouped")
      .iter()
      .any(|g| g.children.is_empty()),
    "expected empty grouped ()"
  );
  let mut kinds_in_groups = HashSet::new();
  for group in collect_kinds_nodes(&program, "grouped") {
    for child in &group.children {
      if let Some(value) = &child.value {
        kinds_in_groups.insert(value.kind());
      }
    }
  }
  for kind in ["array", "dict", "string", "boolean"] {
    assert!(
      kinds_in_groups.contains(kind),
      "grouped missing interior {kind}"
    );
  }
}

#[tokio::test]
async fn chain_and_merge_shape() {
  let program = run_sample("chain_and_merge").await;
  assert!(
    collect_kinds_nodes(&program, "instruction")
      .iter()
      .any(|ins| count_operator(ins, CoreOperator::Dot) >= 2),
    "expected a.b.c style chained dots"
  );
  assert!(
    collect_kinds_nodes(&program, "instruction")
      .iter()
      .any(|ins| count_operator(ins, CoreOperator::Add) >= 2),
    "expected chained array/dict merge Adds"
  );
  let mut ops = HashSet::new();
  collect_operators(&program, &mut ops);
  assert!(ops.contains(&CoreOperator::Sub), "dict-dict subtract");
  assert!(count_kind(&program, "string") >= 1);
  assert!(count_kind(&program, "array") >= 1);
  assert!(count_kind(&program, "dict") >= 1);
}

#[tokio::test]
async fn control_more_shape() {
  let program = run_sample("control_more").await;
  assert!(count_kind(&program, "for") >= 1);
  assert!(count_kind(&program, "invoked_function") >= 1);
  assert!(
    collect_kinds_nodes(&program, "if")
      .iter()
      .any(|if_node| { count_kind(if_node, "elseif") >= 2 }),
    "expected multi-elseif chain"
  );
  assert!(count_kind(&program, "float") >= 1);
  assert!(count_kind(&program, "double") >= 1);
  let mut ops = HashSet::new();
  collect_operators(&program, &mut ops);
  assert!(ops.contains(&CoreOperator::Dot), "string member dot");
  assert!(ops.contains(&CoreOperator::Add), "float infix add");
}

#[tokio::test]
async fn invoked_ops_shape() {
  let program = run_sample("invoked_ops").await;
  assert!(count_kind(&program, "invoked_function") >= 1);
  let mut ops = HashSet::new();
  collect_operators(&program, &mut ops);
  for op in [
    CoreOperator::Sub,
    CoreOperator::Mul,
    CoreOperator::Div,
    CoreOperator::Mod,
    CoreOperator::Pow,
    CoreOperator::Xor,
    CoreOperator::BitAnd,
    CoreOperator::BitOr,
    CoreOperator::And,
    CoreOperator::Or,
    CoreOperator::NotEqual,
    CoreOperator::Lt,
    CoreOperator::LessEqual,
    CoreOperator::Gt,
    CoreOperator::GreaterEqual,
  ] {
    assert!(ops.contains(&op), "missing invoked op {op:?}");
  }
}

#[tokio::test]
async fn call_bare_edges_shape() {
  let program = run_sample("call_bare_edges").await;
  let calls = collect_kinds_nodes(&program, "invoked_function");
  assert!(
    calls.iter().any(|c| c.children.iter().any(|ch| {
      matches!(value_kind_opt(ch), Some("float" | "double")) || count_kind(ch, "float") >= 1
    })),
    "float call arg"
  );
  assert!(
    calls.iter().any(|c| c
      .children
      .iter()
      .any(|ch| value_kind_opt(ch) == Some("array"))),
    "array call arg"
  );
  assert!(
    calls.iter().any(|c| c
      .children
      .iter()
      .any(|ch| value_kind_opt(ch) == Some("ident"))),
    "ident call arg"
  );
  assert!(
    collect_kinds_nodes(&program, "instruction")
      .iter()
      .any(|ins| {
        let Some(first) = ins.children.first() else {
          return false;
        };
        matches!(
          value_kind_opt(first),
          Some("boolean" | "null" | "array" | "dict")
        ) && count_operator(ins, CoreOperator::Assign) == 0
      }),
    "expected bare boolean/null/array/dict instruction"
  );
}

#[tokio::test]
async fn string_num_more_shape() {
  let program = run_sample("string_num_more").await;
  assert!(count_kind(&program, "double") >= 1);
  assert!(
    collect_kinds_nodes(&program, "string").iter().any(|s| {
      s.value
        .as_ref()
        .and_then(|v| v.as_any().downcast_ref::<CoreValue>())
        .is_some_and(|v| match v {
          CoreValue::String(text) => text.contains('\0') || text.contains('\u{7}'),
          _ => false,
        })
    }),
    "expected decoded escape in string"
  );
  let mut ops = HashSet::new();
  collect_operators(&program, &mut ops);
  assert!(ops.contains(&CoreOperator::SymmetricDiff));
  assert!(ops.contains(&CoreOperator::Intersect));
  assert!(ops.contains(&CoreOperator::Dot));
  assert!(ops.contains(&CoreOperator::Increment));
}

#[tokio::test]
async fn accessors_shape() {
  let program = run_sample("accessors").await;
  assert!(count_kind(&program, "accessor") >= 6);
  assert!(count_kind(&program, "array") >= 1);
  assert!(count_kind(&program, "invoked_function") >= 1);
  assert!(count_kind(&program, "grouped") >= 1);
  assert!(count_kind(&program, "string") >= 2);
}

#[tokio::test]
async fn jumps_shape() {
  let program = run_sample("jumps").await;
  assert!(count_kind(&program, "return") >= 3);
  assert!(count_kind(&program, "break") >= 1);
  assert!(count_kind(&program, "if") >= 2);
  assert_eq!(count_kind(&program, "yield"), 0);
  assert_eq!(count_kind(&program, "ternary"), 0);
  assert!(collect_kinds_nodes(&program, "if").iter().any(|node| {
    node.children.iter().skip(1).any(|child| {
      child
        .value
        .as_ref()
        .is_some_and(|value| value.kind() != "program" && value.kind() != "else")
    })
  }));
}

#[tokio::test]
async fn all_samples_cover_core_modes() {
  let mut kinds = HashSet::new();
  let mut ops = HashSet::new();
  for name in SAMPLE_NAMES {
    let state = run_sample(name).await;
    collect_kinds(&state, &mut kinds);
    collect_operators(&state, &mut ops);
  }
  for kind in [
    "program",
    "instruction",
    "function_def",
    "ident",
    "integer",
    "float",
    "double",
    "string",
    "boolean",
    "null",
    "array",
    "dict",
    "grouped",
    "invoked_function",
    "operator",
    "if",
    "else",
    "elseif",
    "for",
    "accessor",
    "return",
    "break",
  ] {
    assert!(
      kinds.contains(kind),
      "coverage missing kind {kind}; have {kinds:?}"
    );
  }
  for op in [
    CoreOperator::Assign,
    CoreOperator::Add,
    CoreOperator::Mul,
    CoreOperator::Pow,
    CoreOperator::And,
    CoreOperator::Or,
    CoreOperator::Equal,
    CoreOperator::NotEqual,
    CoreOperator::LessEqual,
    CoreOperator::Increment,
    CoreOperator::Decrement,
    CoreOperator::Gt,
    CoreOperator::GreaterEqual,
    CoreOperator::BitAnd,
    CoreOperator::BitOr,
    CoreOperator::Intersect,
    CoreOperator::LeftDiff,
    CoreOperator::SymmetricDiff,
    CoreOperator::AddAssign,
    CoreOperator::Dot,
  ] {
    assert!(
      ops.contains(&op),
      "coverage missing operator {op:?}; have {ops:?}"
    );
  }
}

#[tokio::test]
async fn if_allows_statement_after_without_else() {
  parse_fixture_path(&core_dir().join("if_stmt_after_without_else.dk")).await;
}

#[tokio::test]
async fn if_partial_else_is_ident_statement() {
  parse_fixture_path(&core_dir().join("if_partial_else_ident.dk")).await;
}
