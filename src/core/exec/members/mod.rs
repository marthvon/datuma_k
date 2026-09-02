mod array;
mod dict;
mod host;
mod string;

use super::error::RuntimeErrorKind;
use super::value::{MemberHost, RuntimeValue};

macro_rules! member_kinds {
  (@call $fn:path, $inner:expr, ($($arg:expr),*)) => {
    $fn($inner, $($arg),*)
  };
  (
    $self:expr,
    null: $null:expr,
    $method:ident $args:tt for $($kind:ident),+;
    other: $other:expr
  ) => {
    pastey::paste! {
      match $self {
        Self::Null => $null,
        $(
          Self::$kind(inner) => member_kinds!(@call [<$kind:snake>]::$method, inner, $args),
        )+
        _ => $other,
      }
    }
  };
}

pub fn index(base: &RuntimeValue, key: &RuntimeValue) -> Result<RuntimeValue, RuntimeErrorKind> {
  match (base, key) {
    (RuntimeValue::Null, _) => Err(RuntimeErrorKind::NullReference("index")),
    (RuntimeValue::Array(items), RuntimeValue::Integer(position)) => {
      match usize::try_from(*position).ok().and_then(|at| items.get(at)) {
        Some(item) => Ok(item.clone()),
        None => Err(RuntimeErrorKind::IndexOutOfBounds {
          index: *position,
          len: items.len(),
        }),
      }
    }
    (RuntimeValue::String(text), RuntimeValue::Integer(position)) => {
      match usize::try_from(*position)
        .ok()
        .and_then(|at| text.chars().nth(at))
      {
        Some(ch) => Ok(RuntimeValue::String(ch.to_string())),
        None => Err(RuntimeErrorKind::IndexOutOfBounds {
          index: *position,
          len: text.chars().count(),
        }),
      }
    }
    (RuntimeValue::Dict(entries), RuntimeValue::String(key)) => match entries.get(key) {
      Some(value) => Ok(value.clone()),
      None => Err(RuntimeErrorKind::UnknownMember {
        kind: "dict",
        member: key.clone(),
      }),
    },
    _ => Err(RuntimeErrorKind::InvalidIndexType {
      base: base.kind(),
      index: key.kind(),
    }),
  }
}

pub fn property(base: &RuntimeValue, name: &str) -> Result<RuntimeValue, RuntimeErrorKind> {
  MemberHost::property(base, name)
}

/// Takes `&mut` so mutating members can be applied in place when the receiver
/// resolves to a variable path; callers with a temporary pass a local.
pub fn call(
  base: &mut RuntimeValue,
  name: &str,
  args: Vec<RuntimeValue>,
) -> Result<RuntimeValue, RuntimeErrorKind> {
  base.call_mut(name, args)
}

impl MemberHost for RuntimeValue {
  fn kind(&self) -> &'static str {
    match self {
      Self::Null => "null",
      Self::Boolean(_) => "boolean",
      Self::Integer(_) => "integer",
      Self::Float(_) => "float",
      Self::Double(_) => "double",
      Self::String(_) => "string",
      Self::Array(_) => "array",
      Self::Dict(_) => "dict",
      Self::Host(host) => host.kind(),
    }
  }

  fn display(&self) -> String {
    match self {
      Self::Null => String::new(),
      Self::Boolean(value) => value.to_string(),
      Self::Integer(value) => value.to_string(),
      Self::Float(value) => value.to_string(),
      Self::Double(value) => value.to_string(),
      Self::String(value) => value.clone(),
      Self::Array(items) => items.iter().map(Self::stringify).collect(),
      Self::Dict(_) => String::new(),
      Self::Host(host) => host.display(),
    }
  }

  fn property(&self, name: &str) -> Result<RuntimeValue, RuntimeErrorKind> {
    member_kinds!(
      self,
      null: Err(RuntimeErrorKind::NullReference("member access")),
      property(name) for String, Array, Dict, Host;
      other: Err(unknown(self.kind(), name))
    )
  }

  fn call(&self, name: &str, args: Vec<RuntimeValue>) -> Result<RuntimeValue, RuntimeErrorKind> {
    member_kinds!(
      self,
      null: Err(RuntimeErrorKind::NullReference("member call")),
      call(name, args) for Host;
      other: Err(unknown_call(self.kind(), name))
    )
  }

  fn call_mut(
    &mut self,
    name: &str,
    args: Vec<RuntimeValue>,
  ) -> Result<RuntimeValue, RuntimeErrorKind> {
    member_kinds!(
      self,
      null: Err(RuntimeErrorKind::NullReference("member call")),
      call(name, args) for Array, Dict, String, Host;
      other: Err(unknown_call(self.kind(), name))
    )
  }

  fn truthy(&self) -> bool {
    match self {
      Self::Null => false,
      Self::Boolean(value) => *value,
      Self::Integer(value) => *value != 0,
      Self::Float(value) => *value != 0.0,
      Self::Double(value) => *value != 0.0,
      Self::String(value) => !value.is_empty(),
      Self::Array(items) => !items.is_empty(),
      Self::Dict(entries) => !entries.is_empty(),
      Self::Host(host) => host.truthy(),
    }
  }
}

pub(super) fn unknown(kind: &'static str, name: &str) -> RuntimeErrorKind {
  RuntimeErrorKind::UnknownMember {
    kind,
    member: name.to_string(),
  }
}

pub(super) fn unknown_call(kind: &'static str, name: &str) -> RuntimeErrorKind {
  RuntimeErrorKind::UnknownMember {
    kind,
    member: format!("{name}()"),
  }
}

pub(super) fn arity(
  kind: &'static str,
  name: &str,
  expected: usize,
  got: usize,
) -> RuntimeErrorKind {
  RuntimeErrorKind::ArityMismatch {
    function: format!("{kind}.{name}"),
    expected,
    got,
  }
}
