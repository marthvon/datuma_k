use std::fmt::Display;
use std::write;

use super::operator::{OperatorContext, resolve_operators};
use crate::core::modes::on_resolve_capture_whitespace;
use crate::core::parser::{
  ParseMode, ParseResolveStep, ParseStep, ParseStepMutation, ParsetStepFlow,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

#[derive(Debug, Default)]
pub struct NullParseMode;

impl NullParseMode {
  pub fn from_buf(buf: &str) -> Option<Self> {
    if buf.eq_ignore_ascii_case("null") {
      Some(Self)
    } else {
      None
    }
  }

  pub fn state() -> DatumaState {
    DatumaState::leaf(Box::new(CoreValue::Null))
  }
}

impl Display for NullParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/null")
  }
}

impl ParseMode for NullParseMode {
  fn on_parse(&mut self, _input: char) -> ParseStep {
    Ok((
      ParseStepMutation::CloseMode(self.close_state()),
      ParsetStepFlow::Propagate,
    ))
  }

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    on_resolve_capture_whitespace(input).map_or_else(
      || resolve_operators(input, &[OperatorContext::Null]),
      |v| Ok(v),
    )
  }

  fn close_state(&mut self) -> Option<DatumaState> {
    Some(Self::state())
  }
}
