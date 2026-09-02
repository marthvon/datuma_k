use std::fmt::Display;
use std::write;

use super::grouped::GroupedParseMode;
use super::identifier::IdentifierParseMode;
use super::instruction::InstructionParseMode;
use super::program::_stmt::{
  KeywordFlags, close_if_or_ternary, core_value, is_ident_continue, is_ident_start,
  is_operator_char, start_value, ternary_yield_error,
};
use super::program::ProgramParseMode;
use crate::core::common::starting_buf;
use crate::core::modes::on_parse_capture_whitespace;
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow, expected, expected_closing,
};
use crate::core::source::{ParseCursorMetadata, ParseFileMetadata};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

#[derive(Debug, PartialEq, Eq)]
enum IfPhase {
  OpenParen,
  Condition,
  OpenBrace,
  Then,
}

#[derive(Debug)]
pub struct IfParseMode {
  children: Vec<DatumaState>,
  phase: IfPhase,
  flags: KeywordFlags,
  yield_error: bool,
  pending_else: bool,
}

impl IfParseMode {
  pub fn new(flags: KeywordFlags) -> Self {
    Self {
      children: Vec::with_capacity(3),
      phase: IfPhase::OpenParen,
      flags,
      yield_error: false,
      pending_else: false,
    }
  }
}

impl Display for IfParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/if")
  }
}

impl ParseMode for IfParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    match self.phase {
      IfPhase::OpenParen => {
        if input == '(' {
          self.phase = IfPhase::Condition;
          Ok((
            ParseStepMutation::StartMode(Box::new(GroupedParseMode::new())),
            ParsetStepFlow::Captured,
          ))
        } else {
          on_parse_capture_whitespace(input)
            .map_or_else(|| Err(expected_closing(messages::OPEN_PAREN)), |v| Ok(v))
        }
      }
      IfPhase::Condition => Err(crate::core::parser::internal_invariant(
        "if awaiting condition close",
      )),
      IfPhase::OpenBrace => {
        if input == '{' {
          self.phase = IfPhase::Then;
          Ok((
            ParseStepMutation::StartMode(Box::new(ProgramParseMode::with_flags(
              self.flags.with_yield(),
            ))),
            ParsetStepFlow::Captured,
          ))
        } else {
          on_parse_capture_whitespace(input)
            .map_or_else(|| Err(expected(messages::OPEN_BRACE)), |v| Ok(v))
        }
      }
      IfPhase::Then => {
        if self.children.len() < 2 || self.pending_else {
          if input.is_whitespace() {
            Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
          } else {
            Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate))
          }
        } else if input == ';' {
          Ok((
            ParseStepMutation::CloseMode(self.close_state()),
            ParsetStepFlow::Captured,
          ))
        } else if is_ident_start(input) {
          self.pending_else = true;
          Ok((
            ParseStepMutation::StartMode(Box::new(IdentifierParseMode::starting_with_flags(
              input,
              self.flags.with_else(),
            ))),
            ParsetStepFlow::Captured,
          ))
        } else {
          on_parse_capture_whitespace(input).map_or_else(
            || {
              Ok((
                ParseStepMutation::CloseMode(self.close_state()),
                ParsetStepFlow::Propagate,
              ))
            },
            |v| Ok(v),
          )
        }
      }
    }
  }

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    if self.yield_error {
      Err(ternary_yield_error())
    } else if self.phase == IfPhase::Then && self.children.len() == 2 {
      then_else_ident(self.flags, input)
    } else {
      Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
    }
  }

  fn adopt(&mut self, child: DatumaState) {
    self.children.push(child);
    self.pending_else = false;
    if self.phase == IfPhase::Condition {
      self.phase = IfPhase::OpenBrace;
    }
  }

  fn accepts_resolved_child(&self) -> bool {
    self.phase == IfPhase::Then && self.children.len() >= 2
  }

  fn close_after_adopt(&mut self) -> Option<DatumaState> {
    if self.phase == IfPhase::Then && self.children.len() >= 3 {
      Some(self.close_state().unwrap_or_default())
    } else {
      None
    }
  }

  fn close_state(&mut self) -> Option<DatumaState> {
    if self.phase == IfPhase::Then && self.children.len() >= 2 {
      close_if_or_ternary(&mut self.children, &mut self.yield_error)
    } else {
      None
    }
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if self.yield_error {
      Some(ternary_yield_error())
    } else if state.is_some() {
      None
    } else if self.phase == IfPhase::OpenBrace || self.phase == IfPhase::Then {
      Some(expected(messages::OPEN_BRACE))
    } else {
      Some(expected_closing(messages::CLOSE_PAREN))
    }
  }
}

#[derive(Debug, PartialEq, Eq)]
enum ElsePhase {
  OpenBrace,
  Body,
}

#[derive(Debug)]
pub struct ElseParseMode {
  children: Vec<DatumaState>,
  phase: ElsePhase,
  flags: KeywordFlags,
}

impl ElseParseMode {
  pub fn new(flags: KeywordFlags) -> Self {
    Self {
      children: Vec::new(),
      phase: ElsePhase::OpenBrace,
      flags,
    }
  }
}

impl Display for ElseParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/else")
  }
}

impl ParseMode for ElseParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    match self.phase {
      ElsePhase::OpenBrace => {
        if input == '{' {
          self.phase = ElsePhase::Body;
          Ok((
            ParseStepMutation::StartMode(Box::new(ProgramParseMode::with_flags(
              self.flags.with_yield(),
            ))),
            ParsetStepFlow::Captured,
          ))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate))
        } else if is_ident_start(input) {
          Ok((
            ParseStepMutation::StartMode(Box::new(IdentifierParseMode::starting_with_flags(
              input,
              self.flags.union(KeywordFlags::ELSEIF),
            ))),
            ParsetStepFlow::Captured,
          ))
        } else {
          Err(expected(messages::OPEN_BRACE))
        }
      }
      ElsePhase::Body => {
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
    self.children.push(child);
  }

  fn close_after_adopt(&mut self) -> Option<DatumaState> {
    if self
      .children
      .last()
      .is_some_and(|child| matches!(core_value(child), Some(CoreValue::ElseIf)))
    {
      self.children.pop()
    } else if self.phase == ElsePhase::Body && self.children.len() == 1 {
      self.close_state()
    } else {
      None
    }
  }

  fn close_state(&mut self) -> Option<DatumaState> {
    if self.phase == ElsePhase::Body && self.children.len() == 1 {
      Some(DatumaState::node(
        Some(Box::new(CoreValue::Else)),
        std::mem::take(&mut self.children),
      ))
    } else {
      None
    }
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    match state {
      Some(_) => None,
      None => Some(expected(messages::OPEN_BRACE)),
    }
  }
}

#[derive(Debug, PartialEq, Eq)]
enum ElseIfPhase {
  OpenParen,
  Condition,
  OpenBrace,
  Then,
}

#[derive(Debug)]
pub struct ElseIfParseMode {
  children: Vec<DatumaState>,
  phase: ElseIfPhase,
  flags: KeywordFlags,
  pending_else: bool,
}

impl ElseIfParseMode {
  pub fn new(flags: KeywordFlags) -> Self {
    Self {
      children: Vec::new(),
      phase: ElseIfPhase::OpenParen,
      flags,
      pending_else: false,
    }
  }
}

impl Display for ElseIfParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/elseif")
  }
}

impl ParseMode for ElseIfParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    match self.phase {
      ElseIfPhase::OpenParen => {
        if input == '(' {
          self.phase = ElseIfPhase::Condition;
          Ok((
            ParseStepMutation::StartMode(Box::new(GroupedParseMode::new())),
            ParsetStepFlow::Captured,
          ))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate))
        } else {
          Err(expected_closing(messages::OPEN_PAREN))
        }
      }
      ElseIfPhase::Condition => Err(crate::core::parser::internal_invariant(
        "elseif awaiting condition close",
      )),
      ElseIfPhase::OpenBrace => {
        if input == '{' {
          self.phase = ElseIfPhase::Then;
          Ok((
            ParseStepMutation::StartMode(Box::new(ProgramParseMode::with_flags(
              self.flags.with_yield(),
            ))),
            ParsetStepFlow::Captured,
          ))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate))
        } else {
          Err(expected(messages::OPEN_BRACE))
        }
      }
      ElseIfPhase::Then => {
        if self.children.len() < 2 || self.pending_else {
          if input.is_whitespace() {
            Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
          } else {
            Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate))
          }
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if is_ident_start(input) {
          self.pending_else = true;
          Ok((
            ParseStepMutation::StartMode(Box::new(IdentifierParseMode::starting_with_flags(
              input,
              self.flags.with_else(),
            ))),
            ParsetStepFlow::Captured,
          ))
        } else if input == ';' {
          Ok((
            ParseStepMutation::CloseMode(self.close_state()),
            ParsetStepFlow::Captured,
          ))
        } else {
          Ok((
            ParseStepMutation::CloseMode(self.close_state()),
            ParsetStepFlow::Propagate,
          ))
        }
      }
    }
  }

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    if self.phase == ElseIfPhase::Then && self.children.len() == 2 {
      then_else_ident(self.flags, input)
    } else {
      Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
    }
  }

  fn adopt(&mut self, child: DatumaState) {
    self.children.push(child);
    self.pending_else = false;
    if self.phase == ElseIfPhase::Condition {
      self.phase = ElseIfPhase::OpenBrace;
    }
  }

  fn accepts_resolved_child(&self) -> bool {
    self.phase == ElseIfPhase::Then && self.children.len() >= 2
  }

  fn close_after_adopt(&mut self) -> Option<DatumaState> {
    if self.phase == ElseIfPhase::Then && self.children.len() >= 3 {
      self.close_state()
    } else {
      None
    }
  }

  fn close_state(&mut self) -> Option<DatumaState> {
    if self.phase == ElseIfPhase::Then && self.children.len() >= 2 {
      Some(DatumaState::node(
        Some(Box::new(CoreValue::ElseIf)),
        std::mem::take(&mut self.children),
      ))
    } else {
      None
    }
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if state.is_some() {
      None
    } else if self.phase == ElseIfPhase::OpenBrace || self.phase == ElseIfPhase::Then {
      Some(expected(messages::OPEN_BRACE))
    } else {
      Some(expected_closing(messages::CLOSE_PAREN))
    }
  }
}

#[derive(Debug, PartialEq, Eq)]
enum ForPhase {
  OpenParen,
  Head,
  OpenBrace,
  Body,
}

#[derive(Debug, PartialEq, Eq)]
enum ForHeadPhase {
  First,
  AfterFirst,
  InKeyword { buf: String },
  InIterable,
  Classic { clause: u8 },
}

#[derive(Debug)]
pub struct ForParseMode {
  children: Vec<DatumaState>,
  phase: ForPhase,
  head_phase: ForHeadPhase,
  first_ident: Option<DatumaState>,
  head_parts: Vec<DatumaState>,
  for_in: bool,
  flags: KeywordFlags,
}

impl ForParseMode {
  pub fn new(flags: KeywordFlags) -> Self {
    Self {
      children: Vec::new(),
      phase: ForPhase::OpenParen,
      head_phase: ForHeadPhase::First,
      first_ident: None,
      head_parts: Vec::new(),
      for_in: false,
      flags,
    }
  }

  fn head_step(&mut self, input: char) -> ParseStep {
    match &mut self.head_phase {
      ForHeadPhase::First => {
        if input == ')' {
          Err(expected(messages::FOR_HEAD))
        } else if input == ';' {
          self.head_parts.push(instruction_state(Vec::new()));
          self.head_phase = ForHeadPhase::Classic { clause: 1 };
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Ok((
            ParseStepMutation::StartMode(Box::new(InstructionParseMode::until_close_paren())),
            ParsetStepFlow::Repropagate,
          ))
        }
      }
      ForHeadPhase::AfterFirst => {
        if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input == ';' {
          if let Some(first) = self.first_ident.take() {
            self.head_parts.push(instruction_state(vec![first]));
          }
          self.head_phase = ForHeadPhase::Classic { clause: 1 };
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input == ')' {
          Err(expected(messages::FOR_HEAD))
        } else if is_operator_char(input) {
          let first = self.first_ident.take().expect("for first ident");
          self.head_phase = ForHeadPhase::Classic { clause: 0 };
          Ok((
            ParseStepMutation::StartMode(Box::new(InstructionParseMode::for_clause_with_part(
              first,
            ))),
            ParsetStepFlow::Repropagate,
          ))
        } else if is_ident_start(input) {
          self.head_phase = ForHeadPhase::InKeyword {
            buf: starting_buf(input),
          };
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Err(ParseErrorKind::UnexpectedChar(input))
        }
      }
      ForHeadPhase::InKeyword { buf } => {
        if is_ident_continue(input) {
          if buf.as_str() == "in" {
            self.for_in = true;
            self.head_phase = ForHeadPhase::InIterable;
            start_value(input)
          } else {
            buf.push(input);
            if !"in".starts_with(buf.as_str()) {
              Err(expected(messages::IN_KEYWORD))
            } else {
              Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
            }
          }
        } else if buf.as_str() != "in" {
          Err(expected(messages::IN_KEYWORD))
        } else {
          self.for_in = true;
          self.head_phase = ForHeadPhase::InIterable;
          if input.is_whitespace() {
            Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
          } else {
            start_value(input)
          }
        }
      }
      ForHeadPhase::InIterable => {
        if input == ')' {
          self.phase = ForPhase::OpenBrace;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          start_value(input)
        }
      }
      ForHeadPhase::Classic { clause } => {
        if input == ')' {
          while self.head_parts.len() < 3 {
            self.head_parts.push(instruction_state(Vec::new()));
          }
          self.phase = ForPhase::OpenBrace;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input == ';' {
          if self.head_parts.len() <= *clause as usize {
            self.head_parts.push(instruction_state(Vec::new()));
          }
          *clause += 1;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else {
          Ok((
            ParseStepMutation::StartMode(Box::new(InstructionParseMode::for_clause())),
            ParsetStepFlow::Repropagate,
          ))
        }
      }
    }
  }

  fn finish_with_body(&mut self, program: DatumaState) {
    let head = if self.for_in {
      let var = self.first_ident.take().expect("for-in var");
      let iter = if self.head_parts.len() == 1 {
        self.head_parts.pop().expect("for-in iterable")
      } else {
        instruction_state(std::mem::take(&mut self.head_parts))
      };
      DatumaState::node(None, vec![var, iter])
    } else {
      DatumaState::node(None, std::mem::take(&mut self.head_parts))
    };
    self.children = vec![head, program];
  }
}

impl Display for ForParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/for")
  }
}

impl ParseMode for ForParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    match self.phase {
      ForPhase::OpenParen => {
        if input == '(' {
          self.phase = ForPhase::Head;
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate))
        } else {
          Err(expected_closing(messages::OPEN_PAREN))
        }
      }
      ForPhase::Head => self.head_step(input),
      ForPhase::OpenBrace => {
        if input == '{' {
          self.phase = ForPhase::Body;
          Ok((
            ParseStepMutation::StartMode(Box::new(ProgramParseMode::with_flags(
              self.flags.with_break(),
            ))),
            ParsetStepFlow::Captured,
          ))
        } else if input.is_whitespace() {
          Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate))
        } else {
          Err(expected(messages::OPEN_BRACE))
        }
      }
      ForPhase::Body => {
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

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    if self.phase == ForPhase::Head {
      match self.head_step(input) {
        Ok((ParseStepMutation::StartMode(mode), flow)) => {
          Ok((ParseResolveMutation::NoDismissStartMode(mode), flow))
        }
        Ok((ParseStepMutation::Nothing, flow)) => Ok((ParseResolveMutation::Nothing, flow)),
        Ok((_, _)) => Err(crate::core::parser::internal_invariant(
          "unexpected for head parse step",
        )),
        Err(e) => Err(e),
      }
    } else {
      Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
    }
  }

  fn accepts_resolved_child(&self) -> bool {
    self.phase == ForPhase::Head
  }

  fn reactivate_after_child_close(&mut self) -> bool {
    self.phase == ForPhase::Head
  }

  fn adopt(&mut self, child: DatumaState) {
    match self.phase {
      ForPhase::Head => match self.head_phase {
        ForHeadPhase::First => match take_lone_ident(child) {
          Ok(ident) => {
            self.first_ident = Some(ident);
            self.head_phase = ForHeadPhase::AfterFirst;
          }
          Err(child) => {
            self.head_parts.push(child);
            self.head_phase = ForHeadPhase::Classic { clause: 0 };
          }
        },
        ForHeadPhase::InIterable => {
          self.head_parts.push(child);
        }
        ForHeadPhase::Classic { .. } => {
          self.head_parts.push(child);
        }
        ForHeadPhase::AfterFirst | ForHeadPhase::InKeyword { .. } => {
          self.head_parts.push(child);
        }
      },
      ForPhase::Body => self.finish_with_body(child),
      ForPhase::OpenBrace | ForPhase::OpenParen => {}
    }
  }

  fn close_after_adopt(&mut self) -> Option<DatumaState> {
    self.close_state()
  }

  fn close_state(&mut self) -> Option<DatumaState> {
    if self.phase == ForPhase::Body && self.children.len() == 2 {
      Some(DatumaState::node(
        Some(Box::new(CoreValue::For)),
        std::mem::take(&mut self.children),
      ))
    } else {
      None
    }
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if state.is_some() {
      None
    } else if self.phase == ForPhase::Body || self.phase == ForPhase::OpenBrace {
      Some(expected(messages::OPEN_BRACE))
    } else {
      Some(expected(messages::FOR_HEAD))
    }
  }
}

fn then_else_ident(flags: KeywordFlags, input: char) -> ParseResolveStep {
  if input.is_whitespace() {
    Ok((ParseResolveMutation::Nothing, ParsetStepFlow::Captured))
  } else if is_ident_start(input) {
    Ok((
      ParseResolveMutation::NoDismissStartMode(Box::new(IdentifierParseMode::starting_with_flags(
        input,
        flags.with_else(),
      ))),
      ParsetStepFlow::Captured,
    ))
  } else {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }
}

fn take_lone_ident(mut state: DatumaState) -> Result<DatumaState, DatumaState> {
  if matches!(core_value(&state), Some(CoreValue::Ident(_))) {
    Ok(state)
  } else if matches!(core_value(&state), Some(CoreValue::Instruction { .. }))
    && state.children.len() == 1
    && matches!(core_value(&state.children[0]), Some(CoreValue::Ident(_)))
  {
    Ok(state.children.pop().expect("for head ident"))
  } else {
    Err(state)
  }
}

fn instruction_state(parts: Vec<DatumaState>) -> DatumaState {
  DatumaState::node(
    Some(Box::new(CoreValue::Instruction {
      file_meta: ParseFileMetadata::source("<synthetic>"),
      pos_meta: ParseCursorMetadata::default(),
    })),
    parts,
  )
}
