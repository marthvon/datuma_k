use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

pub fn format_datuma_tree(state: &DatumaState) -> String {
  let mut out = String::new();
  format_node(state, 0, &mut out);
  out
}

fn format_node(state: &DatumaState, depth: usize, out: &mut String) {
  for _ in 0..depth {
    out.push_str("  ");
  }
  match &state.value {
    Some(value) => {
      if let Some(core) = value.as_any().downcast_ref::<CoreValue>() {
        out.push_str(&format_core_value(core));
      } else {
        out.push_str(value.kind());
      }
    }
    None => out.push_str("(anon)"),
  }
  out.push('\n');
  for child in &state.children {
    format_node(child, depth + 1, out);
  }
}

fn format_core_value(value: &CoreValue) -> String {
  match value {
    CoreValue::Ident(s) => format!("ident({s:?})"),
    CoreValue::String(s) => format!("string({s:?})"),
    CoreValue::Integer(s) => format!("integer({s:?})"),
    CoreValue::Float(s) => format!("float({s:?})"),
    CoreValue::Double(s) => format!("double({s:?})"),
    CoreValue::Boolean(b) => format!("boolean({b})"),
    CoreValue::Null => "null".to_string(),
    CoreValue::Operator(op) => format!("operator({op:?})"),
    CoreValue::Array => "array".to_string(),
    CoreValue::Dict => "dict".to_string(),
    CoreValue::InvokedFunction(s) => format!("invoked_function({s:?})"),
    CoreValue::Grouped => "grouped".to_string(),
    CoreValue::Program => "program".to_string(),
    CoreValue::Instruction {
      file_meta,
      pos_meta,
    } => format!("instruction({file_meta} @ {pos_meta})"),
    CoreValue::FunctionDef(s) => format!("function_def({s:?})"),
    CoreValue::If => "if".to_string(),
    CoreValue::Else => "else".to_string(),
    CoreValue::ElseIf => "elseif".to_string(),
    CoreValue::For => "for".to_string(),
    CoreValue::Accessor => "accessor".to_string(),
    CoreValue::Return => "return".to_string(),
    CoreValue::Break => "break".to_string(),
    CoreValue::Yield => "yield".to_string(),
  }
}
