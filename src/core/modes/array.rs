use std::fmt::Display;
use std::write;

use super::accessor::resolve_accessor;
use super::operator::{
  OperatorContext, OperatorParseMode, collection_follow_up, resolve_operators,
};
use crate::core::modes::{on_parse_capture_whitespace, start_core_value};
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow, expected, expected_closing,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

#[derive(Debug)]
pub struct ArrayParseMode {
  children: Vec<DatumaState>,
  merge_pending: bool,
  placing_value: bool,
}

impl ArrayParseMode {
  pub fn new() -> Self {
    Self {
      children: Vec::new(),
      merge_pending: false,
      placing_value: false,
    }
  }

  pub fn continuing(children: Vec<DatumaState>) -> Self {
    Self {
      children,
      merge_pending: false,
      placing_value: false,
    }
  }

  fn close_state(&mut self) -> DatumaState {
    DatumaState::node(
      Some(Box::new(CoreValue::Array)),
      std::mem::take(&mut self.children),
    )
  }

  fn start_value(&self, input: char) -> ParseStep {
    start_core_value(input).map_or_else(|| Err(expected_closing(messages::CLOSE_BRACKET)), |v| v)
  }
}

impl Display for ArrayParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/array")
  }
}

impl ParseMode for ArrayParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    match input {
      ']' => Ok((
        ParseStepMutation::CloseMode(Some(self.close_state())),
        ParsetStepFlow::Captured,
      )),
      ',' => {
        if self.children.is_empty() || !self.placing_value {
          Err(ParseErrorKind::UnexpectedChar(','))
        } else {
          self.placing_value = false;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        }
      }
      _ => on_parse_capture_whitespace(input).map_or_else(
        || {
          let res = self.start_value(input)?;
          self.placing_value = true;
          Ok(res)
        },
        |v| Ok(v),
      ),
    }
  }

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    if input == '[' {
      resolve_accessor()
    } else if input.is_whitespace() {
      Ok((ParseResolveMutation::Nothing, ParsetStepFlow::Captured))
    } else if let Some(follow_up) = collection_follow_up(
      OperatorContext::Array,
      input,
      std::mem::take(&mut self.children),
    ) {
      self.merge_pending = true;
      match OperatorParseMode::from_char_with_follow_up(input, OperatorContext::Array, follow_up) {
        Ok(mode) => Ok((
          ParseResolveMutation::NoDismissStartMode(Box::new(mode)),
          ParsetStepFlow::Captured,
        )),
        Err(e) => Err(e),
      }
    } else {
      resolve_operators(input, &[OperatorContext::Array, OperatorContext::Ident])
    }
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    Some(if self.merge_pending {
      expected(messages::COLLECTION_OPERAND)
    } else if state.is_none() {
      expected_closing(messages::CLOSE_BRACKET)
    } else {
      return None;
    })
  }

  fn adopt(&mut self, child: DatumaState) {
    self.children.push(child);
  }

  fn accepts_resolved_child(&self) -> bool {
    self.merge_pending
  }

  fn reactivate_after_child_close(&mut self) -> bool {
    self.merge_pending
  }

  fn close_after_adopt(&mut self) -> Option<DatumaState> {
    if !self.merge_pending
      || self.children.last().is_some_and(|state| {
        state
          .value
          .as_ref()
          .and_then(|value| value.as_any().downcast_ref::<CoreValue>())
          .is_some_and(|value| matches!(value, CoreValue::Operator(_)))
      })
    {
      None
    } else {
      self.merge_pending = false;
      Some(self.close_state())
    }
  }
}
