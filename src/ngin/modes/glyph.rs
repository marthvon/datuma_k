use crate::core::modes::{InstructionParseMode, KeywordFlags, is_statement_start};
use crate::core::parser::{ParseStep, ParseStepMutation, ParsetStepFlow, expected, messages};

use super::emit::NginEmitStarter;
use super::file::NginFileParseMode;
use super::guard::NginGuardParseMode;
use super::interp::NginInterpParseMode;
use super::plus::NginPlusStarter;
use super::template::NginFenceParseMode;

#[derive(Debug, Clone, Copy)]
pub struct GlyphAllow {
  pub file: bool,
  pub emit: bool,
  pub guard: bool,
}

pub fn dispatch_ngin_char(input: char, allow: GlyphAllow) -> Option<ParseStep> {
  match input {
    '|' if allow.file => Some(Ok((
      ParseStepMutation::StartMode(Box::new(NginFileParseMode::new())),
      ParsetStepFlow::Repropagate,
    ))),
    '|' => Some(Err(expected(messages::SECOND_FILE))),
    '`' => Some(Ok((
      ParseStepMutation::StartMode(Box::new(NginFenceParseMode::new(false))),
      ParsetStepFlow::Repropagate,
    ))),
    '@' => Some(Ok((
      ParseStepMutation::StartMode(Box::new(NginInterpParseMode::opening(allow))),
      ParsetStepFlow::Repropagate,
    ))),
    '=' if allow.emit => Some(Ok((
      ParseStepMutation::StartMode(Box::new(NginEmitStarter::new())),
      ParsetStepFlow::Repropagate,
    ))),
    '+' if allow.emit => Some(Ok((
      ParseStepMutation::StartMode(Box::new(NginPlusStarter::new())),
      ParsetStepFlow::Repropagate,
    ))),
    '?' if allow.guard => Some(Ok((
      ParseStepMutation::StartMode(Box::new(NginGuardParseMode::new())),
      ParsetStepFlow::Repropagate,
    ))),
    '?' => Some(Err(expected(messages::NGIN_GUARD))),
    _ => None,
  }
}

pub fn start_instruction_or_propagate(input: char) -> ParseStep {
  if is_statement_start(input) {
    start_instruction()
  } else {
    Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate))
  }
}

pub fn start_instruction() -> ParseStep {
  Ok((
    ParseStepMutation::StartMode(Box::new(InstructionParseMode::with_flags(
      KeywordFlags::top_level(),
    ))),
    ParsetStepFlow::Repropagate,
  ))
}
