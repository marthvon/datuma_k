use std::path::{Path, PathBuf};

use crate::core::common::files_from_dir;
use crate::core::modes::ProgramParseMode;
use crate::core::parser::{
  ParseCursorMetadata, ParseError, ParseErrorKind, ParseErrorSource, ParseFile, ParseFileMetadata,
  ParseMode, ParseStack, expected, messages, parse_stack,
};
use crate::core::state::DatumaState;
use crate::ngin::modes::NginRootParseMode;

pub async fn parse_file(path: &Path) -> Result<DatumaState, ParseError> {
  parse_tree(path).await
}

pub async fn parse_tree(path: &Path) -> Result<DatumaState, ParseError> {
  parse_with_root(
    path,
    Box::new(NginRootParseMode::new()),
    messages::NGIN_COMPLETE,
  )
  .await
}

pub async fn load_def_ngin(dir: &Path) -> Result<Vec<DatumaState>, ParseError> {
  if !dir.exists() {
    Ok(Vec::new())
  } else if !dir.is_dir() {
    Err(io_parse_error(format!(
      "read {}: not a directory",
      dir.display()
    )))
  } else {
    load_def_ngin_paths(
      files_from_dir(dir.to_str().unwrap_or_default(), &["ngin"])
        .into_iter()
        .filter(|path| {
          path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".def.ngin"))
        })
        .collect(),
    )
    .await
  }
}

pub async fn load_def_ngin_paths(paths: Vec<PathBuf>) -> Result<Vec<DatumaState>, ParseError> {
  let mut trees = Vec::new();
  for path in paths {
    trees.push(
      parse_with_root(
        &path,
        Box::new(ProgramParseMode::new()),
        messages::COMPLETE_INPUT,
      )
      .await?,
    );
  }
  Ok(trees)
}

pub async fn load_ngin_dir(dir: &Path) -> Result<Vec<(PathBuf, DatumaState)>, ParseError> {
  if !dir.exists() {
    Ok(Vec::new())
  } else if !dir.is_dir() {
    Err(io_parse_error(format!(
      "read {}: not a directory",
      dir.display()
    )))
  } else {
    load_ngin_paths(
      files_from_dir(dir.to_str().unwrap_or_default(), &["ngin"])
        .into_iter()
        .filter(|path| {
          path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| !name.ends_with(".def.ngin"))
        })
        .collect(),
    )
    .await
  }
}

pub async fn load_ngin_paths(
  paths: Vec<PathBuf>,
) -> Result<Vec<(PathBuf, DatumaState)>, ParseError> {
  let mut trees = Vec::new();
  for path in paths {
    let tree = parse_tree(&path).await?;
    trees.push((path, tree));
  }
  Ok(trees)
}

async fn parse_with_root(
  path: &Path,
  root: Box<dyn ParseMode>,
  complete: &'static str,
) -> Result<DatumaState, ParseError> {
  let mut stack = ParseStack::with_root(root);
  let mut file = open_file(path).await?;
  run_parse_stack(&mut stack, &mut file).await?;
  finish_tree(stack, &file, complete)
}

async fn open_file(path: &Path) -> Result<ParseFile, ParseError> {
  ParseFile::open(path.to_str().unwrap_or_default())
    .await
    .map_err(|err| io_parse_error(format!("read {}: {err}", path.display())))
}

#[inline(always)]
async fn run_parse_stack(stack: &mut ParseStack, file: &mut ParseFile) -> Result<(), ParseError> {
  #[cfg(feature = "parse-trace")]
  return parse_stack(stack, file, None).await;
  #[cfg(not(feature = "parse-trace"))]
  parse_stack(stack, file).await
}

fn finish_tree(
  stack: ParseStack,
  source: &dyn ParseErrorSource,
  complete: &'static str,
) -> Result<DatumaState, ParseError> {
  let mut stack = stack;
  stack.dismiss_resolved();
  if stack.has_active_frames() {
    Err(source.error(expected(complete)))
  } else {
    let root = stack.into_root();
    root
      .into_datuma_state()
      .ok_or_else(|| source.error(expected(complete)))
  }
}

fn io_parse_error(message: String) -> ParseError {
  ParseError {
    file_meta: ParseFileMetadata::source(""),
    pos_meta: ParseCursorMetadata::default(),
    curr_mode: None,
    kind: ParseErrorKind::IoError(Box::new(std::io::Error::other(message))),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::state::DatumaState;
  use crate::core::state_fmt::format_datuma_tree;
  use crate::ngin::value::NginValue;
  use std::path::PathBuf;

  fn ngin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/ngin")
  }

  fn fixture(name: &str) -> PathBuf {
    ngin_dir().join(name)
  }

  fn ngin_kind(state: &DatumaState) -> Option<&NginValue> {
    state
      .value
      .as_ref()
      .and_then(|value| value.as_any().downcast_ref::<NginValue>())
  }

  fn contains_kind(state: &DatumaState, want: fn(&NginValue) -> bool) -> bool {
    ngin_kind(state).is_some_and(want)
      || state
        .children
        .iter()
        .any(|child| contains_kind(child, want))
  }

  #[tokio::test]
  async fn parses_sample_fixture() {
    let state = parse_file(&fixture("sample.ngin"))
      .await
      .unwrap_or_else(|err| panic!("{err}"));
    let dump = format_datuma_tree(&state);
    assert!(
      contains_kind(&state, |v| matches!(v, NginValue::File)),
      "missing file\n{dump}"
    );
    assert!(
      contains_kind(
        &state,
        |v| matches!(v, NginValue::Env { name } if name == "ROOT_DIRECTORY")
      ),
      "missing env\n{dump}"
    );
    assert!(
      contains_kind(&state, |v| matches!(v, NginValue::Emit { .. })),
      "missing emit\n{dump}"
    );
    assert!(
      contains_kind(&state, |v| matches!(v, NginValue::Plus { .. })),
      "missing plus\n{dump}"
    );
    assert!(
      contains_kind(&state, |v| matches!(v, NginValue::Guard { .. })),
      "missing guard\n{dump}"
    );
    assert!(
      contains_kind(&state, |v| matches!(v, NginValue::Template { .. })),
      "missing template\n{dump}"
    );
  }

  #[tokio::test]
  async fn nested_pipe_in_bound_template_is_error() {
    let err = parse_file(&fixture("nested_pipe.ngin"))
      .await
      .expect_err("nested | should fail");
    let text = err.to_string();
    assert!(
      text.contains("single |path> per file-root") || text.contains("Unexpected"),
      "{text}"
    );
  }

  #[tokio::test]
  async fn emit_before_file_is_error() {
    parse_file(&fixture("emit_before_file.ngin"))
      .await
      .expect_err("=> before | should fail");
  }

  fn contains_fn(state: &DatumaState, name: &str) -> bool {
    matches!(
      state
        .value
        .as_ref()
        .and_then(|value| value.as_any().downcast_ref::<crate::core::value::CoreValue>()),
      Some(crate::core::value::CoreValue::FunctionDef(def)) if def == name
    ) || state.children.iter().any(|child| contains_fn(child, name))
  }

  #[tokio::test]
  async fn load_def_ngin_walks_tree_and_skips_plain_ngin() {
    let trees = load_def_ngin(&ngin_dir())
      .await
      .unwrap_or_else(|err| panic!("{err}"));
    assert_eq!(trees.len(), 2, "expected two *.def.ngin files");
    assert!(
      trees.iter().any(|tree| contains_fn(tree, "ident")),
      "missing ident"
    );
    assert!(
      trees.iter().any(|tree| contains_fn(tree, "nested_tag")),
      "missing nested_tag"
    );
    assert!(
      trees.iter().any(|tree| contains_fn(tree, "title_case")),
      "missing title_case"
    );
  }

  #[tokio::test]
  async fn load_ngin_dir_skips_def_and_walks() {
    let dir = std::env::temp_dir().join(format!(
      "ngin-load-{}",
      std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos()
    ));
    std::fs::create_dir_all(dir.join("sub")).expect("dir");
    std::fs::write(
      dir.join("hello.ngin"),
      "|$ROOT_DIRECTORY/out.txt>\n```\nok\n```\n",
    )
    .expect("write ngin");
    std::fs::write(dir.join("skip.def.ngin"), "fn x() { return 1; }\n").expect("write def");
    std::fs::write(
      dir.join("sub").join("nested.ngin"),
      "|$ROOT_DIRECTORY/nested.txt>\n```\nn\n```\n",
    )
    .expect("write nested");
    let trees = load_ngin_dir(&dir)
      .await
      .unwrap_or_else(|err| panic!("{err}"));
    assert_eq!(trees.len(), 2, "expected two templates, got {trees:?}");
  }

  #[tokio::test]
  async fn load_def_ngin_empty_dir() {
    let dir = std::env::temp_dir().join(format!(
      "ngin-def-empty-{}",
      std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("dir");
    let trees = load_def_ngin(&dir)
      .await
      .unwrap_or_else(|err| panic!("{err}"));
    assert!(trees.is_empty());
  }
}
