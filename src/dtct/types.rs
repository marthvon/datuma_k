use std::write;

use lasso::Spur;
use tinyvec::TinyVec;

pub type FactId = u32;

#[derive(Debug, Clone, PartialEq)]
pub struct DtctFact {
  pub trait_name: Option<Spur>,
  pub model: Spur,
  pub field: Option<Spur>,
  pub ty: Option<Spur>,
  pub attribute: Option<Spur>,
  pub args: TinyVec<[AttrArg; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttrArg {
  Ident(Spur),
  String(Spur),
  Integer(i64),
  Float(f64),
  Boolean(bool),
  Null,
}

impl Default for AttrArg {
  fn default() -> Self {
    Self::Null
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dim {
  Trait,
  Model,
  Field,
  Type,
  Attribute,
}

impl Dim {
  pub fn label(self) -> &'static str {
    match self {
      Self::Trait => "trait",
      Self::Model => "model",
      Self::Field => "field",
      Self::Type => "type",
      Self::Attribute => "attribute",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
  pub dim: Dim,
  pub names: Vec<Spur>,
  pub exclude: bool,
}

impl Filter {
  pub fn r#in(dim: Dim, names: Vec<Spur>) -> Self {
    Self {
      dim,
      names,
      exclude: false,
    }
  }

  pub fn not(dim: Dim, names: Vec<Spur>) -> Self {
    Self {
      dim,
      names,
      exclude: true,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QueryFilter(pub Vec<Filter>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
  DuplicateFilterDim(Dim),
  EmptyFilter(Dim),
}

impl std::fmt::Display for QueryError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::DuplicateFilterDim(dim) => {
        write!(f, "duplicate {} filter", dim.label())
      }
      Self::EmptyFilter(dim) => write!(f, "empty {} filter", dim.label()),
    }
  }
}

impl std::error::Error for QueryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DtctDbError {
  DuplicateModel(Spur),
}

impl std::fmt::Display for DtctDbError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::DuplicateModel(_) => write!(f, "duplicate model"),
    }
  }
}

impl std::error::Error for DtctDbError {}
