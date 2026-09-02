use std::fmt::Display;
use std::write;

use crate::core::modes::GroupedParseMode;
use crate::core::parser::{
  ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation, ParsetStepFlow,
  expected, expected_closing, messages,
};
use crate::core::state::DatumaState;
use crate::ngin::value::NginValue;

use super::emit::NginEmitStarter;

#[derive(Debug, PartialEq, Eq)]
enum GuardPhase {
  Q,
  Cond,
  AfterCond,
  SepQuote,
  SepBody,
  AfterSep,
  Emit,
}

#[derive(Debug)]
pub struct NginGuardParseMode {
  children: Vec<DatumaState>,
  sep: String,
  phase: GuardPhase,
}

impl NginGuardParseMode {
  pub fn new() -> Self {
    Self {
      children: Vec::new(),
      sep: String::new(),
      phase: GuardPhase::Q,
    }
  }

  fn close_guard(&mut self) -> DatumaState {
    DatumaState::node(
      Some(Box::new(NginValue::Guard {
        sep: std::mem::take(&mut self.sep),
      })),
      std::mem::take(&mut self.children),
    )
  }
}

impl Display for NginGuardParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/ngin/guard")
  }
}

impl ParseMode for NginGuardParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    match self.phase {
      GuardPhase::Q => {
        if input == '?' {
          self.phase = GuardPhase::Cond;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Err(expected(messages::NGIN_GUARD))
        }
      }
      GuardPhase::Cond => {
        if input == '(' {
          Ok((
            ParseStepMutation::StartMode(Box::new(GroupedParseMode::new())),
            ParsetStepFlow::Captured,
          ))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Err(expected_closing(messages::OPEN_PAREN))
        }
      }
      GuardPhase::AfterCond => {
        if input == '?' {
          self.phase = GuardPhase::SepQuote;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Err(expected(messages::NGIN_GUARD))
        }
      }
      GuardPhase::SepQuote => {
        if input == '"' {
          self.phase = GuardPhase::SepBody;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Err(expected(messages::DOUBLE_QUOTE))
        }
      }
      GuardPhase::SepBody => {
        if input == '"' {
          self.phase = GuardPhase::AfterSep;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          self.sep.push(input);
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        }
      }
      GuardPhase::AfterSep => {
        if input == '=' {
          self.phase = GuardPhase::Emit;
          Ok((
            ParseStepMutation::StartMode(Box::new(NginEmitStarter::new())),
            ParsetStepFlow::Repropagate,
          ))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Err(expected(messages::NGIN_EMIT))
        }
      }
      GuardPhase::Emit => Ok((
        ParseStepMutation::CloseMode(Some(self.close_guard())),
        ParsetStepFlow::Propagate,
      )),
    }
  }

  fn on_parse_resolved(&mut self, _input: char) -> ParseResolveStep {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }

  fn adopt(&mut self, child: DatumaState) {
    match self.phase {
      GuardPhase::Cond => {
        self.children.push(child);
        self.phase = GuardPhase::AfterCond;
      }
      GuardPhase::Emit | GuardPhase::AfterSep => {
        self.children.push(child);
        self.phase = GuardPhase::Emit;
      }
      GuardPhase::Q | GuardPhase::AfterCond | GuardPhase::SepQuote | GuardPhase::SepBody => {}
    }
  }

  fn close_after_adopt(&mut self) -> Option<DatumaState> {
    if self.phase == GuardPhase::Emit && self.children.len() >= 2 {
      Some(self.close_guard())
    } else {
      None
    }
  }
}
