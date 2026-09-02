use std::path::{Path, PathBuf};

use datuma_k::core::modes::ProgramParseMode;
use datuma_k::core::state::DatumaState;
use datuma_k::core::value::{CoreOperator, CoreValue};
use datuma_k::core::{ParseFile, ParseStack, parse_stack};

fn operators_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/core/operators")
}

fn fixture(name: &str) -> PathBuf {
  operators_dir().join(format!("{name}.dk"))
}

async fn parse_fixture(path: &Path) -> DatumaState {
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

async fn parse_ok(name: &str) -> bool {
  let path = fixture(name);
  let mut file = match ParseFile::open(path.to_str().unwrap()).await {
    Ok(file) => file,
    Err(_) => return false,
  };
  let mut stack = ParseStack::with_root(Box::new(ProgramParseMode::new()));
  parse_stack(&mut stack, &mut file).await.is_ok()
}

fn collect_operators(state: &DatumaState) -> Vec<CoreOperator> {
  let mut ops = Vec::new();
  collect_operators_rec(state, &mut ops);
  ops
}

fn collect_operators_rec(state: &DatumaState, ops: &mut Vec<CoreOperator>) {
  if let Some(value) = &state.value {
    if let Some(core) = value.as_any().downcast_ref::<CoreValue>() {
      if let CoreValue::Operator(op) = core {
        ops.push(*op);
      }
    }
  }
  for child in &state.children {
    collect_operators_rec(child, ops);
  }
}

async fn assert_operators(name: &str, expected: &[CoreOperator]) {
  let program = parse_fixture(&fixture(name)).await;
  assert_eq!(collect_operators(&program), expected, "fixture: {name}");
}

#[tokio::test]
async fn ident_compound_assign_operators_parse() {
  assert_operators("ident_pow_assign", &[CoreOperator::PowAssign]).await;
  assert_operators("ident_right_diff_assign", &[CoreOperator::RightDiffAssign]).await;
  assert_operators("ident_left_diff_assign", &[CoreOperator::LeftDiffAssign]).await;
  assert_operators("ident_and_and_assign", &[CoreOperator::AndAndAssign]).await;
  assert_operators("ident_or_or_assign", &[CoreOperator::OrOrAssign]).await;
}

#[tokio::test]
async fn ident_compound_operators_without_assign_parse() {
  assert_operators("ident_pow", &[CoreOperator::Pow]).await;
  assert_operators("ident_right_diff", &[CoreOperator::RightDiff]).await;
  assert_operators("ident_left_diff", &[CoreOperator::LeftDiff]).await;
  assert_operators("ident_and_and", &[CoreOperator::And]).await;
  assert_operators("ident_or_or", &[CoreOperator::Or]).await;
}

#[tokio::test]
async fn collection_diff_operators_parse() {
  assert_operators("ident_right_diff", &[CoreOperator::RightDiff]).await;
  assert_operators("ident_left_diff", &[CoreOperator::LeftDiff]).await;
}

#[tokio::test]
async fn array_dict_collection_operators_parse() {
  assert_operators("array_left_diff", &[CoreOperator::LeftDiff]).await;
  assert_operators("dict_left_diff", &[CoreOperator::LeftDiff]).await;
  assert_operators("array_right_diff", &[CoreOperator::RightDiff]).await;
  assert_operators("dict_right_diff", &[CoreOperator::RightDiff]).await;
  assert_operators("array_symmetric_diff", &[CoreOperator::SymmetricDiff]).await;
  assert_operators("array_intersect", &[CoreOperator::Intersect]).await;
  assert_operators("array_add", &[CoreOperator::Add]).await;
  assert_operators("dict_add", &[CoreOperator::Add]).await;
  assert_operators("array_sub", &[CoreOperator::Sub]).await;
  assert_operators("dict_sub_array", &[CoreOperator::Sub]).await;
  assert!(!parse_ok("array_and_and").await, "array does not allow &&");
  assert!(!parse_ok("dict_and_and").await, "dict does not allow &&");
}

#[tokio::test]
async fn nested_array_literal_parses() {
  assert!(
    parse_ok("nested_array_single").await,
    "nested array literal parses"
  );
  assert!(
    parse_ok("nested_array_trailing").await,
    "nested array with trailing element parses"
  );
}

#[tokio::test]
async fn multi_arg_call_parses() {
  assert!(
    parse_ok("call_two_args").await,
    "comma after numeric is a call-arg separator"
  );
  assert_eq!(
    call_arg_kinds("call_holes_middle").await,
    ["ident", "null", "null", "ident"]
  );
  assert_eq!(call_arg_kinds("call_leading_hole").await, ["null", "ident"]);
  assert_eq!(
    call_arg_kinds("call_trailing_comma").await,
    ["ident", "null"]
  );
  assert_eq!(call_arg_kinds("call_empty").await, Vec::<&str>::new());
}

#[tokio::test]
async fn grouped_allows_expression_tokens() {
  assert!(
    parse_ok("grouped_sum").await,
    "grouped holds a token stream so (1 + 2) and (a.length > 1) parse"
  );
  assert!(
    parse_ok("grouped_single").await,
    "single-value grouped is allowed"
  );
}

#[tokio::test]
async fn invoked_function_left_diff_without_assign_parses() {
  assert_operators("invoke_left_diff", &[CoreOperator::LeftDiff]).await;
  assert_operators("invoke_right_diff", &[CoreOperator::RightDiff]).await;
  assert_operators("invoke_add", &[CoreOperator::Add]).await;
  assert_operators("invoke_equal", &[CoreOperator::Equal]).await;
  assert_operators("invoke_increment", &[CoreOperator::Increment]).await;
  assert_operators("invoke_decrement", &[CoreOperator::Decrement]).await;
}

#[tokio::test]
async fn invoked_function_rejects_assign_operators() {
  assert!(
    !parse_ok("invoke_add_assign").await,
    "invoked function rejects +="
  );
  assert!(
    !parse_ok("invoke_sub_assign").await,
    "invoked function rejects -="
  );
  assert!(
    !parse_ok("invoke_mul_assign").await,
    "invoked function rejects *="
  );
  assert!(
    !parse_ok("invoke_div_assign").await,
    "invoked function rejects /="
  );
  assert!(
    !parse_ok("invoke_mod_assign").await,
    "invoked function rejects %="
  );
  assert!(
    !parse_ok("invoke_xor_assign").await,
    "invoked function rejects ^="
  );
  assert!(
    !parse_ok("invoke_bit_and_assign").await,
    "invoked function rejects &="
  );
  assert!(
    !parse_ok("invoke_bit_or_assign").await,
    "invoked function rejects |="
  );
  assert!(
    !parse_ok("invoke_right_diff_assign").await,
    "invoked function rejects ^&="
  );
  assert!(
    !parse_ok("invoke_left_diff_assign").await,
    "invoked function rejects &^="
  );
  assert!(
    !parse_ok("invoke_and_and_assign").await,
    "invoked function rejects &&="
  );
  assert!(
    !parse_ok("invoke_or_or_assign").await,
    "invoked function rejects ||="
  );
  assert!(
    !parse_ok("invoke_bare_assign").await,
    "invoked function rejects bare ="
  );
}

#[tokio::test]
async fn boolean_operators_parse() {
  assert_operators("bool_and", &[CoreOperator::And]).await;
  assert_operators("bool_or", &[CoreOperator::Or]).await;
  assert_operators("bool_xor", &[CoreOperator::Xor]).await;
  assert_operators("bool_equal", &[CoreOperator::Equal]).await;
  assert_operators("bool_not_equal", &[CoreOperator::NotEqual]).await;
}

#[tokio::test]
async fn null_literal_operators_parse() {
  assert_operators("null_and", &[CoreOperator::And]).await;
  assert_operators("null_or", &[CoreOperator::Or]).await;
  assert_operators("null_equal", &[CoreOperator::Equal]).await;
  assert_operators("null_not_equal", &[CoreOperator::NotEqual]).await;
}

#[tokio::test]
async fn null_rejects_xor() {
  assert!(!parse_ok("null_xor").await, "null context does not allow ^");
}

#[tokio::test]
async fn boolean_rejects_single_amp_and_pipe() {
  assert!(
    !parse_ok("bool_single_amp").await,
    "boolean context requires &&, not single &"
  );
  assert!(
    !parse_ok("bool_single_pipe").await,
    "boolean context requires ||, not single |"
  );
  assert!(
    !parse_ok("bool_bare_assign").await,
    "boolean context rejects bare ="
  );
}

#[tokio::test]
async fn string_operators_parse() {
  assert_operators("string_add", &[CoreOperator::Add]).await;
  assert_operators("string_mul", &[CoreOperator::Mul]).await;
  assert_operators("string_equal", &[CoreOperator::Equal]).await;
  assert_operators("string_not_equal", &[CoreOperator::NotEqual]).await;
}

#[tokio::test]
async fn numeric_comparison_and_assign_rules() {
  assert_operators("num_equal", &[CoreOperator::Equal]).await;
  assert_operators("num_not_equal", &[CoreOperator::NotEqual]).await;
  assert_operators("num_less_equal", &[CoreOperator::LessEqual]).await;
  assert_operators("num_greater_equal", &[CoreOperator::GreaterEqual]).await;
  assert_operators("num_lt", &[CoreOperator::Lt]).await;
  assert_operators("num_gt", &[CoreOperator::Gt]).await;
  assert!(
    !parse_ok("num_bare_assign").await,
    "numeric context rejects bare ="
  );
  assert!(
    !parse_ok("num_add_assign").await,
    "numeric context rejects +="
  );
}

#[tokio::test]
async fn member_dot_operator_parse() {
  assert_operators("member_dot", &[CoreOperator::Dot]).await;
  assert_operators("member_dot_call", &[CoreOperator::Dot]).await;
}

#[tokio::test]
async fn ident_postfix_increment_decrement_parse() {
  assert_operators("ident_increment", &[CoreOperator::Increment]).await;
  assert_operators("ident_decrement", &[CoreOperator::Decrement]).await;
}

#[tokio::test]
async fn incomplete_else_rejected() {
  assert!(
    !parse_ok("else_without_body").await,
    "else without body is rejected"
  );
  assert!(
    !parse_ok("else_without_brace").await,
    "else requires a brace body"
  );
}

#[tokio::test]
async fn structural_edge_rejects() {
  assert!(parse_ok("nested_dict").await, "nested dict value parses");
  assert!(!parse_ok("unary_bang").await, "unary bang rejected");
  assert!(!parse_ok("nested_grouped").await, "nested grouped rejected");
  assert!(!parse_ok("top_level_else").await, "top-level else rejected");
  assert!(!parse_ok("if_no_paren").await, "if without ( rejected");
  assert!(!parse_ok("fn_no_name").await, "fn without name rejected");
  assert!(
    !parse_ok("dict_numeric_key").await,
    "numeric dict key rejected"
  );
  assert!(
    !parse_ok("array_leading_comma").await,
    "array leading comma rejected"
  );
  assert!(!parse_ok("bool_member").await, "boolean member rejected");
  assert!(!parse_ok("for_no_body").await, "for without body rejected");
  assert!(!parse_ok("for_bad_in").await, "bad in keyword rejected");
}

#[tokio::test]
async fn remaining_path_rejects() {
  assert!(
    parse_ok("for_update_add_assign").await,
    "for update += parses via instruction reuse"
  );
  assert!(
    parse_ok("for_update_assign").await,
    "for update = parses via instruction reuse"
  );
  assert!(!parse_ok("null_member").await, "null member rejected");
  assert!(
    !parse_ok("double_frac_overflow").await,
    "too many double frac digits rejected"
  );
  assert!(
    !parse_ok("invoke_pow_assign").await,
    "invoked function rejects **="
  );
  assert!(
    !parse_ok("array_sub_dict").await,
    "array subtract expects array RHS"
  );
  assert!(
    !parse_ok("string_bare_assign").await,
    "string rejects bare ="
  );
  assert!(!parse_ok("null_single_amp").await, "null requires &&");
  assert!(!parse_ok("null_single_pipe").await, "null requires ||");
  assert!(
    !parse_ok("dict_missing_value").await,
    "dict missing value rejected"
  );
  assert!(
    !parse_ok("dict_leading_comma").await,
    "dict leading comma rejected"
  );
  assert!(
    !parse_ok("for_in_no_iter").await,
    "for-in without iterable rejected"
  );
  assert!(
    !parse_ok("elseif_no_paren").await,
    "elseif without ( rejected"
  );
}

#[tokio::test]
async fn accessor_index_parses() {
  let program = parse_fixture(&fixture("accessor_index")).await;
  assert!(collect_value_kinds(&program).contains(&"accessor"));
  assert!(parse_ok("string_accessor").await);
  assert!(parse_ok("call_accessor").await);
  assert!(parse_ok("dict_accessor").await);
  assert!(parse_ok("accessor_chain").await);
  assert!(parse_ok("array_merge_rhs").await);
  assert!(parse_ok("accessor_expr_index").await);
  assert!(!parse_ok("accessor_empty").await, "empty accessor rejected");
}

#[tokio::test]
async fn return_break_yield_parse() {
  assert!(parse_ok("return_value").await);
  assert!(parse_ok("return_in_fn").await);
  assert!(parse_ok("break_in_for").await);
  assert!(parse_ok("yield_if_else").await);
  assert!(
    parse_ok("stmt_after_if_no_else").await,
    "statement after if without else parses"
  );
  assert!(
    !parse_ok("yield_no_else").await,
    "yield without else rejected"
  );
  assert!(
    !parse_ok("yield_with_elseif").await,
    "yield with elseif rejected"
  );
  assert!(
    !parse_ok("break_with_expr").await,
    "break with expression rejected"
  );
  assert!(!parse_ok("empty_yield").await, "empty yield rejected");
}

#[tokio::test]
async fn null_and_boolean_literals_materialize_kinds() {
  let program = parse_fixture(&fixture("bool_null_literals")).await;
  let kinds = collect_value_kinds(&program);
  assert!(kinds.contains(&"boolean"));
  assert!(kinds.contains(&"null"));
}

async fn call_arg_kinds(name: &str) -> Vec<&'static str> {
  fn find_call(state: &DatumaState) -> Option<&DatumaState> {
    if state
      .value
      .as_ref()
      .and_then(|value| value.as_any().downcast_ref::<CoreValue>())
      .is_some_and(|value| matches!(value, CoreValue::InvokedFunction(_)))
    {
      Some(state)
    } else {
      state.children.iter().find_map(find_call)
    }
  }
  find_call(&parse_fixture(&fixture(name)).await)
    .expect("call")
    .children
    .iter()
    .map(|child| child.value.as_ref().expect("arg value").kind())
    .collect()
}

fn collect_value_kinds(state: &DatumaState) -> Vec<&'static str> {
  let mut kinds = Vec::new();
  collect_value_kinds_rec(state, &mut kinds);
  kinds
}

fn collect_value_kinds_rec(state: &DatumaState, kinds: &mut Vec<&'static str>) {
  if let Some(value) = &state.value {
    kinds.push(value.kind());
  }
  for child in &state.children {
    collect_value_kinds_rec(child, kinds);
  }
}
