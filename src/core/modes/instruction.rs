use std::fmt::Display;
use std::{matches, vec, write};

use super::identifier::IdentifierParseMode;
use super::operator::{OperatorContext, start_operator};
use super::program::_stmt::{
  KeywordFlags, core_value, is_ident_start, is_operator_char, is_prefix_operator, start_value,
};
use crate::core::parser::{
  ParseErrorKind, ParseErrorSource, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep,
  ParseStepMutation, ParsetStepFlow, expected, messages,
};
use crate::core::source::{ParseCursorMetadata, ParseFileMetadata};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

const STATEMENT_OPERATOR_CONTEXTS: &[OperatorContext] = &[
  OperatorContext::Ident,
  OperatorContext::Numeric,
  OperatorContext::InvokedFunction,
];

#[derive(Debug)]
pub struct InstructionParseMode {
  parts: Vec<DatumaState>,
  flags: KeywordFlags,
  require_semicolon: bool,
  close_on_paren: bool,
  ended: bool,
  span: Option<(ParseFileMetadata, ParseCursorMetadata)>,
}

impl InstructionParseMode {
  pub fn new() -> Self {
    Self::with_flags(KeywordFlags::NONE)
  }

  pub fn with_flags(flags: KeywordFlags) -> Self {
    Self {
      parts: Vec::new(),
      flags,
      require_semicolon: false,
      close_on_paren: false,
      ended: false,
      span: None,
    }
  }

  pub fn with_part(part: DatumaState) -> Self {
    Self {
      parts: vec![part],
      flags: KeywordFlags::NONE,
      require_semicolon: false,
      close_on_paren: false,
      ended: false,
      span: None,
    }
  }

  pub fn require_semicolon() -> Self {
    Self {
      parts: Vec::new(),
      flags: KeywordFlags::NONE,
      require_semicolon: true,
      close_on_paren: false,
      ended: false,
      span: None,
    }
  }

  pub fn require_semicolon_with_part(part: DatumaState) -> Self {
    Self {
      parts: vec![part],
      flags: KeywordFlags::NONE,
      require_semicolon: true,
      close_on_paren: false,
      ended: false,
      span: None,
    }
  }

  pub fn for_clause() -> Self {
    Self {
      parts: Vec::new(),
      flags: KeywordFlags::NONE,
      require_semicolon: true,
      close_on_paren: true,
      ended: false,
      span: None,
    }
  }

  pub fn for_clause_with_part(part: DatumaState) -> Self {
    Self {
      parts: vec![part],
      flags: KeywordFlags::NONE,
      require_semicolon: true,
      close_on_paren: true,
      ended: false,
      span: None,
    }
  }

  pub fn until_close_paren() -> Self {
    Self {
      parts: Vec::new(),
      flags: KeywordFlags::NONE,
      require_semicolon: false,
      close_on_paren: true,
      ended: false,
      span: None,
    }
  }

  pub fn until_close_paren_with_part(part: DatumaState) -> Self {
    Self {
      parts: vec![part],
      flags: KeywordFlags::NONE,
      require_semicolon: false,
      close_on_paren: true,
      ended: false,
      span: None,
    }
  }
}

impl Display for InstructionParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/instruction")
  }
}

impl ParseMode for InstructionParseMode {
  fn note_source(&mut self, source: &dyn ParseErrorSource) {
    if self.span.is_none() {
      self.span = Some((source.file_meta(), source.pos_meta()));
    }
  }

  fn on_parse(&mut self, input: char) -> ParseStep {
    if self.parts.is_empty() && is_ident_start(input) {
      Ok((
        ParseStepMutation::StartMode(Box::new(
          if self.close_on_paren && !self.require_semicolon {
            IdentifierParseMode::key_starting(input)
          } else {
            IdentifierParseMode::statement_starting(input, self.flags)
          },
        )),
        ParsetStepFlow::Captured,
      ))
    } else if input == ';' {
      self.ended = true;
      Ok((
        ParseStepMutation::CloseMode(self.close_state()),
        ParsetStepFlow::Captured,
      ))
    } else if self.close_on_paren && input == ')' {
      self.ended = true;
      Ok((
        ParseStepMutation::CloseMode(self.close_state()),
        ParsetStepFlow::Propagate,
      ))
    } else if input.is_whitespace() {
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    } else if is_operator_char(input)
      && (self.parts.is_empty()
        && (is_prefix_operator(input) || input == '-' && self.close_on_paren)
        || !self.parts.is_empty() && self.close_on_paren)
    {
      start_operator(input, STATEMENT_OPERATOR_CONTEXTS)
    } else if !self.parts.is_empty() {
      match start_value(input) {
        Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate)) => Ok((
          ParseStepMutation::CloseMode(self.close_state()),
          ParsetStepFlow::Propagate,
        )),
        other => other,
      }
    } else {
      start_value(input)
    }
  }

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    if input.is_whitespace() {
      Ok((ParseResolveMutation::Nothing, ParsetStepFlow::Captured))
    } else if input == ';' {
      Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
    } else {
      Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
    }
  }

  fn adopt(&mut self, child: DatumaState) {
    self.parts.push(child);
  }

  fn close_after_adopt(&mut self) -> Option<DatumaState> {
    if self.require_semicolon {
      None
    } else if self.parts.last().is_some_and(is_control_statement) && self.parts.len() == 1 {
      self.emit_instruction()
    } else if self.close_on_paren && !self.parts.is_empty() {
      self.close_state()
    } else {
      None
    }
  }

  fn close_state(&mut self) -> Option<DatumaState> {
    self.emit_instruction()
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if self.require_semicolon && !self.ended {
      Some(expected(messages::SEMICOLON))
    } else if state.is_some() {
      None
    } else {
      None
    }
  }
}

impl InstructionParseMode {
  fn emit_instruction(&mut self) -> Option<DatumaState> {
    if self.parts.len() == 1 && is_control_statement(&self.parts[0]) {
      Some(self.parts.pop().expect("control part"))
    } else {
      let (file_meta, pos_meta) = self.span.take().unwrap_or_else(|| {
        (
          ParseFileMetadata::source("<synthetic>"),
          ParseCursorMetadata::default(),
        )
      });
      Some(DatumaState::node(
        Some(Box::new(CoreValue::Instruction {
          file_meta,
          pos_meta,
        })),
        std::mem::take(&mut self.parts),
      ))
    }
  }
}

fn is_control_statement(state: &DatumaState) -> bool {
  core_value(state).is_some_and(|value| {
    matches!(
      value,
      CoreValue::If
        | CoreValue::ElseIf
        | CoreValue::Else
        | CoreValue::For
        | CoreValue::FunctionDef(_)
        | CoreValue::Return
        | CoreValue::Break
        | CoreValue::Yield
    )
  })
}
