use std::fmt::Display;
use std::{panic, vec, write};

use super::accessor::resolve_accessor;
use super::operator::{
  OperatorContext, OperatorParseMode, collection_follow_up, resolve_operators,
};
use super::{IdentifierParseMode, StringParseMode};
use crate::core::modes::{
  on_parse_capture_whitespace, on_resolve_capture_whitespace, start_core_value,
};
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow, expected, expected_closing,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DictPhase {
  Key,
  Colon,
  Value,
}

#[derive(Debug)]
pub struct DictParseMode {
  entries: Vec<DatumaState>,
  pending_key: Option<DatumaState>,
  phase: DictPhase,
  merge_pending: bool,
}

impl ParseMode for DictParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    on_parse_capture_whitespace(input).map_or_else(
      || match input {
        '}' => {
          if self.pending_key.is_some() {
            Err(match self.phase {
              DictPhase::Key | DictPhase::Colon => expected(messages::COLON),
              DictPhase::Value => expected(messages::DICT_VALUE),
            })
          } else {
            Ok((
              ParseStepMutation::CloseMode(Some(self.close_state())),
              ParsetStepFlow::Captured,
            ))
          }
        }
        ',' => {
          if self.entries.is_empty() || self.pending_key.is_some() {
            Err(match self.phase {
              DictPhase::Colon => expected(messages::COLON),
              DictPhase::Key | DictPhase::Value => ParseErrorKind::UnexpectedChar(','),
            })
          } else {
            if self.phase == DictPhase::Value {
              self.phase = DictPhase::Key;
            }
            Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
          }
        }
        _ => match self.phase {
          DictPhase::Key => self.on_key_phase(input),
          DictPhase::Colon => self.on_colon_phase(input),
          DictPhase::Value => self.on_value_phase(input),
        },
      },
      |v| Ok(v),
    )
  }

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    on_resolve_capture_whitespace(input).map_or_else(
      || {
        if input == '[' {
          resolve_accessor()
        } else if let Some(follow_up) = collection_follow_up(
          OperatorContext::Dict,
          input,
          std::mem::take(&mut self.entries),
        ) {
          self.merge_pending = true;
          match OperatorParseMode::from_char_with_follow_up(input, OperatorContext::Dict, follow_up)
          {
            Ok(mode) => Ok((
              ParseResolveMutation::NoDismissStartMode(Box::new(mode)),
              ParsetStepFlow::Captured,
            )),
            Err(e) => Err(e),
          }
        } else {
          resolve_operators(input, &[OperatorContext::Dict, OperatorContext::Ident])
        }
      },
      |v| Ok(v),
    )
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if self.merge_pending {
      Some(expected(messages::COLLECTION_OPERAND))
    } else if self.pending_key.is_some() {
      Some(match self.phase {
        DictPhase::Colon | DictPhase::Key => expected(messages::COLON),
        DictPhase::Value => expected(messages::DICT_VALUE),
      })
    } else if state.is_none() {
      Some(expected_closing(messages::CLOSE_BRACE))
    } else {
      None
    }
  }

  fn adopt(&mut self, child: DatumaState) {
    if self.merge_pending {
      self.entries.push(child);
    } else {
      match self.phase {
        DictPhase::Key => {
          self.pending_key = Some(child);
          self.phase = DictPhase::Colon;
        }
        DictPhase::Value => {
          let key = self
            .pending_key
            .take()
            .expect("dict value adopted without pending key");
          self.entries.push(DatumaState::node(None, vec![key, child]));
          self.phase = DictPhase::Key;
        }
        DictPhase::Colon => panic!("dict adopted child while waiting for colon"),
      }
    }
  }

  fn accepts_resolved_child(&self) -> bool {
    self.merge_pending
  }

  fn reactivate_after_child_close(&mut self) -> bool {
    self.merge_pending
  }

  fn close_after_adopt(&mut self) -> Option<DatumaState> {
    if !self.merge_pending {
      None
    } else if self.entries.last().is_some_and(is_operator_state) {
      None
    } else {
      self.merge_pending = false;
      Some(self.close_state())
    }
  }
}

impl DictParseMode {
  pub fn new() -> Self {
    Self {
      entries: Vec::new(),
      pending_key: None,
      phase: DictPhase::Key,
      merge_pending: false,
    }
  }

  pub fn continuing(entries: Vec<DatumaState>) -> Self {
    Self {
      entries,
      pending_key: None,
      phase: DictPhase::Key,
      merge_pending: false,
    }
  }

  fn close_state(&mut self) -> DatumaState {
    DatumaState::node(
      Some(Box::new(CoreValue::Dict)),
      std::mem::take(&mut self.entries),
    )
  }

  fn on_key_phase(&mut self, input: char) -> ParseStep {
    Ok((
      ParseStepMutation::StartMode(if input == '"' {
        Box::new(StringParseMode::new())
      } else if input == '_' || input.is_ascii_alphabetic() {
        Box::new(IdentifierParseMode::key_starting(input))
      } else {
        return Err(expected(messages::DICT_KEY));
      }),
      ParsetStepFlow::Captured,
    ))
  }

  fn on_colon_phase(&mut self, input: char) -> ParseStep {
    if input == ':' {
      self.phase = DictPhase::Value;
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    } else {
      Err(expected(messages::COLON))
    }
  }

  fn on_value_phase(&mut self, input: char) -> ParseStep {
    start_core_value(input).map_or_else(|| Err(expected(messages::DICT_VALUE)), |v| v)
  }
}

impl Display for DictParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/dict")
  }
}

fn is_operator_state(state: &DatumaState) -> bool {
  state
    .value
    .as_ref()
    .and_then(|value| value.as_any().downcast_ref::<CoreValue>())
    .is_some_and(|value| matches!(value, CoreValue::Operator(_)))
}
