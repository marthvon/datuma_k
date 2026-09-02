use std::any::Any;
use std::fmt::Debug;

pub trait DatumaFinished: Debug + Send {
  fn kind(&self) -> &'static str;
  fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoreOperator {
  Add,
  Sub,
  Mul,
  Div,
  Mod,
  Xor,
  BitAnd,
  BitOr,
  Lt,
  Gt,
  Assign,
  Equal,
  NotEqual,
  LessEqual,
  GreaterEqual,
  And,
  Or,
  Increment,
  Decrement,
  Pow,
  PowAssign,
  AddAssign,
  SubAssign,
  MulAssign,
  DivAssign,
  ModAssign,
  XorAssign,
  AndAssign,
  AndAndAssign,
  OrAssign,
  OrOrAssign,
  SymmetricDiff,
  Intersect,
  RightDiff,
  LeftDiff,
  RightDiffAssign,
  LeftDiffAssign,
  Dot,
  Not,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreValue {
  Ident(String),
  String(String),
  Integer(String),
  Float(String),
  Double(String),
  Boolean(bool),
  Null,
  Operator(CoreOperator),
  Array,
  Dict,
  InvokedFunction(String),
  Grouped,
  Program,
  Instruction {
    file_meta: crate::core::source::ParseFileMetadata,
    pos_meta: crate::core::source::ParseCursorMetadata,
  },
  FunctionDef(String),
  If,
  Else,
  ElseIf,
  For,
  Accessor,
  Return,
  Break,
  Yield,
}

impl DatumaFinished for CoreValue {
  fn kind(&self) -> &'static str {
    match self {
      Self::Ident(_) => "ident",
      Self::String(_) => "string",
      Self::Integer(_) => "integer",
      Self::Float(_) => "float",
      Self::Double(_) => "double",
      Self::Boolean(_) => "boolean",
      Self::Null => "null",
      Self::Operator(_) => "operator",
      Self::Array => "array",
      Self::Dict => "dict",
      Self::InvokedFunction(_) => "invoked_function",
      Self::Grouped => "grouped",
      Self::Program => "program",
      Self::Instruction { .. } => "instruction",
      Self::FunctionDef(_) => "function_def",
      Self::If => "if",
      Self::Else => "else",
      Self::ElseIf => "elseif",
      Self::For => "for",
      Self::Accessor => "accessor",
      Self::Return => "return",
      Self::Break => "break",
      Self::Yield => "yield",
    }
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}
