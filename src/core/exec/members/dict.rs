use indexmap::IndexMap;

use super::{arity, unknown, unknown_call};
use crate::core::exec::error::RuntimeErrorKind;
use crate::core::exec::value::RuntimeValue;

pub fn property(
  entries: &IndexMap<String, RuntimeValue>,
  name: &str,
) -> Result<RuntimeValue, RuntimeErrorKind> {
  if name == "length" {
    Ok(RuntimeValue::Integer(entries.len() as i64))
  } else {
    match entries.get(name) {
      Some(value) => Ok(value.clone()),
      None => Err(unknown("dict", name)),
    }
  }
}

pub fn call(
  entries: &mut IndexMap<String, RuntimeValue>,
  name: &str,
  args: Vec<RuntimeValue>,
) -> Result<RuntimeValue, RuntimeErrorKind> {
  match name {
    "insert" => {
      let [RuntimeValue::String(key), value] = &args[..] else {
        return Err(arity("dict", "insert", 2, args.len()));
      };
      Ok(
        entries
          .insert(key.clone(), value.clone())
          .unwrap_or(RuntimeValue::Null),
      )
    }
    "remove" => {
      let [RuntimeValue::String(key)] = &args[..] else {
        return Err(arity("dict", "remove", 1, args.len()));
      };
      Ok(entries.shift_remove(key).unwrap_or(RuntimeValue::Null))
    }
    "asArray" if args.is_empty() => Ok(RuntimeValue::Array(
      entries
        .iter()
        .map(|(key, value)| {
          RuntimeValue::Array(vec![RuntimeValue::String(key.clone()), value.clone()])
        })
        .collect(),
    )),
    "asArray" => Err(arity("dict", "asArray", 0, args.len())),
    _ if entries.contains_key(name) => Err(RuntimeErrorKind::NotCallable(name.to_string())),
    _ => Err(unknown_call("dict", name)),
  }
}
