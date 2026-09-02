use super::{arity, unknown, unknown_call};
use crate::core::exec::error::RuntimeErrorKind;
use crate::core::exec::value::RuntimeValue;

pub fn property(text: &str, name: &str) -> Result<RuntimeValue, RuntimeErrorKind> {
  if name == "length" {
    Ok(RuntimeValue::Integer(text.chars().count() as i64))
  } else {
    Err(unknown("string", name))
  }
}

pub fn call(
  text: &mut String,
  name: &str,
  args: Vec<RuntimeValue>,
) -> Result<RuntimeValue, RuntimeErrorKind> {
  match name {
    "upper" if args.is_empty() => Ok(RuntimeValue::String(text.to_uppercase())),
    "upper" => Err(arity("string", "upper", 0, args.len())),
    "lower" if args.is_empty() => Ok(RuntimeValue::String(text.to_lowercase())),
    "lower" => Err(arity("string", "lower", 0, args.len())),
    _ => Err(unknown_call("string", name)),
  }
}
