//! Shared helpers for the core test binaries. Not every binary uses every item.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use datuma_k::core::modes::ProgramParseMode;
use datuma_k::core::state::DatumaState;
use datuma_k::core::{ParseFile, ParseStack, parse_stack};

pub fn core_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/core")
}

pub fn errors_dir() -> PathBuf {
  core_dir().join("errors")
}

/// Every runnable fixture: the top level plus `errors/`, but never `operators/`,
/// whose files are parse-level accept/reject cases rather than programs.
/// Dot-directories such as `.output` are skipped by the extension filter.
pub fn discover_fixtures() -> Vec<(String, PathBuf)> {
  let mut found = fixtures_in(&core_dir());
  found.extend(fixtures_in(&errors_dir()));
  found.sort();
  for pair in found.windows(2) {
    assert_ne!(
      pair[0].0, pair[1].0,
      "duplicate fixture name across core/ and errors/"
    );
  }
  assert!(!found.is_empty(), "no fixtures found");
  found
}

pub fn fixtures_in(dir: &Path) -> Vec<(String, PathBuf)> {
  let mut found = fs::read_dir(dir)
    .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
    .filter_map(|entry| entry.ok().map(|entry| entry.path()))
    .filter(|path| path.extension().is_some_and(|ext| ext == "dk"))
    .map(|path| {
      let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_else(|| panic!("unusable fixture name {}", path.display()))
        .to_string();
      (name, path)
    })
    .collect::<Vec<_>>();
  found.sort();
  found
}

pub async fn parse_program(path: &Path) -> DatumaState {
  let mut file = ParseFile::open(path.to_str().unwrap())
    .await
    .unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
  let mut stack = ParseStack::with_root(Box::new(ProgramParseMode::new()));
  parse_stack(&mut stack, &mut file)
    .await
    .unwrap_or_else(|e| panic!("parse error in {}: {e}", path.display()));
  stack.dismiss_resolved();
  stack
    .into_root()
    .into_datuma_state()
    .expect("program state")
}
