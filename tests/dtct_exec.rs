use std::fs;
use std::path::PathBuf;

use datuma_k::dtct::{load_dtct_dir, parse_file};

fn dtct_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/dtct")
}

fn discover_fixtures() -> Vec<(String, PathBuf)> {
  let dir = dtct_dir();
  let mut found: Vec<(String, PathBuf)> = fs::read_dir(&dir)
    .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
    .filter_map(|entry| entry.ok().map(|entry| entry.path()))
    .filter(|path| path.extension().is_some_and(|ext| ext == "dtct"))
    .map(|path| {
      let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_else(|| panic!("unusable fixture name {}", path.display()))
        .to_string();
      (name, path)
    })
    .collect();
  found.sort();
  assert!(!found.is_empty(), "no fixtures found");
  found
}

/// Snapshots are debugging aids, not assertions, so they always regenerate.
/// One serial test owns the directory to avoid concurrent writes.
#[tokio::test(flavor = "multi_thread")]
async fn fact_vector_dumps() {
  let dir = dtct_dir().join(".output");
  fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("mkdir {}: {e}", dir.display()));

  let mut failures = Vec::new();
  for (name, path) in discover_fixtures() {
    match parse_file(&path).await {
      Ok(db) => {
        let out = dir.join(format!("{name}.dump"));
        fs::write(&out, db.dump_string())
          .unwrap_or_else(|e| panic!("write {}: {e}", out.display()));
      }
      Err(err) => failures.push(format!("{name}: {err}")),
    }
  }
  match load_dtct_dir(&dtct_dir()).await {
    Ok(db) => {
      let out = dir.join("dir.dump");
      fs::write(&out, db.dump_string()).unwrap_or_else(|e| panic!("write {}: {e}", out.display()));
    }
    Err(err) => failures.push(format!("dir: {err}")),
  }
  assert!(
    failures.is_empty(),
    "fixtures failed to parse: {failures:#?}"
  );
}

/// Every fixture is meant to parse, so any error is a defect in the fixture or the parser.
#[tokio::test]
async fn fixtures_parse_cleanly() {
  let mut failures = Vec::new();
  for (name, path) in discover_fixtures() {
    if let Err(err) = parse_file(&path).await {
      failures.push(format!("{name}: {err}"));
    }
  }
  assert!(
    failures.is_empty(),
    "fixtures failed to parse: {failures:#?}"
  );
}
