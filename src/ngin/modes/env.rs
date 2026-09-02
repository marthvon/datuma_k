use std::fmt::Display;
use std::write;

use crate::core::common::starting_buf;
use crate::core::modes::{is_ident_continue, is_ident_start};
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow, expected, expected_closing, messages,
};
use crate::core::state::DatumaState;
use crate::ngin::value::NginValue;

#[derive(Debug, PartialEq, Eq)]
enum EnvPhase {
  Dollar,
  Name,
  Brace,
  BraceName,
}

#[derive(Debug)]
pub struct NginEnvParseMode {
  name: String,
  phase: EnvPhase,
}

impl NginEnvParseMode {
  pub fn new() -> Self {
    Self {
      name: String::new(),
      phase: EnvPhase::Dollar,
    }
  }

  fn close_env(&mut self) -> DatumaState {
    DatumaState::leaf(Box::new(NginValue::Env {
      name: std::mem::take(&mut self.name),
    }))
  }
}

impl Display for NginEnvParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/ngin/env")
  }
}

impl ParseMode for NginEnvParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    match self.phase {
      EnvPhase::Dollar => {
        if input == '$' {
          self.phase = EnvPhase::Name;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Err(expected(messages::NGIN_ENV))
        }
      }
      EnvPhase::Name => {
        if input == '{' {
          self.phase = EnvPhase::Brace;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if self.name.is_empty() && is_ident_start(input) {
          self.name = starting_buf(input);
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if is_ident_continue(input) {
          self.name.push(input);
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if self.name.is_empty() {
          Err(expected(messages::NGIN_ENV))
        } else {
          Ok((
            ParseStepMutation::CloseMode(Some(self.close_env())),
            ParsetStepFlow::Propagate,
          ))
        }
      }
      EnvPhase::Brace => {
        if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if is_ident_start(input) {
          self.name = starting_buf(input);
          self.phase = EnvPhase::BraceName;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Err(expected(messages::NGIN_ENV))
        }
      }
      EnvPhase::BraceName => {
        if is_ident_continue(input) {
          self.name.push(input);
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input == '}' {
          Ok((
            ParseStepMutation::CloseMode(Some(self.close_env())),
            ParsetStepFlow::Captured,
          ))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Err(expected_closing(messages::CLOSE_BRACE))
        }
      }
    }
  }

  fn on_parse_resolved(&mut self, _input: char) -> ParseResolveStep {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if state.is_some() {
      None
    } else {
      Some(expected(messages::NGIN_ENV))
    }
  }
}
