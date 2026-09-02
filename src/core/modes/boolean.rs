use std::fmt::Display;
use std::write;

use super::operator::{OperatorContext, resolve_operators};
use crate::core::modes::on_resolve_capture_whitespace;
use crate::core::parser::{
  ParseMode, ParseResolveStep, ParseStep, ParseStepMutation, ParsetStepFlow,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanLiteral {
  True,
  False,
}

#[derive(Debug)]
pub struct BooleanParseMode {
  literal: BooleanLiteral,
}

impl BooleanParseMode {
  pub fn from_buf(buf: &str) -> Option<Self> {
    Some(Self {
      literal: if buf.eq_ignore_ascii_case("true") {
        BooleanLiteral::True
      } else if buf.eq_ignore_ascii_case("false") {
        BooleanLiteral::False
      } else {
        return None;
      },
    })
  }
}

impl Display for BooleanParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/boolean")
  }
}

impl ParseMode for BooleanParseMode {
  fn on_parse(&mut self, _input: char) -> ParseStep {
    Ok((
      ParseStepMutation::CloseMode(self.close_state()),
      ParsetStepFlow::Propagate,
    ))
  }

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    on_resolve_capture_whitespace(input).map_or_else(
      || resolve_operators(input, &[OperatorContext::Boolean]),
      |v| Ok(v),
    )
  }

  fn close_state(&mut self) -> Option<DatumaState> {
    Some(match self.literal {
      BooleanLiteral::True => DatumaState::leaf(Box::new(CoreValue::Boolean(true))),
      BooleanLiteral::False => DatumaState::leaf(Box::new(CoreValue::Boolean(false))),
    })
  }
}
