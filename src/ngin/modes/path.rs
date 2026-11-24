use std::fmt::Display;
use std::write;

use mvparser::common::starting_buf;
use mvparser::parser::{ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation, ParsetStepFlow};
use mvparser::state::ParseState;
use crate::ngin::value::NginValue;

use super::env::NginEnvParseMode;
use super::glyph::GlyphAllow;
use super::interp::NginInterpParseMode;

#[derive(Debug, PartialEq, Eq)]
enum PathPhase {
  Body,
  Quoted,
}

#[derive(Debug)]
pub struct NginPathParseMode {
  children: Vec<ParseState>,
  lit: String,
  phase: PathPhase,
}

impl NginPathParseMode {
  pub fn new() -> Self {
    Self {
      children: Vec::new(),
      lit: String::new(),
      phase: PathPhase::Body,
    }
  }

  fn flush_lit(&mut self) {
    if !self.lit.is_empty() {
      self
        .children
        .push(ParseState::leaf(Box::new(NginValue::PathLit {
          text: std::mem::take(&mut self.lit),
        })));
    }
  }

  fn push_lit(&mut self, input: char) {
    if self.lit.is_empty() {
      self.lit = starting_buf(input);
    } else {
      self.lit.push(input);
    }
  }

  fn close_path(&mut self) -> ParseState {
    self.flush_lit();
    ParseState::node(
      Some(Box::new(NginValue::Path)),
      std::mem::take(&mut self.children),
    )
  }
}

impl Display for NginPathParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/ngin/path")
  }
}

impl ParseMode for NginPathParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    match self.phase {
      PathPhase::Quoted => {
        if input == '"' {
          self.flush_lit();
          self.phase = PathPhase::Body;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          self.push_lit(input);
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        }
      }
      PathPhase::Body => match input {
        '>' => Ok((
          ParseStepMutation::CloseMode(Some(self.close_path())),
          ParsetStepFlow::Propagate,
        )),
        '/' => {
          self.flush_lit();
          self
            .children
            .push(ParseState::leaf(Box::new(NginValue::PathLit {
              text: "/".to_string(),
            })));
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        }
        '"' => {
          self.flush_lit();
          self.phase = PathPhase::Quoted;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        }
        '$' => {
          self.flush_lit();
          Ok((
            ParseStepMutation::StartMode(Box::new(NginEnvParseMode::new())),
            ParsetStepFlow::Repropagate,
          ))
        }
        '@' => {
          self.flush_lit();
          Ok((
            ParseStepMutation::StartMode(Box::new(NginInterpParseMode::opening(GlyphAllow {
              file: false,
              emit: true,
              guard: false,
            }))),
            ParsetStepFlow::Repropagate,
          ))
        }
        _ => {
          self.push_lit(input);
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        }
      },
    }
  }

  fn on_parse_resolved(&mut self, _input: char) -> ParseResolveStep {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }

  fn adopt(&mut self, child: ParseState) {
    self.children.push(child);
  }
}
