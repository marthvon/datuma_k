use std::fmt::Display;
use std::write;

use super::double::DoubleParseMode;
use super::operator::{OperatorContext, resolve_operators};
use crate::core::common::STARTING_BUF_CAPACITY;
use crate::core::modes::{on_resolve_capture_whitespace, on_resolve_dot_operator};
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveStep, ParseStep, ParseStepMutation, ParsetStepFlow,
  expected,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

pub const MAX_FLOAT_FRAC_DIGITS: usize = 7;

#[derive(Debug)]
pub struct FloatParseMode {
  buf: String,
  frac_start: usize,
}

impl Default for FloatParseMode {
  fn default() -> Self {
    let mut buf = String::with_capacity(STARTING_BUF_CAPACITY);
    buf.push('.');
    Self { buf, frac_start: 1 }
  }
}

impl FloatParseMode {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn from_buf(mut buf: String) -> Self {
    buf.push('.');
    Self {
      frac_start: buf.len(),
      buf,
    }
  }

  /// `buf` always carries the `.` separator, so a lone `.` or `-.` is not yet a number.
  fn has_digits(&self) -> bool {
    self.buf.bytes().any(|byte| byte.is_ascii_digit())
  }
}

impl Display for FloatParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/float")
  }
}

impl ParseMode for FloatParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    if input.is_ascii_digit() {
      self.buf.push(input);
      if self.buf.len().saturating_sub(self.frac_start) > MAX_FLOAT_FRAC_DIGITS {
        Ok((
          ParseStepMutation::ReplaceMode(Box::new(DoubleParseMode::from_float(
            std::mem::take(&mut self.buf),
            self.frac_start,
          ))),
          ParsetStepFlow::Captured,
        ))
      } else {
        Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
      }
    } else {
      match input {
        'f' => {
          if let Some(state) = self.close_state() {
            Ok((
              ParseStepMutation::CloseMode(Some(state)),
              ParsetStepFlow::Captured,
            ))
          } else {
            Err(expected(messages::NUMERIC))
          }
        }
        'd' => {
          if !self.has_digits() {
            Err(expected(messages::NUMERIC))
          } else {
            Ok((
              ParseStepMutation::ReplaceMode(Box::new(DoubleParseMode::from_float(
                std::mem::take(&mut self.buf),
                self.frac_start,
              ))),
              ParsetStepFlow::Repropagate,
            ))
          }
        }
        _ => {
          if let Some(state) = self.close_state() {
            Ok((
              ParseStepMutation::CloseMode(Some(state)),
              ParsetStepFlow::Propagate,
            ))
          } else {
            Err(expected(messages::NUMERIC))
          }
        }
      }
    }
  }

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    on_resolve_dot_operator(input).map_or_else(
      || {
        on_resolve_capture_whitespace(input).map_or_else(
          || resolve_operators(input, &[OperatorContext::Numeric]),
          |v| Ok(v),
        )
      },
      |v| v,
    )
  }

  fn close_state(&mut self) -> Option<DatumaState> {
    if !self.has_digits() {
      None
    } else {
      Some(DatumaState::leaf(Box::new(CoreValue::Float(
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
