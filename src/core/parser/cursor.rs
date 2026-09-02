use std::io::Error;
use std::path::PathBuf;
use std::rc::Rc;
use std::str::Chars;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader, Lines};

use super::stack::ParseStack;
use crate::core::parser::step::{ParseError, ParseErrorKind};

pub use crate::core::source::{ParseCursorMetadata, ParseFileMetadata};

pub async fn parse_stack(
  stack: &mut ParseStack,
  file: &mut ParseFile,
  #[cfg(feature = "parse-trace")] mut on_input: Option<&mut dyn FnMut(char)>,
) -> Result<(), ParseError> {
  loop {
    match file.line().await {
      Ok(None) => break Ok(()),
      Err(err) => break Err(file.error(ParseErrorKind::IoError(Box::new(err)))),
      #[cfg(feature = "parse-trace")]
      Ok(Some(line)) => parse_line(stack, &mut line.chars(), &mut on_input)?,
      #[cfg(not(feature = "parse-trace"))]
      Ok(Some(line)) => parse_line(stack, &mut line.chars())?,
    }
  }
}

fn parse_line(
  stack: &mut ParseStack,
  chars: &mut ParseCursorChar<'_>,
  #[cfg(feature = "parse-trace")] on_input: &mut Option<&mut dyn FnMut(char)>,
) -> Result<(), ParseError> {
  while let Some(ch) = chars.next() {
    #[cfg(feature = "parse-trace")]
    if let Some(on_input) = on_input.as_deref_mut() {
      on_input(ch);
    }
    parse_at(stack, chars, ch)?;
  }
  chars.advance_newline();
  #[cfg(feature = "parse-trace")]
  if let Some(on_input) = on_input.as_deref_mut() {
    on_input('\n');
  }
  parse_at(stack, chars, '\n')
}

fn parse_at(
  stack: &mut ParseStack,
  chars: &ParseCursorChar<'_>,
  ch: char,
) -> Result<(), ParseError> {
  stack.set_source(chars);
  let result = stack.parse(ch);
  stack.clear_source();
  result.map_err(|kind| chars.error(kind))
}

#[derive(Debug)]
pub struct ParseFile {
  path: PathBuf,
  lines: Lines<BufReader<File>>,
  meta: Rc<ParseCursorMetadata>,
}

#[derive(Debug)]
pub struct ParseCursorLine<'src> {
  file: &'src ParseFile,
  line: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParseCursorChar<'src> {
  file: &'src ParseFile,
  chars: Chars<'src>,
  meta: Rc<ParseCursorMetadata>,
}

pub trait ParseErrorSource {
  fn file_meta(&self) -> ParseFileMetadata;
  fn pos_meta(&self) -> ParseCursorMetadata;
  fn error(&self, kind: ParseErrorKind) -> ParseError {
    ParseError {
      file_meta: self.file_meta(),
      pos_meta: self.pos_meta(),
      curr_mode: None,
      kind,
    }
  }
}

impl ParseFile {
  pub async fn open(path_str: &str) -> Result<Self, Error> {
    let path = PathBuf::from(path_str).canonicalize()?;
    Ok(Self {
      lines: BufReader::new(File::open(&path).await?).lines(),
      path,
      meta: Rc::new(ParseCursorMetadata::default()),
    })
  }

  pub fn metadata(&self) -> ParseFileMetadata {
    ParseFileMetadata {
      absolute_path: self.path.to_string_lossy().to_string(),
      filename: self
        .path
        .file_name()
        .map(|os| os.to_string_lossy().into_owned()),
      size: self.path.metadata().ok().map(|meta| meta.len()),
    }
  }

  pub async fn line(&mut self) -> Result<Option<ParseCursorLine<'_>>, Error> {
    if let Some(line) = self.lines.next_line().await? {
      let meta = Rc::make_mut(&mut self.meta);
      meta.line += 1;
      meta.index += 1;
      meta.col = 0;
      Ok(Some(ParseCursorLine {
        file: self,
        line: Some(line),
      }))
    } else {
      Ok(None)
    }
  }
}

impl<'src> ParseCursorLine<'src> {
  pub fn chars(&self) -> ParseCursorChar<'_> {
    ParseCursorChar {
      file: self.file,
      chars: self.line.as_deref().unwrap_or("").chars(),
      meta: Rc::clone(&self.file.meta),
    }
  }
}

impl<'src> ParseCursorChar<'src> {
  pub fn next(&mut self) -> Option<char> {
    let ch = self.chars.next()?;
    let meta = Rc::make_mut(&mut self.meta);
    meta.col += 1;
    meta.index += 1;
    Some(ch)
  }

  fn advance_newline(&mut self) {
    let meta = Rc::make_mut(&mut self.meta);
    meta.col += 1;
    meta.index += 1;
  }
}

impl ParseErrorSource for ParseFile {
  fn file_meta(&self) -> ParseFileMetadata {
    self.metadata()
  }
  fn pos_meta(&self) -> ParseCursorMetadata {
    *self.meta
  }
}

impl ParseErrorSource for ParseCursorLine<'_> {
  fn file_meta(&self) -> ParseFileMetadata {
    self.file.metadata()
  }
  fn pos_meta(&self) -> ParseCursorMetadata {
    *self.file.meta
  }
}

impl ParseErrorSource for ParseCursorChar<'_> {
  fn file_meta(&self) -> ParseFileMetadata {
    self.file.metadata()
  }
  fn pos_meta(&self) -> ParseCursorMetadata {
    *self.meta
  }
}
