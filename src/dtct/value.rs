use std::any::Any;

use crate::core::value::DatumaFinished;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DtctValue {
  Attribute { name: String },
  Type { name: String },
  Field { name: String },
  Model { name: String, traits: Vec<String> },
  Traits(Vec<String>),
}

impl DatumaFinished for DtctValue {
  fn kind(&self) -> &'static str {
    match self {
      Self::Attribute { .. } => "attribute",
      Self::Type { .. } => "type",
      Self::Field { .. } => "field",
      Self::Model { .. } => "model",
      Self::Traits(_) => "traits",
    }
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}
