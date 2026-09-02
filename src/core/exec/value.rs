use std::fmt::{Debug, Display};
use std::sync::Arc;

use indexmap::IndexMap;

use super::error::RuntimeErrorKind;

pub trait MemberHost: Send + Sync + Debug {
  fn kind(&self) -> &'static str;
  fn display(&self) -> String;
  fn property(&self, name: &str) -> Result<RuntimeValue, RuntimeErrorKind>;
  fn call(&self, name: &str, args: Vec<RuntimeValue>) -> Result<RuntimeValue, RuntimeErrorKind>;
  fn call_mut(
    &mut self,
    name: &str,
    args: Vec<RuntimeValue>,
  ) -> Result<RuntimeValue, RuntimeErrorKind> {
    self.call(name, args)
  }
  fn truthy(&self) -> bool {
    true
  }
}

#[derive(Debug, Clone)]
pub enum RuntimeValue {
  Null,
  Boolean(bool),
  Integer(i64),
  Float(f32),
  Double(f64),
  String(String),
  Array(Vec<RuntimeValue>),
  Dict(IndexMap<String, RuntimeValue>),
  Host(Arc<dyn MemberHost>),
}

impl PartialEq for RuntimeValue {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::Null, Self::Null) => true,
      (Self::Boolean(left), Self::Boolean(right)) => left == right,
      (Self::Integer(left), Self::Integer(right)) => left == right,
      (Self::Float(left), Self::Float(right)) => left == right,
      (Self::Double(left), Self::Double(right)) => left == right,
      (Self::String(left), Self::String(right)) => left == right,
      (Self::Array(left), Self::Array(right)) => left == right,
      (Self::Dict(left), Self::Dict(right)) => left == right,
      (Self::Host(left), Self::Host(right)) => Arc::ptr_eq(left, right),
      _ => false,
    }
  }
}

impl RuntimeValue {
  pub fn kind(&self) -> &'static str {
    MemberHost::kind(self)
  }

  pub fn truthy(&self) -> bool {
    MemberHost::truthy(self)
  }

  pub fn stringify(&self) -> String {
    MemberHost::display(self)
  }
}

impl Display for RuntimeValue {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.stringify())
  }
}
