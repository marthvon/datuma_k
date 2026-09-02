use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::core::common::is_dangerous_dir;
use crate::dkcache::commit;
use crate::dtct::Dim;
use crate::project::{
  KEYWORDS_STUB, catalog_project, check_project, failure_result, keywords_path, load_project,
  plan_project, preview_project,
};

const CASES_DEF: &str = include_str!("../tests/ngin/defs/helpers.def.ngin");
const ENV_FILE: &str = "\
ROOT_DIRECTORY=.
DTCT_DIRECTORY=data
NGIN_DIRECTORY=engine
DEF_DIRECTORY=definition
";
const USAGE: &str = "usage: datuma_k <check | catalog | preview | run | start <project-name>>";
const CATALOG_USAGE: &str = "usage: datuma_k catalog [--trait NAME] [--model NAME] [--field NAME] [--attribute NAME] [--type NAME]";

#[derive(Debug)]
pub struct CliError {
  message: String,
  reported: bool,
}

impl CliError {
  fn new(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
      reported: false,
    }
  }

  fn reported() -> Self {
    Self {
      message: String::new(),
      reported: true,
    }
  }

  pub fn is_reported(&self) -> bool {
    self.reported
  }
}

impl std::fmt::Display for CliError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.message)
  }
}

impl std::error::Error for CliError {}

pub async fn dispatch(args: impl IntoIterator<Item = String>) -> Result<(), CliError> {
  let mut args = args.into_iter();
  args.next();
  match args.next().as_deref() {
    Some("run") => {
      if args.next().is_some() {
        Err(CliError::new("usage: datuma_k run"))
      } else {
        let cwd = current_dir()?;
        confirm_walk(&cwd)?;
        run_project(&cwd).await
      }
    }
    Some("check") => {
      if args.next().is_some() {
        Err(CliError::new("usage: datuma_k check"))
      } else {
        let cwd = current_dir()?;
        confirm_walk(&cwd)?;
        json_check(check_project(&cwd).await)
      }
    }
    Some("preview") => {
      if args.next().is_some() {
        Err(CliError::new("usage: datuma_k preview"))
      } else {
        let cwd = current_dir()?;
        confirm_walk(&cwd)?;
        match preview_project(&cwd).await {
          Ok(result) => print_json(&result),
          Err(err) => json_err(&err),
        }
      }
    }
    Some("catalog") => match catalog_filters(args) {
      Err(err) => Err(err),
      Ok(filters) => {
        let cwd = current_dir()?;
        confirm_walk(&cwd)?;
        match catalog_project(&cwd, &filters).await {
          Ok(result) => print_json(&result),
          Err(err) => json_err(&err),
        }
      }
    },
    Some("start") => match args.next() {
      Some(name) if args.next().is_none() => {
        let cwd = current_dir()?;
        start_project(&cwd, &name).map(|_| ())
      }
      _ => Err(CliError::new("usage: datuma_k start <project-name>")),
    },
    _ => Err(CliError::new(USAGE)),
  }
}

pub async fn run_project(cwd: &Path) -> Result<(), CliError> {
  let project = load_project(cwd)
    .await
    .map_err(|err| CliError::new(err.to_string()))?;
  let planned = plan_project(&project).map_err(|err| CliError::new(err.to_string()))?;
  commit(&project.root_str, &planned).map_err(|err| CliError::new(err.to_string()))
}

pub fn start_project(cwd: &Path, name: &str) -> Result<PathBuf, CliError> {
  if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\') {
    Err(CliError::new(format!("invalid project name {name:?}")))
  } else {
    let dir = cwd.join(name);
    if dir.exists() {
      Err(CliError::new(format!("{} already exists", dir.display())))
    } else {
      for folder in ["data", "engine", "definition"] {
        std::fs::create_dir_all(dir.join(folder)).map_err(|err| CliError::new(err.to_string()))?;
      }
      std::fs::write(dir.join(".env"), ENV_FILE).map_err(|err| CliError::new(err.to_string()))?;
      std::fs::write(dir.join("definition").join("cases.def.ngin"), CASES_DEF)
        .map_err(|err| CliError::new(err.to_string()))?;
      std::fs::write(keywords_path(&dir.join("data")), KEYWORDS_STUB)
        .map_err(|err| CliError::new(err.to_string()))?;
      Ok(dir)
    }
  }
}

fn current_dir() -> Result<PathBuf, CliError> {
  std::env::current_dir().map_err(|err| CliError::new(err.to_string()))
}

fn confirm_walk(cwd: &Path) -> Result<(), CliError> {
  if !is_dangerous_dir(cwd) {
    Ok(())
  } else if !io::stdin().is_terminal() {
    Err(CliError::new(format!(
      "refusing to walk {} without a TTY; run from a project directory",
      cwd.display()
    )))
  } else {
    eprintln!(
      "Walking {} for *.dtct, *.ngin, and *.def.ngin. Type yes to continue:",
      cwd.display()
    );
    io::stdout().flush().ok();
    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
      Err(err) => Err(CliError::new(err.to_string())),
      Ok(_) if line.trim() == "yes" => Ok(()),
      Ok(_) => Err(CliError::new("aborted")),
    }
  }
}

fn catalog_filters(args: impl Iterator<Item = String>) -> Result<Vec<(Dim, String)>, CliError> {
  let mut args = args.into_iter();
  let mut filters = Vec::new();
  loop {
    match args.next().as_deref() {
      None => break,
      Some("--trait") => push_filter(&mut filters, Dim::Trait, args.next())?,
      Some("--model") => push_filter(&mut filters, Dim::Model, args.next())?,
      Some("--field") => push_filter(&mut filters, Dim::Field, args.next())?,
      Some("--attribute") => push_filter(&mut filters, Dim::Attribute, args.next())?,
      Some("--type") => push_filter(&mut filters, Dim::Type, args.next())?,
      Some(_) => return Err(CliError::new(CATALOG_USAGE)),
    }
  }
  Ok(filters)
}

fn push_filter(
  filters: &mut Vec<(Dim, String)>,
  dim: Dim,
  name: Option<String>,
) -> Result<(), CliError> {
  match name {
    None => Err(CliError::new(CATALOG_USAGE)),
    Some(name) if name.starts_with("--") => Err(CliError::new(CATALOG_USAGE)),
    Some(name) if filters.iter().any(|(existing, _)| *existing == dim) => {
      Err(CliError::new(format!("duplicate {} filter", dim.label())))
    }
    Some(name) => {
      filters.push((dim, name));
      Ok(())
    }
  }
}

fn json_check(
  result: Result<crate::project::CheckResult, crate::project::ProjectError>,
) -> Result<(), CliError> {
  match result {
    Ok(result) => {
      print_json(&result)?;
      if result.ok {
        Ok(())
      } else {
        Err(CliError::reported())
      }
    }
    Err(err) => json_err(&err),
  }
}

fn json_err(err: &crate::project::ProjectError) -> Result<(), CliError> {
  print_json(&failure_result(err))?;
  Err(CliError::reported())
}

fn print_json(value: &impl Serialize) -> Result<(), CliError> {
  match serde_json::to_string_pretty(value) {
    Ok(text) => {
      println!("{text}");
      Ok(())
    }
    Err(err) => Err(CliError::new(err.to_string())),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::project::{catalog_project, check_project, preview_project};
  use std::time::{SystemTime, UNIX_EPOCH};

  fn scratch(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("time")
      .as_nanos();
    let dir = std::env::temp_dir().join(format!("datuma-cli-{name}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("dir");
    dir
  }

  #[test]
  fn start_writes_layout_env_and_title_case() {
    let parent = scratch("start");
    let dir = start_project(&parent, "demo").expect("start");
    assert!(dir.join("data").is_dir());
    assert!(dir.join("engine").is_dir());
    assert!(dir.join("definition").is_dir());
    let env = std::fs::read_to_string(dir.join(".env")).expect("env");
    assert!(env.contains("ROOT_DIRECTORY=."));
    assert!(env.contains("DTCT_DIRECTORY=data"));
    assert!(env.contains("NGIN_DIRECTORY=engine"));
    assert!(env.contains("DEF_DIRECTORY=definition"));
    let cases =
      std::fs::read_to_string(dir.join("definition").join("cases.def.ngin")).expect("cases");
    assert!(cases.contains("fn title_case"));
    let keywords = std::fs::read_to_string(dir.join("data").join("keywords.md")).expect("keywords");
    assert_eq!(keywords, KEYWORDS_STUB);
    start_project(&parent, "demo").expect_err("exists");
  }

  #[tokio::test]
  async fn run_materializes_started_project_template() {
    let parent = scratch("run");
    let dir = start_project(&parent, "app").expect("start");
    std::fs::write(
      dir.join("engine").join("hello.ngin"),
      "|$ROOT_DIRECTORY/hello.txt>\n```\nhi\n```\n",
    )
    .expect("ngin");
    run_project(&dir).await.expect("run");
    let out = std::fs::read_to_string(dir.join("hello.txt")).expect("out");
    assert!(out.contains("hi"), "{out}");
  }

  #[tokio::test]
  async fn run_keeps_files_from_each_template() {
    let parent = scratch("two-ngin");
    let dir = start_project(&parent, "app").expect("start");
    std::fs::write(
      dir.join("engine").join("a.ngin"),
      "|$ROOT_DIRECTORY/a.txt>\n```\na\n```\n",
    )
    .expect("a");
    std::fs::write(
      dir.join("engine").join("b.ngin"),
      "|$ROOT_DIRECTORY/b.txt>\n```\nb\n```\n",
    )
    .expect("b");
    run_project(&dir).await.expect("run");
    let a = std::fs::read_to_string(dir.join("a.txt")).expect("a.txt");
    let b = std::fs::read_to_string(dir.join("b.txt")).expect("b.txt");
    assert!(a.contains('a'), "{a}");
    assert!(b.contains('b'), "{b}");
    run_project(&dir).await.expect("rerun");
    assert!(dir.join("a.txt").exists());
    assert!(dir.join("b.txt").exists());
  }

  #[tokio::test]
  async fn check_passes_started_project() {
    let parent = scratch("check-ok");
    let dir = start_project(&parent, "app").expect("start");
    let result = check_project(&dir).await.expect("check");
    assert!(result.ok, "{:?}", result.diagnostics);
  }

  #[tokio::test]
  async fn check_fails_when_keywords_missing() {
    let parent = scratch("check-missing");
    let dir = start_project(&parent, "app").expect("start");
    std::fs::remove_file(dir.join("data").join("keywords.md")).expect("rm");
    let result = check_project(&dir).await.expect("check");
    assert!(!result.ok);
    assert!(
      result
        .diagnostics
        .iter()
        .any(|item| item.message.contains("missing keywords.md")),
      "{:?}",
      result.diagnostics
    );
  }

  #[tokio::test]
  async fn check_fails_when_keyword_undocumented() {
    let parent = scratch("check-undoc");
    let dir = start_project(&parent, "app").expect("start");
    std::fs::write(
      dir.join("data").join("app.dtct"),
      "Item {\n  n: int_type<min(1)>\n}\n",
    )
    .expect("dtct");
    let result = check_project(&dir).await.expect("check");
    assert!(!result.ok);
    assert!(
      result
        .diagnostics
        .iter()
        .any(|item| item.message.contains("undocumented") && item.message.contains("min")),
      "{:?}",
      result.diagnostics
    );
  }

  #[tokio::test]
  async fn preview_does_not_write_output() {
    let parent = scratch("preview");
    let dir = start_project(&parent, "app").expect("start");
    std::fs::write(
      dir.join("engine").join("hello.ngin"),
      "|$ROOT_DIRECTORY/hello.txt>\n```\nhi\n```\n",
    )
    .expect("ngin");
    let result = preview_project(&dir).await.expect("preview");
    assert!(result.ok, "{:?}", result.diagnostics);
    assert!(
      result
        .files
        .iter()
        .any(|file| file.path.ends_with("hello.txt") && file.content.contains("hi")),
      "{:?}",
      result.files
    );
    assert!(!dir.join("hello.txt").exists());
    run_project(&dir).await.expect("run");
    assert!(dir.join("hello.txt").exists());
  }

  #[tokio::test]
  async fn catalog_lists_models_and_filters() {
    let parent = scratch("catalog");
    let dir = start_project(&parent, "app").expect("start");
    std::fs::write(
      dir.join("data").join("app.dtct"),
      "Event [Resource] {\n  title: text_type<required>\n}\nVenue {\n  name: text_type<>\n}\n",
    )
    .expect("dtct");
    let all = catalog_project(&dir, &[]).await.expect("catalog");
    assert!(all.ok);
    assert_eq!(all.models.len(), 2, "{:?}", all.models);
    let filtered = catalog_project(&dir, &[(Dim::Trait, "Resource".into())])
      .await
      .expect("filter");
    assert_eq!(filtered.models.len(), 1);
    assert_eq!(filtered.models[0].name, "Event");
    let missing = catalog_project(&dir, &[(Dim::Model, "Nope".into())])
      .await
      .expect("missing");
    assert!(missing.ok);
    assert!(missing.models.is_empty());
  }
}
