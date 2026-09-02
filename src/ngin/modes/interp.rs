use std::fmt::Display;
use std::write;

use crate::core::parser::{
  ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation, ParsetStepFlow,
  expected, expected_closing, messages,
};
use crate::core::state::DatumaState;
use crate::ngin::value::NginValue;

use super::glyph::{GlyphAllow, dispatch_ngin_char, start_instruction_or_propagate};

#[derive(Debug, PartialEq, Eq)]
enum InterpPhase {
  At,
  OpenBrace,
  Body,
  CloseAt,
}

#[derive(Debug)]
pub struct NginInterpParseMode {
  children: Vec<DatumaState>,
  phase: InterpPhase,
  allow: GlyphAllow,
}

impl NginInterpParseMode {
  pub fn in_body(allow: GlyphAllow) -> Self {
    Self {
      children: Vec::new(),
      phase: InterpPhase::Body,
      allow,
    }
  }

  pub fn opening(allow: GlyphAllow) -> Self {
    Self {
      children: Vec::new(),
      phase: InterpPhase::At,
      allow,
    }
  }

  fn close_interp(&mut self) -> DatumaState {
    DatumaState::node(
      Some(Box::new(NginValue::Interp)),
      std::mem::take(&mut self.children),
    )
  }
}

impl Display for NginInterpParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/ngin/interp")
  }
}

impl ParseMode for NginInterpParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    match self.phase {
      InterpPhase::At => {
        if input == '@' {
          self.phase = InterpPhase::OpenBrace;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Err(expected(messages::NGIN_INTERP))
        }
      }
      InterpPhase::OpenBrace => {
        if input == '{' {
          self.phase = InterpPhase::Body;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Err(expected_closing(messages::OPEN_BRACE))
        }
      }
      InterpPhase::Body => {
        if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input == '}' {
          self.phase = InterpPhase::CloseAt;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          dispatch_ngin_char(input, self.allow)
            .unwrap_or_else(|| start_instruction_or_propagate(input))
        }
      }
      InterpPhase::CloseAt => {
        if input == '@' {
          Ok((
            ParseStepMutation::CloseMode(Some(self.close_interp())),
            ParsetStepFlow::Captured,
          ))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Err(expected(messages::NGIN_INTERP))
        }
      }
    }
  }

  fn on_parse_resolved(&mut self, _input: char) -> ParseResolveStep {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }

  fn adopt(&mut self, child: DatumaState) {
    self.children.push(child);
  }
}
