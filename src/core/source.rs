use std::write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFileMetadata {
  pub absolute_path: String,
  pub filename: Option<String>,
  pub size: Option<u64>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ParseCursorMetadata {
  pub line: usize,
  pub col: usize,
  pub index: usize,
}

impl ParseFileMetadata {
  pub fn source(label: impl Into<String>) -> Self {
    Self {
      absolute_path: label.into(),
      filename: None,
      size: None,
    }
  }
}

impl ParseCursorMetadata {
  pub fn at(line: usize, col: usize) -> Self {
    Self {
      line,
      col,
      index: 0,
    }
  }
}

impl std::fmt::Display for ParseFileMetadata {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if let Some(filename) = &self.filename {
      write!(f, "|{}", filename)?;
    }
    if let Some(size) = self.size {
      write!(
        f,
        "{}{}b",
        if self.filename.is_some() { "; " } else { "|" },
        size
      )?;
    }
    write!(f, "|{}>", self.absolute_path)
  }
}

impl std::fmt::Display for ParseCursorMetadata {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "line {}, col {}, idx {}",
      self.line, self.col, self.index,
    )
  }
}
