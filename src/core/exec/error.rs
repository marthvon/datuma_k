use std::fmt::Display;
use std::write;

use crate::core::source::{ParseCursorMetadata, ParseFileMetadata};
use crate::core::value::CoreOperator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeErrorKind {
  InvalidOperation {
    op: CoreOperator,
    lhs: &'static str,
    rhs: &'static str,
  },
  InvalidUnaryOperation {
    op: CoreOperator,
    operand: &'static str,
  },
  UndefinedVariable(String),
  UndefinedFunction(String),
  NotCallable(String),
  ArityMismatch {
    function: String,
    expected: usize,
    got: usize,
  },
  NullReference(&'static str),
  UnknownMember {
    kind: &'static str,
    member: String,
  },
  IndexOutOfBounds {
    index: i64,
    len: usize,
  },
  InvalidIndexType {
    base: &'static str,
    index: &'static str,
  },
  DivideByZero,
  StackOverflow {
    depth: usize,
  },
  NotIterable(&'static str),
  NotAssignable,
  LoopLimitExceeded(usize),
  MalformedTree(&'static str),
}

/// ParseError-shaped wrapper: optional source location, call stack, and kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
  pub file_meta: Option<ParseFileMetadata>,
  pub pos_meta: Option<ParseCursorMetadata>,
  pub stack: Vec<String>,
  pub kind: RuntimeErrorKind,
}

impl RuntimeError {
  pub fn from_kind(kind: RuntimeErrorKind, stack: Vec<String>) -> Self {
    Self {
      file_meta: None,
      pos_meta: None,
      stack,
      kind,
    }
  }

  pub fn with_span(
    kind: RuntimeErrorKind,
    stack: Vec<String>,
    file_meta: ParseFileMetadata,
    pos_meta: ParseCursorMetadata,
  ) -> Self {
    Self {
      file_meta: Some(file_meta),
      pos_meta: Some(pos_meta),
      stack,
      kind,
    }
  }
}

impl RuntimeErrorKind {
  fn title(&self) -> &'static str {
    match self {
      Self::InvalidOperation { .. } => "Invalid Operation",
      Self::InvalidUnaryOperation { .. } => "Invalid Unary Operation",
      Self::UndefinedVariable(_) => "Undefined Variable",
      Self::UndefinedFunction(_) => "Undefined Function",
      Self::NotCallable(_) => "Not Callable",
      Self::ArityMismatch { .. } => "Arity Mismatch",
      Self::NullReference(_) => "Null Reference",
      Self::UnknownMember { .. } => "Unknown Member",
      Self::IndexOutOfBounds { .. } => "Index Out Of Bounds",
      Self::InvalidIndexType { .. } => "Invalid Index Type",
      Self::DivideByZero => "Divide By Zero",
      Self::StackOverflow { .. } => "Stack Overflow",
      Self::NotIterable(_) => "Not Iterable",
      Self::NotAssignable => "Not Assignable",
      Self::LoopLimitExceeded(_) => "Loop Limit Exceeded",
      Self::MalformedTree(_) => "Malformed Tree",
    }
  }
}

impl Display for RuntimeErrorKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::InvalidOperation { op, lhs, rhs } => {
        write!(f, "invalid operation {op:?} between {lhs} and {rhs}")
      }
      Self::InvalidUnaryOperation { op, operand } => {
        write!(f, "invalid unary operation {op:?} on {operand}")
      }
      Self::UndefinedVariable(name) => write!(f, "undefined variable {name}"),
      Self::UndefinedFunction(name) => write!(f, "undefined function {name}"),
      Self::NotCallable(name) => write!(f, "{name} is not callable"),
      Self::ArityMismatch {
        function,
        expected,
        got,
      } => write!(f, "{function} expects {expected} argument(s), got {got}"),
      Self::NullReference(context) => write!(f, "null reference during {context}"),
      Self::UnknownMember { kind, member } => write!(f, "{kind} has no member {member}"),
      Self::IndexOutOfBounds { index, len } => {
        write!(f, "index {index} out of bounds for length {len}")
      }
      Self::InvalidIndexType { base, index } => write!(f, "cannot index {base} with {index}"),
      Self::DivideByZero => write!(f, "division by zero"),
      Self::StackOverflow { depth } => write!(f, "call depth limit {depth} exceeded"),
      Self::NotIterable(kind) => write!(f, "{kind} is not iterable"),
      Self::NotAssignable => write!(f, "expression is not assignable"),
      Self::LoopLimitExceeded(limit) => write!(f, "loop exceeded {limit} iterations"),
      Self::MalformedTree(context) => write!(f, "malformed tree: {context}"),
    }
  }
}

impl Display for RuntimeError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "[{}] ", self.kind.title())?;
    if let (Some(file), Some(pos)) = (&self.file_meta, &self.pos_meta) {
      write!(f, "{file} ({pos}) ")?;
    }
    write!(f, "{}", self.kind)?;
    for frame in self.stack.iter().rev() {
      write!(f, "\n  in {frame}")?;
    }
    Ok(())
  }
}

impl std::error::Error for RuntimeError {}
impl std::error::Error for RuntimeErrorKind {}
