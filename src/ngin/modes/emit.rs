use std::fmt::Display;
use std::write;

use crate::core::modes::InstructionParseMode;
use crate::core::parser::{
  ParseErrorSource, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep,
  ParseStepMutation, ParsetStepFlow, expected, messages,
};
use crate::core::state::DatumaState;
use crate::ngin::value::NginValue;

use super::template::NginFenceParseMode;

#[derive(Debug)]
pub struct NginEmitStarter {
  seen_eq: bool,
}

impl NginEmitStarter {
  pub fn new() -> Self {
    Self { seen_eq: false }
  }
}

impl Display for NginEmitStarter {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/ngin/gt")
  }
}

impl ParseMode for NginEmitStarter {
  fn on_parse(&mut self, input: char) -> ParseStep {
    if !self.seen_eq {
      if input == '=' {
        self.seen_eq = true;
        Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
      } else {
        Err(expected(messages::NGIN_EMIT))
      }
    } else if input == '>' {
      Ok((
        ParseStepMutation::ReplaceMode(Box::new(NginEmitParseMode::new())),
        ParsetStepFlow::Captured,
      ))
    } else {
      Err(expected(messages::NGIN_EMIT))
    }
  }

  fn on_parse_resolved(&mut self, _input: char) -> ParseResolveStep {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }
}

#[derive(Debug)]
pub struct NginEmitParseMode {
  children: Vec<DatumaState>,
  line: usize,
  col: usize,
}

impl NginEmitParseMode {
  pub fn new() -> Self {
    Self {
      children: Vec::new(),
      line: 0,
      col: 0,
    }
  }

  fn close_emit(&mut self) -> DatumaState {
    DatumaState::node(
      Some(Box::new(NginValue::Emit {
        line: self.line,
        col: self.col,
      })),
      std::mem::take(&mut self.children),
    )
  }
}

impl Display for NginEmitParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/ngin/emit")
  }
}

impl ParseMode for NginEmitParseMode {
  fn note_source(&mut self, source: &dyn ParseErrorSource) {
    if self.line == 0 && self.col == 0 {
      let pos = source.pos_meta();
      self.line = pos.line;
      self.col = pos.col;
    }
  }

  fn on_parse(&mut self, input: char) -> ParseStep {
    if !self.children.is_empty() {
      Ok((
        ParseStepMutation::CloseMode(Some(self.close_emit())),
        ParsetStepFlow::Propagate,
      ))
    } else if input.is_whitespace() {
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    } else if input == '`' {
      Ok((
        ParseStepMutation::StartMode(Box::new(NginFenceParseMode::new(false))),
        ParsetStepFlow::Repropagate,
      ))
    } else {
      Ok((
        ParseStepMutation::StartMode(Box::new(InstructionParseMode::new())),
        ParsetStepFlow::Repropagate,
      ))
    }
  }

  fn on_parse_resolved(&mut self, _input: char) -> ParseResolveStep {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }

  fn adopt(&mut self, child: DatumaState) {
    self.children.push(child);
  }

  fn close_after_adopt(&mut self) -> Option<DatumaState> {
    if self.children.is_empty() {
      None
    } else {
      Some(self.close_emit())
    }
  }
}
