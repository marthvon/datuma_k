use std::any::Any;

use crate::core::value::DatumaFinished;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NginValue {
  File,
  Path,
  Env { name: String },
  PathLit { text: String },
  Template { line: usize, col: usize },
  Interp,
  Emit { line: usize, col: usize },
  Plus { line: usize, col: usize },
  Guard { sep: String },
}

impl DatumaFinished for NginValue {
  fn kind(&self) -> &'static str {
    match self {
      Self::File => "ngin_file",
      Self::Path => "ngin_path",
      Self::Env { .. } => "ngin_env",
      Self::PathLit { .. } => "ngin_path_lit",
      Self::Template { .. } => "ngin_template",
      Self::Interp => "ngin_interp",
      Self::Emit { .. } => "ngin_emit",
      Self::Plus { .. } => "ngin_plus",
      Self::Guard { .. } => "ngin_guard",
    }
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}
