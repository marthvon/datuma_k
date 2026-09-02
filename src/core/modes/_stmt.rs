use std::matches;

use super::super::{
  ArrayParseMode, DictParseMode, FloatParseMode, GroupedParseMode, IdentifierParseMode,
  IntegerParseMode, StringParseMode,
};
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseStep, ParseStepMutation, ParsetStepFlow,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordFlags(u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Keyword {
  Fn,
  If,
  For,
  Else,
  In,
  Return,
  Break,
  Yield,
}

impl KeywordFlags {
  pub const NONE: Self = Self(0);
  pub const RETURN: Self = Self(1 << 0);
  pub const BREAK: Self = Self(1 << 1);
  pub const YIELD: Self = Self(1 << 2);
  pub const ELSE: Self = Self(1 << 3);
  pub const STATEMENT: Self = Self(1 << 4);
  pub const IN: Self = Self(1 << 5);
  pub const ELSEIF: Self = Self(1 << 6);

  pub const fn contains(self, flag: Self) -> bool {
    self.0 & flag.0 == flag.0
  }

  pub const fn union(self, other: Self) -> Self {
    Self(self.0 | other.0)
  }

  pub const fn difference(self, other: Self) -> Self {
    Self(self.0 & !other.0)
  }

  pub const fn with_break(self) -> Self {
    self.union(Self::BREAK)
  }

  pub const fn with_yield(self) -> Self {
    self.union(Self::YIELD)
  }

  pub const fn with_else(self) -> Self {
    self.union(Self::ELSE)
  }

  pub const fn top_level() -> Self {
    Self(Self::RETURN.0 | Self::STATEMENT.0)
  }

  pub const fn function_body() -> Self {
    Self(Self::RETURN.0 | Self::STATEMENT.0)
  }
}

pub(crate) fn value_parse_mode(input: char) -> Result<Box<dyn ParseMode>, ParseErrorKind> {
  match input {
    '"' => Ok(Box::new(StringParseMode::new())),
    '[' => Ok(Box::new(ArrayParseMode::new())),
    '{' => Ok(Box::new(DictParseMode::new())),
    '.' => Ok(Box::new(FloatParseMode::new())),
    input if input.is_ascii_digit() || input == '-' => {
      Ok(Box::new(IntegerParseMode::starting(input)))
    }
    input if input.is_ascii_alphabetic() || input == '_' => {
      Ok(Box::new(IdentifierParseMode::starting(input)))
    }
    _ => Err(ParseErrorKind::UnexpectedChar(input)),
  }
}

pub(crate) fn start_value(input: char) -> ParseStep {
  if input == '(' {
    Ok((
      ParseStepMutation::StartMode(Box::new(GroupedParseMode::new())),
      ParsetStepFlow::Captured,
    ))
  } else {
    match value_parse_mode(input) {
      Ok(mode) => Ok((ParseStepMutation::StartMode(mode), ParsetStepFlow::Captured)),
      Err(ParseErrorKind::UnexpectedChar(_)) => {
        Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate))
      }
      Err(e) => Err(e),
    }
  }
}

pub(crate) fn is_statement_start(ch: char) -> bool {
  is_ident_start(ch) || ch.is_ascii_digit() || matches!(ch, '"' | '[' | '{' | '(' | '-' | '+' | '!')
}

pub(crate) fn is_prefix_operator(ch: char) -> bool {
  matches!(ch, '+' | '!')
}

pub(crate) fn is_ident_continue(ch: char) -> bool {
  ch.is_ascii_alphanumeric() || ch == '_'
}

pub(crate) fn is_ident_start(ch: char) -> bool {
  ch.is_ascii_alphabetic() || ch == '_'
}

pub(crate) fn is_operator_char(ch: char) -> bool {
  matches!(
    ch,
    '+' | '-' | '*' | '/' | '%' | '^' | '&' | '|' | '!' | '=' | '<' | '>' | '.'
  )
}

pub(crate) fn keyword_from_buf(buf: &str) -> Option<Keyword> {
  match buf {
    "fn" => Some(Keyword::Fn),
    "if" => Some(Keyword::If),
    "for" => Some(Keyword::For),
    "else" => Some(Keyword::Else),
    "in" => Some(Keyword::In),
    "return" => Some(Keyword::Return),
    "break" => Some(Keyword::Break),
    "yield" => Some(Keyword::Yield),
    _ => None,
  }
}

pub(crate) fn core_value(state: &DatumaState) -> Option<&CoreValue> {
  state
    .value
    .as_ref()
    .and_then(|value| value.as_any().downcast_ref::<CoreValue>())
}

pub(crate) fn close_if_or_ternary(
  children: &mut Vec<DatumaState>,
  yield_error: &mut bool,
) -> Option<DatumaState> {
  let is_ternary = children.len() == 3
    && program_sole_yield(&children[1])
    && else_program_sole_yield(&children[2]);
  if is_ternary {
    let mut taken = std::mem::take(children);
    let cond = taken.remove(0);
    let then_program = taken.remove(0);
    let else_node = taken.remove(0);
    let then_yield = then_program
      .children
      .into_iter()
      .next()
      .expect("then yield");
    let else_program = else_node.children.into_iter().next().expect("else program");
    let else_yield = else_program
      .children
      .into_iter()
      .next()
      .expect("else yield");
    Some(DatumaState::node(
      Some(Box::new(CoreValue::If)),
      vec![
        cond,
        take_yield_payload(then_yield),
        take_yield_payload(else_yield),
      ],
    ))
  } else if children.iter().any(subtree_has_yield) {
    *yield_error = true;
    None
  } else {
    Some(DatumaState::node(
      Some(Box::new(CoreValue::If)),
      std::mem::take(children),
    ))
  }
}

fn subtree_has_yield(state: &DatumaState) -> bool {
  matches!(core_value(state), Some(CoreValue::Yield))
    || state.children.iter().any(subtree_has_yield)
}

fn program_sole_yield(program: &DatumaState) -> bool {
  matches!(core_value(program), Some(CoreValue::Program))
    && program.children.len() == 1
    && matches!(core_value(&program.children[0]), Some(CoreValue::Yield))
}

fn else_program_sole_yield(else_state: &DatumaState) -> bool {
  matches!(core_value(else_state), Some(CoreValue::Else))
    && else_state.children.len() == 1
    && program_sole_yield(&else_state.children[0])
}

fn take_yield_payload(mut yield_state: DatumaState) -> DatumaState {
  if yield_state.children.len() == 1 {
    yield_state.children.pop().expect("yield payload")
  } else {
    DatumaState::node(None, std::mem::take(&mut yield_state.children))
  }
}

pub(crate) fn ternary_yield_error() -> ParseErrorKind {
  crate::core::parser::expected(messages::TERNARY_YIELD)
}
