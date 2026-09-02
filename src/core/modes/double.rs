use std::fmt::Display;
use std::write;

use super::operator::{OperatorContext, resolve_operators};
use crate::core::modes::on_resolve_capture_whitespace;
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveStep, ParseStep, ParseStepMutation, ParsetStepFlow,
  expected, too_many_decimal_places,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

pub const MAX_DOUBLE_FRAC_DIGITS: usize = 15;

#[derive(Debug)]
pub struct DoubleParseMode {
  buf: String,
  frac_start: usize,
}

impl DoubleParseMode {
  pub fn from_buf(buf: String) -> Self {
    Self {
      frac_start: buf.len(),
      buf,
    }
  }

  pub fn from_float(buf: String, frac_start: usize) -> Self {
    Self { buf, frac_start }
  }
}

impl Display for DoubleParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/double")
  }
}

impl ParseMode for DoubleParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    if input.is_ascii_digit() {
      self.buf.push(input);
      if self.buf.len().saturating_sub(self.frac_start) > MAX_DOUBLE_FRAC_DIGITS {
        Err(too_many_decimal_places(MAX_DOUBLE_FRAC_DIGITS))
      } else {
        Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
      }
    } else if let Some(state) = self.close_state() {
      Ok((
        ParseStepMutation::CloseMode(Some(state)),
        if input == 'd' {
          ParsetStepFlow::Captured
        } else {
          ParsetStepFlow::Propagate
        },
      ))
    } else {
      Err(expected(messages::NUMERIC))
    }
  }

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    on_resolve_capture_whitespace(input).map_or_else(
      || {
        on_resolve_capture_whitespace(input).map_or_else(
          || resolve_operators(input, &[OperatorContext::Numeric]),
          |v| Ok(v),
        )
      },
      |v| Ok(v),
    )
  }

  fn close_state(&mut self) -> Option<DatumaState> {
    if !self.buf.bytes().any(|byte| byte.is_ascii_digit()) {
      None
    } else {
      Some(DatumaState::leaf(Box::new(CoreValue::Double(
        std::mem::take(&mut self.buf),
      ))))
    }
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if state.is_none() {
      Some(expected(messages::NUMERIC))
    } else {
      None
    }
  }
}
