use std::fmt::Display;
use std::write;

use super::accessor::resolve_accessor;
use super::operator::{OperatorContext, resolve_dot_operator, resolve_operators};
use crate::core::modes::{
  on_parse_capture_whitespace, on_resolve_capture_whitespace, start_core_value,
};
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow, expected_closing,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

#[derive(Debug)]
pub struct ArgParseMode {
  args: Vec<DatumaState>,
  comma_pending: bool,
}

impl ArgParseMode {
  pub fn new() -> Self {
    Self {
      args: Vec::new(),
      comma_pending: false,
    }
  }

  fn close_state(&mut self) -> DatumaState {
    if self.comma_pending {
      self.args.push(DatumaState::leaf(Box::new(CoreValue::Null)));
      self.comma_pending = false;
    }
    DatumaState::node(None, std::mem::take(&mut self.args))
  }

  fn apply_comma(&mut self) {
    if self.comma_pending || self.args.is_empty() {
      self.args.push(DatumaState::leaf(Box::new(CoreValue::Null)));
    }
    self.comma_pending = true;
  }
}

impl Display for ArgParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/args")
  }
}

impl ParseMode for ArgParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    if input.is_whitespace() {
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    } else {
      match input {
        ')' => Ok((
          ParseStepMutation::CloseMode(Some(self.close_state())),
          ParsetStepFlow::Repropagate,
        )),
        ',' => {
          self.apply_comma();
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        }
        _ => {
          start_core_value(input).map_or_else(|| Err(ParseErrorKind::UnexpectedChar(input)), |v| v)
        }
      }
    }
  }

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    if input.is_whitespace() {
      Ok((ParseResolveMutation::Nothing, ParsetStepFlow::Captured))
    } else {
      Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
    }
  }

  fn adopt(&mut self, child: DatumaState) {
    self.args.push(child);
    self.comma_pending = false;
  }
}

#[derive(Debug)]
pub struct CallParseMode {
  callee: String,
  args: Vec<DatumaState>,
  has_args: bool,
}

impl CallParseMode {
  pub fn new(callee: String) -> Self {
    Self {
      callee,
      args: Vec::new(),
      has_args: false,
    }
  }

  fn close_state(&mut self) -> DatumaState {
    DatumaState::node(
      Some(Box::new(CoreValue::InvokedFunction(std::mem::take(
        &mut self.callee,
      )))),
      std::mem::take(&mut self.args),
    )
  }
}

impl Display for CallParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/call")
  }
}

impl ParseMode for CallParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    on_parse_capture_whitespace(input).map_or_else(
      || {
        if input == ')' {
          Ok((
            ParseStepMutation::CloseMode(Some(self.close_state())),
            ParsetStepFlow::Captured,
          ))
        } else if !self.has_args {
          self.has_args = true;
          Ok((
            ParseStepMutation::StartMode(Box::new(ArgParseMode::new())),
            ParsetStepFlow::Repropagate,
          ))
        } else {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        }
      },
      |v| Ok(v),
    )
  }

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    match input {
      '[' => resolve_accessor(),
      '.' => resolve_dot_operator(),
      _ => on_resolve_capture_whitespace(input).map_or_else(
        || {
          resolve_operators(
            input,
            &[OperatorContext::InvokedFunction, OperatorContext::Ident],
          )
        },
        |v| Ok(v),
      ),
    }
  }

  fn adopt(&mut self, child: DatumaState) {
    self.args = child.children;
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if state.is_none() {
      Some(expected_closing(messages::CLOSE_PAREN))
    } else {
      None
    }
  }
}
