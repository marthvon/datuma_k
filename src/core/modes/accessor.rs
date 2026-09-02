use std::fmt::Display;
use std::write;

use super::operator::{OperatorContext, resolve_dot_operator, resolve_operators};
use super::program::_stmt::start_value;
use crate::core::modes::on_resolve_capture_whitespace;
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow, expected, expected_closing,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

const ACCESSOR_OPERATOR_CONTEXTS: &[OperatorContext] = &[
  OperatorContext::Ident,
  OperatorContext::Numeric,
  OperatorContext::InvokedFunction,
];

#[derive(Debug)]
pub struct AccessorParseMode {
  children: Vec<DatumaState>,
}

impl AccessorParseMode {
  pub fn new() -> Self {
    Self {
      children: Vec::new(),
    }
  }

  fn close_accessor(&mut self) -> DatumaState {
    DatumaState::node(
      Some(Box::new(CoreValue::Accessor)),
      std::mem::take(&mut self.children),
    )
  }
}

impl Display for AccessorParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/accessor")
  }
}

impl ParseMode for AccessorParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    if input == ']' {
      if self.children.is_empty() {
        Err(expected(messages::ACCESSOR_INDEX))
      } else {
        Ok((
          ParseStepMutation::CloseMode(Some(self.close_accessor())),
          ParsetStepFlow::Captured,
        ))
      }
    } else if input.is_whitespace() {
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    } else if self.children.last().is_some_and(|state| {
      state
        .value
        .as_ref()
        .and_then(|value| value.as_any().downcast_ref::<CoreValue>())
        .is_some_and(|value| matches!(value, CoreValue::Operator(_)))
    }) || self.children.is_empty()
    {
      start_value(input)
    } else {
      Err(expected_closing(messages::CLOSE_BRACKET))
    }
  }

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    match input {
      '[' => resolve_accessor(),
      '.' => resolve_dot_operator(),
      _ => on_resolve_capture_whitespace(input).map_or_else(
        || resolve_operators(input, ACCESSOR_OPERATOR_CONTEXTS),
        |v| Ok(v),
      ),
    }
  }

  fn adopt(&mut self, child: DatumaState) {
    self.children.push(child);
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if state.is_some() {
      None
    } else if self.children.is_empty() {
      Some(expected(messages::ACCESSOR_INDEX))
    } else {
      Some(expected_closing(messages::CLOSE_BRACKET))
    }
  }
}

pub(crate) fn resolve_accessor() -> ParseResolveStep {
  Ok((
    ParseResolveMutation::StartMode(Box::new(AccessorParseMode::new())),
    ParsetStepFlow::Captured,
  ))
}
