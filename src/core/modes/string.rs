use std::fmt::Display;
use std::write;

use crate::core::modes::on_resolve_capture_whitespace;
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveStep, ParseStep, ParseStepMutation, ParsetStepFlow,
  expected,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

use super::accessor::resolve_accessor;
use super::operator::{OperatorContext, resolve_dot_operator, resolve_operators};

fn decode_escape(ch: char) -> Option<char> {
  match ch {
    '\\' => Some('\\'),
    '\'' => Some('\''),
    '"' => Some('"'),
    '?' => Some('?'),
    '0' => Some('\0'),
    'a' => Some('\u{7}'),
    'b' => Some('\u{8}'),
    'f' => Some('\u{c}'),
    'n' => Some('\n'),
    'r' => Some('\r'),
    't' => Some('\t'),
    'v' => Some('\u{b}'),
    'e' => Some('\u{1b}'),
    _ => None,
  }
}

#[derive(Debug)]
pub struct StringParseMode {
  buf: String,
  escaped: bool,
}

impl Default for StringParseMode {
  fn default() -> Self {
    Self {
      buf: String::with_capacity(32),
      escaped: false,
    }
  }
}

impl StringParseMode {
  pub fn new() -> Self {
    Self::default()
  }
}

impl Display for StringParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/string")
  }
}

impl ParseMode for StringParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    if self.escaped {
      self.escaped = false;
      if let Some(decoded) = decode_escape(input) {
        self.buf.push(decoded);
      } else {
        self.buf.push('\\');
        self.buf.push(input);
      }
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    } else {
      match input {
        '"' => Ok((
          ParseStepMutation::CloseMode(Some(DatumaState::leaf(Box::new(CoreValue::String(
            std::mem::take(&mut self.buf),
          ))))),
          ParsetStepFlow::Captured,
        )),
        '\\' => {
          self.escaped = true;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        }
        _ => {
          self.buf.push(input);
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        }
      }
    }
  }

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    on_resolve_capture_whitespace(input).map_or_else(
      || match input {
        '[' => resolve_accessor(),
        '.' => resolve_dot_operator(),
        _ => resolve_operators(input, &[OperatorContext::String]),
      },
      |v| Ok(v),
    )
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if state.is_none() {
      Some(expected(messages::DOUBLE_QUOTE))
    } else {
      None
    }
  }
}
