use std::matches;

use super::super::{ArrayParseMode, DictParseMode, GroupedParseMode, IdentifierParseMode};
use super::OperatorContext;
use super::OperatorParseMode;
use crate::core::modes::program::_stmt::{is_ident_start, value_parse_mode};
use crate::core::parser::messages;
use crate::core::parser::{
  ParseErrorKind, ParseMode, ParseStep, ParseStepMutation, ParsetStepFlow, expected,
};
use crate::core::state::DatumaState;
use crate::core::value::{CoreOperator, CoreValue};

/// Array or dict side of a same-type collection operator (`^`, `&`, compounds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionKind {
  Array,
  Dict,
}

/// Parse mode to enter after the operator token closes (merge, member access, grouping, etc.).
#[derive(Debug)]
pub enum OperatorFollowUp {
  ArrayMerge {
    outer: Vec<DatumaState>,
  },
  DictMerge {
    outer: Vec<DatumaState>,
  },
  ArraySubtract {
    outer: Vec<DatumaState>,
  },
  DictSubtract {
    outer: Vec<DatumaState>,
  },
  SameCollection {
    outer: Vec<DatumaState>,
    kind: CollectionKind,
  },
  DotMember,
  UnaryNot,
  GroupedExpr,
}

/// Second-character suffix dimensions while the operator token is open.
///
/// Four dimensions (`=`, repeat, assign, single), two bits each:
/// - bit 0: enabled
/// - bit 1: optional (1 = may fall back to base op; 0 = required)
///
/// `ALLOW_ALL` (`0xFF`) enables every dimension as optional — inline disambiguation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExpectFlags(u8);

impl ExpectFlags {
  pub(crate) const DIM_ENABLED: u8 = 0b01;
  pub(crate) const DIM_OPTIONAL: u8 = 0b10;

  pub(crate) const EQUALS_SHIFT: u8 = 0;
  pub(crate) const REPEAT_SHIFT: u8 = 2;
  pub(crate) const ASSIGN_SHIFT: u8 = 4;
  pub(crate) const SINGLE_SHIFT: u8 = 6;

  pub(crate) const ALLOW_ALL: Self = Self(0x00FF);
  pub(crate) const EQUALS_REQUIRED: Self = Self(Self::DIM_ENABLED);
  pub(crate) const REPEAT_REQUIRED: Self = Self(Self::DIM_ENABLED << Self::REPEAT_SHIFT);
  pub(crate) const ASSIGN_EQUALS_OPTIONAL: Self = Self(
    (Self::DIM_ENABLED | Self::DIM_OPTIONAL)
      | ((Self::DIM_ENABLED | Self::DIM_OPTIONAL) << Self::ASSIGN_SHIFT),
  );
  pub(crate) const SINGLE_REQUIRED: Self = Self(Self::DIM_ENABLED << Self::SINGLE_SHIFT);

  fn dim(self, shift: u8) -> u8 {
    (self.0 >> shift) & 0b11
  }
  pub(crate) fn dim_enabled(self, shift: u8) -> bool {
    self.dim(shift) & Self::DIM_ENABLED != 0
  }

  /// Suffix may be omitted; base op is still valid.
  pub(crate) fn dim_optional(self, shift: u8) -> bool {
    self.dim(shift) & Self::DIM_OPTIONAL != 0
  }

  /// Suffix must appear; base-only close is invalid.
  pub(crate) fn dim_required(self, shift: u8) -> bool {
    self.dim_enabled(shift) && !self.dim_optional(shift)
  }

  /// Every suffix dimension optional — pick compound vs base from the next char.
  pub(crate) fn disambiguates(self) -> bool {
    self.0 & 0xFF == Self::ALLOW_ALL.0
  }

  /// Only waiting for the right-hand operand (no `=`, repeat, or assign suffix).
  pub(crate) fn single_close_mode(self) -> bool {
    self.dim_enabled(Self::SINGLE_SHIFT)
      && !self.dim_enabled(Self::EQUALS_SHIFT)
      && !self.dim_enabled(Self::REPEAT_SHIFT)
      && !self.dim_enabled(Self::ASSIGN_SHIFT)
  }

  /// Next non-matching char may close as the bare/base operator.
  pub(crate) fn can_close_base(self) -> bool {
    self.disambiguates()
      || self.dim_optional(Self::EQUALS_SHIFT)
      || self.dim_optional(Self::REPEAT_SHIFT)
      || self.dim_optional(Self::ASSIGN_SHIFT)
  }

  /// At least one of `=`, repeat, or assign must still be satisfied.
  pub(crate) fn any_suffix_required(self) -> bool {
    self.dim_required(Self::EQUALS_SHIFT)
      || self.dim_required(Self::REPEAT_SHIFT)
      || self.dim_required(Self::ASSIGN_SHIFT)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OperatorKind {
  Plus,
  Minus,
  Star,
  Slash,
  Percent,
  Caret,
  Amp,
  Pipe,
  Bang,
  Eq,
  Lt,
  Gt,
  Dot,
}

macro_rules! expect_error {
  ($kind:expr, $flags:expr) => {
    Err(expected(expect_label($kind, $flags)))
  };
}

macro_rules! start_assign_tail {
  ($mode:expr, $assign:expr, $fallback:expr) => {{
    $mode.assign_tail = Some(($assign, $fallback));
    Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
  }};
}

macro_rules! resolve_assign_tail {
  ($input:expr, $assign:expr, $fallback:expr) => {
    if $input == '=' {
      close_captured($assign)
    } else {
      close($fallback)
    }
  };
}

macro_rules! compound_assign_or_captured {
  ($mode:expr, $assign_when:expr, $assign:expr, $compound:expr) => {
    if $assign_when || matches!($mode.context, OperatorContext::InvokedFunction) {
      start_assign_tail!($mode, $assign, $compound)
    } else {
      close_captured($compound)
    }
  };
}

macro_rules! compound_or_base_or_error {
  (
    $mode:expr,
    $flags:expr,
    $kind:expr,
    $compound_when:expr,
    $assign_when:expr,
    $assign:expr,
    $compound:expr,
    $base:expr
  ) => {
    if $compound_when {
      compound_assign_or_captured!($mode, $assign_when, $assign, $compound)
    } else if $flags.can_close_base() {
      close($base)
    } else {
      expect_error!($kind, $flags)
    }
  };
}

macro_rules! close_or_error {
  ($kind:expr, $flags:expr, $base:expr) => {
    if $flags.any_suffix_required() {
      expect_error!($kind, $flags)
    } else if $flags.can_close_base() {
      close($base)
    } else {
      expect_error!($kind, $flags)
    }
  };
}

macro_rules! close_equals_or_error {
  ($kind:expr, $flags:expr, $base:expr) => {
    if $flags.dim_required(ExpectFlags::EQUALS_SHIFT) {
      expect_error!($kind, $flags)
    } else if $flags.can_close_base() {
      close($base)
    } else {
      expect_error!($kind, $flags)
    }
  };
}

macro_rules! assign_or_strict {
  ($flags:expr, $ctx:expr, $kind:expr, $assign:expr, $base:expr) => {
    if $ctx == OperatorContext::Ident
      && ($flags.disambiguates() || $flags.dim_optional(ExpectFlags::ASSIGN_SHIFT))
    {
      close_captured($assign)
    } else if $flags.disambiguates() && $ctx != OperatorContext::Ident {
      Err(expected(messages::ASSIGN))
    } else if $flags.any_suffix_required() {
      expect_error!($kind, $flags)
    } else {
      close_or_error!($kind, $flags, $base)
    }
  };
}

macro_rules! resolve_repeat_assign_base {
  (
    $mode:expr,
    $input:expr,
    kind: $kind:ident,
    repeat: $repeat:literal,
    repeat_op: $repeat_op:expr,
    repeat_ok: $repeat_ok:expr,
    assign: $assign:expr,
    base: $base:expr
  ) => {{
    let flags = $mode.expect;
    match $input {
      $repeat
        if {
          flags.dim_required(ExpectFlags::REPEAT_SHIFT)
            || (flags.dim_optional(ExpectFlags::REPEAT_SHIFT) && ($repeat_ok))
        } =>
      {
        close_captured($repeat_op)
      }
      $repeat if flags.can_close_base() => close($base),
      $repeat => expect_error!(OperatorKind::$kind, flags),
      '=' => $assign,
      _ => close_or_error!(OperatorKind::$kind, flags, $base),
    }
  }};
}

macro_rules! resolve_assign_only {
  (
    $mode:expr,
    $input:expr,
    kind: $kind:ident,
    assign: $assign:expr,
    base: $base:expr
  ) => {{
    let flags = $mode.expect;
    let ctx = $mode.context;
    if $input == '=' {
      assign_or_strict!(flags, ctx, OperatorKind::$kind, $assign, $base)
    } else {
      close_or_error!(OperatorKind::$kind, flags, $base)
    }
  }};
}

pub(crate) fn follow_up_expect_label(follow_up: &OperatorFollowUp) -> &'static str {
  match follow_up {
    OperatorFollowUp::ArrayMerge { .. } | OperatorFollowUp::ArraySubtract { .. } => {
      messages::ARRAY_MERGE
    }
    OperatorFollowUp::DictMerge { .. } => messages::DICT_MERGE,
    OperatorFollowUp::DictSubtract { .. } => messages::COLLECTION_OPERAND,
    OperatorFollowUp::SameCollection { .. } => messages::COLLECTION_OPERAND,
    OperatorFollowUp::DotMember => messages::DOT_MEMBER,
    OperatorFollowUp::UnaryNot => messages::UNARY_NOT,
    OperatorFollowUp::GroupedExpr => messages::GROUPED_EXPR,
  }
}

pub(crate) fn follow_up_start_mode(
  follow_up: OperatorFollowUp,
  input: char,
) -> Result<Box<dyn ParseMode>, ParseErrorKind> {
  match follow_up {
    OperatorFollowUp::ArrayMerge { outer } | OperatorFollowUp::ArraySubtract { outer }
      if input == '[' =>
    {
      Ok(Box::new(ArrayParseMode::continuing(outer)))
    }
    OperatorFollowUp::DictMerge { outer } if input == '{' => {
      Ok(Box::new(DictParseMode::continuing(outer)))
    }
    OperatorFollowUp::DictSubtract { outer } if input == '[' => {
      Ok(Box::new(ArrayParseMode::continuing(outer)))
    }
    OperatorFollowUp::DictSubtract { outer } if input == '{' => {
      Ok(Box::new(DictParseMode::continuing(outer)))
    }
    OperatorFollowUp::SameCollection {
      outer,
      kind: CollectionKind::Array,
    } if input == '[' => Ok(Box::new(ArrayParseMode::continuing(outer))),
    OperatorFollowUp::SameCollection {
      outer,
      kind: CollectionKind::Dict,
    } if input == '{' => Ok(Box::new(DictParseMode::continuing(outer))),
    OperatorFollowUp::DotMember if is_ident_start(input) => {
      Ok(Box::new(IdentifierParseMode::starting(input)))
    }
    OperatorFollowUp::UnaryNot => value_parse_mode(input),
    OperatorFollowUp::GroupedExpr if input == '(' => Ok(Box::new(GroupedParseMode::new())),
    ref pending => Err(expected(follow_up_expect_label(pending))),
  }
}

pub(crate) fn initial_expect(kind: OperatorKind, context: OperatorContext) -> ExpectFlags {
  match (kind, context) {
    (OperatorKind::Bang, _) => ExpectFlags::EQUALS_REQUIRED,
    (OperatorKind::Dot, _) => ExpectFlags::SINGLE_REQUIRED,
    (OperatorKind::Plus | OperatorKind::Star, OperatorContext::String) => {
      ExpectFlags::SINGLE_REQUIRED
    }
    (OperatorKind::Eq, OperatorContext::Ident) => ExpectFlags::ASSIGN_EQUALS_OPTIONAL,
    (OperatorKind::Eq, _) => ExpectFlags::EQUALS_REQUIRED,
    (_, OperatorContext::Boolean | OperatorContext::Null)
      if matches!(kind, OperatorKind::Amp | OperatorKind::Pipe) =>
    {
      ExpectFlags::REPEAT_REQUIRED
    }
    (OperatorKind::Plus | OperatorKind::Minus, OperatorContext::Array | OperatorContext::Dict) => {
      ExpectFlags::SINGLE_REQUIRED
    }
    (OperatorKind::Caret | OperatorKind::Amp, OperatorContext::Array | OperatorContext::Dict) => {
      ExpectFlags::ALLOW_ALL
    }
    _ => ExpectFlags::ALLOW_ALL,
  }
}

pub(crate) fn expect_label(kind: OperatorKind, flags: ExpectFlags) -> &'static str {
  if flags.dim_required(ExpectFlags::EQUALS_SHIFT) {
    match kind {
      OperatorKind::Bang => messages::NOT_EQUAL,
      OperatorKind::Eq => messages::EQUAL_EQUAL,
      OperatorKind::Lt => messages::LESS_EQUAL,
      OperatorKind::Gt => messages::GREATER_EQUAL,
      _ => messages::EQUAL_EQUAL,
    }
  } else if flags.dim_required(ExpectFlags::REPEAT_SHIFT) {
    match kind {
      OperatorKind::Star => messages::POW,
      OperatorKind::Plus => messages::INCREMENT,
      OperatorKind::Minus => messages::DECREMENT,
      OperatorKind::Amp => messages::LOGICAL_AND,
      OperatorKind::Pipe => messages::LOGICAL_OR,
      _ => messages::EQUAL_EQUAL,
    }
  } else if flags.dim_optional(ExpectFlags::ASSIGN_SHIFT) && !flags.disambiguates() {
    messages::ASSIGN
  } else {
    messages::OPERATOR
  }
}

pub(crate) fn single_op(kind: OperatorKind) -> CoreOperator {
  match kind {
    OperatorKind::Plus => CoreOperator::Add,
    OperatorKind::Minus => CoreOperator::Sub,
    OperatorKind::Star => CoreOperator::Mul,
    OperatorKind::Dot => CoreOperator::Dot,
    OperatorKind::Caret => CoreOperator::SymmetricDiff,
    OperatorKind::Amp => CoreOperator::Intersect,
    _ => CoreOperator::Add,
  }
}

pub(crate) fn value_start_mode(input: char) -> Result<Box<dyn ParseMode>, ParseErrorKind> {
  value_parse_mode(input)
}

pub(crate) fn op_leaf(op: CoreOperator) -> DatumaState {
  DatumaState::leaf(Box::new(CoreValue::Operator(op)))
}

pub(crate) fn close(op: CoreOperator) -> ParseStep {
  Ok((
    ParseStepMutation::CloseMode(Some(op_leaf(op))),
    ParsetStepFlow::Propagate,
  ))
}

pub(crate) fn close_captured(op: CoreOperator) -> ParseStep {
  Ok((
    ParseStepMutation::CloseMode(Some(op_leaf(op))),
    ParsetStepFlow::Captured,
  ))
}

pub(crate) fn operator_kind(ch: char) -> Option<OperatorKind> {
  match ch {
    '+' => Some(OperatorKind::Plus),
    '-' => Some(OperatorKind::Minus),
    '*' => Some(OperatorKind::Star),
    '/' => Some(OperatorKind::Slash),
    '%' => Some(OperatorKind::Percent),
    '^' => Some(OperatorKind::Caret),
    '&' => Some(OperatorKind::Amp),
    '|' => Some(OperatorKind::Pipe),
    '!' => Some(OperatorKind::Bang),
    '=' => Some(OperatorKind::Eq),
    '<' => Some(OperatorKind::Lt),
    '>' => Some(OperatorKind::Gt),
    '.' => Some(OperatorKind::Dot),
    _ => None,
  }
}

pub(crate) fn char_allowed_in_context(kind: OperatorKind, context: OperatorContext) -> bool {
  match context {
    OperatorContext::Numeric => matches!(
      kind,
      OperatorKind::Plus
        | OperatorKind::Minus
        | OperatorKind::Star
        | OperatorKind::Slash
        | OperatorKind::Percent
        | OperatorKind::Caret
        | OperatorKind::Amp
        | OperatorKind::Pipe
        | OperatorKind::Bang
        | OperatorKind::Eq
        | OperatorKind::Lt
        | OperatorKind::Gt
    ),
    OperatorContext::String => {
      matches!(
        kind,
        OperatorKind::Plus | OperatorKind::Star | OperatorKind::Eq | OperatorKind::Bang
      )
    }
    OperatorContext::Boolean => {
      matches!(
        kind,
        OperatorKind::Caret
          | OperatorKind::Amp
          | OperatorKind::Pipe
          | OperatorKind::Bang
          | OperatorKind::Eq
      )
    }
    OperatorContext::Null => {
      matches!(
        kind,
        OperatorKind::Amp | OperatorKind::Pipe | OperatorKind::Bang | OperatorKind::Eq
      )
    }
    OperatorContext::Ident => matches!(
      kind,
      OperatorKind::Plus
        | OperatorKind::Minus
        | OperatorKind::Star
        | OperatorKind::Slash
        | OperatorKind::Percent
        | OperatorKind::Caret
        | OperatorKind::Amp
        | OperatorKind::Pipe
        | OperatorKind::Bang
        | OperatorKind::Eq
        | OperatorKind::Lt
        | OperatorKind::Gt
        | OperatorKind::Dot
    ),
    OperatorContext::InvokedFunction => matches!(
      kind,
      OperatorKind::Plus
        | OperatorKind::Minus
        | OperatorKind::Star
        | OperatorKind::Slash
        | OperatorKind::Percent
        | OperatorKind::Caret
        | OperatorKind::Amp
        | OperatorKind::Pipe
        | OperatorKind::Bang
        | OperatorKind::Eq
        | OperatorKind::Lt
        | OperatorKind::Gt
        | OperatorKind::Dot
    ),
    OperatorContext::Array => matches!(
      kind,
      OperatorKind::Plus | OperatorKind::Minus | OperatorKind::Caret | OperatorKind::Amp
    ),
    OperatorContext::Dict => matches!(
      kind,
      OperatorKind::Plus | OperatorKind::Minus | OperatorKind::Caret | OperatorKind::Amp
    ),
  }
}

pub(crate) fn resolve_second_char(mode: &mut OperatorParseMode, input: char) -> ParseStep {
  match mode.kind {
    OperatorKind::Plus => resolve_plus(mode, input),
    OperatorKind::Minus => resolve_minus(mode, input),
    OperatorKind::Star => resolve_star(mode, input),
    OperatorKind::Slash => resolve_slash(mode, input),
    OperatorKind::Percent => resolve_percent(mode, input),
    OperatorKind::Caret => resolve_caret(mode, input),
    OperatorKind::Amp => resolve_amp(mode, input),
    OperatorKind::Pipe => resolve_pipe(mode, input),
    OperatorKind::Eq => resolve_eq(mode, input),
    OperatorKind::Lt => resolve_lt(mode, input),
    OperatorKind::Gt => resolve_gt(mode, input),
    OperatorKind::Bang => resolve_bang(mode, input),
    OperatorKind::Dot => resolve_dot(mode, input),
  }
}

fn resolve_plus(mode: &mut OperatorParseMode, input: char) -> ParseStep {
  resolve_repeat_assign_base! {
    mode,
    input,
    kind: Plus,
    repeat: '+',
    repeat_op: CoreOperator::Increment,
    repeat_ok: !matches!(mode.context, OperatorContext::String),
    assign: assign_or_strict!(
      mode.expect,
      mode.context,
      OperatorKind::Plus,
      CoreOperator::AddAssign,
      CoreOperator::Add
    ),
    base: CoreOperator::Add
  }
}

fn resolve_minus(mode: &mut OperatorParseMode, input: char) -> ParseStep {
  resolve_repeat_assign_base! {
    mode,
    input,
    kind: Minus,
    repeat: '-',
    repeat_op: CoreOperator::Decrement,
    repeat_ok: !matches!(mode.context, OperatorContext::String),
    assign: assign_or_strict!(
      mode.expect,
      mode.context,
      OperatorKind::Minus,
      CoreOperator::SubAssign,
      CoreOperator::Sub
    ),
    base: CoreOperator::Sub
  }
}

fn resolve_star(mode: &mut OperatorParseMode, input: char) -> ParseStep {
  let flags = mode.expect;
  let ctx = mode.context;
  match input {
    '*'
      if {
        flags.dim_optional(ExpectFlags::REPEAT_SHIFT)
          || flags.dim_required(ExpectFlags::REPEAT_SHIFT)
      } && ctx != OperatorContext::String =>
    {
      compound_assign_or_captured!(
        mode,
        ctx == OperatorContext::Ident,
        CoreOperator::PowAssign,
        CoreOperator::Pow
      )
    }
    '*' if flags.can_close_base() => close(CoreOperator::Mul),
    '*' => expect_error!(OperatorKind::Star, flags),
    '=' => assign_or_strict!(
      flags,
      ctx,
      OperatorKind::Star,
      CoreOperator::MulAssign,
      CoreOperator::Mul
    ),
    _ => close_or_error!(OperatorKind::Star, flags, CoreOperator::Mul),
  }
}

fn resolve_slash(mode: &mut OperatorParseMode, input: char) -> ParseStep {
  resolve_assign_only! {
    mode,
    input,
    kind: Slash,
    assign: CoreOperator::DivAssign,
    base: CoreOperator::Div
  }
}

fn resolve_percent(mode: &mut OperatorParseMode, input: char) -> ParseStep {
  resolve_assign_only! {
    mode,
    input,
    kind: Percent,
    assign: CoreOperator::ModAssign,
    base: CoreOperator::Mod
  }
}

fn caret_base(ctx: OperatorContext) -> CoreOperator {
  if matches!(ctx, OperatorContext::Array | OperatorContext::Dict) {
    CoreOperator::SymmetricDiff
  } else {
    CoreOperator::Xor
  }
}

fn amp_base(ctx: OperatorContext) -> CoreOperator {
  if matches!(ctx, OperatorContext::Array | OperatorContext::Dict) {
    CoreOperator::Intersect
  } else {
    CoreOperator::BitAnd
  }
}

fn resolve_caret(mode: &mut OperatorParseMode, input: char) -> ParseStep {
  let flags = mode.expect;
  let ctx = mode.context;
  match input {
    '&'
      if matches!(
        ctx,
        OperatorContext::Ident
          | OperatorContext::InvokedFunction
          | OperatorContext::Array
          | OperatorContext::Dict
      ) =>
    {
      compound_or_base_or_error!(
        mode,
        flags,
        OperatorKind::Caret,
        flags.disambiguates(),
        ctx == OperatorContext::Ident,
        CoreOperator::RightDiffAssign,
        CoreOperator::RightDiff,
        caret_base(ctx)
      )
    }
    '=' => assign_or_strict!(
      flags,
      ctx,
      OperatorKind::Caret,
      CoreOperator::XorAssign,
      caret_base(ctx)
    ),
    _ => close_or_error!(OperatorKind::Caret, flags, caret_base(ctx)),
  }
}

fn resolve_amp(mode: &mut OperatorParseMode, input: char) -> ParseStep {
  let flags = mode.expect;
  let ctx = mode.context;
  match input {
    '&' if matches!(ctx, OperatorContext::Array | OperatorContext::Dict) => {
      Err(ParseErrorKind::UnexpectedChar('&'))
    }
    '&' => {
      compound_or_base_or_error!(
        mode,
        flags,
        OperatorKind::Amp,
        flags.dim_required(ExpectFlags::REPEAT_SHIFT)
          || flags.dim_optional(ExpectFlags::REPEAT_SHIFT),
        flags.dim_optional(ExpectFlags::ASSIGN_SHIFT) && ctx == OperatorContext::Ident,
        CoreOperator::AndAndAssign,
        CoreOperator::And,
        amp_base(ctx)
      )
    }
    '^'
      if matches!(
        ctx,
        OperatorContext::Ident
          | OperatorContext::InvokedFunction
          | OperatorContext::Array
          | OperatorContext::Dict
      ) =>
    {
      compound_or_base_or_error!(
        mode,
        flags,
        OperatorKind::Amp,
        flags.disambiguates(),
        ctx == OperatorContext::Ident,
        CoreOperator::LeftDiffAssign,
        CoreOperator::LeftDiff,
        amp_base(ctx)
      )
    }
    '=' => assign_or_strict!(
      flags,
      ctx,
      OperatorKind::Amp,
      CoreOperator::AndAssign,
      amp_base(ctx)
    ),
    _ => close_or_error!(OperatorKind::Amp, flags, amp_base(ctx)),
  }
}

fn resolve_pipe(mode: &mut OperatorParseMode, input: char) -> ParseStep {
  let flags = mode.expect;
  let ctx = mode.context;
  match input {
    '|'
      if flags.dim_required(ExpectFlags::REPEAT_SHIFT)
        || flags.dim_optional(ExpectFlags::REPEAT_SHIFT) =>
    {
      compound_assign_or_captured!(
        mode,
        flags.dim_optional(ExpectFlags::ASSIGN_SHIFT) && ctx == OperatorContext::Ident,
        CoreOperator::OrOrAssign,
        CoreOperator::Or
      )
    }
    '|' if flags.can_close_base() => close(CoreOperator::BitOr),
    '|' => expect_error!(OperatorKind::Pipe, flags),
    '=' => assign_or_strict!(
      flags,
      ctx,
      OperatorKind::Pipe,
      CoreOperator::OrAssign,
      CoreOperator::BitOr
    ),
    _ => close_or_error!(OperatorKind::Pipe, flags, CoreOperator::BitOr),
  }
}

fn resolve_eq(mode: &mut OperatorParseMode, input: char) -> ParseStep {
  let flags = mode.expect;
  let ctx = mode.context;
  if input == '=' {
    close_captured(CoreOperator::Equal)
  } else if flags.dim_required(ExpectFlags::EQUALS_SHIFT) {
    Err(expected(messages::EQUAL_EQUAL))
  } else if ctx == OperatorContext::Ident
    && (flags.disambiguates() || flags.dim_optional(ExpectFlags::ASSIGN_SHIFT))
  {
    close(CoreOperator::Assign)
  } else {
    expect_error!(OperatorKind::Eq, flags)
  }
}

fn resolve_lt(mode: &mut OperatorParseMode, input: char) -> ParseStep {
  if input == '=' {
    close_captured(CoreOperator::LessEqual)
  } else {
    close_equals_or_error!(OperatorKind::Lt, mode.expect, CoreOperator::Lt)
  }
}

fn resolve_gt(mode: &mut OperatorParseMode, input: char) -> ParseStep {
  if input == '=' {
    close_captured(CoreOperator::GreaterEqual)
  } else {
    close_equals_or_error!(OperatorKind::Gt, mode.expect, CoreOperator::Gt)
  }
}

fn resolve_bang(mode: &mut OperatorParseMode, input: char) -> ParseStep {
  let flags = mode.expect;
  if input == '=' && flags.dim_enabled(ExpectFlags::EQUALS_SHIFT) {
    close_captured(CoreOperator::NotEqual)
  } else if flags.dim_required(ExpectFlags::EQUALS_SHIFT) {
    expect_error!(OperatorKind::Bang, flags)
  } else if flags.can_close_base() {
    close(CoreOperator::Not)
  } else {
    expect_error!(OperatorKind::Bang, flags)
  }
}

fn resolve_dot(mode: &mut OperatorParseMode, _input: char) -> ParseStep {
  if mode.expect.can_close_base() {
    close(CoreOperator::Dot)
  } else {
    expect_error!(OperatorKind::Dot, mode.expect)
  }
}
