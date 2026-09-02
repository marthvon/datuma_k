use std::fmt::Display;
use std::write;

use crate::core::parser::{
  ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation, ParsetStepFlow,
};
use crate::core::state::DatumaState;

use super::glyph::{GlyphAllow, dispatch_ngin_char};

#[derive(Debug, Default)]
pub struct NginRootParseMode {
  tree: DatumaState,
}

impl NginRootParseMode {
  pub fn new() -> Self {
    Self::default()
  }
}

impl Display for NginRootParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/ngin/root")
  }
}

impl ParseMode for NginRootParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    if input.is_whitespace() {
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    } else {
      dispatch_ngin_char(
        input,
        GlyphAllow {
          file: true,
          emit: false,
          guard: false,
        },
      )
      .unwrap_or_else(|| Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate)))
    }
  }

  fn on_parse_resolved(&mut self, _input: char) -> ParseResolveStep {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }

  fn adopt(&mut self, child: DatumaState) {
    self.tree.adopt(child);
  }

  fn into_datuma_state(self: Box<Self>) -> Option<DatumaState> {
    Some(self.tree)
  }
}
