use std::sync::Arc;

use indexmap::IndexMap;

use crate::core::common::EnvMap;
use crate::core::exec::{
  Flow, Interp, MAX_LOOP_ITERATIONS, RuntimeError, RuntimeErrorKind, RuntimeValue, Scope,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;
use crate::dkcache::{self, CacheError, VNode, fence_token, sanitize_id};
use crate::dtct::registry::DtctDb;
use crate::ngin::dk::dk_host;
use crate::ngin::value::NginValue;

#[derive(Debug)]
pub enum MaterializeError {
  Runtime(RuntimeError),
  Cache(CacheError),
  Message(String),
}

impl std::fmt::Display for MaterializeError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Runtime(err) => write!(f, "{err}"),
      Self::Cache(err) => write!(f, "{err}"),
      Self::Message(text) => write!(f, "{text}"),
    }
  }
}

impl std::error::Error for MaterializeError {}

impl From<RuntimeError> for MaterializeError {
  fn from(err: RuntimeError) -> Self {
    Self::Runtime(err)
  }
}

impl From<CacheError> for MaterializeError {
  fn from(err: CacheError) -> Self {
    Self::Cache(err)
  }
}

enum Sink {
  Nodes {
    children: Vec<VNode>,
    fires: usize,
    last_emit: bool,
  },
}

struct Walker<'tree> {
  interp: Interp<'tree>,
  loop_keys: Vec<(String, String)>,
  files: IndexMap<String, Vec<VNode>>,
  current_path: Option<String>,
  sinks: Vec<Sink>,
}

pub fn materialize(
  root: &DatumaState,
  db: Arc<DtctDb>,
  root_directory: &str,
) -> Result<IndexMap<String, Vec<VNode>>, MaterializeError> {
  materialize_with_defs(root, &[], db, root_directory)
}

pub fn materialize_with_defs(
  root: &DatumaState,
  defs: &[DatumaState],
  db: Arc<DtctDb>,
  root_directory: &str,
) -> Result<IndexMap<String, Vec<VNode>>, MaterializeError> {
  materialize_with_env(root, defs, db, root_directory, &EnvMap::empty())
}

pub fn materialize_with_env(
  root: &DatumaState,
  defs: &[DatumaState],
  db: Arc<DtctDb>,
  root_directory: &str,
  env: &EnvMap,
) -> Result<IndexMap<String, Vec<VNode>>, MaterializeError> {
  let files = plan_materialize(root, defs, db, root_directory, env)?;
  dkcache::commit(root_directory, &files)?;
  Ok(files)
}

pub fn plan_materialize(
  root: &DatumaState,
  defs: &[DatumaState],
  db: Arc<DtctDb>,
  root_directory: &str,
  env: &EnvMap,
) -> Result<IndexMap<String, Vec<VNode>>, MaterializeError> {
  let mut scope = Scope::new();
  for (key, value) in env.iter() {
    scope.assign(key, RuntimeValue::String(value.to_string()));
  }
  scope.assign("dk", dk_host(Arc::clone(&db)));
  scope.assign(
    "ROOT_DIRECTORY",
    RuntimeValue::String(root_directory.to_string()),
  );
  let mut roots: Vec<&DatumaState> = defs.iter().collect();
  roots.push(root);
  let mut interp = Interp::from_roots(&roots, scope)
    .map_err(|kind| MaterializeError::Runtime(RuntimeError::from_kind(kind, Vec::new())))?;
  for def in defs {
    interp.run_tree(def)?;
  }
  let mut walker = Walker {
    interp,
    loop_keys: Vec::new(),
    files: IndexMap::new(),
    current_path: None,
    sinks: Vec::new(),
  };
  for child in &root.children {
    walker.walk_node(child)?;
  }
  Ok(walker.files)
}

fn core_value(state: &DatumaState) -> Option<&CoreValue> {
  state
    .value
    .as_ref()
    .and_then(|value| value.as_any().downcast_ref::<CoreValue>())
}

fn ngin_value(state: &DatumaState) -> Option<&NginValue> {
  state
    .value
    .as_ref()
    .and_then(|value| value.as_any().downcast_ref::<NginValue>())
}

fn region_id(site: &str, keys: &[(String, String)]) -> String {
  let mut identity = site.to_string();
  if !keys.is_empty() {
    identity.push_str("::");
    let mut first = true;
    for (key, value) in keys {
      if first {
        first = false;
      } else {
        identity.push(',');
      }
      identity.push_str(key);
      identity.push('=');
      identity.push_str(value);
    }
  }
  fence_token(&sanitize_id(&identity))
}

impl<'tree> Walker<'tree> {
  fn walk_node(&mut self, node: &'tree DatumaState) -> Result<Flow, MaterializeError> {
    match ngin_value(node) {
      Some(NginValue::File) => self.walk_file(node),
      Some(NginValue::Interp) => {
        self.walk_children(node)?;
        Ok(Flow::Normal)
      }
      Some(NginValue::Emit { .. }) => {
        self.emit_from(node)?;
        Ok(Flow::Normal)
      }
      Some(NginValue::Plus { .. }) => {
        self.plus_from(node)?;
        Ok(Flow::Normal)
      }
      Some(NginValue::Guard { sep }) => self.walk_guard(node, sep),
      Some(NginValue::Template { .. }) => self.walk_template(node),
      Some(NginValue::Path | NginValue::Env { .. } | NginValue::PathLit { .. }) => Ok(Flow::Normal),
      None => match core_value(node) {
        Some(CoreValue::Program) => self.walk_children(node),
        Some(CoreValue::For) => self.walk_for(node),
        Some(CoreValue::If) | Some(CoreValue::ElseIf) => self.walk_if(node),
        Some(CoreValue::Else) => match node.children.first() {
          Some(body) => self.walk_node(body),
          None => Ok(Flow::Normal),
        },
        Some(CoreValue::Return) => Ok(Flow::Return(self.interp.eval_tokens(&node.children)?)),
        Some(CoreValue::Break) => Ok(Flow::Break),
        Some(CoreValue::Instruction { .. }) => match node.children.as_slice() {
          [child]
            if matches!(
              core_value(child),
              Some(
                CoreValue::Return
                  | CoreValue::Break
                  | CoreValue::If
                  | CoreValue::ElseIf
                  | CoreValue::Else
                  | CoreValue::For
                  | CoreValue::Program
              )
            ) =>
          {
            self.walk_node(child)
          }
          _ => Ok(self.interp.run_statement(node)?),
        },
        Some(CoreValue::FunctionDef(_)) => Ok(Flow::Normal),
        _ => Ok(Flow::Normal),
      },
    }
  }

  fn walk_children(&mut self, node: &'tree DatumaState) -> Result<Flow, MaterializeError> {
    let mut outcome = Ok(Flow::Normal);
    for child in &node.children {
      match self.walk_node(child) {
        Ok(Flow::Normal) => {}
        interrupted => {
          outcome = interrupted;
          break;
        }
      }
    }
    outcome
  }

  fn walk_file(&mut self, node: &'tree DatumaState) -> Result<Flow, MaterializeError> {
    let path = match node.children.first() {
      Some(path_node) => self.resolve_path(path_node)?,
      None => {
        return Err(MaterializeError::Message("file missing path".into()));
      }
    };
    let prev = self.current_path.replace(path);
    let outcome = match node.children.get(1) {
      Some(template) => self.walk_template(template),
      None => Err(MaterializeError::Message("file missing template".into())),
    };
    self.current_path = prev;
    outcome?;
    Ok(Flow::Normal)
  }

  fn walk_template(&mut self, node: &'tree DatumaState) -> Result<Flow, MaterializeError> {
    let (tline, tcol) = match ngin_value(node) {
      Some(NginValue::Template { line, col }) => (*line, *col),
      _ => (0, 0),
    };
    for (index, child) in node.children.iter().enumerate() {
      if let Some(ngin) = ngin_value(child) {
        match ngin {
          NginValue::Interp => {
            self.walk_node(child)?;
          }
          NginValue::Emit { .. } => {
            self.emit_from(child)?;
          }
          NginValue::Plus { .. } => {
            self.plus_from(child)?;
          }
          NginValue::Guard { sep } => {
            self.walk_guard(child, sep)?;
          }
          NginValue::Template { .. } => {
            self.walk_template(child)?;
          }
          _ => {}
        }
      } else if let Some(CoreValue::String(text)) = core_value(child) {
        self.append_literal(&format!("{tline}:{tcol}:lit{index}"), text.clone());
      } else {
        self.walk_node(child)?;
      }
    }
    Ok(Flow::Normal)
  }

  fn walk_guard(&mut self, node: &'tree DatumaState, sep: &str) -> Result<Flow, MaterializeError> {
    let Some(cond) = node.children.first() else {
      return Err(MaterializeError::Message("guard missing condition".into()));
    };
    let Some(emit) = node.children.get(1) else {
      return Err(MaterializeError::Message("guard missing emit".into()));
    };
    if self.interp.eval_operand(cond)?.truthy() {
      let should_sep = matches!(
        self.sinks.last(),
        Some(Sink::Nodes {
          last_emit: true,
          ..
        })
      );
      if should_sep {
        let site = match ngin_value(emit) {
          Some(NginValue::Emit { line, col }) => format!("{line}:{col}:sep"),
          _ => "guard:sep".to_string(),
        };
        self.append_vnode(
          VNode::host(region_id(&site, &self.loop_keys), sep.to_string()),
          false,
        );
      }
      self.emit_from(emit)?;
    }
    Ok(Flow::Normal)
  }

  fn walk_for(&mut self, node: &'tree DatumaState) -> Result<Flow, MaterializeError> {
    let [head, body] = &node.children[..] else {
      return Err(MaterializeError::Message(
        "for needs a head and a body".into(),
      ));
    };
    match head.children.first().and_then(core_value) {
      Some(CoreValue::Ident(name)) => {
        let Some(iterable) = head.children.get(1) else {
          return Err(MaterializeError::Message("for-in needs an iterable".into()));
        };
        let items = match self.interp.eval_operand(iterable)? {
          RuntimeValue::Array(items) => items,
          RuntimeValue::Dict(entries) => {
            entries.keys().cloned().map(RuntimeValue::String).collect()
          }
          RuntimeValue::String(text) => text
            .chars()
            .map(|ch| RuntimeValue::String(ch.to_string()))
            .collect(),
          other => {
            return Err(MaterializeError::Runtime(RuntimeError::from_kind(
              RuntimeErrorKind::NotIterable(other.kind()),
              Vec::new(),
            )));
          }
        };
        for item in items {
          self.interp.scope_mut().assign(name, item.clone());
          self.loop_keys.push((name.clone(), item.stringify()));
          let flow = self.walk_node(body);
          self.loop_keys.pop();
          match flow? {
            Flow::Normal => {}
            Flow::Break => return Ok(Flow::Normal),
            Flow::Return(value) => return Ok(Flow::Return(value)),
          }
        }
        Ok(Flow::Normal)
      }
      _ => {
        let [init, condition, step] = &head.children[..] else {
          return Err(MaterializeError::Message(
            "classic for needs three clauses".into(),
          ));
        };
        self.interp.eval_tokens(&init.children)?;
        for index in 0..MAX_LOOP_ITERATIONS {
          let admitted = if condition.children.is_empty() {
            true
          } else {
            self.interp.eval_tokens(&condition.children)?.truthy()
          };
          if !admitted {
            return Ok(Flow::Normal);
          } else {
            self.loop_keys.push(("_i".to_string(), index.to_string()));
            let flow = self.walk_node(body);
            self.loop_keys.pop();
            match flow? {
              Flow::Normal => {
                self.interp.eval_tokens(&step.children)?;
              }
              Flow::Break => return Ok(Flow::Normal),
              Flow::Return(value) => return Ok(Flow::Return(value)),
            }
          }
        }
        Err(MaterializeError::Runtime(RuntimeError::from_kind(
          RuntimeErrorKind::LoopLimitExceeded(MAX_LOOP_ITERATIONS),
          Vec::new(),
        )))
      }
    }
  }

  fn walk_if(&mut self, node: &'tree DatumaState) -> Result<Flow, MaterializeError> {
    let [condition, then_branch, tail @ ..] = &node.children[..] else {
      return Err(MaterializeError::Message(
        "if needs a condition and a branch".into(),
      ));
    };
    if self.interp.eval_operand(condition)?.truthy() {
      self.walk_node(then_branch)
    } else {
      match tail.first() {
        None => Ok(Flow::Normal),
        Some(next) => match core_value(next) {
          Some(CoreValue::ElseIf) => self.walk_if(next),
          Some(CoreValue::Else) => match next.children.first() {
            Some(body) => self.walk_node(body),
            None => Ok(Flow::Normal),
          },
          _ => self.walk_node(next),
        },
      }
    }
  }

  fn resolve_path(&mut self, node: &'tree DatumaState) -> Result<String, MaterializeError> {
    let mut out = String::new();
    for child in &node.children {
      match ngin_value(child) {
        Some(NginValue::Env { name }) => match self.interp.scope_mut().get(name).cloned() {
          Some(value) => out.push_str(&value.stringify()),
          None => {
            return Err(MaterializeError::Runtime(RuntimeError::from_kind(
              RuntimeErrorKind::UndefinedVariable(name.clone()),
              Vec::new(),
            )));
          }
        },
        Some(NginValue::PathLit { text }) => out.push_str(text),
        Some(NginValue::Interp) => {
          self.sinks.push(Sink::Nodes {
            children: Vec::new(),
            fires: 0,
            last_emit: false,
          });
          self.walk_node(child)?;
          if let Some(Sink::Nodes { children, .. }) = self.sinks.pop() {
            out.push_str(&VNode::flatten_all(&children));
          }
        }
        Some(NginValue::Emit { .. }) => out.push_str(&self.build_emit(child)?.flatten()),
        _ => {}
      }
    }
    Ok(out)
  }

  fn emit_from(&mut self, node: &'tree DatumaState) -> Result<(), MaterializeError> {
    let vnode = self.build_emit(node)?;
    self.append_vnode(vnode, true);
    Ok(())
  }

  fn plus_from(&mut self, node: &'tree DatumaState) -> Result<(), MaterializeError> {
    if let Some(vnode) = self.build_plus(node)? {
      self.append_vnode(vnode, true);
    }
    Ok(())
  }

  fn emit_site(node: &DatumaState) -> String {
    match ngin_value(node) {
      Some(NginValue::Emit { line, col } | NginValue::Plus { line, col }) => {
        format!("{line}:{col}")
      }
      _ => "emit".to_string(),
    }
  }

  fn build_emit(&mut self, node: &'tree DatumaState) -> Result<VNode, MaterializeError> {
    let site = Self::emit_site(node);
    match node.children.first() {
      Some(child) if matches!(ngin_value(child), Some(NginValue::Template { .. })) => {
        self.sinks.push(Sink::Nodes {
          children: Vec::new(),
          fires: 0,
          last_emit: false,
        });
        self.walk_template(child)?;
        match self.take_sink_frame(&site, false) {
          Some(frame) => Ok(frame),
          None => Ok(VNode::frame(region_id(&site, &self.loop_keys), Vec::new())),
        }
      }
      Some(child) => {
        let text = self.interp.eval_operand(child)?.stringify();
        Ok(VNode::host(region_id(&site, &self.loop_keys), text))
      }
      None => Ok(VNode::host(
        region_id(&site, &self.loop_keys),
        String::new(),
      )),
    }
  }

  fn build_plus(&mut self, node: &'tree DatumaState) -> Result<Option<VNode>, MaterializeError> {
    let site = Self::emit_site(node);
    self.sinks.push(Sink::Nodes {
      children: Vec::new(),
      fires: 0,
      last_emit: false,
    });
    if let Some(child) = node.children.first() {
      self.walk_template(child)?;
    }
    Ok(self.take_sink_frame(&site, true))
  }

  fn take_sink_frame(&mut self, site: &str, require_fires: bool) -> Option<VNode> {
    match self.sinks.pop() {
      Some(Sink::Nodes {
        children, fires, ..
      }) => {
        if require_fires && fires == 0 {
          None
        } else {
          Some(VNode::frame(region_id(site, &self.loop_keys), children))
        }
      }
      None => None,
    }
  }

  fn append_literal(&mut self, site: &str, text: String) {
    self.append_vnode(VNode::host(region_id(site, &self.loop_keys), text), false);
  }

  fn append_vnode(&mut self, node: VNode, emitted: bool) {
    match self.sinks.last_mut() {
      Some(Sink::Nodes {
        children,
        fires,
        last_emit,
      }) => {
        children.push(node);
        if emitted {
          *fires += 1;
          *last_emit = true;
        }
      }
      None => {
        if let Some(path) = self.current_path.clone() {
          self.files.entry(path).or_default().push(node);
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::common::EnvMap;
  use crate::core::exec::execute;
  use crate::core::modes::ProgramParseMode;
  use crate::core::{ParseFile, ParseStack, parse_stack};
  use crate::dtct::types::{AttrArg, DtctFact};
  use crate::ngin::parse::{load_def_ngin, parse_file};
  use lasso::ThreadedRodeo;
  use std::fs;
  use std::path::{Path, PathBuf};
  use std::time::{SystemTime, UNIX_EPOCH};
  use tinyvec::TinyVec;

  fn scratch(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("time")
      .as_nanos();
    let dir = std::env::temp_dir().join(format!("ngin-mat-{name}-{nanos}"));
    fs::create_dir_all(&dir).expect("dir");
    dir
  }

  fn out_file(inner: &str) -> String {
    format!("|$ROOT_DIRECTORY/out.ts>\n```\n{inner}\n```\n")
  }

  fn plus_source(a_len: &str, b_len: &str) -> String {
    out_file(&format!(
      "@{{\n  a = {a_len};\n  b = {b_len};\n  += ```\n    @{{ ?(a.length > 1)?\",\"=> ```first```\n       ?(b.length > 1)?\",\"=> ```second``` }}@\n  ```\n}}@"
    ))
  }

  fn models_source() -> String {
    "@{\nfor (model in dk.models) {\n    |$ROOT_DIRECTORY/@{ => model }@.ts>\n    ```\n    @{ => model }@\n    ```\n}\n}@\n".to_string()
  }

  fn models_db(with_post: bool) -> Arc<DtctDb> {
    let pool = ThreadedRodeo::new();
    let mut facts = vec![DtctFact {
      trait_name: None,
      model: pool.get_or_intern("User"),
      field: None,
      ty: None,
      attribute: None,
      args: TinyVec::new(),
    }];
    if with_post {
      facts.push(DtctFact {
        trait_name: None,
        model: pool.get_or_intern("Post"),
        field: None,
        ty: None,
        attribute: None,
        args: TinyVec::new(),
      });
    }
    Arc::new(DtctDb::build(pool, facts))
  }

  fn fields_db(email_type: &str, with_email: bool) -> Arc<DtctDb> {
    let pool = ThreadedRodeo::new();
    let user = pool.get_or_intern("User");
    let mut facts = vec![DtctFact {
      trait_name: None,
      model: user,
      field: Some(pool.get_or_intern("title")),
      ty: Some(pool.get_or_intern("text_type")),
      attribute: None,
      args: TinyVec::<[AttrArg; 2]>::new(),
    }];
    if with_email {
      facts.insert(
        0,
        DtctFact {
          trait_name: None,
          model: user,
          field: Some(pool.get_or_intern("email")),
          ty: Some(pool.get_or_intern(email_type)),
          attribute: None,
          args: TinyVec::new(),
        },
      );
    }
    Arc::new(DtctDb::build(pool, facts))
  }

  async fn materialize_source(dir: &Path, source: &str, db: Arc<DtctDb>) {
    let ngin = dir.join("t.ngin");
    fs::write(&ngin, source).expect("write ngin");
    let state = parse_file(&ngin).await.expect("parse");
    materialize(&state, db, dir.to_str().expect("dir")).expect("mat");
  }

  async fn run_plus(name: &str, a_len: &str, b_len: &str) -> String {
    let dir = scratch(name);
    materialize_source(&dir, &plus_source(a_len, b_len), Arc::new(DtctDb::empty())).await;
    fs::read_to_string(dir.join("out.ts")).expect("out")
  }

  async fn run_file(name: &str, inner: &str) -> String {
    let dir = scratch(name);
    materialize_source(&dir, &out_file(inner), Arc::new(DtctDb::empty())).await;
    fs::read_to_string(dir.join("out.ts")).expect("out")
  }

  #[tokio::test]
  async fn f_vanished_model_file_is_deleted() {
    let dir = scratch("f");
    materialize_source(&dir, &models_source(), models_db(true)).await;
    assert!(dir.join("User.ts").exists());
    assert!(dir.join("Post.ts").exists());
    materialize_source(&dir, &models_source(), models_db(false)).await;
    assert!(dir.join("User.ts").exists());
    assert!(!dir.join("Post.ts").exists());
  }

  #[tokio::test]
  async fn g_plus_second_run_cuts_fence() {
    let dir = scratch("g");
    materialize_source(
      &dir,
      &plus_source("[1, 2]", "[1, 2]"),
      Arc::new(DtctDb::empty()),
    )
    .await;
    let first = fs::read_to_string(dir.join("out.ts")).expect("out");
    assert!(first.contains("first,second"), "{first}");
    materialize_source(&dir, &plus_source("[1]", "[1]"), Arc::new(DtctDb::empty())).await;
    let out = fs::read_to_string(dir.join("out.ts")).expect("out2");
    assert!(!out.contains("first"), "{out}");
    assert!(!out.contains("second"), "{out}");
  }

  #[tokio::test]
  async fn h_plus_both_true_joins_with_sep() {
    let out = run_plus("h", "[1, 2]", "[1, 2]").await;
    assert!(out.contains("first,second"), "{out}");
  }

  #[tokio::test]
  async fn i_plus_only_second_has_no_leading_sep() {
    let out = run_plus("i", "[1]", "[1, 2]").await;
    assert!(out.contains("second"), "{out}");
    assert!(!out.contains(",second"), "{out}");
    assert!(!out.contains("first"), "{out}");
  }

  #[tokio::test]
  async fn c_d_field_regions_replace_and_cut() {
    let dir = scratch("cd");
    let source = out_file("@{\nfor (field in dk.fields) {\n  => field.type\n}\n}@");
    materialize_source(&dir, &source, fields_db("email_type", true)).await;
    let first = fs::read_to_string(dir.join("out.ts")).expect("out");
    assert!(first.contains("email_type"), "{first}");
    assert!(first.contains("text_type"), "{first}");
    materialize_source(&dir, &source, fields_db("new_type", true)).await;
    let replaced = fs::read_to_string(dir.join("out.ts")).expect("out2");
    assert!(replaced.contains("new_type"), "{replaced}");
    assert!(!replaced.contains("email_type"), "{replaced}");
    assert!(replaced.contains("text_type"), "{replaced}");
    materialize_source(&dir, &source, fields_db("new_type", false)).await;
    let cut = fs::read_to_string(dir.join("out.ts")).expect("out3");
    assert!(!cut.contains("email_type"), "{cut}");
    assert!(!cut.contains("new_type"), "{cut}");
    assert!(cut.contains("text_type"), "{cut}");
  }

  #[tokio::test]
  async fn nested_emit_keeps_unmarked_between_hosts() {
    let dir = scratch("nested-unmarked");
    let first = out_file("@{\nx = \"old\";\n=> ```pre @{ => x }@ post```\n}@");
    materialize_source(&dir, &first, Arc::new(DtctDb::empty())).await;
    let generated = fs::read_to_string(dir.join("out.ts")).expect("out");
    fs::write(
      dir.join("out.ts"),
      generated.replace("old post", "old /* wow */ post"),
    )
    .expect("edit");
    let second = out_file("@{\nx = \"new\";\n=> ```pre @{ => x }@ post```\n}@");
    materialize_source(&dir, &second, Arc::new(DtctDb::empty())).await;
    let out = fs::read_to_string(dir.join("out.ts")).expect("out2");
    assert!(out.contains("pre new /* wow */ post"), "{out}");
  }

  #[tokio::test]
  async fn flow_if_else_emits_taken_branch() {
    let out = run_file(
      "ifelse",
      "@{\nif (true) {\n  => ```foo```\n} else {\n  => ```bar```\n}\n}@",
    )
    .await;
    assert!(out.contains("foo"), "{out}");
    assert!(!out.contains("bar"), "{out}");
  }

  #[tokio::test]
  async fn flow_return_skips_later_emit() {
    let out = run_file(
      "ret",
      "keep\n@{\nif (true) { return }\n=> ```otherwise```\n}@\ntail",
    )
    .await;
    assert!(out.contains("keep"), "{out}");
    assert!(out.contains("tail"), "{out}");
    assert!(!out.contains("otherwise"), "{out}");
  }

  #[tokio::test]
  async fn flow_return_false_still_emits() {
    let out = run_file("retf", "@{\nif (false) { return }\n=> ```otherwise```\n}@").await;
    assert!(out.contains("otherwise"), "{out}");
  }

  #[tokio::test]
  async fn flow_break_stops_remaining_iters() {
    let out = run_file(
      "brk",
      "@{\nfor (x in [1, 2]) {\n  => ```item```\n  if (x == 1) { break }\n}\n}@",
    )
    .await;
    assert_eq!(out.matches("item").count(), 1, "{out}");
  }

  #[tokio::test]
  async fn flow_return_from_for_skips_after() {
    let out = run_file(
      "retfor",
      "@{\nfor (x in [1, 2]) {\n  if (x == 1) { return }\n  => ```item```\n}\n=> ```after```\n}@",
    )
    .await;
    assert!(!out.contains("item"), "{out}");
    assert!(!out.contains("after"), "{out}");
  }

  #[tokio::test]
  async fn flow_interp_swallows_return() {
    let out = run_file(
      "isw",
      "@{\n@{\nif (true) { return }\n=> ```inner```\n}@\n=> ```after```\n}@",
    )
    .await;
    assert!(!out.contains("inner"), "{out}");
    assert!(out.contains("after"), "{out}");
  }

  #[tokio::test]
  async fn def_ngin_ident_is_callable() {
    let dir = scratch("def-ident");
    let defs = load_def_ngin(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/ngin"))
      .await
      .unwrap_or_else(|err| panic!("{err}"));
    let ngin = dir.join("t.ngin");
    fs::write(&ngin, out_file("@{ => ident(\"ok\") }@")).expect("write");
    let state = parse_file(&ngin).await.expect("parse");
    materialize_with_defs(
      &state,
      &defs,
      Arc::new(DtctDb::empty()),
      dir.to_str().expect("dir"),
    )
    .expect("mat");
    let out = fs::read_to_string(dir.join("out.ts")).expect("out");
    assert!(out.contains("ok"), "{out}");
  }

  #[tokio::test]
  async fn def_ngin_nested_fn_is_callable() {
    let dir = scratch("def-nested");
    let defs = load_def_ngin(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/ngin"))
      .await
      .unwrap_or_else(|err| panic!("{err}"));
    let ngin = dir.join("t.ngin");
    fs::write(&ngin, out_file("@{ => nested_tag() }@")).expect("write");
    let state = parse_file(&ngin).await.expect("parse");
    materialize_with_defs(
      &state,
      &defs,
      Arc::new(DtctDb::empty()),
      dir.to_str().expect("dir"),
    )
    .expect("mat");
    let out = fs::read_to_string(dir.join("out.ts")).expect("out");
    assert!(out.contains("nested"), "{out}");
  }

  #[tokio::test]
  async fn def_fn_undefined_without_defs() {
    let dir = scratch("def-missing");
    let ngin = dir.join("t.ngin");
    fs::write(&ngin, out_file("@{ => ident(\"ok\") }@")).expect("write");
    let state = parse_file(&ngin).await.expect("parse");
    let err = materialize(
      &state,
      Arc::new(DtctDb::empty()),
      dir.to_str().expect("dir"),
    )
    .expect_err("ident should be undefined");
    let text = err.to_string();
    assert!(
      text.contains("UndefinedFunction") || text.to_lowercase().contains("undefined"),
      "{text}"
    );
  }

  async fn exec_def_return(expr: &str) -> RuntimeValue {
    let helpers =
      PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/ngin/defs/helpers.def.ngin");
    let source = format!(
      "{}\nreturn {expr};",
      fs::read_to_string(&helpers).expect("helpers")
    );
    let dir = scratch("def-exec");
    let path = dir.join("t.dk");
    fs::write(&path, source).expect("write");
    let mut file = ParseFile::open(path.to_str().expect("utf8"))
      .await
      .expect("open");
    let mut stack = ParseStack::with_root(Box::new(ProgramParseMode::new()));
    #[cfg(feature = "parse-trace")]
    {
      parse_stack(&mut stack, &mut file, None)
        .await
        .expect("parse");
    }
    #[cfg(not(feature = "parse-trace"))]
    {
      parse_stack(&mut stack, &mut file).await.expect("parse");
    }
    stack.dismiss_resolved();
    execute(
      &stack
        .into_root()
        .into_datuma_state()
        .expect("program state"),
    )
    .expect("exec")
    .returned
  }

  #[tokio::test]
  async fn title_case_hello_world() {
    assert_eq!(
      exec_def_return("title_case(\"hello_world\", \"_\")").await,
      RuntimeValue::String("Hello World".into())
    );
  }

  #[tokio::test]
  async fn title_case_collapses_lone_initials() {
    assert_eq!(
      exec_def_return("title_case(\"a_b_cd\", \"_\")").await,
      RuntimeValue::String("AB Cd".into())
    );
    assert_eq!(
      exec_def_return("title_case(\"x_y_z_cd\", \"_\")").await,
      RuntimeValue::String("XYZ Cd".into())
    );
  }

  #[tokio::test]
  async fn env_map_seeds_ngin_scope() {
    let dir = scratch("env-seed");
    let ngin = dir.join("t.ngin");
    fs::write(&ngin, "|$ROOT_DIRECTORY/${FOO}.txt>\n```\nok\n```\n").expect("write");
    let state = parse_file(&ngin).await.expect("parse");
    let env = EnvMap::from_vars([("FOO".into(), "bar".into())]);
    materialize_with_env(
      &state,
      &[],
      Arc::new(DtctDb::empty()),
      dir.to_str().expect("dir"),
      &env,
    )
    .expect("mat");
    let out = fs::read_to_string(dir.join("bar.txt")).expect("out");
    assert!(out.contains("ok"), "{out}");
  }
}
