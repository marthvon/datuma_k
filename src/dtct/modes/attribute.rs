use std::fmt::Display;
use std::write;

use crate::core::common::{on_parse_capture_whitespace, starting_buf};
use crate::core::modes::{
  IdentifierParseMode, IntegerParseMode, StringParseMode, is_ident_continue, is_ident_start,
};
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow, expected_closing,
};
use crate::core::state::DatumaState;
use crate::dtct::value::DtctValue;

#[derive(Debug)]
enum AttributePhase {
  Name,
  AfterName,
  Args,
}

#[derive(Debug)]
pub struct AttributeParseMode {
  name: String,
  args: Vec<DatumaState>,
  phase: AttributePhase,
}

impl AttributeParseMode {
  pub fn starting(ch: char) -> Self {
    Self {
      name: starting_buf(ch),
      args: Vec::new(),
      phase: AttributePhase::Name,
    }
  }

  fn close_state(&mut self) -> DatumaState {
    DatumaState::node(
      Some(Box::new(DtctValue::Attribute {
        name: std::mem::take(&mut self.name),
      })),
      std::mem::take(&mut self.args),
    )
  }
}

impl Display for AttributeParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/dtct/attribute")
  }
}

impl ParseMode for AttributeParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    match self.phase {
      AttributePhase::Name => {
        if let Some(step) = on_parse_capture_whitespace(input) {
          self.phase = AttributePhase::AfterName;
          Ok(step)
        } else if is_ident_continue(input) {
          self.name.push(input);
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input == '(' {
          self.phase = AttributePhase::Args;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input == ',' || input == '>' {
          Ok((
            ParseStepMutation::CloseMode(Some(self.close_state())),
            ParsetStepFlow::Propagate,
          ))
        } else {
          Err(ParseErrorKind::UnexpectedChar(input))
        }
      }
      AttributePhase::AfterName => on_parse_capture_whitespace(input).map_or_else(
        || {
          if input == '(' {
            self.phase = AttributePhase::Args;
            Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
          } else if input == ',' || input == '>' {
            Ok((
              ParseStepMutation::CloseMode(Some(self.close_state())),
              ParsetStepFlow::Propagate,
            ))
          } else {
            Err(ParseErrorKind::UnexpectedChar(input))
          }
        },
        Ok,
      ),
      AttributePhase::Args => on_parse_capture_whitespace(input).map_or_else(
        || {
          if input == ')' {
            Ok((
              ParseStepMutation::CloseMode(Some(self.close_state())),
              ParsetStepFlow::Captured,
            ))
          } else if input == ',' {
            Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
          } else if is_ident_start(input) {
            Ok((
              ParseStepMutation::StartMode(Box::new(IdentifierParseMode::starting(input))),
              ParsetStepFlow::Captured,
            ))
          } else if input == '"' {
            Ok((
              ParseStepMutation::StartMode(Box::new(StringParseMode::new())),
              ParsetStepFlow::Captured,
            ))
          } else if input.is_ascii_digit() || input == '-' {
            Ok((
              ParseStepMutation::StartMode(Box::new(IntegerParseMode::starting(input))),
              ParsetStepFlow::Captured,
            ))
          } else {
            Err(expected_closing(messages::CLOSE_PAREN))
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
    self.args.push(child);
  }
}
