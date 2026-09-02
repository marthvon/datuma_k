use std::fmt::Display;
use std::write;

use super::field::FieldParseMode;
use super::trait_list::TraitListParseMode;
use crate::core::common::{on_parse_capture_whitespace, starting_buf};
use crate::core::modes::{is_ident_continue, is_ident_start};
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow, expected, expected_closing,
};
use crate::core::state::DatumaState;
use crate::dtct::value::DtctValue;

#[derive(Debug, PartialEq)]
enum ModelPhase {
  Name,
  AfterName,
  WaitingTraits,
  AfterTraits,
  BodyEmpty,
  BodyAfterComma,
  BodyAfterField,
}

#[derive(Debug)]
pub struct ModelParseMode {
  name: String,
  traits: Vec<String>,
  fields: Vec<DatumaState>,
  phase: ModelPhase,
}

impl ModelParseMode {
  pub fn starting(ch: char) -> Self {
    Self {
      name: starting_buf(ch),
      traits: Vec::new(),
      fields: Vec::new(),
      phase: ModelPhase::Name,
    }
  }

  fn close_model(&mut self) -> DatumaState {
    DatumaState::node(
      Some(Box::new(DtctValue::Model {
        name: std::mem::take(&mut self.name),
        traits: std::mem::take(&mut self.traits),
      })),
      std::mem::take(&mut self.fields),
    )
  }
}

impl Display for ModelParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/dtct/model")
  }
}

impl ParseMode for ModelParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    match self.phase {
      ModelPhase::Name | ModelPhase::AfterName | ModelPhase::AfterTraits if input == '{' => {
        self.phase = ModelPhase::BodyEmpty;
        Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
      }
      ModelPhase::Name | ModelPhase::AfterName if input == '[' => {
        self.phase = ModelPhase::WaitingTraits;
        Ok((
          ParseStepMutation::StartMode(Box::new(TraitListParseMode::new())),
          ParsetStepFlow::Captured,
        ))
      }
      ModelPhase::Name => {
        if let Some(step) = on_parse_capture_whitespace(input) {
          self.phase = ModelPhase::AfterName;
          Ok(step)
        } else if is_ident_continue(input) {
          self.name.push(input);
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Err(expected_closing(messages::OPEN_BRACE))
        }
      }
      ModelPhase::AfterName | ModelPhase::AfterTraits => on_parse_capture_whitespace(input)
        .map_or_else(|| Err(expected_closing(messages::OPEN_BRACE)), Ok),
      ModelPhase::WaitingTraits => Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate)),
      ModelPhase::BodyEmpty | ModelPhase::BodyAfterComma | ModelPhase::BodyAfterField => {
        on_parse_capture_whitespace(input).map_or_else(
          || match input {
            '}' => match self.phase {
              ModelPhase::BodyEmpty | ModelPhase::BodyAfterField => Ok((
                ParseStepMutation::CloseMode(Some(self.close_model())),
                ParsetStepFlow::Captured,
              )),
              ModelPhase::BodyAfterComma => Err(expected(messages::FIELD_NAME)),
              _ => unreachable!("body close"),
            },
            ',' => {
              if self.phase == ModelPhase::BodyAfterField {
                self.phase = ModelPhase::BodyAfterComma;
                Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
              } else {
                Err(ParseErrorKind::UnexpectedChar(','))
              }
            }
            _ => {
              if !is_ident_start(input) {
                Err(ParseErrorKind::UnexpectedChar(input))
              } else if self.phase == ModelPhase::BodyAfterField {
                Err(expected_closing(messages::COMMA))
              } else {
                Ok((
                  ParseStepMutation::StartMode(Box::new(FieldParseMode::starting(input))),
                  ParsetStepFlow::Captured,
                ))
              }
            }
          },
          Ok,
        )
      }
    }
  }

  fn on_parse_resolved(&mut self, _input: char) -> ParseResolveStep {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }

  fn adopt(&mut self, child: DatumaState) {
    match self.phase {
      ModelPhase::WaitingTraits => {
        self.traits = child
          .value
          .as_ref()
          .and_then(|value| value.as_any().downcast_ref::<DtctValue>())
          .and_then(|value| match value {
            DtctValue::Traits(names) => Some(names.clone()),
            _ => None,
          })
          .expect("dtct model expected Traits child");
        self.phase = ModelPhase::AfterTraits;
      }
      ModelPhase::BodyEmpty | ModelPhase::BodyAfterComma => {
        self.fields.push(child);
        self.phase = ModelPhase::BodyAfterField;
      }
      ModelPhase::Name
      | ModelPhase::AfterName
      | ModelPhase::AfterTraits
      | ModelPhase::BodyAfterField => {
        unreachable!("dtct model cannot adopt a child in {:?}", self.phase);
      }
    }
  }
}
