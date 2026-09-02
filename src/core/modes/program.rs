use std::fmt::Display;
use std::write;

use crate::core::parser::{
  ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation, ParsetStepFlow,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;
#[path = "_stmt.rs"]
pub(crate) mod _stmt;

use _stmt::{KeywordFlags, is_statement_start};

#[derive(Debug)]
pub struct ProgramParseMode {
  instructions: Vec<DatumaState>,
  flags: KeywordFlags,
}

impl ProgramParseMode {
  pub fn new() -> Self {
    Self::with_flags(KeywordFlags::top_level())
  }

  pub fn with_flags(flags: KeywordFlags) -> Self {
    Self {
      instructions: Vec::new(),
      flags,
    }
  }
}

impl Display for ProgramParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/program")
  }
}

impl ParseMode for ProgramParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    if input.is_whitespace() {
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    } else if input == '}' {
      Ok((
        ParseStepMutation::CloseMode(self.close_state()),
        ParsetStepFlow::Captured,
      ))
    } else if input == ';' {
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    } else if is_statement_start(input) {
      Ok((
        ParseStepMutation::StartMode(Box::new(
          super::instruction::InstructionParseMode::with_flags(self.flags),
        )),
        ParsetStepFlow::Repropagate,
      ))
    } else {
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate))
    }
  }

  fn on_parse_resolved(&mut self, _input: char) -> ParseResolveStep {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }

  fn adopt(&mut self, child: DatumaState) {
    self.instructions.push(child);
  }

  fn close_state(&mut self) -> Option<DatumaState> {
    Some(DatumaState::node(
      Some(Box::new(CoreValue::Program)),
      std::mem::take(&mut self.instructions),
    ))
  }

  fn into_datuma_state(mut self: Box<Self>) -> Option<DatumaState> {
    self.close_state()
  }
}
