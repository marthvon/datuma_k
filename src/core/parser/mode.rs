use std::fmt::{Debug, Display};
use std::write;

use crate::core::state::DatumaState;

use super::cursor::ParseErrorSource;
use super::step::{
  ParseErrorKind, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow,
};

pub trait ParseMode: Send + Debug + Display {
  fn on_parse(&mut self, input: char) -> ParseStep;
  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep;

  fn on_replace(&mut self, _replaced: Box<dyn ParseMode>) {}

  fn note_source(&mut self, _source: &dyn ParseErrorSource) {}

  fn adopt(&mut self, _child: DatumaState) {}
  fn accepts_resolved_child(&self) -> bool {
    false
  }
  fn reactivate_after_child_close(&mut self) -> bool {
    false
  }
  fn close_after_adopt(&mut self) -> Option<DatumaState> {
    None
  }
  fn close_state(&mut self) -> Option<DatumaState> {
    None
  }
  fn incomplete_close_error(&self, _state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    None
  }

  fn into_datuma_state(self: Box<Self>) -> Option<DatumaState> {
    None
  }
}

#[derive(Debug, Default)]
pub struct RootParseMode {
  tree: DatumaState,
}

impl RootParseMode {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn into_state(self) -> DatumaState {
    self.tree
  }
}

impl Display for RootParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/root")
  }
}

impl ParseMode for RootParseMode {
  fn on_parse(&mut self, _input: char) -> ParseStep {
    Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate))
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
