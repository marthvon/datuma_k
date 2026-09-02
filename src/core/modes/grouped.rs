use std::fmt::Display;
use std::write;

use super::accessor::resolve_accessor;
use super::operator::{OperatorContext, resolve_operators, start_operator};
use super::{
  ArrayParseMode, DictParseMode, FloatParseMode, IdentifierParseMode, IntegerParseMode,
  StringParseMode,
};
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow, expected_closing,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

#[derive(Debug)]
pub struct GroupedParseMode {
  children: Vec<DatumaState>,
}

impl GroupedParseMode {
  pub fn new() -> Self {
    Self {
      children: Vec::new(),
    }
  }

  fn close_state(&mut self) -> DatumaState {
    DatumaState::node(
      Some(Box::new(CoreValue::Grouped)),
      std::mem::take(&mut self.children),
    )
  }

  fn start_value(&self, input: char) -> ParseStep {
    match input {
      '(' => Err(expected_closing(messages::CLOSE_PAREN)),
      '[' => Ok((
        ParseStepMutation::StartMode(Box::new(ArrayParseMode::new())),
        ParsetStepFlow::Captured,
      )),
      '{' => Ok((
        ParseStepMutation::StartMode(Box::new(DictParseMode::new())),
        ParsetStepFlow::Captured,
      )),
      '"' => Ok((
        ParseStepMutation::StartMode(Box::new(StringParseMode::new())),
        ParsetStepFlow::Captured,
      )),
      '.' if self.children.is_empty() => Ok((
        ParseStepMutation::StartMode(Box::new(FloatParseMode::new())),
        ParsetStepFlow::Captured,
      )),
      ch if ch.is_ascii_digit() || ch == '-' => Ok((
        ParseStepMutation::StartMode(Box::new(IntegerParseMode::starting(ch))),
        ParsetStepFlow::Captured,
      )),
      ch if ch.is_ascii_alphabetic() || ch == '_' => Ok((
        ParseStepMutation::StartMode(Box::new(IdentifierParseMode::starting(ch))),
        ParsetStepFlow::Captured,
      )),
      _ => Err(expected_closing(messages::CLOSE_PAREN)),
    }
  }
}

impl Display for GroupedParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/grouped")
  }
}

impl ParseMode for GroupedParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    if input.is_whitespace() {
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    } else if input == ')' {
      Ok((
        ParseStepMutation::CloseMode(Some(self.close_state())),
        ParsetStepFlow::Captured,
      ))
    } else if self.children.is_empty() {
      self.start_value(input)
    } else {
      match start_operator(
        input,
        &[
          OperatorContext::InvokedFunction,
          OperatorContext::Ident,
          OperatorContext::Numeric,
        ],
      ) {
        Ok(step) => Ok(step),
        Err(ParseErrorKind::UnexpectedChar(_)) => self.start_value(input),
        Err(err) => Err(err),
      }
    }
  }

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    if input.is_whitespace() {
      Ok((ParseResolveMutation::Nothing, ParsetStepFlow::Captured))
    } else {
      match input {
        ')' => Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate)),
        '[' => resolve_accessor(),
        _ => resolve_operators(
          input,
          &[
            OperatorContext::InvokedFunction,
            OperatorContext::Ident,
            OperatorContext::Numeric,
          ],
        ),
      }
    }
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if state.is_none() {
      Some(expected_closing(messages::CLOSE_PAREN))
    } else {
      None
    }
  }

  fn adopt(&mut self, child: DatumaState) {
    self.children.push(child);
  }
}
