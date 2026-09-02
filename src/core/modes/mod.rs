mod accessor;
mod args;
mod array;
mod boolean;
mod control;
mod dict;
mod double;
mod float;
mod function;
mod grouped;
mod identifier;
mod instruction;
mod integer;
mod jump;
mod null;
mod operator;
mod program;
mod string;

pub use accessor::AccessorParseMode;
pub use array::ArrayParseMode;
pub use boolean::{BooleanLiteral, BooleanParseMode};
pub use control::{ElseIfParseMode, ElseParseMode, ForParseMode, IfParseMode};
pub use dict::DictParseMode;
pub use double::{DoubleParseMode, MAX_DOUBLE_FRAC_DIGITS};
pub use float::{FloatParseMode, MAX_FLOAT_FRAC_DIGITS};
pub use function::FunctionDefParseMode;
pub use grouped::GroupedParseMode;
pub use identifier::IdentifierParseMode;
pub use instruction::InstructionParseMode;
pub use integer::IntegerParseMode;
pub use jump::JumpParseMode;
pub use null::NullParseMode;
pub use operator::{CollectionKind, OperatorContext, OperatorFollowUp, OperatorParseMode};
pub use program::_stmt::KeywordFlags;
pub(crate) use program::_stmt::{is_ident_continue, is_ident_start, is_statement_start};
pub use program::ProgramParseMode;
pub use string::StringParseMode;

pub use crate::core::common::{on_parse_capture_whitespace, on_resolve_capture_whitespace};

use crate::core::{
  modes::operator::resolve_dot_operator,
  parser::{ParseResolveMutation, ParseResolveStep, ParseStep, ParseStepMutation, ParsetStepFlow},
};

#[inline]
pub fn on_resolve_dot_operator(input: char) -> Option<ParseResolveStep> {
  if input == '.' {
    Some(resolve_dot_operator())
  } else {
    None
  }
}

pub fn start_core_value(input: char) -> Option<ParseStep> {
  Some(Ok((
    ParseStepMutation::StartMode(match input {
      '[' => Box::new(ArrayParseMode::new()),
      '{' => Box::new(DictParseMode::new()),
      '"' => Box::new(StringParseMode::new()),
      '.' => Box::new(FloatParseMode::new()),
      _ => {
        if input == '-' || input.is_ascii_digit() {
          Box::new(IntegerParseMode::starting(input))
        } else if input == '_' || input.is_ascii_alphabetic() {
          Box::new(IdentifierParseMode::starting(input))
        } else {
          return None;
        }
      }
    }),
    ParsetStepFlow::Captured,
  )))
}

pub fn do_nothing_on_parse() -> ParseStep {
  Ok((ParseStepMutation::Nothing, ParsetStepFlow::Propagate))
}

pub fn do_nothing_on_resolve() -> ParseResolveStep {
  Ok((ParseResolveMutation::Nothing, ParsetStepFlow::Propagate))
}
