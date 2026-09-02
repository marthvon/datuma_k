use super::mode::ParseMode;
use crate::core::common::Either;
use crate::core::source::{ParseCursorMetadata, ParseFileMetadata};
use crate::core::state::DatumaState;
use std::borrow::Cow;
use std::io::Error as IoError;
use std::write;

pub type ParseStep = Result<(ParseStepMutation, ParsetStepFlow), ParseErrorKind>;

pub type ParseResolveStep = Result<(ParseResolveMutation, ParsetStepFlow), ParseErrorKind>;

#[derive(PartialEq, Eq)]
pub enum ParsetStepFlow {
  Captured,
  Propagate,
  Repropagate,
}

pub enum ParseStepMutation {
  StartMode(Box<dyn ParseMode>),
  ReplaceMode(Box<dyn ParseMode>),
  CloseMode(Option<DatumaState>),
  CloseAndStartMode(Option<DatumaState>, Box<dyn ParseMode>),
  ParentForceDismissMode,
  ParentForceDismissAndStartMode(Box<dyn ParseMode>),
  Nothing,
}

pub enum ParseResolveMutation {
  Dismiss,
  StartMode(Box<dyn ParseMode>),
  NoDismissStartMode(Box<dyn ParseMode>),
  ParentForceDismissMode,
  ParentForceDismissAndStartMode(Box<dyn ParseMode>),
  Nothing,
}

#[derive(Debug)]
pub struct ParseError {
  pub file_meta: ParseFileMetadata,
  pub pos_meta: ParseCursorMetadata,
  pub curr_mode: Option<Box<dyn ParseMode>>,
  pub kind: ParseErrorKind,
}

#[derive(Debug)]
pub enum ParseErrorKind {
  UnexpectedChar(char),
  Expected(Cow<'static, str>),
  ExpectedClosingSyntax(Either<Cow<'static, str>, Vec<Cow<'static, str>>>),
  TooManyDecimalPlaces { max: usize },
  InternalInvariant(Cow<'static, str>),
  IoError(Box<IoError>),
}

impl ParseErrorKind {
  fn title(&self) -> &'static str {
    match self {
      Self::UnexpectedChar(_) => "Unexpected Char",
      Self::Expected(_) => "Expected",
      Self::ExpectedClosingSyntax(_) => "Expected Closing Syntax",
      Self::TooManyDecimalPlaces { .. } => "Too Many Decimal Places",
      Self::InternalInvariant(_) => "Internal Invariant",
      Self::IoError(_) => "Io Error",
    }
  }
}

impl<'inst> std::error::Error for ParseError {}
impl<'inst> std::fmt::Display for ParseError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "[{}] {} ({}) {}",
      self.kind.title(),
      self.file_meta,
      self.pos_meta,
      self.kind
    )?;
    if let Some(mode) = &self.curr_mode {
      write!(f, " /{mode}")?;
    }
    Ok(())
  }
}

impl<'inst> std::fmt::Display for ParseErrorKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::UnexpectedChar(ch) => write!(f, "unexpected character {ch}"),
      Self::Expected(label) => write!(f, "expected {label}"),
      Self::ExpectedClosingSyntax(expected) => {
        write!(f, "expected closing ")?;
        match expected {
          Either::Left(label) => write!(f, "{}", label.as_ref()),
          Either::Right(alts) => {
            for (i, alt) in alts.iter().enumerate() {
              if i == 0 {
                write!(f, "{}", alt.as_ref())
              } else {
                write!(f, " {}", alt.as_ref())
              }?
            }
            Ok(())
          }
        }
      }
      Self::IoError(err) => write!(f, "{}", err),
      Self::TooManyDecimalPlaces { max } => {
        write!(f, "too many decimal places (max {max})")
      }
      Self::InternalInvariant(msg) => write!(f, "internal parser invariant violated: {msg}"),
    }
  }
}
