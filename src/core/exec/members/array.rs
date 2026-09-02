use super::{arity, unknown, unknown_call};
use crate::core::exec::error::RuntimeErrorKind;
use crate::core::exec::value::RuntimeValue;

pub fn property(items: &[RuntimeValue], name: &str) -> Result<RuntimeValue, RuntimeErrorKind> {
  if name == "length" {
    Ok(RuntimeValue::Integer(items.len() as i64))
  } else {
    Err(unknown("array", name))
  }
}

pub fn call(
  items: &mut Vec<RuntimeValue>,
  name: &str,
  args: Vec<RuntimeValue>,
) -> Result<RuntimeValue, RuntimeErrorKind> {
  match name {
    "insert" => insert(items, args),
    "remove" => remove(items, args),
    _ => Err(unknown_call("array", name)),
  }
}

fn insert(
  items: &mut Vec<RuntimeValue>,
  args: Vec<RuntimeValue>,
) -> Result<RuntimeValue, RuntimeErrorKind> {
  match &args[..] {
    [value] => items.push(value.clone()),
    [RuntimeValue::Integer(position), value] => {
      match usize::try_from(*position)
        .ok()
        .filter(|at| *at <= items.len())
      {
        Some(at) => items.insert(at, value.clone()),
        None => {
          return Err(RuntimeErrorKind::IndexOutOfBounds {
            index: *position,
            len: items.len(),
          });
        }
      }
    }
    _ => return Err(arity("array", "insert", 1, args.len())),
  }
  Ok(RuntimeValue::Integer(items.len() as i64))
}

fn remove(
  items: &mut Vec<RuntimeValue>,
  args: Vec<RuntimeValue>,
) -> Result<RuntimeValue, RuntimeErrorKind> {
  match &args[..] {
    [] => items
      .pop()
      .ok_or(RuntimeErrorKind::IndexOutOfBounds { index: -1, len: 0 }),
    [RuntimeValue::Integer(position)] => {
      match usize::try_from(*position)
        .ok()
        .filter(|at| *at < items.len())
      {
        Some(at) => Ok(items.remove(at)),
        None => Err(RuntimeErrorKind::IndexOutOfBounds {
          index: *position,
          len: items.len(),
        }),
      }
    }
    _ => Err(arity("array", "remove", 0, args.len())),
  }
}
