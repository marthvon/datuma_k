use std::fmt::Display;
use std::write;

use crate::core::common::on_parse_capture_whitespace;
use crate::core::modes::{is_ident_continue, is_ident_start};
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow, expected_closing,
};
use crate::core::state::DatumaState;
use crate::dtct::value::DtctValue;

#[derive(Debug)]
enum TraitListPhase {
  ExpectItem,
  InIdent,
  AfterIdent,
}

#[derive(Debug)]
pub struct TraitListParseMode {
  names: Vec<String>,
  current: String,
  phase: TraitListPhase,
}

impl TraitListParseMode {
  pub fn new() -> Self {
    Self {
      names: Vec::new(),
      current: String::new(),
      phase: TraitListPhase::ExpectItem,
    }
  }

  fn finish_ident(&mut self) {
    self.names.push(std::mem::take(&mut self.current));
  }

  fn close_state(&mut self) -> DatumaState {
    DatumaState::leaf(Box::new(DtctValue::Traits(std::mem::take(&mut self.names))))
  }
}

impl Display for TraitListParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/dtct/traits")
  }
}

impl ParseMode for TraitListParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    match self.phase {
      TraitListPhase::ExpectItem => on_parse_capture_whitespace(input).map_or_else(
        || {
          if is_ident_start(input) {
            self.current.push(input);
            self.phase = TraitListPhase::InIdent;
            Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
          } else if input == ']' {
            Ok((
              ParseStepMutation::CloseMode(Some(self.close_state())),
              ParsetStepFlow::Captured,
            ))
          } else {
            Err(ParseErrorKind::UnexpectedChar(input))
          }
        },
        Ok,
      ),
      TraitListPhase::InIdent => match input {
        ',' => {
          self.finish_ident();
          self.phase = TraitListPhase::ExpectItem;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        }
        ']' => {
          self.finish_ident();
          Ok((
            ParseStepMutation::CloseMode(Some(self.close_state())),
            ParsetStepFlow::Captured,
          ))
        }
        _ => {
          if let Some(step) = on_parse_capture_whitespace(input) {
            self.finish_ident();
            self.phase = TraitListPhase::AfterIdent;
            Ok(step)
          } else if is_ident_continue(input) {
            self.current.push(input);
            Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
          } else {
            Err(ParseErrorKind::UnexpectedChar(input))
          }
        }
      },
      TraitListPhase::AfterIdent => on_parse_capture_whitespace(input).map_or_else(
        || {
          if input == ',' {
            self.phase = TraitListPhase::ExpectItem;
            Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
          } else if input == ']' {
            Ok((
              ParseStepMutation::CloseMode(Some(self.close_state())),
              ParsetStepFlow::Captured,
            ))
          } else {
            Err(expected_closing(messages::CLOSE_BRACKET))
          }
        },
        Ok,
      ),
    }
  }

  fn on_parse_resolved(&mut self, _input: char) -> ParseResolveStep {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if state.is_none() {
      Some(expected_closing(messages::CLOSE_BRACKET))
    } else {
      None
    }
  }
}
