use std::fmt::Display;
use std::write;

use super::operator::{OperatorContext, start_operator};
use super::program::_stmt::{is_operator_char, start_value};
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow, expected,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

const JUMP_OPERATOR_CONTEXTS: &[OperatorContext] = &[
  OperatorContext::Ident,
  OperatorContext::Numeric,
  OperatorContext::InvokedFunction,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpKind {
  Return,
  Break,
  Yield,
}

#[derive(Debug)]
pub struct JumpParseMode {
  kind: JumpKind,
  parts: Vec<DatumaState>,
}

impl JumpParseMode {
  pub fn return_jump() -> Self {
    Self {
      kind: JumpKind::Return,
      parts: Vec::new(),
    }
  }

  pub fn break_jump() -> Self {
    Self {
      kind: JumpKind::Break,
      parts: Vec::new(),
    }
  }

  pub fn yield_jump() -> Self {
    Self {
      kind: JumpKind::Yield,
      parts: Vec::new(),
    }
  }

  fn value(&self) -> CoreValue {
    match self.kind {
      JumpKind::Return => CoreValue::Return,
      JumpKind::Break => CoreValue::Break,
      JumpKind::Yield => CoreValue::Yield,
    }
  }
}

impl Display for JumpParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self.kind {
      JumpKind::Return => write!(f, "/return"),
      JumpKind::Break => write!(f, "/break"),
      JumpKind::Yield => write!(f, "/yield"),
    }
  }
}

impl ParseMode for JumpParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    if input.is_whitespace() {
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    } else if input == ';' {
      if let Some(state) = self.close_state() {
        Ok((
          ParseStepMutation::CloseMode(Some(state)),
          ParsetStepFlow::Captured,
        ))
      } else {
        Err(expected(messages::YIELD_EXPR))
      }
    } else if input == '}' && !matches!(self.kind, JumpKind::Yield) {
      Ok((
        ParseStepMutation::CloseMode(self.close_state()),
        ParsetStepFlow::Propagate,
      ))
    } else if matches!(self.kind, JumpKind::Break) {
      Err(ParseErrorKind::UnexpectedChar(input))
    } else if self.parts.is_empty() && is_operator_char(input) && input != '-' {
      start_operator(input, JUMP_OPERATOR_CONTEXTS)
    } else {
      start_value(input)
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
    self.parts.push(child);
  }

  fn close_state(&mut self) -> Option<DatumaState> {
    if matches!(self.kind, JumpKind::Yield) && self.parts.is_empty() {
      None
    } else {
      Some(DatumaState::node(
        Some(Box::new(self.value())),
        std::mem::take(&mut self.parts),
      ))
    }
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if state.is_some() {
      None
    } else if matches!(self.kind, JumpKind::Yield) {
      Some(expected(messages::YIELD_EXPR))
    } else {
      None
    }
  }
}
