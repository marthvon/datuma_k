use std::borrow::Cow;

use super::Either;
use crate::core::parser::messages;
use crate::core::parser::ParseErrorKind;

pub fn expected(label: &'static str) -> ParseErrorKind {
  ParseErrorKind::Expected(Cow::Borrowed(label))
}

pub fn expected_closing(label: &'static str) -> ParseErrorKind {
  ParseErrorKind::ExpectedClosingSyntax(Either::Left(Cow::Borrowed(label)))
}

pub fn expected_root_close() -> ParseErrorKind {
  expected(messages::ROOT_MODE)
}

pub fn internal_invariant(msg: &'static str) -> ParseErrorKind {
  ParseErrorKind::InternalInvariant(Cow::Borrowed(msg))
}

pub fn too_many_decimal_places(max: usize) -> ParseErrorKind {
  ParseErrorKind::TooManyDecimalPlaces { max }
}
