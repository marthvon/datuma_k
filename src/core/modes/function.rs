use std::fmt::Display;
use std::write;

use super::args::ArgParseMode;
use super::program::_stmt::{KeywordFlags, is_ident_continue};
use super::program::ProgramParseMode;
use crate::core::modes::on_parse_capture_whitespace;
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow, expected, expected_closing,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

#[derive(Debug, PartialEq, Eq)]
enum FunctionDefPhase {
  Name { buf: String },
  Params,
  OpenBrace,
  Body,
}

#[derive(Debug)]
pub struct FunctionDefParseMode {
  name: String,
  params: Option<DatumaState>,
  children: Vec<DatumaState>,
  phase: FunctionDefPhase,
  has_args: bool,
}

impl FunctionDefParseMode {
  pub fn new() -> Self {
    Self {
      name: String::new(),
      params: None,
      children: Vec::new(),
      has_args: false,
      phase: FunctionDefPhase::Name { buf: String::new() },
    }
  }
}

impl Display for FunctionDefParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/function-def")
  }
}

impl ParseMode for FunctionDefParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    match &mut self.phase {
      FunctionDefPhase::Name { buf } => {
        if is_ident_continue(input) {
          buf.push(input);
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate))
        } else if input == '(' {
          self.name = std::mem::take(buf);
          if self.name.is_empty() {
            Err(expected(messages::IDENT))
          } else {
            self.phase = FunctionDefPhase::Params;
            Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
          }
        } else {
          Err(expected_closing(messages::OPEN_PAREN))
        }
      }
      FunctionDefPhase::Params => {
        if let Some(v) = on_parse_capture_whitespace(input) {
          Ok(v)
        } else if input == ')' {
          self.phase = FunctionDefPhase::OpenBrace;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if !self.has_args {
          self.has_args = true;
          Ok((
            ParseStepMutation::StartMode(Box::new(ArgParseMode::new())),
            ParsetStepFlow::Repropagate,
          ))
        } else {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        }
      }
      FunctionDefPhase::OpenBrace => {
        if input == '{' {
          self.phase = FunctionDefPhase::Body;
          Ok((
            ParseStepMutation::StartMode(Box::new(ProgramParseMode::with_flags(
              KeywordFlags::function_body(),
            ))),
            ParsetStepFlow::Captured,
          ))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate))
        } else {
          Err(expected(messages::OPEN_BRACE))
        }
      }
      FunctionDefPhase::Body => {
        if let Some(state) = self.close_state() {
          Ok((
            ParseStepMutation::CloseMode(Some(state)),
            ParsetStepFlow::Propagate,
          ))
        } else {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate))
        }
      }
    }
  }

  fn on_parse_resolved(&mut self, _input: char) -> ParseResolveStep {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }

  fn adopt(&mut self, child: DatumaState) {
    if self.phase == FunctionDefPhase::Params {
      self.params = Some(child);
    } else if self.phase == FunctionDefPhase::Body {
      self.children.push(child);
    }
  }

  fn close_after_adopt(&mut self) -> Option<DatumaState> {
    self.close_state()
  }

  fn close_state(&mut self) -> Option<DatumaState> {
    if self.phase == FunctionDefPhase::Body && self.children.len() == 1 {
      let program = self.children.pop().expect("function body");
      Some(DatumaState::node(
        Some(Box::new(CoreValue::FunctionDef(std::mem::take(
          &mut self.name,
        )))),
        vec![
          self
            .params
            .take()
            .unwrap_or_else(|| DatumaState::node(None, Vec::new())),
          program,
        ],
      ))
    } else {
      None
    }
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if state.is_some() {
      None
    } else if self.phase == FunctionDefPhase::Body || self.phase == FunctionDefPhase::OpenBrace {
      Some(expected(messages::OPEN_BRACE))
    } else {
      Some(expected(messages::IDENT))
    }
  }
}
