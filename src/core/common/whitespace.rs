use crate::core::parser::{ParseResolveMutation, ParseStepMutation, ParsetStepFlow};

#[inline]
pub fn on_resolve_capture_whitespace(
  input: char,
) -> Option<(ParseResolveMutation, ParsetStepFlow)> {
  if input.is_whitespace() {
    Some((ParseResolveMutation::Nothing, ParsetStepFlow::Captured))
  } else {
    None
  }
}

#[inline]
pub fn on_parse_capture_whitespace(input: char) -> Option<(ParseStepMutation, ParsetStepFlow)> {
  if input.is_whitespace() {
    Some((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
  } else {
    None
  }
}
