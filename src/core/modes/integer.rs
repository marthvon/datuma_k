use std::fmt::Display;
use std::write;

use super::double::DoubleParseMode;
use super::float::FloatParseMode;
use super::operator::{OperatorContext, resolve_operators};
use crate::core::common::{STARTING_BUF_CAPACITY, starting_buf};
use crate::core::modes::{on_resolve_capture_whitespace, on_resolve_dot_operator};
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveStep, ParseStep, ParseStepMutation, ParsetStepFlow,
  expected,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

#[derive(Debug)]
pub struct IntegerParseMode {
  buf: String,
}

impl Default for IntegerParseMode {
  fn default() -> Self {
    Self {
      buf: String::with_capacity(STARTING_BUF_CAPACITY),
    }
  }
}

impl IntegerParseMode {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn starting(ch: char) -> Self {
    Self {
      buf: starting_buf(ch),
    }
  }
}

impl Display for IntegerParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/integer")
  }
}

impl ParseMode for IntegerParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    if input.is_ascii_digit() || (self.buf.is_empty() && input == '-') {
      self.buf.push(input);
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    } else {
      match input {
        '.' => Ok((
          ParseStepMutation::ReplaceMode(Box::new(FloatParseMode::from_buf(std::mem::take(
            &mut self.buf,
          )))),
          ParsetStepFlow::Captured,
        )),
        'd' => {
          if self.buf.is_empty() || self.buf == "-" {
            Err(expected(messages::NUMERIC))
          } else {
            Ok((
              ParseStepMutation::ReplaceMode(Box::new(DoubleParseMode::from_buf(std::mem::take(
                &mut self.buf,
              )))),
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
    if self.buf.is_empty() || self.buf == "-" {
      None
    } else {
      Some(DatumaState::leaf(Box::new(CoreValue::Integer(
        std::mem::take(&mut self.buf),
      ))))
    }
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if state.is_none() && !self.buf.is_empty() {
      Some(expected(messages::NUMERIC))
    } else {
      None
    }
  }
}
