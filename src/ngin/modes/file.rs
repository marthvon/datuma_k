use std::fmt::Display;
use std::write;

use crate::core::parser::{
  ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation, ParsetStepFlow,
  expected, messages,
};
use crate::core::state::DatumaState;
use crate::ngin::value::NginValue;

use super::path::NginPathParseMode;
use super::template::NginFenceParseMode;

#[derive(Debug, PartialEq, Eq)]
enum FilePhase {
  Pipe,
  Path,
  AfterPath,
  Template,
}

#[derive(Debug)]
pub struct NginFileParseMode {
  children: Vec<DatumaState>,
  phase: FilePhase,
}

impl NginFileParseMode {
  pub fn new() -> Self {
    Self {
      children: Vec::new(),
      phase: FilePhase::Pipe,
    }
  }

  fn close_file(&mut self) -> DatumaState {
    DatumaState::node(
      Some(Box::new(NginValue::File)),
      std::mem::take(&mut self.children),
    )
  }
}

impl Display for NginFileParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/ngin/file")
  }
}

impl ParseMode for NginFileParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    match self.phase {
      FilePhase::Pipe => {
        if input == '|' {
          self.phase = FilePhase::Path;
          Ok((
            ParseStepMutation::StartMode(Box::new(NginPathParseMode::new())),
            ParsetStepFlow::Captured,
          ))
        } else {
          Err(expected(messages::NGIN_FILE))
        }
      }
      FilePhase::Path => Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate)),
      FilePhase::AfterPath => {
        if input == '>' {
          self.phase = FilePhase::Template;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Err(crate::core::parser::expected_closing(
            messages::NGIN_PATH_GT,
          ))
        }
      }
      FilePhase::Template => {
        if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input == '`' {
          Ok((
            ParseStepMutation::StartMode(Box::new(NginFenceParseMode::new(false))),
            ParsetStepFlow::Repropagate,
          ))
        } else {
          Err(expected(messages::NGIN_FENCE))
        }
      }
    }
  }

  fn on_parse_resolved(&mut self, _input: char) -> ParseResolveStep {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }

  fn adopt(&mut self, child: DatumaState) {
    match self.phase {
      FilePhase::Path => {
        self.children.push(child);
        self.phase = FilePhase::AfterPath;
      }
      FilePhase::Template => {
        self.children.push(child);
      }
      FilePhase::Pipe | FilePhase::AfterPath => {}
    }
  }

  fn close_after_adopt(&mut self) -> Option<DatumaState> {
    if self.phase == FilePhase::Template && self.children.len() == 2 {
      Some(self.close_file())
    } else {
      None
    }
  }
}
