use std::path::{Path, PathBuf};
use std::sync::Arc;

use lasso::ThreadedRodeo;
use tokio::task::JoinSet;

use crate::core::common::files_from_dir;
use crate::core::parser::{
  ParseCursorMetadata, ParseError, ParseErrorKind, ParseErrorSource, ParseFile, ParseFileMetadata,
  ParseStack, expected, messages, parse_stack,
};
use crate::core::state::DatumaState;
use crate::dtct::materialize::{MaterializeError, materialize, materialize_with};
use crate::dtct::modes::DtctRootParseMode;
use crate::dtct::registry::{DtctDb, merge_model_names};

pub async fn parse_file(path: &Path) -> Result<DtctDb, ParseError> {
  let state = parse_tree(path).await?;
  materialize(&state).map_err(|err| materialize_parse_error_path(path, err))
}

#[inline(always)]
pub async fn load_dtct_dir(dir: &Path) -> Result<DtctDb, ParseError> {
  load_dtct_paths(dtct_paths(dir)?).await
}

pub async fn load_dtct_paths(paths: Vec<PathBuf>) -> Result<DtctDb, ParseError> {
  if paths.is_empty() {
    return Ok(DtctDb::empty());
  }
  let mut parse_set = JoinSet::new();
  for path in paths {
    parse_set.spawn_blocking(move || parse_tree_blocking(path));
  }
  let mut trees = Vec::new();
  while let Some(joined) = parse_set.join_next().await {
    trees.push(join_task(joined)??);
  }
  let pool = Arc::new(ThreadedRodeo::new());
  let mut fact_set = JoinSet::new();
  for (path, state) in trees {
    let pool = Arc::clone(&pool);
    fact_set.spawn_blocking(move || match materialize_with(&state, &pool) {
      Ok(facts) => Ok((path, facts)),
      Err(err) => Err((path, err)),
    });
  }
  let mut loaded = Vec::new();
  while let Some(joined) = fact_set.join_next().await {
    let (path, facts) =
      join_task(joined)?.map_err(|(path, err)| materialize_parse_error_path(&path, err))?;
    loaded.push((path, facts));
  }
  loaded.sort_by(|a, b| a.0.cmp(&b.0));
  let mut all_facts = Vec::new();
  let mut seen_models = std::collections::HashSet::new();
  for (path, facts) in loaded {
    merge_model_names(
      &mut seen_models,
      facts
        .iter()
        .map(|fact| fact.model)
        .collect::<std::collections::HashSet<_>>(),
    )
    .map_err(|err| materialize_parse_error_path(&path, MaterializeError::Db(err)))?;
    all_facts.extend(facts);
  }
  let pool = Arc::try_unwrap(pool).expect("all materialize tasks joined");
  Ok(DtctDb::build(pool, all_facts))
}

fn parse_tree_blocking(path: PathBuf) -> Result<(PathBuf, DatumaState), ParseError> {
  tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .map_err(|err| io_parse_error(format!("runtime: {err}")))?
    .block_on(parse_tree(&path))
    .map(|state| (path, state))
}

async fn parse_tree(path: &Path) -> Result<DatumaState, ParseError> {
  let mut stack = ParseStack::with_root(Box::new(DtctRootParseMode::new()));
  let mut file = open_file(path).await?;
  run_parse_stack(&mut stack, &mut file).await?;
  finish_tree(stack, &file)
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

fn dtct_paths(dir: &Path) -> Result<Vec<PathBuf>, ParseError> {
  if !dir.exists() {
    Ok(Vec::new())
  } else if !dir.is_dir() {
    Err(io_parse_error(format!(
      "read {}: not a directory",
      dir.display()
    )))
  } else {
    let mut paths = files_from_dir(dir.to_str().unwrap_or_default(), &["dtct"]);
    paths.sort();
    Ok(paths)
  }
}

fn finish_tree(
  stack: ParseStack,
  source: &dyn ParseErrorSource,
) -> Result<DatumaState, ParseError> {
  let mut stack = stack;
  stack.dismiss_resolved();
  if stack.has_active_frames() {
    Err(source.error(expected(messages::COMPLETE_INPUT)))
  } else {
    let root = stack.into_root();
    root
      .into_datuma_state()
      .ok_or_else(|| source.error(expected(messages::COMPLETE_INPUT)))
  }
}

fn join_task<T>(joined: Result<T, tokio::task::JoinError>) -> Result<T, ParseError> {
  joined.map_err(|err| io_parse_error(format!("task join: {err}")))
}

fn io_parse_error(message: String) -> ParseError {
  ParseError {
    file_meta: ParseFileMetadata::source(""),
    pos_meta: ParseCursorMetadata::default(),
    curr_mode: None,
    kind: ParseErrorKind::IoError(Box::new(std::io::Error::other(message))),
  }
}

fn materialize_parse_error_path(path: &Path, err: MaterializeError) -> ParseError {
  ParseError {
    file_meta: ParseFileMetadata::source(path.display().to_string()),
    pos_meta: ParseCursorMetadata::default(),
    curr_mode: None,
    kind: ParseErrorKind::IoError(Box::new(std::io::Error::other(err.to_string()))),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::dtct::types::{Dim, Filter, QueryError, QueryFilter};

  fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
      .join("tests/dtct")
      .join(name)
  }

  fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/dtct")
  }

  fn names(db: &DtctDb, spurs: &[lasso::Spur]) -> Vec<String> {
    let mut names: Vec<String> = spurs
      .iter()
      .map(|spur| db.resolve(*spur).to_string())
      .collect();
    names.sort();
    names
  }

  #[tokio::test]
  async fn parses_sample_fixture() {
    let db = parse_file(&fixture("sample.dtct"))
      .await
      .expect("parse sample");
    let models = db
      .query(&QueryFilter(vec![]), Dim::Model)
      .expect("models")
      .spurs();
    assert_eq!(names(&db, &models), ["MyModel", "OtherModel"]);
  }

  #[tokio::test]
  async fn parses_faker_contracts_fixture() {
    let db = parse_file(&fixture("faker_contracts.dtct"))
      .await
      .expect("parse faker");
    let models = db
      .query(&QueryFilter(vec![]), Dim::Model)
      .expect("models")
      .spurs();
    assert_eq!(
      names(&db, &models),
      [
        "AuditEvent",
        "OrderSummary",
        "ProductCatalog",
        "UserAccount"
      ]
    );
    let immutable = db.spur("Immutable").expect("Immutable");
    let by_trait = db
      .query(
        &QueryFilter(vec![Filter::r#in(Dim::Trait, vec![immutable])]),
        Dim::Model,
      )
      .expect("trait -> model")
      .spurs();
    assert_eq!(names(&db, &by_trait), ["AuditEvent"]);
    let model = db.spur("AuditEvent").expect("AuditEvent");
    let field = db.spur("actor").expect("actor");
    let attrs = db
      .query(
        &QueryFilter(vec![
          Filter::r#in(Dim::Model, vec![model]),
          Filter::r#in(Dim::Field, vec![field]),
        ]),
        Dim::Attribute,
      )
      .expect("model+field -> attribute")
      .spurs();
    assert_eq!(names(&db, &attrs), ["max_length"]);
    let attribute = db.spur("required").expect("required");
    let fields = db
      .query(
        &QueryFilter(vec![
          Filter::r#in(Dim::Model, vec![model]),
          Filter::r#in(Dim::Attribute, vec![attribute]),
        ]),
        Dim::Field,
      )
      .expect("model+attribute -> field")
      .spurs();
    assert_eq!(names(&db, &fields), ["target"]);
    let or_models = db
      .query(
        &QueryFilter(vec![Filter::r#in(
          Dim::Trait,
          vec![immutable, db.spur("Archivable").expect("Archivable")],
        )]),
        Dim::Model,
      )
      .expect("trait OR")
      .spurs();
    assert_eq!(names(&db, &or_models), ["AuditEvent", "ProductCatalog"]);
  }

  #[tokio::test]
  async fn query_rejects_empty_and_duplicate_filters() {
    let db = parse_file(&fixture("faker_contracts.dtct"))
      .await
      .expect("parse faker");
    let model = db.spur("AuditEvent").unwrap();
    let same_dim = db
      .query(
        &QueryFilter(vec![Filter::r#in(Dim::Model, vec![model])]),
        Dim::Model,
      )
      .expect("same dim is spurs of those facts");
    assert_eq!(names(&db, &same_dim.spurs()), ["AuditEvent"]);
    assert!(matches!(
      db.query(
        &QueryFilter(vec![Filter::r#in(Dim::Model, vec![])]),
        Dim::Field
      )
      .expect_err("empty"),
      QueryError::EmptyFilter(Dim::Model)
    ));
    assert!(matches!(
      db.query(
        &QueryFilter(vec![
          Filter::r#in(Dim::Model, vec![model]),
          Filter::r#in(Dim::Model, vec![model]),
        ]),
        Dim::Field,
      )
      .expect_err("duplicate include"),
      QueryError::DuplicateFilterDim(Dim::Model)
    ));
    assert!(matches!(
      db.query(
        &QueryFilter(vec![
          Filter::not(Dim::Trait, vec![db.spur("Immutable").unwrap()]),
          Filter::not(Dim::Trait, vec![db.spur("Archivable").unwrap()]),
        ]),
        Dim::Model,
      )
      .expect_err("duplicate exclude"),
      QueryError::DuplicateFilterDim(Dim::Trait)
    ));
  }

  #[tokio::test]
  async fn query_not_and_include_exclude_same_dim() {
    let db = parse_file(&fixture("faker_contracts.dtct"))
      .await
      .expect("parse faker");
    let immutable = db.spur("Immutable").unwrap();
    let without_immutable = db
      .query(
        &QueryFilter(vec![Filter::not(Dim::Trait, vec![immutable])]),
        Dim::Model,
      )
      .expect("not trait")
      .spurs();
    assert_eq!(
      names(&db, &without_immutable),
      ["OrderSummary", "ProductCatalog", "UserAccount"]
    );
    let archivable = db.spur("Archivable").unwrap();
    let include_and_exclude = db
      .query(
        &QueryFilter(vec![
          Filter::r#in(Dim::Trait, vec![immutable, archivable]),
          Filter::not(Dim::Trait, vec![immutable]),
        ]),
        Dim::Model,
      )
      .expect("in and not same dim")
      .spurs();
    assert_eq!(names(&db, &include_and_exclude), ["ProductCatalog"]);
  }

  #[tokio::test]
  async fn query_view_projects_and_narrows() {
    let db = parse_file(&fixture("faker_contracts.dtct"))
      .await
      .expect("parse faker");
    let models = db.query(&QueryFilter(vec![]), Dim::Model).expect("models");
    let audit = db.spur("AuditEvent").unwrap();
    let fields = models.of(audit).project(Dim::Field).spurs();
    assert_eq!(names(&db, &fields), ["actor", "note", "target"]);
    let actor = db.spur("actor").unwrap();
    let tys = models
      .of(audit)
      .project(Dim::Field)
      .of(actor)
      .project(Dim::Type)
      .spurs();
    assert_eq!(names(&db, &tys), ["email_type"]);
    let required = db.spur("required").unwrap();
    let required_fields = models
      .of(audit)
      .include(Dim::Attribute, &[required])
      .expect("include attr")
      .project(Dim::Field)
      .spurs();
    assert_eq!(names(&db, &required_fields), ["target"]);
    let not_required = models
      .of(audit)
      .exclude(Dim::Attribute, &[required])
      .expect("exclude attr")
      .project(Dim::Field)
      .spurs();
    assert_eq!(names(&db, &not_required), ["actor", "note"]);
  }

  #[tokio::test]
  async fn query_view_skips_none_cells() {
    let db = parse_file(&fixture("empty_models.dtct"))
      .await
      .expect("parse empty models");
    let models = db.query(&QueryFilter(vec![]), Dim::Model).expect("models");
    let marker = db.spur("Marker").unwrap();
    assert!(models.of(marker).project(Dim::Field).spurs().is_empty());
    let tagged = db.spur("Tagged").unwrap();
    assert_eq!(
      names(&db, &models.of(tagged).project(Dim::Trait).spurs()),
      ["TagA", "TagB"]
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn loads_fixture_dir_in_parallel() {
    let db = load_dtct_dir(&fixtures_dir()).await.expect("load dir");
    let models = db
      .query(&QueryFilter(vec![]), Dim::Model)
      .expect("models")
      .spurs();
    assert_eq!(
      names(&db, &models),
      [
        "AuditEvent",
        "BareRecord",
        "BillingAddress",
        "FeatureFlag",
        "InventoryItem",
        "Marker",
        "MyModel",
        "OrderSummary",
        "OtherModel",
        "ProductCatalog",
        "RateLimit",
        "SessionToken",
        "Tagged",
        "UserAccount"
      ]
    );
  }

  #[tokio::test]
  async fn dumps_database() {
    let db = parse_file(&fixture("sample.dtct"))
      .await
      .expect("parse sample");
    let text = db.dump_string();
    assert!(text.starts_with("# facts "));
    assert!(text.contains("model=MyModel"));
    assert!(text.contains("# index by_model"));
  }
}
