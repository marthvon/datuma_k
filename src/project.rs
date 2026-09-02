use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use indexmap::IndexMap;
use serde::Serialize;

use crate::core::common::{DirFiles, EnvMap, collect_unique_dir_files};
use crate::core::parser::ParseError;
use crate::core::state::DatumaState;
use crate::dkcache::{VNode, merge_planned};
use crate::dtct::registry::DtctDb;
use crate::dtct::types::{AttrArg, Dim, Filter, QueryFilter};
use crate::dtct::{DtctFact, load_dtct_paths};
use crate::ngin::{MaterializeError, load_def_ngin_paths, load_ngin_paths, plan_materialize};

pub const KEYWORDS_FILE: &str = "keywords.md";
pub const KEYWORDS_STUB: &str = "\
| keyword | kind | description | purpose | platforms |
| --- | --- | --- | --- | --- |
";

const PLATFORMS: [&str; 3] = ["api_server", "web_frontend", "mobile_frontend"];

#[derive(Debug)]
pub enum ProjectError {
  Message(String),
  Parse(ParseError),
  Plan {
    path: PathBuf,
    err: MaterializeError,
  },
}

#[derive(Debug, Serialize)]
pub struct Diagnostic {
  pub path: String,
  pub line: usize,
  pub col: usize,
  pub message: String,
}

#[derive(Debug, Serialize)]
pub struct CheckResult {
  pub ok: bool,
  pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Serialize)]
pub struct Catalog {
  pub ok: bool,
  pub diagnostics: Vec<Diagnostic>,
  pub models: Vec<CatalogModel>,
  pub traits: Vec<String>,
  pub types: Vec<String>,
  pub attributes: Vec<String>,
  pub fields: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CatalogModel {
  pub name: String,
  pub traits: Vec<String>,
  pub fields: Vec<CatalogField>,
}

#[derive(Debug, Serialize)]
pub struct CatalogField {
  pub name: String,
  #[serde(rename = "type")]
  pub ty: Option<String>,
  pub attributes: Vec<CatalogAttr>,
}

#[derive(Debug, Serialize)]
pub struct CatalogAttr {
  pub name: String,
  pub args: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct PreviewResult {
  pub ok: bool,
  pub diagnostics: Vec<Diagnostic>,
  pub files: Vec<PreviewFile>,
}

#[derive(Debug, Serialize)]
pub struct PreviewFile {
  pub path: String,
  pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum KeywordKind {
  Model,
  Trait,
  Type,
  Attribute,
  Field,
}

#[derive(Debug)]
struct KeywordDoc {
  kind: KeywordKind,
  description: String,
  purpose: String,
  platforms: Vec<String>,
}

pub struct LoadedProject {
  pub env: EnvMap,
  pub root_str: String,
  pub dtct_dir: PathBuf,
  pub db: Arc<DtctDb>,
  pub defs: Vec<DatumaState>,
  pub templates: Vec<(PathBuf, DatumaState)>,
}

impl std::fmt::Display for ProjectError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Message(text) => write!(f, "{text}"),
      Self::Parse(err) => write!(f, "{err}"),
      Self::Plan { path, err } => write!(f, "{}: {err}", path.display()),
    }
  }
}

impl std::error::Error for ProjectError {}

impl From<ParseError> for ProjectError {
  fn from(err: ParseError) -> Self {
    Self::Parse(err)
  }
}

impl KeywordKind {
  fn parse(label: &str) -> Option<Self> {
    match label {
      "model" => Some(Self::Model),
      "trait" => Some(Self::Trait),
      "type" => Some(Self::Type),
      "attribute" => Some(Self::Attribute),
      "field" => Some(Self::Field),
      _ => None,
    }
  }

  fn label(self) -> &'static str {
    match self {
      Self::Model => "model",
      Self::Trait => "trait",
      Self::Type => "type",
      Self::Attribute => "attribute",
      Self::Field => "field",
    }
  }
}

pub fn failure_result(err: &ProjectError) -> CheckResult {
  CheckResult {
    ok: false,
    diagnostics: vec![diagnostic_from(err)],
  }
}

pub fn keywords_path(dtct_dir: &Path) -> PathBuf {
  dtct_dir.join(KEYWORDS_FILE)
}

pub async fn load_project(cwd: &Path) -> Result<LoadedProject, ProjectError> {
  let (env, root_str, dtct_dir, files) = project_files(cwd)?;
  let db = Arc::new(load_dtct_paths(files.dtct).await?);
  let defs = load_def_ngin_paths(files.def_ngin).await?;
  let templates = load_ngin_paths(files.ngin).await?;
  Ok(LoadedProject {
    env,
    root_str,
    dtct_dir,
    db,
    defs,
    templates,
  })
}

pub fn plan_project(project: &LoadedProject) -> Result<IndexMap<String, Vec<VNode>>, ProjectError> {
  let mut planned = IndexMap::new();
  for (path, tree) in &project.templates {
    match plan_materialize(
      tree,
      &project.defs,
      Arc::clone(&project.db),
      &project.root_str,
      &project.env,
    ) {
      Ok(files) => merge_planned(&mut planned, files),
      Err(err) => {
        return Err(ProjectError::Plan {
          path: path.clone(),
          err,
        });
      }
    }
  }
  Ok(planned)
}

pub async fn check_project(cwd: &Path) -> Result<CheckResult, ProjectError> {
  let project = load_project(cwd).await?;
  plan_project(&project)?;
  let diagnostics = check_keywords(&project.dtct_dir, &project.db);
  Ok(CheckResult {
    ok: diagnostics.is_empty(),
    diagnostics,
  })
}

pub async fn catalog_project(
  cwd: &Path,
  filters: &[(Dim, String)],
) -> Result<Catalog, ProjectError> {
  let (_, _, _, files) = project_files(cwd)?;
  build_catalog(&load_dtct_paths(files.dtct).await?, filters)
}

pub async fn preview_project(cwd: &Path) -> Result<PreviewResult, ProjectError> {
  let project = load_project(cwd).await?;
  let planned = plan_project(&project)?;
  Ok(PreviewResult {
    ok: true,
    diagnostics: Vec::new(),
    files: planned
      .iter()
      .map(|(path, nodes)| PreviewFile {
        path: path.clone(),
        content: VNode::flatten_all(nodes),
      })
      .collect(),
  })
}

fn project_files(cwd: &Path) -> Result<(EnvMap, String, PathBuf, DirFiles), ProjectError> {
  let env = EnvMap::load_from(cwd);
  let root = env.root_dir(cwd);
  let root = match root.canonicalize() {
    Ok(path) => path,
    Err(_) => root,
  };
  let root_str = root
    .to_str()
    .ok_or_else(|| ProjectError::Message("ROOT_DIRECTORY is not valid UTF-8".into()))?
    .to_string();
  let dtct_dir = env.dtct_dir(cwd);
  let def_dir = env.def_dir(cwd);
  let ngin_dir = env.ngin_dir(cwd);
  let mut files = collect_unique_dir_files(&[&dtct_dir, &def_dir, &ngin_dir]);
  files.dtct.retain(|path| path.starts_with(&dtct_dir));
  files.def_ngin.retain(|path| path.starts_with(&def_dir));
  files.ngin.retain(|path| path.starts_with(&ngin_dir));
  Ok((env, root_str, dtct_dir, files))
}

fn diagnostic_from(err: &ProjectError) -> Diagnostic {
  match err {
    ProjectError::Message(text) => Diagnostic {
      path: String::new(),
      line: 0,
      col: 0,
      message: text.clone(),
    },
    ProjectError::Parse(err) => Diagnostic {
      path: err.file_meta.absolute_path.clone(),
      line: err.pos_meta.line,
      col: err.pos_meta.col,
      message: err.to_string(),
    },
    ProjectError::Plan { path, err } => match err {
      MaterializeError::Runtime(rt) => Diagnostic {
        path: match &rt.file_meta {
          Some(meta) if !meta.absolute_path.is_empty() => meta.absolute_path.clone(),
          _ => path.display().to_string(),
        },
        line: rt.pos_meta.map(|pos| pos.line).unwrap_or(0),
        col: rt.pos_meta.map(|pos| pos.col).unwrap_or(0),
        message: rt.to_string(),
      },
      _ => Diagnostic {
        path: path.display().to_string(),
        line: 0,
        col: 0,
        message: err.to_string(),
      },
    },
  }
}

fn check_keywords(dtct_dir: &Path, db: &DtctDb) -> Vec<Diagnostic> {
  let path = keywords_path(dtct_dir);
  if !path.is_file() {
    vec![Diagnostic {
      path: path.display().to_string(),
      line: 0,
      col: 0,
      message: "missing keywords.md: document every dtct model, trait, type, attribute, and field"
        .into(),
    }]
  } else {
    match std::fs::read_to_string(&path) {
      Err(err) => vec![Diagnostic {
        path: path.display().to_string(),
        line: 0,
        col: 0,
        message: format!("read keywords.md: {err}"),
      }],
      Ok(text) => match parse_keywords_table(&path, &text) {
        Err(diagnostics) => diagnostics,
        Ok(docs) => compare_keywords(&path, db, &docs),
      },
    }
  }
}

fn parse_keywords_table(
  path: &Path,
  text: &str,
) -> Result<BTreeMap<String, KeywordDoc>, Vec<Diagnostic>> {
  let mut docs = BTreeMap::new();
  let mut diagnostics = Vec::new();
  let mut seen_header = false;
  let mut skip_sep = false;
  for (idx, line) in text.lines().enumerate() {
    let line_no = idx + 1;
    match table_cells(line) {
      None => {}
      Some(cells) if !seen_header => {
        if is_keywords_header(&cells) {
          seen_header = true;
          skip_sep = true;
        }
      }
      Some(cells) if skip_sep && is_table_sep(&cells) => {
        skip_sep = false;
      }
      Some(cells) => {
        skip_sep = false;
        parse_keyword_row(path, line_no, cells, &mut docs, &mut diagnostics);
      }
    }
  }
  if !diagnostics.is_empty() {
    Err(diagnostics)
  } else if !seen_header {
    Err(vec![Diagnostic {
      path: path.display().to_string(),
      line: 0,
      col: 0,
      message: "keywords.md needs a markdown table with columns keyword, kind, description, purpose, platforms"
        .into(),
    }])
  } else {
    Ok(docs)
  }
}

fn parse_keyword_row(
  path: &Path,
  line: usize,
  cells: Vec<String>,
  docs: &mut BTreeMap<String, KeywordDoc>,
  diagnostics: &mut Vec<Diagnostic>,
) {
  if cells.len() != 5 {
    diagnostics.push(Diagnostic {
      path: path.display().to_string(),
      line,
      col: 1,
      message: format!("expected 5 table columns, got {}", cells.len()),
    });
  } else {
    let name = cells[0].clone();
    let platforms: Vec<String> = cells[4]
      .split(',')
      .map(|item| item.trim().to_string())
      .filter(|item| !item.is_empty())
      .collect();
    if name.is_empty() {
      diagnostics.push(Diagnostic {
        path: path.display().to_string(),
        line,
        col: 1,
        message: "keyword column is empty".into(),
      });
    } else if docs.contains_key(&name) {
      diagnostics.push(Diagnostic {
        path: path.display().to_string(),
        line,
        col: 1,
        message: format!("duplicate keyword {name}"),
      });
    } else if let Some(kind) = KeywordKind::parse(&cells[1]) {
      docs.insert(
        name,
        KeywordDoc {
          kind,
          description: cells[2].clone(),
          purpose: cells[3].clone(),
          platforms,
        },
      );
    } else {
      diagnostics.push(Diagnostic {
        path: path.display().to_string(),
        line,
        col: 1,
        message: format!(
          "invalid kind {}; use model, trait, type, attribute, or field",
          cells[1]
        ),
      });
    }
  }
}

fn table_cells(line: &str) -> Option<Vec<String>> {
  let trimmed = line.trim();
  if !trimmed.starts_with('|') {
    None
  } else {
    let inner = trimmed.trim_start_matches('|').trim_end_matches('|');
    Some(
      inner
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect(),
    )
  }
}

fn is_keywords_header(cells: &[String]) -> bool {
  if cells.len() != 5 {
    false
  } else {
    let keyword = cells[0].to_ascii_lowercase();
    cells[1].eq_ignore_ascii_case("kind")
      && cells[2].eq_ignore_ascii_case("description")
      && cells[3].eq_ignore_ascii_case("purpose")
      && cells[4].eq_ignore_ascii_case("platforms")
      && (keyword == "keyword" || keyword == "name")
  }
}

fn is_table_sep(cells: &[String]) -> bool {
  !cells.is_empty()
    && cells.iter().all(|cell| {
      let stripped = cell.trim_matches(':').trim_matches('-');
      !cell.is_empty() && stripped.is_empty()
    })
}

fn compare_keywords(
  path: &Path,
  db: &DtctDb,
  docs: &BTreeMap<String, KeywordDoc>,
) -> Vec<Diagnostic> {
  let used = used_keywords(db);
  let mut diagnostics = Vec::new();
  for (name, kinds) in &used {
    match docs.get(name) {
      None => diagnostics.push(doc_diagnostic(
        path,
        format!(
          "undocumented {}: {name}: add a row to keywords.md (description, purpose, platforms)",
          kind_labels(kinds)
        ),
      )),
      Some(doc) => {
        if !kinds.contains(&doc.kind) {
          diagnostics.push(doc_diagnostic(
            path,
            format!(
              "{name} is documented as {} but used as {}",
              doc.kind.label(),
              kind_labels(kinds)
            ),
          ));
        }
        if doc.description.trim().is_empty() {
          diagnostics.push(doc_diagnostic(
            path,
            format!("{name}: description must be non-empty"),
          ));
        }
        if doc.purpose.trim().is_empty() {
          diagnostics.push(doc_diagnostic(
            path,
            format!("{name}: purpose must be non-empty"),
          ));
        }
        if doc.platforms.is_empty() {
          diagnostics.push(doc_diagnostic(
            path,
            format!("{name}: platforms must be a non-empty list"),
          ));
        } else {
          for platform in &doc.platforms {
            if !PLATFORMS.contains(&platform.as_str()) {
              diagnostics.push(doc_diagnostic(
                path,
                format!(
                  "{name}: invalid platform {platform}; use api_server, web_frontend, or mobile_frontend"
                ),
              ));
            }
          }
        }
      }
    }
  }
  for name in docs.keys() {
    if !used.contains_key(name) {
      diagnostics.push(doc_diagnostic(
        path,
        format!("keywords.md documents unused name {name}"),
      ));
    }
  }
  diagnostics
}

fn kind_labels(kinds: &BTreeSet<KeywordKind>) -> String {
  kinds
    .iter()
    .map(|kind| kind.label())
    .collect::<Vec<_>>()
    .join("/")
}

fn doc_diagnostic(path: &Path, message: String) -> Diagnostic {
  Diagnostic {
    path: path.display().to_string(),
    line: 0,
    col: 0,
    message,
  }
}

fn used_keywords(db: &DtctDb) -> BTreeMap<String, BTreeSet<KeywordKind>> {
  let mut used: BTreeMap<String, BTreeSet<KeywordKind>> = BTreeMap::new();
  for fact in db.facts() {
    used
      .entry(db.resolve(fact.model).to_string())
      .or_default()
      .insert(KeywordKind::Model);
    if let Some(trait_name) = fact.trait_name {
      used
        .entry(db.resolve(trait_name).to_string())
        .or_default()
        .insert(KeywordKind::Trait);
    }
    if let Some(field) = fact.field {
      used
        .entry(db.resolve(field).to_string())
        .or_default()
        .insert(KeywordKind::Field);
    }
    if let Some(ty) = fact.ty {
      used
        .entry(db.resolve(ty).to_string())
        .or_default()
        .insert(KeywordKind::Type);
    }
    if let Some(attribute) = fact.attribute {
      used
        .entry(db.resolve(attribute).to_string())
        .or_default()
        .insert(KeywordKind::Attribute);
    }
  }
  used
}

fn build_catalog(db: &DtctDb, filters: &[(Dim, String)]) -> Result<Catalog, ProjectError> {
  let mut query = QueryFilter::default();
  for (dim, name) in filters {
    match db.spur(name) {
      None => {
        return Ok(empty_catalog());
      }
      Some(spur) => {
        if query.0.iter().any(|item| item.dim == *dim) {
          return Err(ProjectError::Message(format!(
            "duplicate {} filter",
            dim.label()
          )));
        } else {
          query.0.push(Filter::r#in(*dim, vec![spur]));
        }
      }
    }
  }
  match db.query(&query, Dim::Model) {
    Err(err) => Err(ProjectError::Message(err.to_string())),
    Ok(view) => Ok(catalog_from_facts(db, view.ids())),
  }
}

fn empty_catalog() -> Catalog {
  Catalog {
    ok: true,
    diagnostics: Vec::new(),
    models: Vec::new(),
    traits: Vec::new(),
    types: Vec::new(),
    attributes: Vec::new(),
    fields: Vec::new(),
  }
}

fn catalog_from_facts(db: &DtctDb, ids: &[u32]) -> Catalog {
  let facts: Vec<&DtctFact> = ids.iter().map(|id| &db.facts()[*id as usize]).collect();
  let mut model_names = BTreeSet::new();
  let mut trait_names = BTreeSet::new();
  let mut type_names = BTreeSet::new();
  let mut attribute_names = BTreeSet::new();
  let mut field_names = BTreeSet::new();
  for fact in &facts {
    model_names.insert(db.resolve(fact.model).to_string());
    if let Some(name) = fact.trait_name {
      trait_names.insert(db.resolve(name).to_string());
    }
    if let Some(name) = fact.ty {
      type_names.insert(db.resolve(name).to_string());
    }
    if let Some(name) = fact.attribute {
      attribute_names.insert(db.resolve(name).to_string());
    }
    if let Some(name) = fact.field {
      field_names.insert(db.resolve(name).to_string());
    }
  }
  let models = model_names
    .iter()
    .map(|name| catalog_model(db, &facts, name))
    .collect();
  Catalog {
    ok: true,
    diagnostics: Vec::new(),
    models,
    traits: trait_names.into_iter().collect(),
    types: type_names.into_iter().collect(),
    attributes: attribute_names.into_iter().collect(),
    fields: field_names.into_iter().collect(),
  }
}

fn catalog_model(db: &DtctDb, facts: &[&DtctFact], name: &str) -> CatalogModel {
  let model_facts: Vec<&DtctFact> = facts
    .iter()
    .copied()
    .filter(|fact| db.resolve(fact.model) == name)
    .collect();
  let mut traits = BTreeSet::new();
  let mut field_set = BTreeSet::new();
  for fact in &model_facts {
    if let Some(trait_name) = fact.trait_name {
      traits.insert(db.resolve(trait_name).to_string());
    }
    if let Some(field) = fact.field {
      field_set.insert(db.resolve(field).to_string());
    }
  }
  CatalogModel {
    name: name.to_string(),
    traits: traits.into_iter().collect(),
    fields: field_set
      .iter()
      .map(|field| catalog_field(db, &model_facts, field))
      .collect(),
  }
}

fn catalog_field(db: &DtctDb, facts: &[&DtctFact], name: &str) -> CatalogField {
  let field_facts: Vec<&DtctFact> = facts
    .iter()
    .copied()
    .filter(|fact| fact.field.is_some_and(|field| db.resolve(field) == name))
    .collect();
  let mut ty = None;
  let mut seen_attrs = HashSet::new();
  let mut attributes = Vec::new();
  for fact in &field_facts {
    if ty.is_none() {
      if let Some(ty_spur) = fact.ty {
        ty = Some(db.resolve(ty_spur).to_string());
      }
    }
    if let Some(attr) = fact.attribute {
      let attr_name = db.resolve(attr).to_string();
      if seen_attrs.insert(attr_name.clone()) {
        attributes.push(CatalogAttr {
          name: attr_name,
          args: fact
            .args
            .iter()
            .copied()
            .map(|arg| attr_arg_json(db, arg))
            .collect(),
        });
      }
    }
  }
  attributes.sort_by(|a, b| a.name.cmp(&b.name));
  CatalogField {
    name: name.to_string(),
    ty,
    attributes,
  }
}

fn attr_arg_json(db: &DtctDb, arg: AttrArg) -> serde_json::Value {
  match arg {
    AttrArg::Ident(spur) | AttrArg::String(spur) => {
      serde_json::Value::String(db.resolve(spur).to_string())
    }
    AttrArg::Integer(value) => serde_json::json!(value),
    AttrArg::Float(value) => serde_json::json!(value),
    AttrArg::Boolean(value) => serde_json::json!(value),
    AttrArg::Null => serde_json::Value::Null,
  }
}
