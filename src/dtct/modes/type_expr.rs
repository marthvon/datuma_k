use std::fmt::Display;
use std::write;

use super::attribute::AttributeParseMode;
use crate::core::common::{on_parse_capture_whitespace, starting_buf};
use crate::core::modes::{is_ident_continue, is_ident_start};
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow, expected_closing,
};
use crate::core::state::DatumaState;
use crate::dtct::value::DtctValue;

#[derive(Debug)]
enum TypePhase {
  Name,
  AfterName,
  Attributes,
}

#[derive(Debug)]
pub struct TypeExprParseMode {
  name: String,
  attributes: Vec<DatumaState>,
  phase: TypePhase,
}

impl TypeExprParseMode {
  pub fn starting(ch: char) -> Self {
    Self {
      name: starting_buf(ch),
      attributes: Vec::new(),
      phase: TypePhase::Name,
    }
  }
}

impl Display for TypeExprParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/dtct/type")
  }
}

impl ParseMode for TypeExprParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    match self.phase {
      TypePhase::Name => {
        if let Some(step) = on_parse_capture_whitespace(input) {
          self.phase = TypePhase::AfterName;
          Ok(step)
        } else if is_ident_continue(input) {
          self.name.push(input);
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input == '<' {
          self.phase = TypePhase::Attributes;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Err(expected_closing(messages::LESS_THAN))
        }
      }
      TypePhase::AfterName => on_parse_capture_whitespace(input).map_or_else(
        || {
          if input == '<' {
            self.phase = TypePhase::Attributes;
            Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
          } else {
            Err(expected_closing(messages::LESS_THAN))
          }
        },
        Ok,
      ),
      TypePhase::Attributes => on_parse_capture_whitespace(input).map_or_else(
        || {
          if input == ',' {
            Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
          } else if is_ident_start(input) {
            Ok((
              ParseStepMutation::StartMode(Box::new(AttributeParseMode::starting(input))),
              ParsetStepFlow::Captured,
            ))
          } else if input == '>' {
            Ok((
              ParseStepMutation::CloseMode(Some(DatumaState::node(
                Some(Box::new(DtctValue::Type {
                  name: std::mem::take(&mut self.name),
                })),
                std::mem::take(&mut self.attributes),
              ))),
              ParsetStepFlow::Captured,
            ))
          } else {
            Err(ParseErrorKind::UnexpectedChar(input))
          }
        },
        Ok,
      ),
    }
  }

  fn on_parse_resolved(&mut self, _input: char) -> ParseResolveStep {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }

  fn adopt(&mut self, child: DatumaState) {
    self.attributes.push(child);
  }
}
