use std::fs;
use std::path::{Path, PathBuf};

use datuma_k::core::exec::{
  Execution, RuntimeError, RuntimeErrorKind, RuntimeValue, Step, StepEvent, execute, execute_traced,
};
use datuma_k::core::state::DatumaState;
use datuma_k::core::value::CoreOperator;

mod common;

use common::{core_dir, discover_fixtures, errors_dir, fixtures_in, parse_program};

/// Fixtures expected to run clean live at the top level; the ones that must
/// fail live in `errors/`.
fn fixture(name: &str) -> PathBuf {
  let top = core_dir().join(format!("{name}.dk"));
  if top.is_file() {
    top
  } else {
    errors_dir().join(format!("{name}.dk"))
  }
}

async fn run(name: &str) -> Result<Execution, RuntimeError> {
  run_path(&fixture(name)).await
}

async fn run_path(path: &Path) -> Result<Execution, RuntimeError> {
  execute(&parse_program(path).await)
}

async fn run_ok(name: &str) -> Execution {
  run(name)
    .await
    .unwrap_or_else(|e| panic!("runtime error in {name}: {e}"))
}

async fn run_err(name: &str) -> RuntimeError {
  match run(name).await {
    Err(err) => err,
    Ok(exec) => panic!("expected {name} to fail, returned {:?}", exec.returned),
  }
}

/// Renders values as source-like text so collection assertions stay readable.
fn render(value: &RuntimeValue) -> String {
  match value {
    RuntimeValue::Null => "null".to_string(),
    RuntimeValue::Boolean(flag) => flag.to_string(),
    RuntimeValue::Integer(number) => number.to_string(),
    RuntimeValue::Float(number) => format!("{number:?}f"),
    RuntimeValue::Double(number) => format!("{number:?}d"),
    RuntimeValue::String(text) => format!("{text:?}"),
    RuntimeValue::Array(items) => format!(
      "[{}]",
      items.iter().map(render).collect::<Vec<_>>().join(", ")
    ),
    RuntimeValue::Dict(entries) => format!(
      "{{{}}}",
      entries
        .iter()
        .map(|(key, value)| format!("{key}: {}", render(value)))
        .collect::<Vec<_>>()
        .join(", ")
    ),
    RuntimeValue::Host(host) => host.display(),
  }
}

fn assert_vars(exec: &Execution, expected: &[(&str, &str)]) {
  for (name, want) in expected {
    let Some(value) = exec.scope.get(name) else {
      panic!("variable {name} is unbound");
    };
    assert_eq!(&render(value), want, "variable {name}");
  }
}

#[tokio::test]
async fn arithmetic_precedence_and_promotion() {
  let exec = run_ok("arithmetic").await;
  assert_vars(
    &exec,
    &[
      ("sum", "2"),
      ("pow", "512"),
      ("mixf", "4.0f"),
      ("mixd", "2.0d"),
      ("promote", "2.5f"),
      ("frac", "1.0f"),
      ("cmp", "true"),
      ("neg", "-8"),
      ("bits", "1"),
      ("bor", "7"),
      ("bxor", "4"),
    ],
  );
}

#[tokio::test]
async fn string_operations() {
  let exec = run_ok("strings").await;
  assert_vars(
    &exec,
    &[
      ("cat", "\"ab\""),
      ("rep", "\"xyxyxy\""),
      ("len", "5"),
      ("eq", "true"),
      ("ne", "true"),
      ("ch", "\"b\""),
      ("up", "\"AB\""),
      ("low", "\"ab\""),
    ],
  );
}

#[tokio::test]
async fn string_rejects_arithmetic() {
  assert!(matches!(
    run_err("string_div").await.kind,
    RuntimeErrorKind::InvalidOperation {
      op: CoreOperator::Div,
      lhs: "string",
      rhs: "integer"
    }
  ));
  assert!(matches!(
    run_err("string_sub").await.kind,
    RuntimeErrorKind::InvalidOperation {
      op: CoreOperator::Sub,
      lhs: "string",
      rhs: "string"
    }
  ));
}

#[tokio::test]
async fn array_merge_and_set_operations() {
  let exec = run_ok("collections").await;
  assert_vars(
    &exec,
    &[
      ("lit", "[1]"),
      ("chain", "[1, 2]"),
      ("merge", "[1, 2, 2, 3]"),
      ("sub", "[1]"),
      ("sym", "[1, 3]"),
      ("inter", "[2]"),
      ("ldiff", "[1]"),
      ("rdiff", "[3]"),
      ("nested", "[[1, 2], [3]]"),
      ("grouped", "[3, 3]"),
    ],
  );
}

#[tokio::test]
async fn dict_merge_and_lookup() {
  let exec = run_ok("dicts").await;
  assert_vars(
    &exec,
    &[
      ("merged", "{a: 1, b: 9, c: 3}"),
      ("lit", "{a: 1}"),
      ("subd", "{a: 1}"),
      ("interd", "{b: 2}"),
      ("symd", "{a: 1, c: 3}"),
      ("pairs", "[[\"a\", 1], [\"b\", 2]]"),
      ("n", "2"),
      ("got", "1"),
      ("dot", "2"),
    ],
  );
}

#[tokio::test]
async fn builtin_members_mutate_in_place() {
  let exec = run_ok("members").await;
  assert_vars(
    &exec,
    &[
      ("n", "3"),
      ("pushed", "4"),
      ("popped", "4"),
      ("removed", "9"),
      ("xs", "[1, 2, 3]"),
      ("prev", "null"),
      ("gone", "1"),
      ("pairs", "[[\"b\", 2]]"),
      ("dn", "1"),
      ("d", "{b: 2}"),
    ],
  );
}

#[tokio::test]
async fn accessor_chains_and_compound_assignment() {
  let exec = run_ok("accessors").await;
  assert_vars(
    &exec,
    &[
      ("deep", "2"),
      ("after", "9"),
      ("db", "2"),
      ("dc", "3"),
      ("plus", "5"),
      ("inc", "13"),
      ("xs", "[[1, 9], [13, 4]]"),
      ("d", "{a: 1, b: 2, c: 3}"),
    ],
  );
}

#[tokio::test]
async fn accessor_errors() {
  assert!(matches!(
    run_err("index_oob").await.kind,
    RuntimeErrorKind::IndexOutOfBounds { index: 5, len: 1 }
  ));
  assert!(matches!(
    run_err("index_type").await.kind,
    RuntimeErrorKind::InvalidIndexType {
      base: "array",
      index: "string"
    }
  ));
  assert!(matches!(
    run_err("unknown_member").await.kind,
    RuntimeErrorKind::UnknownMember { kind: "array", .. }
  ));
}

#[tokio::test]
async fn control_flow_statements_and_ternary() {
  let exec = run_ok("control").await;
  assert_vars(
    &exec,
    &[
      ("out", "1"),
      ("tier", "2"),
      ("tern", "10"),
      ("tern2", "20"),
      ("total", "6"),
      ("count", "6"),
      ("cond_only", "1"),
      ("init_only", "7"),
      ("keys", "\"ab\""),
      ("chars", "3"),
      ("member_sum", "6"),
      ("chain_sum", "9"),
    ],
  );
}

#[tokio::test]
async fn functions_recurse_and_restore_shadowed_names() {
  let exec = run_ok("functions").await;
  assert_eq!(render(&exec.returned), "5");
  assert_vars(
    &exec,
    &[
      ("f5", "120"),
      ("sum", "5"),
      ("nothing", "null"),
      ("nested", "42"),
      ("shaded", "11"),
      ("after", "1"),
    ],
  );
  assert!(
    exec.scope.get("v").is_none(),
    "call frame parameters must be popped on return"
  );
}

#[tokio::test]
async fn assignment_inside_a_call_shadows_instead_of_writing_through() {
  let exec = run_ok("scope_frames").await;
  // 10 from reading the outer binding, 99 from reading the new local one.
  assert_vars(&exec, &[("sum", "109"), ("outer", "10")]);
  assert!(exec.scope.get("mine").is_none());
}

#[tokio::test]
async fn recursion_gives_each_level_its_own_bindings() {
  let exec = run_ok("scope_recursion").await;
  assert_vars(&exec, &[("total", "6")]);
  assert!(exec.scope.get("mine").is_none());
}

#[tokio::test]
async fn names_first_bound_in_a_call_do_not_outlive_it() {
  assert!(matches!(
    run_err("scope_destroyed").await.kind,
    RuntimeErrorKind::UndefinedVariable(name) if name == "shared"
  ));
}

#[tokio::test]
async fn function_call_errors() {
  assert!(matches!(
    run_err("arity").await.kind,
    RuntimeErrorKind::ArityMismatch {
      expected: 2,
      got: 1,
      ..
    }
  ));
  assert!(matches!(
    run_err("undefined_fn").await.kind,
    RuntimeErrorKind::UndefinedFunction(_)
  ));
  assert!(matches!(
    run_err("recursion_depth").await.kind,
    RuntimeErrorKind::StackOverflow { .. }
  ));
}

#[tokio::test]
async fn increment_and_compound_assignment() {
  let exec = run_ok("incdec").await;
  assert_vars(
    &exec,
    &[("post", "2"), ("pre", "3"), ("dec", "2"), ("n", "4")],
  );
}

#[tokio::test]
async fn logical_and_equality_operators() {
  let exec = run_ok("logic").await;
  assert_vars(
    &exec,
    &[
      ("andv", "false"),
      ("orv", "true"),
      ("xorv", "true"),
      ("nullor", "false"),
      ("nulleq", "true"),
      ("numeq", "true"),
      ("mix", "true"),
    ],
  );
}

fn json_object(fields: &[(&str, serde_json::Value)]) -> String {
  let parts = fields
    .iter()
    .map(|(key, value)| {
      format!(
        "{}:{}",
        serde_json::to_string(key).expect("json key"),
        serde_json::to_string(value).expect("json value")
      )
    })
    .collect::<Vec<_>>();
  format!("{{{}}}", parts.join(","))
}

fn scope_json(scope: &[(String, RuntimeValue)]) -> serde_json::Value {
  let mut map = serde_json::Map::new();
  for (name, value) in scope {
    map.insert(name.clone(), serde_json::Value::String(render(value)));
  }
  serde_json::Value::Object(map)
}

fn step_json(step: &Step) -> String {
  let mut fields = Vec::new();
  match &step.event {
    StepEvent::Assign { target, value } => {
      fields.push(("op", serde_json::json!("assign")));
      fields.push(("target", serde_json::json!(target)));
      fields.push(("value", serde_json::json!(render(value))));
    }
    StepEvent::Expression { value } => {
      fields.push(("op", serde_json::json!("expression")));
      fields.push(("value", serde_json::json!(render(value))));
    }
    StepEvent::Return { value } => {
      fields.push(("op", serde_json::json!("return")));
      fields.push(("value", serde_json::json!(render(value))));
    }
    StepEvent::Break => fields.push(("op", serde_json::json!("break"))),
    StepEvent::Branch { condition, taken } => {
      fields.push(("op", serde_json::json!("branch")));
      fields.push(("condition", serde_json::json!(condition)));
      fields.push(("taken", serde_json::json!(taken)));
    }
    StepEvent::Iteration {
      index,
      variable,
      element,
      condition,
    } => {
      fields.push(("op", serde_json::json!("iterate")));
      fields.push(("index", serde_json::json!(index)));
      if let Some(name) = variable {
        fields.push(("variable", serde_json::json!(name)));
      }
      if let Some(item) = element {
        fields.push(("element", serde_json::json!(render(item))));
      }
      if let Some(flag) = condition {
        fields.push(("condition", serde_json::json!(flag)));
      }
    }
    StepEvent::Failed { error } => {
      fields.push(("op", serde_json::json!("failed")));
      fields.push(("error", serde_json::json!(error.to_string())));
    }
  }
  fields.push((
    "fn",
    step
      .function
      .as_deref()
      .map_or(serde_json::Value::Null, |name| serde_json::json!(name)),
  ));
  fields.push(("stack", serde_json::json!(step.stack)));
  fields.push(("frame", serde_json::json!(step.frame)));
  fields.push(("scope", scope_json(&step.scope)));
  json_object(&fields)
}

fn snapshot(program: &DatumaState) -> String {
  let run = execute_traced(program);
  let mut out = String::new();
  for (index, step) in run.steps.iter().enumerate() {
    if index > 0 {
      out.push_str("\n\n");
    }
    out.push_str(&step_json(step));
  }
  if !out.is_empty() {
    out.push('\n');
  }
  out
}

/// Snapshots are debugging aids, not assertions, so they always regenerate.
/// One serial test owns the directory to avoid concurrent writes.
#[tokio::test]
async fn execution_snapshots() {
  let dir = core_dir().join(".output");
  fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("mkdir {}: {e}", dir.display()));
  for (name, path) in discover_fixtures() {
    let out = dir.join(format!("{name}.exec"));
    let text = snapshot(&parse_program(&path).await);
    fs::write(&out, text).unwrap_or_else(|e| panic!("write {}: {e}", out.display()));
  }
}

/// Every top-level fixture is meant to be a working program, so any runtime
/// error is a defect in the fixture or the evaluator.
#[tokio::test]
async fn top_level_fixtures_execute_cleanly() {
  let mut failures = Vec::new();
  for (name, path) in fixtures_in(&core_dir()) {
    if let Err(err) = run_path(&path).await {
      failures.push(format!("{name}: {err}"));
    }
  }
  assert!(failures.is_empty(), "fixtures failed to run: {failures:#?}");
}

#[tokio::test]
async fn error_fixtures_all_fail() {
  for (name, path) in fixtures_in(&errors_dir()) {
    match run_path(&path).await {
      Err(err) if matches!(err.kind, RuntimeErrorKind::MalformedTree(_)) => {
        panic!(
          "{name} hit an unhandled shape rather than a runtime error: {}",
          err.kind
        )
      }
      Err(_) => {}
      Ok(exec) => panic!("{name} was expected to fail, returned {:?}", exec.returned),
    }
  }
}

#[tokio::test]
async fn null_and_divide_by_zero_errors() {
  assert!(matches!(
    run_err("null_member").await.kind,
    RuntimeErrorKind::NullReference(_)
  ));
  assert!(matches!(
    run_err("null_arith").await.kind,
    RuntimeErrorKind::NullReference(_)
  ));
  assert!(matches!(
    run_err("divide_zero").await.kind,
    RuntimeErrorKind::DivideByZero
  ));
  assert!(matches!(
    run_err("for_in_not_iterable").await.kind,
    RuntimeErrorKind::NotIterable(_)
  ));
  let undefined = run_err("undefined_var").await;
  assert!(matches!(
    undefined.kind,
    RuntimeErrorKind::UndefinedVariable(_)
  ));
  let path = fixture("undefined_var");
  let file = undefined.file_meta.as_ref().expect("instruction span file");
  let pos = undefined.pos_meta.expect("instruction span position");
  assert!(
    file.absolute_path.ends_with("undefined_var.dk")
      || Path::new(&file.absolute_path) == path.as_path(),
    "expected fixture path in file_meta, got {}",
    file.absolute_path
  );
  assert_eq!((pos.line, pos.col), (1, 1));
}
