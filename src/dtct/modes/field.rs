use std::fmt::Display;
use std::{vec, write};

use super::type_expr::TypeExprParseMode;
use crate::core::common::{on_parse_capture_whitespace, starting_buf};
use crate::core::modes::{is_ident_continue, is_ident_start};
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow, expected, expected_closing,
};
use crate::core::state::DatumaState;
use crate::dtct::value::DtctValue;

#[derive(Debug)]
enum FieldPhase {
  Name,
  AfterName,
  AfterColon,
  WaitingType,
  Ready,
}

#[derive(Debug)]
pub struct FieldParseMode {
  name: String,
  ty: Option<DatumaState>,
  phase: FieldPhase,
}

impl FieldParseMode {
  pub fn starting(ch: char) -> Self {
    Self {
      name: starting_buf(ch),
      ty: None,
      phase: FieldPhase::Name,
    }
  }
}

impl Display for FieldParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/dtct/field")
  }
}

impl ParseMode for FieldParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    match self.phase {
      FieldPhase::Name => {
        if let Some(step) = on_parse_capture_whitespace(input) {
          self.phase = FieldPhase::AfterName;
          Ok(step)
        } else if input == ':' {
          self.phase = FieldPhase::AfterColon;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if is_ident_continue(input) {
          self.name.push(input);
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Err(expected_closing(messages::COLON))
        }
      }
      FieldPhase::AfterName => on_parse_capture_whitespace(input).map_or_else(
        || {
          if input == ':' {
            self.phase = FieldPhase::AfterColon;
            Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
          } else {
            Err(expected_closing(messages::COLON))
          }
        },
        Ok,
      ),
      FieldPhase::AfterColon => on_parse_capture_whitespace(input).map_or_else(
        || {
          if is_ident_start(input) {
            self.phase = FieldPhase::WaitingType;
            Ok((
              ParseStepMutation::StartMode(Box::new(TypeExprParseMode::starting(input))),
              ParsetStepFlow::Captured,
            ))
          } else {
            Err(expected(messages::TYPE_NAME))
          }
        },
        Ok,
      ),
      FieldPhase::WaitingType => {
        on_parse_capture_whitespace(input).map_or_else(|| Err(expected(messages::TYPE_EXPR)), Ok)
      }
      FieldPhase::Ready => on_parse_capture_whitespace(input).map_or_else(
        || {
          if input == ',' || input == '}' {
            Ok((
              ParseStepMutation::CloseMode(self.close_state()?),
              ParsetStepFlow::Propagate,
            ))
          } else {
            Err(expected_closing(messages::COMMA))
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
    self.ty = Some(child);
    self.phase = FieldPhase::Ready;
  }
}

impl FieldParseMode {
  fn close_state(&mut self) -> Result<Option<DatumaState>, ParseErrorKind> {
    let ty = self.ty.take().ok_or(expected(messages::TYPE_EXPR))?;
    Ok(Some(DatumaState::node(
      Some(Box::new(DtctValue::Field {
        name: std::mem::take(&mut self.name),
      })),
      vec![ty],
    )))
  }
}
