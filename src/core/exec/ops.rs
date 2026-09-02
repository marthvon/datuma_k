use std::sync::Arc;

use indexmap::IndexMap;

use super::error::RuntimeErrorKind;
use super::value::RuntimeValue;
use crate::core::value::CoreOperator;

/// Widest numeric kind shared by both operands: Integer -> Float -> Double.
enum Promoted {
  Integer(i64, i64),
  Float(f32, f32),
  Double(f64, f64),
}

pub fn binary(
  op: CoreOperator,
  lhs: RuntimeValue,
  rhs: RuntimeValue,
) -> Result<RuntimeValue, RuntimeErrorKind> {
  match op {
    CoreOperator::Equal => Ok(RuntimeValue::Boolean(equals(&lhs, &rhs))),
    CoreOperator::NotEqual => Ok(RuntimeValue::Boolean(!equals(&lhs, &rhs))),
    CoreOperator::And => logical(op, &lhs, &rhs, true),
    CoreOperator::Or => logical(op, &lhs, &rhs, false),
    _ => match (&lhs, &rhs) {
      (RuntimeValue::Null, _) | (_, RuntimeValue::Null) => {
        Err(RuntimeErrorKind::NullReference("operator"))
      }
      (RuntimeValue::String(left), _) => string_op(op, left, &rhs),
      (RuntimeValue::Boolean(left), RuntimeValue::Boolean(right)) => match op {
        CoreOperator::Xor => Ok(RuntimeValue::Boolean(left != right)),
        _ => Err(invalid(op, &lhs, &rhs)),
      },
      (RuntimeValue::Array(left), _) => array_op(op, left, &rhs),
      (RuntimeValue::Dict(left), _) => dict_op(op, left, &rhs),
      _ => numeric_op(op, &lhs, &rhs),
    },
  }
}

pub fn unary(op: CoreOperator, operand: RuntimeValue) -> Result<RuntimeValue, RuntimeErrorKind> {
  match (op, &operand) {
    (CoreOperator::Not, RuntimeValue::Boolean(_) | RuntimeValue::Null) => {
      Ok(RuntimeValue::Boolean(!operand.truthy()))
    }
    _ => Err(RuntimeErrorKind::InvalidUnaryOperation {
      op,
      operand: operand.kind(),
    }),
  }
}

pub fn step(op: CoreOperator, operand: RuntimeValue) -> Result<RuntimeValue, RuntimeErrorKind> {
  let delta = if op == CoreOperator::Decrement { -1 } else { 1 };
  match operand {
    RuntimeValue::Integer(value) => Ok(RuntimeValue::Integer(value + delta)),
    RuntimeValue::Float(value) => Ok(RuntimeValue::Float(value + delta as f32)),
    RuntimeValue::Double(value) => Ok(RuntimeValue::Double(value + delta as f64)),
    _ => Err(RuntimeErrorKind::InvalidUnaryOperation {
      op,
      operand: operand.kind(),
    }),
  }
}

/// Structural equality that promotes across numeric kinds, so `1 == 1.0`.
pub fn equals(lhs: &RuntimeValue, rhs: &RuntimeValue) -> bool {
  match (lhs, rhs) {
    (RuntimeValue::Null, RuntimeValue::Null) => true,
    (RuntimeValue::Boolean(left), RuntimeValue::Boolean(right)) => left == right,
    (RuntimeValue::String(left), RuntimeValue::String(right)) => left == right,
    (RuntimeValue::Array(left), RuntimeValue::Array(right)) => {
      left.len() == right.len() && left.iter().zip(right).all(|(a, b)| equals(a, b))
    }
    (RuntimeValue::Dict(left), RuntimeValue::Dict(right)) => {
      left.len() == right.len()
        && left
          .iter()
          .all(|(key, value)| right.get(key).is_some_and(|other| equals(value, other)))
    }
    (RuntimeValue::Host(left), RuntimeValue::Host(right)) => Arc::ptr_eq(left, right),
    (RuntimeValue::Host(left), RuntimeValue::String(right)) => left.display() == *right,
    (RuntimeValue::String(left), RuntimeValue::Host(right)) => *left == right.display(),
    _ => match promote(lhs, rhs) {
      Some(Promoted::Integer(left, right)) => left == right,
      Some(Promoted::Float(left, right)) => left == right,
      Some(Promoted::Double(left, right)) => left == right,
      None => false,
    },
  }
}

fn logical(
  op: CoreOperator,
  lhs: &RuntimeValue,
  rhs: &RuntimeValue,
  conjunction: bool,
) -> Result<RuntimeValue, RuntimeErrorKind> {
  if !matches!(lhs, RuntimeValue::Boolean(_) | RuntimeValue::Null)
    || !matches!(rhs, RuntimeValue::Boolean(_) | RuntimeValue::Null)
  {
    Err(invalid(op, lhs, rhs))
  } else if conjunction {
    Ok(RuntimeValue::Boolean(lhs.truthy() && rhs.truthy()))
  } else {
    Ok(RuntimeValue::Boolean(lhs.truthy() || rhs.truthy()))
  }
}

fn string_op(
  op: CoreOperator,
  left: &str,
  rhs: &RuntimeValue,
) -> Result<RuntimeValue, RuntimeErrorKind> {
  match (op, rhs) {
    (CoreOperator::Add, right) => Ok(RuntimeValue::String(format!("{left}{}", right.stringify()))),
    (CoreOperator::Mul, RuntimeValue::Integer(count)) if *count >= 0 => {
      Ok(RuntimeValue::String(left.repeat(*count as usize)))
    }
    _ => Err(RuntimeErrorKind::InvalidOperation {
      op,
      lhs: "string",
      rhs: rhs.kind(),
    }),
  }
}

fn array_op(
  op: CoreOperator,
  left: &[RuntimeValue],
  rhs: &RuntimeValue,
) -> Result<RuntimeValue, RuntimeErrorKind> {
  let RuntimeValue::Array(right) = rhs else {
    return Err(RuntimeErrorKind::InvalidOperation {
      op,
      lhs: "array",
      rhs: rhs.kind(),
    });
  };
  let contains = |items: &[RuntimeValue], probe: &RuntimeValue| -> bool {
    items.iter().any(|item| equals(item, probe))
  };
  let only_in_left = || -> Vec<RuntimeValue> {
    left
      .iter()
      .filter(|item| !contains(right, item))
      .cloned()
      .collect()
  };
  let only_in_right = || -> Vec<RuntimeValue> {
    right
      .iter()
      .filter(|item| !contains(left, item))
      .cloned()
      .collect()
  };
  match op {
    CoreOperator::Add => Ok(RuntimeValue::Array([left, right.as_slice()].concat())),
    CoreOperator::Sub | CoreOperator::LeftDiff => Ok(RuntimeValue::Array(only_in_left())),
    CoreOperator::RightDiff => Ok(RuntimeValue::Array(only_in_right())),
    CoreOperator::SymmetricDiff | CoreOperator::Xor => Ok(RuntimeValue::Array(
      [only_in_left(), only_in_right()].concat(),
    )),
    CoreOperator::Intersect | CoreOperator::BitAnd => Ok(RuntimeValue::Array(
      left
        .iter()
        .filter(|item| contains(right, item))
        .cloned()
        .collect(),
    )),
    _ => Err(RuntimeErrorKind::InvalidOperation {
      op,
      lhs: "array",
      rhs: "array",
    }),
  }
}

fn dict_op(
  op: CoreOperator,
  left: &IndexMap<String, RuntimeValue>,
  rhs: &RuntimeValue,
) -> Result<RuntimeValue, RuntimeErrorKind> {
  // `dict - array` treats the array as a key list; every other operand must be a dict.
  if let (CoreOperator::Sub | CoreOperator::LeftDiff, RuntimeValue::Array(keys)) = (op, rhs) {
    let mut result = left.clone();
    for key in keys {
      let RuntimeValue::String(key) = key else {
        return Err(RuntimeErrorKind::InvalidIndexType {
          base: "dict",
          index: key.kind(),
        });
      };
      result.shift_remove(key);
    }
    return Ok(RuntimeValue::Dict(result));
  }
  let RuntimeValue::Dict(right) = rhs else {
    return Err(RuntimeErrorKind::InvalidOperation {
      op,
      lhs: "dict",
      rhs: rhs.kind(),
    });
  };
  let filtered = |source: &IndexMap<String, RuntimeValue>,
                  other: &IndexMap<String, RuntimeValue>,
                  keep_shared: bool|
   -> IndexMap<String, RuntimeValue> {
    source
      .iter()
      .filter(|(key, _)| other.contains_key(*key) == keep_shared)
      .map(|(key, value)| (key.clone(), value.clone()))
      .collect()
  };
  match op {
    CoreOperator::Add => {
      let mut result = left.clone();
      for (key, value) in right {
        result.insert(key.clone(), value.clone());
      }
      Ok(RuntimeValue::Dict(result))
    }
    CoreOperator::Sub | CoreOperator::LeftDiff => {
      Ok(RuntimeValue::Dict(filtered(left, right, false)))
    }
    CoreOperator::RightDiff => Ok(RuntimeValue::Dict(filtered(right, left, false))),
    CoreOperator::SymmetricDiff | CoreOperator::Xor => {
      let mut result = filtered(left, right, false);
      result.extend(filtered(right, left, false));
      Ok(RuntimeValue::Dict(result))
    }
    CoreOperator::Intersect | CoreOperator::BitAnd => {
      Ok(RuntimeValue::Dict(filtered(left, right, true)))
    }
    _ => Err(RuntimeErrorKind::InvalidOperation {
      op,
      lhs: "dict",
      rhs: "dict",
    }),
  }
}

fn numeric_op(
  op: CoreOperator,
  lhs: &RuntimeValue,
  rhs: &RuntimeValue,
) -> Result<RuntimeValue, RuntimeErrorKind> {
  if matches!(
    op,
    CoreOperator::BitAnd | CoreOperator::BitOr | CoreOperator::Xor
  ) {
    return match (lhs, rhs) {
      (RuntimeValue::Integer(left), RuntimeValue::Integer(right)) => match op {
        CoreOperator::BitAnd => Ok(RuntimeValue::Integer(left & right)),
        CoreOperator::BitOr => Ok(RuntimeValue::Integer(left | right)),
        _ => Ok(RuntimeValue::Integer(left ^ right)),
      },
      _ => Err(invalid(op, lhs, rhs)),
    };
  }
  let Some(promoted) = promote(lhs, rhs) else {
    return Err(invalid(op, lhs, rhs));
  };
  if matches!(
    op,
    CoreOperator::Lt | CoreOperator::Gt | CoreOperator::LessEqual | CoreOperator::GreaterEqual
  ) {
    let ordering = match promoted {
      Promoted::Integer(left, right) => left.partial_cmp(&right),
      Promoted::Float(left, right) => left.partial_cmp(&right),
      Promoted::Double(left, right) => left.partial_cmp(&right),
    };
    let Some(ordering) = ordering else {
      return Ok(RuntimeValue::Boolean(false));
    };
    return Ok(RuntimeValue::Boolean(match op {
      CoreOperator::Lt => ordering.is_lt(),
      CoreOperator::Gt => ordering.is_gt(),
      CoreOperator::LessEqual => ordering.is_le(),
      _ => ordering.is_ge(),
    }));
  }
  match promoted {
    Promoted::Integer(left, right) => integer_arithmetic(op, left, right),
    Promoted::Float(left, right) => {
      float_arithmetic(op, left as f64, right as f64).map(|value| RuntimeValue::Float(value as f32))
    }
    Promoted::Double(left, right) => float_arithmetic(op, left, right).map(RuntimeValue::Double),
  }
}

fn integer_arithmetic(
  op: CoreOperator,
  left: i64,
  right: i64,
) -> Result<RuntimeValue, RuntimeErrorKind> {
  match op {
    CoreOperator::Add => Ok(RuntimeValue::Integer(left.wrapping_add(right))),
    CoreOperator::Sub => Ok(RuntimeValue::Integer(left.wrapping_sub(right))),
    CoreOperator::Mul => Ok(RuntimeValue::Integer(left.wrapping_mul(right))),
    CoreOperator::Div if right == 0 => Err(RuntimeErrorKind::DivideByZero),
    CoreOperator::Div => Ok(RuntimeValue::Integer(left.wrapping_div(right))),
    CoreOperator::Mod if right == 0 => Err(RuntimeErrorKind::DivideByZero),
    CoreOperator::Mod => Ok(RuntimeValue::Integer(left.wrapping_rem(right))),
    // A negative or overflowing exponent cannot stay integral, so it widens.
    CoreOperator::Pow => match u32::try_from(right)
      .ok()
      .and_then(|exp| left.checked_pow(exp))
    {
      Some(value) => Ok(RuntimeValue::Integer(value)),
      None => Ok(RuntimeValue::Double((left as f64).powf(right as f64))),
    },
    _ => Err(RuntimeErrorKind::InvalidOperation {
      op,
      lhs: "integer",
      rhs: "integer",
    }),
  }
}

fn float_arithmetic(op: CoreOperator, left: f64, right: f64) -> Result<f64, RuntimeErrorKind> {
  match op {
    CoreOperator::Add => Ok(left + right),
    CoreOperator::Sub => Ok(left - right),
    CoreOperator::Mul => Ok(left * right),
    CoreOperator::Div if right == 0.0 => Err(RuntimeErrorKind::DivideByZero),
    CoreOperator::Div => Ok(left / right),
    CoreOperator::Mod if right == 0.0 => Err(RuntimeErrorKind::DivideByZero),
    CoreOperator::Mod => Ok(left % right),
    CoreOperator::Pow => Ok(left.powf(right)),
    _ => Err(RuntimeErrorKind::InvalidOperation {
      op,
      lhs: "double",
      rhs: "double",
    }),
  }
}

fn promote(lhs: &RuntimeValue, rhs: &RuntimeValue) -> Option<Promoted> {
  match (lhs, rhs) {
    (RuntimeValue::Integer(left), RuntimeValue::Integer(right)) => {
      Some(Promoted::Integer(*left, *right))
    }
    (RuntimeValue::Double(_), _) | (_, RuntimeValue::Double(_)) => {
      Some(Promoted::Double(as_double(lhs)?, as_double(rhs)?))
    }
    _ => Some(Promoted::Float(as_float(lhs)?, as_float(rhs)?)),
  }
}

fn as_float(value: &RuntimeValue) -> Option<f32> {
  match value {
    RuntimeValue::Integer(inner) => Some(*inner as f32),
    RuntimeValue::Float(inner) => Some(*inner),
    RuntimeValue::Double(inner) => Some(*inner as f32),
    _ => None,
  }
}

fn as_double(value: &RuntimeValue) -> Option<f64> {
  match value {
    RuntimeValue::Integer(inner) => Some(*inner as f64),
    RuntimeValue::Float(inner) => Some(*inner as f64),
    RuntimeValue::Double(inner) => Some(*inner),
    _ => None,
  }
}

fn invalid(op: CoreOperator, lhs: &RuntimeValue, rhs: &RuntimeValue) -> RuntimeErrorKind {
  RuntimeErrorKind::InvalidOperation {
    op,
    lhs: lhs.kind(),
    rhs: rhs.kind(),
  }
}
