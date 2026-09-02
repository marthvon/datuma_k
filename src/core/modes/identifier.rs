use std::fmt::Display;
use std::write;

use super::accessor::AccessorParseMode;
use super::accessor::resolve_accessor;
use super::args::CallParseMode;
use super::boolean::BooleanParseMode;
use super::control::{ElseIfParseMode, ElseParseMode, ForParseMode, IfParseMode};
use super::function::FunctionDefParseMode;
use super::instruction::InstructionParseMode;
use super::jump::JumpParseMode;
use super::null::NullParseMode;
use super::operator::{
  OperatorContext, OperatorParseMode, is_ident_operator, resolve_dot_operator, resolve_operators,
};
use super::program::_stmt::{Keyword, KeywordFlags, keyword_from_buf};
use crate::core::common::STARTING_BUF_CAPACITY;
use crate::core::common::starting_buf;
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow, expected, internal_invariant,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

#[derive(Debug)]
pub struct IdentifierParseMode {
  buf: String,
  no_operators: bool,
  flags: KeywordFlags,
}

impl Default for IdentifierParseMode {
  fn default() -> Self {
    Self {
      buf: String::with_capacity(STARTING_BUF_CAPACITY),
      no_operators: false,
      flags: KeywordFlags::NONE,
    }
  }
}

impl IdentifierParseMode {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn starting(ch: char) -> Self {
    Self {
      buf: starting_buf(ch),
      no_operators: false,
      flags: KeywordFlags::NONE,
    }
  }

  pub fn starting_with_flags(ch: char, flags: KeywordFlags) -> Self {
    Self {
      buf: starting_buf(ch),
      no_operators: false,
      flags,
    }
  }

  pub fn statement_starting(ch: char, flags: KeywordFlags) -> Self {
    Self {
      buf: starting_buf(ch),
      no_operators: false,
      flags,
    }
  }

  pub fn key_starting(ch: char) -> Self {
    Self {
      buf: starting_buf(ch),
      no_operators: true,
      flags: KeywordFlags::NONE,
    }
  }

  fn close_ident(&mut self) -> DatumaState {
    DatumaState::leaf(Box::new(CoreValue::Ident(std::mem::take(&mut self.buf))))
  }

  fn try_keyword_replace(&mut self, input: char) -> Option<ParseStep> {
    if input.is_ascii_alphanumeric() || input == '_' {
      None
    } else if let Some(keyword) = keyword_from_buf(&self.buf) {
      self.replace_keyword(keyword, input)
    } else if self.flags.contains(KeywordFlags::ELSEIF) {
      Some(Err(expected(messages::ELSEIF_KEYWORD)))
    } else {
      None
    }
  }

  fn replace_keyword(&mut self, keyword: Keyword, input: char) -> Option<ParseStep> {
    match keyword {
      Keyword::If if self.flags.contains(KeywordFlags::ELSEIF) => self.emit_replace(Box::new(
        ElseIfParseMode::new(self.flags.difference(KeywordFlags::ELSEIF)),
      )),
      Keyword::Else if self.flags.contains(KeywordFlags::ELSE) => self.emit_replace(Box::new(
        ElseParseMode::new(self.flags.difference(KeywordFlags::ELSE)),
      )),
      Keyword::In if self.flags.contains(KeywordFlags::IN) => None,
      Keyword::Else | Keyword::In if self.flags.contains(KeywordFlags::STATEMENT) => {
        Some(Err(ParseErrorKind::UnexpectedChar(input)))
      }
      Keyword::Else | Keyword::In => None,
      Keyword::Fn | Keyword::For if !self.flags.contains(KeywordFlags::STATEMENT) => None,
      Keyword::Return if !self.flags.contains(KeywordFlags::RETURN) => None,
      Keyword::Break if !self.flags.contains(KeywordFlags::BREAK) => None,
      Keyword::Yield if !self.flags.contains(KeywordFlags::YIELD) => None,
      _ => self.emit_replace(match keyword {
        Keyword::Fn => Box::new(FunctionDefParseMode::new()),
        Keyword::If => Box::new(IfParseMode::new(self.flags)),
        Keyword::For => Box::new(ForParseMode::new(self.flags)),
        Keyword::Return => Box::new(JumpParseMode::return_jump()),
        Keyword::Break => Box::new(JumpParseMode::break_jump()),
        Keyword::Yield => Box::new(JumpParseMode::yield_jump()),
        Keyword::Else | Keyword::In => {
          return Some(Err(internal_invariant("else/in replace without flag")));
        }
      }),
    }
  }

  fn emit_replace(&mut self, mode: Box<dyn ParseMode>) -> Option<ParseStep> {
    self.buf.clear();
    Some(Ok((
      ParseStepMutation::ReplaceMode(mode),
      ParsetStepFlow::Repropagate,
    )))
  }

  fn try_literal_replace(&mut self, input: char) -> Option<ParseStep> {
    if input.is_ascii_alphanumeric() || input == '_' {
      None
    } else if let Some(mode) = BooleanParseMode::from_buf(&self.buf) {
      self.buf.clear();
      Some(Ok((
        ParseStepMutation::ReplaceMode(Box::new(mode)),
        ParsetStepFlow::Repropagate,
      )))
    } else if let Some(mode) = NullParseMode::from_buf(&self.buf) {
      self.buf.clear();
      Some(Ok((
        ParseStepMutation::ReplaceMode(Box::new(mode)),
        ParsetStepFlow::Repropagate,
      )))
    } else {
      None
    }
  }
}

impl Display for IdentifierParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/ident")
  }
}

impl ParseMode for IdentifierParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    if self.flags.contains(KeywordFlags::ELSE) && !input.is_ascii_alphanumeric() && input != '_' {
      if keyword_from_buf(&self.buf) == Some(Keyword::Else) {
        match self.replace_keyword(Keyword::Else, input) {
          Some(step) => step,
          None => Err(internal_invariant("else replace without flag")),
        }
      } else {
        self.flags = self.flags.difference(KeywordFlags::ELSE);
        Ok((
          ParseStepMutation::ParentForceDismissAndStartMode(Box::new(
            InstructionParseMode::with_flags(self.flags),
          )),
          ParsetStepFlow::Repropagate,
        ))
      }
    } else if self.no_operators {
      if let Some(step) = self.try_literal_replace(input) {
        step
      } else {
        self.on_parse_ident(input)
      }
    } else if let Some(step) = self.try_keyword_replace(input) {
      step
    } else if let Some(step) = self.try_literal_replace(input) {
      step
    } else {
      self.on_parse_ident(input)
    }
  }

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    if input.is_whitespace() {
      Ok((ParseResolveMutation::Nothing, ParsetStepFlow::Captured))
    } else if self.flags.contains(KeywordFlags::ELSEIF) {
      Err(expected(messages::ELSEIF_KEYWORD))
    } else if self.no_operators {
      Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
    } else if input == '[' {
      resolve_accessor()
    } else if input == '.' {
      resolve_dot_operator()
    } else {
      resolve_operators(input, &[OperatorContext::Ident, OperatorContext::Numeric])
    }
  }

  fn close_state(&mut self) -> Option<DatumaState> {
    if self.buf.is_empty() || self.flags.contains(KeywordFlags::ELSEIF) {
      None
    } else {
      Some(self.close_ident())
    }
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if state.is_some() {
      None
    } else if self.flags.contains(KeywordFlags::ELSEIF) {
      Some(expected(messages::ELSEIF_KEYWORD))
    } else {
      Some(expected(messages::IDENT))
    }
  }
}

impl IdentifierParseMode {
  fn on_parse_ident(&mut self, input: char) -> ParseStep {
    if input.is_ascii_alphanumeric() || input == '_' {
      self.buf.push(input);
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    } else if self.flags.contains(KeywordFlags::ELSEIF) {
      Err(expected(messages::ELSEIF_KEYWORD))
    } else if !self.no_operators && input == '(' {
      if self.buf.is_empty() {
        Err(internal_invariant("identifier closed with empty buffer"))
      } else {
        Ok((
          ParseStepMutation::ReplaceMode(Box::new(CallParseMode::new(std::mem::take(
            &mut self.buf,
          )))),
          ParsetStepFlow::Captured,
        ))
      }
    } else if !self.no_operators && input == '[' {
      if let Some(state) = self.close_state() {
        Ok((
          ParseStepMutation::CloseAndStartMode(Some(state), Box::new(AccessorParseMode::new())),
          ParsetStepFlow::Captured,
        ))
      } else {
        Err(internal_invariant("identifier closed with empty buffer"))
      }
    } else if !self.no_operators && is_ident_operator(input) {
      if let Some(state) = self.close_state() {
        Ok((
          ParseStepMutation::CloseAndStartMode(
            Some(state),
            Box::new(
              OperatorParseMode::from_char(input, OperatorContext::Ident).expect("ident operator"),
            ),
          ),
          ParsetStepFlow::Captured,
        ))
      } else {
        Err(internal_invariant("identifier closed with empty buffer"))
      }
    } else if let Some(state) = self.close_state() {
      Ok((
        ParseStepMutation::CloseMode(Some(state)),
        ParsetStepFlow::Propagate,
      ))
    } else {
      Err(internal_invariant("identifier closed with empty buffer"))
    }
  }
}
