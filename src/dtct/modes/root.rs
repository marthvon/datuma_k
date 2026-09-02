use std::fmt::Display;
use std::write;

use crate::core::modes::is_ident_start;
use crate::core::parser::{
  ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation, ParsetStepFlow,
};
use crate::core::state::DatumaState;
use crate::dtct::modes::model::ModelParseMode;

#[derive(Debug, Default)]
pub struct DtctRootParseMode {
  tree: DatumaState,
}

impl DtctRootParseMode {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn into_state(self) -> DatumaState {
    self.tree
  }
}

impl Display for DtctRootParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/dtct/root")
  }
}

impl ParseMode for DtctRootParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    if input.is_whitespace() {
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    } else if is_ident_start(input) {
      Ok((
        ParseStepMutation::StartMode(Box::new(ModelParseMode::starting(input))),
        ParsetStepFlow::Captured,
      ))
    } else {
      Err(crate::core::parser::ParseErrorKind::UnexpectedChar(input))
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
