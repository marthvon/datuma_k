use std::fmt::Display;
use std::write;

use crate::core::parser::{
  ParseErrorSource, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep,
  ParseStepMutation, ParsetStepFlow, expected, messages,
};
use crate::core::state::DatumaState;
use crate::ngin::value::NginValue;

use super::template::NginFenceParseMode;

#[derive(Debug)]
pub struct NginPlusStarter {
  seen_plus: bool,
}

impl NginPlusStarter {
  pub fn new() -> Self {
    Self { seen_plus: false }
  }
}

impl Display for NginPlusStarter {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/ngin/peq")
  }
}

impl ParseMode for NginPlusStarter {
  fn on_parse(&mut self, input: char) -> ParseStep {
    if !self.seen_plus {
      if input == '+' {
        self.seen_plus = true;
        Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
      } else {
        Err(expected(messages::NGIN_PLUS))
      }
    } else if input == '=' {
      Ok((
        ParseStepMutation::ReplaceMode(Box::new(NginPlusParseMode::new())),
        ParsetStepFlow::Captured,
      ))
    } else {
      Err(expected(messages::NGIN_PLUS))
    }
  }

  fn on_parse_resolved(&mut self, _input: char) -> ParseResolveStep {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }
}

#[derive(Debug)]
pub struct NginPlusParseMode {
  children: Vec<DatumaState>,
  line: usize,
  col: usize,
}

impl NginPlusParseMode {
  pub fn new() -> Self {
    Self {
      children: Vec::new(),
      line: 0,
      col: 0,
    }
  }

  fn close_plus(&mut self) -> DatumaState {
    DatumaState::node(
      Some(Box::new(NginValue::Plus {
        line: self.line,
        col: self.col,
      })),
      std::mem::take(&mut self.children),
    )
  }
}

impl Display for NginPlusParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/ngin/plus")
  }
}

impl ParseMode for NginPlusParseMode {
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
        ParseStepMutation::CloseMode(Some(self.close_plus())),
        ParsetStepFlow::Propagate,
      ))
    } else if input.is_whitespace() {
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    } else if input == '`' {
      Ok((
        ParseStepMutation::StartMode(Box::new(NginFenceParseMode::new(true))),
        ParsetStepFlow::Repropagate,
      ))
    } else {
      Err(expected(messages::NGIN_FENCE))
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
      Some(self.close_plus())
    }
  }
}
