#[path = "_operator.rs"]
#[macro_use]
mod _operator;

pub use _operator::{CollectionKind, OperatorFollowUp};

use std::fmt::Display;
use std::{matches, write};

use super::program::_stmt::is_ident_start;
use crate::core::modes::on_parse_capture_whitespace;
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation,
  ParsetStepFlow, expected,
};
use crate::core::state::DatumaState;
use crate::core::value::{CoreOperator, CoreValue};

use _operator::{
  ExpectFlags, OperatorKind, close, close_captured, follow_up_expect_label, follow_up_start_mode,
  initial_expect, op_leaf, operator_kind, resolve_second_char, single_op, value_start_mode,
};

/// Lexical context that selects which operator tokens and compounds are valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorContext {
  Numeric,
  Ident,
  String,
  Boolean,
  Null,
  Array,
  Dict,
  InvokedFunction,
}

#[derive(Debug)]
pub struct OperatorParseMode {
  pub(crate) kind: OperatorKind,
  pub(crate) context: OperatorContext,
  pub(crate) expect: ExpectFlags,
  pub(crate) assign_tail: Option<(CoreOperator, CoreOperator)>,
  pub(crate) follow_up: Option<OperatorFollowUp>,
}

impl ParseMode for OperatorParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    if let Some((assign, fallback)) = self.assign_tail.take() {
      if input == '=' && self.context != OperatorContext::Ident {
        Err(expected(messages::ASSIGN))
      } else {
        resolve_assign_tail!(input, assign, fallback)
      }
    } else if matches!(self.follow_up, Some(OperatorFollowUp::UnaryNot)) {
      self.on_unary_not(input)
    } else if self.expect.single_close_mode() {
      self.on_single_close(input)
    } else {
      resolve_second_char(self, input)
    }
  }

  fn on_parse_resolved(&mut self, input: char) -> ParseResolveStep {
    if input.is_whitespace() {
      Ok((ParseResolveMutation::Nothing, ParsetStepFlow::Captured))
    } else if let Some(follow_up) = self.follow_up.take() {
      match follow_up_start_mode(follow_up, input) {
        Ok(mode) => Ok((
          ParseResolveMutation::StartMode(mode),
          ParsetStepFlow::Captured,
        )),
        Err(e) => Err(e),
      }
    } else {
      Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
    }
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if state.is_some() {
      None
    } else if let Some(follow_up) = &self.follow_up {
      Some(expected(follow_up_expect_label(follow_up)))
    } else if self.assign_tail.is_some() {
      Some(expected(messages::ASSIGN))
    } else if matches!(self.follow_up, Some(OperatorFollowUp::UnaryNot)) {
      Some(expected(messages::UNARY_NOT))
    } else {
      Some(expected(_operator::expect_label(self.kind, self.expect)))
    }
  }
}

impl Display for OperatorParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/operator")
  }
}

/// Start dot-member operator (`.`).
pub(crate) fn resolve_dot_operator() -> ParseResolveStep {
  Ok((
    ParseResolveMutation::StartMode(Box::new(OperatorParseMode::dot_member())),
    ParsetStepFlow::Captured,
  ))
}

/// Start a collection operator after an array or dict literal operand.
#[expect(dead_code)]
pub(crate) fn resolve_collection_operator(input: char, left: DatumaState) -> ParseResolveStep {
  if is_ident_start(input) {
    Err(ParseErrorKind::UnexpectedChar(input))
  } else if let Some(core) = left
    .value
    .as_ref()
    .and_then(|value| value.as_any().downcast_ref::<CoreValue>())
  {
    let context = match core {
      CoreValue::Array => OperatorContext::Array,
      CoreValue::Dict => OperatorContext::Dict,
      _ => return Err(ParseErrorKind::UnexpectedChar(input)),
    };
    if let Some(follow_up) = collection_follow_up(context, input, left.children) {
      match OperatorParseMode::from_char_with_follow_up(input, context, follow_up) {
        Ok(mode) => Ok((
          ParseResolveMutation::StartMode(Box::new(mode)),
          ParsetStepFlow::Captured,
        )),
        Err(e) => Err(e),
      }
    } else {
      Err(ParseErrorKind::UnexpectedChar(input))
    }
  } else {
    Err(ParseErrorKind::UnexpectedChar(input))
  }
}

/// Start an operator after a parsed left operand, using invoked-function context when applicable.
#[expect(dead_code)]
pub(crate) fn resolve_operators_for_left(input: char, left: &DatumaState) -> ParseResolveStep {
  if is_ident_start(input) {
    Err(ParseErrorKind::UnexpectedChar(input))
  } else if let Some(kind) = operator_kind(input) {
    if let Some(CoreValue::InvokedFunction(_)) = left
      .value
      .as_ref()
      .and_then(|value| value.as_any().downcast_ref())
    {
      if _operator::char_allowed_in_context(kind, OperatorContext::InvokedFunction) {
        Ok((
          ParseResolveMutation::StartMode(Box::new(OperatorParseMode {
            kind,
            context: OperatorContext::InvokedFunction,
            expect: initial_expect(kind, OperatorContext::InvokedFunction),
            assign_tail: None,
            follow_up: None,
          })),
          ParsetStepFlow::Captured,
        ))
      } else {
        Err(ParseErrorKind::UnexpectedChar(input))
      }
    } else {
      resolve_operators(
        input,
        &[
          OperatorContext::Ident,
          OperatorContext::Numeric,
          OperatorContext::InvokedFunction,
        ],
      )
    }
  } else {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }
}

/// Start an operator from `on_parse` using the first matching context in `contexts`.
pub(crate) fn start_operator(input: char, contexts: &[OperatorContext]) -> ParseStep {
  match resolve_operators(input, contexts)? {
    (ParseResolveMutation::StartMode(mode), flow) => Ok((ParseStepMutation::StartMode(mode), flow)),
    _ => Err(ParseErrorKind::UnexpectedChar(input)),
  }
}

/// Start an operator mode for the first matching context in `contexts`.
pub(crate) fn resolve_operators(input: char, contexts: &[OperatorContext]) -> ParseResolveStep {
  if is_ident_start(input) {
    Err(ParseErrorKind::UnexpectedChar(input))
  } else if let Some(kind) = operator_kind(input) {
    let mut step = Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate));
    for &context in contexts {
      if _operator::char_allowed_in_context(kind, context) {
        step = Ok((
          ParseResolveMutation::StartMode(Box::new(OperatorParseMode {
            kind,
            context,
            expect: initial_expect(kind, context),
            assign_tail: None,
            follow_up: None,
          })),
          ParsetStepFlow::Captured,
        ));
        break;
      }
    }
    step
  } else {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }
}

/// Whether `ch` may close an ident and start an infix operator (not `.`).
pub fn is_ident_operator(ch: char) -> bool {
  matches!(
    operator_kind(ch),
    Some(kind) if _operator::char_allowed_in_context(kind, OperatorContext::Ident) && kind != OperatorKind::Dot
  )
}

/// Build collection merge/subtract follow-up when array or dict sees `+`, `-`, `^`, or `&`.
pub fn collection_follow_up(
  context: OperatorContext,
  kind: char,
  outer: Vec<DatumaState>,
) -> Option<OperatorFollowUp> {
  let op_kind = operator_kind(kind)?;
  match (context, op_kind) {
    (OperatorContext::Array, OperatorKind::Plus) => Some(OperatorFollowUp::ArrayMerge { outer }),
    (OperatorContext::Dict, OperatorKind::Plus) => Some(OperatorFollowUp::DictMerge { outer }),
    (OperatorContext::Array, OperatorKind::Minus) => {
      Some(OperatorFollowUp::ArraySubtract { outer })
    }
    (OperatorContext::Dict, OperatorKind::Minus) => Some(OperatorFollowUp::DictSubtract { outer }),
    (OperatorContext::Array, OperatorKind::Caret | OperatorKind::Amp) => {
      Some(OperatorFollowUp::SameCollection {
        outer,
        kind: CollectionKind::Array,
      })
    }
    (OperatorContext::Dict, OperatorKind::Caret | OperatorKind::Amp) => {
      Some(OperatorFollowUp::SameCollection {
        outer,
        kind: CollectionKind::Dict,
      })
    }
    _ => None,
  }
}

impl OperatorParseMode {
  pub fn from_char(ch: char, context: OperatorContext) -> Result<Self, ParseErrorKind> {
    if let Some(kind) = operator_kind(ch) {
      if _operator::char_allowed_in_context(kind, context) {
        Ok(Self {
          kind,
          context,
          expect: initial_expect(kind, context),
          assign_tail: None,
          follow_up: None,
        })
      } else {
        Err(ParseErrorKind::UnexpectedChar(ch))
      }
    } else {
      Err(ParseErrorKind::UnexpectedChar(ch))
    }
  }

  pub fn from_char_with_follow_up(
    ch: char,
    context: OperatorContext,
    follow_up: OperatorFollowUp,
  ) -> Result<Self, ParseErrorKind> {
    let mut mode = Self::from_char(ch, context)?;
    mode.follow_up = Some(follow_up);
    match (mode.kind, mode.context) {
      (OperatorKind::Plus, OperatorContext::Array)
      | (OperatorKind::Plus, OperatorContext::Dict)
      | (OperatorKind::Minus, OperatorContext::Array)
      | (OperatorKind::Minus, OperatorContext::Dict) => {
        mode.expect = ExpectFlags::SINGLE_REQUIRED;
      }
      (OperatorKind::Caret, OperatorContext::Array)
      | (OperatorKind::Caret, OperatorContext::Dict)
      | (OperatorKind::Amp, OperatorContext::Array)
      | (OperatorKind::Amp, OperatorContext::Dict) => {
        mode.expect = ExpectFlags::ALLOW_ALL;
      }
      _ => {}
    }
    Ok(mode)
  }

  pub fn unary_not() -> Self {
    Self {
      kind: OperatorKind::Bang,
      context: OperatorContext::Numeric,
      expect: ExpectFlags::SINGLE_REQUIRED,
      assign_tail: None,
      follow_up: Some(OperatorFollowUp::UnaryNot),
    }
  }

  pub fn dot_member() -> Self {
    Self {
      kind: OperatorKind::Dot,
      context: OperatorContext::Ident,
      expect: ExpectFlags::SINGLE_REQUIRED,
      assign_tail: None,
      follow_up: Some(OperatorFollowUp::DotMember),
    }
  }

  pub fn grouped_open() -> Self {
    Self {
      kind: OperatorKind::Plus,
      context: OperatorContext::Numeric,
      expect: ExpectFlags::SINGLE_REQUIRED,
      assign_tail: None,
      follow_up: Some(OperatorFollowUp::GroupedExpr),
    }
  }

  fn on_single_close(&mut self, input: char) -> ParseStep {
    on_parse_capture_whitespace(input).map_or_else(
      || {
        if self.follow_up.is_some() {
          self.close_with_follow_up(single_op(self.kind), input)
        } else {
          close(single_op(self.kind))
        }
      },
      |v| Ok(v),
    )
  }

  fn on_unary_not(&mut self, input: char) -> ParseStep {
    on_parse_capture_whitespace(input).map_or_else(
      || match value_start_mode(input) {
        Ok(mode) => Ok((
          ParseStepMutation::CloseAndStartMode(Some(op_leaf(CoreOperator::Not)), mode),
          ParsetStepFlow::Captured,
        )),
        Err(e) => Err(e),
      },
      |v| Ok(v),
    )
  }

  fn close_with_follow_up(&mut self, op: CoreOperator, input: char) -> ParseStep {
    if let Some(follow_up) = self.follow_up.take() {
      if input.is_whitespace() {
        self.follow_up = Some(follow_up);
        Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
      } else {
        match follow_up_start_mode(follow_up, input) {
          Ok(mode) => Ok((
            ParseStepMutation::CloseAndStartMode(Some(op_leaf(op)), mode),
            ParsetStepFlow::Captured,
          )),
          Err(e) => Err(e),
        }
      }
    } else {
      close(op)
    }
  }
}
