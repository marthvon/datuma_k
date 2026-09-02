use std::fmt::Display;
use std::write;

use crate::core::parser::{
  ParseErrorKind, ParseErrorSource, ParseMode, ParseResolveMutation, ParseResolveStep, ParseStep,
  ParseStepMutation, ParsetStepFlow, expected, messages,
};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;
use crate::ngin::value::NginValue;

use super::glyph::GlyphAllow;
use super::interp::NginInterpParseMode;

#[derive(Debug)]
pub struct NginFenceParseMode {
  count: u8,
  in_plus: bool,
}

impl NginFenceParseMode {
  pub fn new(in_plus: bool) -> Self {
    Self { count: 0, in_plus }
  }
}

impl Display for NginFenceParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/ngin/fence")
  }
}

impl ParseMode for NginFenceParseMode {
  fn on_parse(&mut self, input: char) -> ParseStep {
    if input == '`' {
      self.count += 1;
      if self.count == 3 {
        Ok((
          ParseStepMutation::ReplaceMode(Box::new(NginTemplateParseMode::new(self.in_plus))),
          ParsetStepFlow::Captured,
        ))
      } else {
        Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
      }
    } else {
      Err(expected(messages::NGIN_FENCE))
    }
  }

  fn on_parse_resolved(&mut self, _input: char) -> ParseResolveStep {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if state.is_some() {
      None
    } else {
      Some(expected(messages::NGIN_FENCE))
    }
  }
}

#[derive(Debug)]
pub struct NginTemplateParseMode {
  children: Vec<DatumaState>,
  lit: String,
  close_ticks: u8,
  in_plus: bool,
  pending_at: bool,
  line: usize,
  col: usize,
}

impl NginTemplateParseMode {
  pub fn new(in_plus: bool) -> Self {
    Self {
      children: Vec::new(),
      lit: String::new(),
      close_ticks: 0,
      in_plus,
      pending_at: false,
      line: 0,
      col: 0,
    }
  }

  fn flush_lit(&mut self) {
    if !self.lit.is_empty() {
      self
        .children
        .push(DatumaState::leaf(Box::new(CoreValue::String(
          std::mem::take(&mut self.lit),
        ))));
    }
  }

  fn flush_ticks(&mut self) {
    for _ in 0..self.close_ticks {
      self.lit.push('`');
    }
    self.close_ticks = 0;
  }

  fn close_template(&mut self) -> DatumaState {
    if self.pending_at {
      self.lit.push('@');
      self.pending_at = false;
    }
    self.flush_lit();
    DatumaState::node(
      Some(Box::new(NginValue::Template {
        line: self.line,
        col: self.col,
      })),
      std::mem::take(&mut self.children),
    )
  }
}

impl Display for NginTemplateParseMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "/ngin/template")
  }
}

impl ParseMode for NginTemplateParseMode {
  fn note_source(&mut self, source: &dyn ParseErrorSource) {
    if self.line == 0 && self.col == 0 {
      let pos = source.pos_meta();
      self.line = pos.line;
      self.col = pos.col;
    }
  }

  fn on_parse(&mut self, input: char) -> ParseStep {
    if self.pending_at {
      self.pending_at = false;
      if input == '{' {
        self.flush_lit();
        Ok((
          ParseStepMutation::StartMode(Box::new(NginInterpParseMode::in_body(GlyphAllow {
            file: false,
            emit: true,
            guard: self.in_plus,
          }))),
          ParsetStepFlow::Captured,
        ))
      } else {
        self.lit.push('@');
        self.on_parse(input)
      }
    } else if input == '`' {
      self.close_ticks += 1;
      if self.close_ticks == 3 {
        Ok((
          ParseStepMutation::CloseMode(Some(self.close_template())),
          ParsetStepFlow::Captured,
        ))
      } else {
        Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
      }
    } else if self.close_ticks > 0 {
      self.flush_ticks();
      self.push_body(input)
    } else {
      self.push_body(input)
    }
  }

  fn on_parse_resolved(&mut self, _input: char) -> ParseResolveStep {
    Ok((ParseResolveMutation::Dismiss, ParsetStepFlow::Propagate))
  }

  fn adopt(&mut self, child: DatumaState) {
    self.children.push(child);
  }

  fn incomplete_close_error(&self, state: &Option<DatumaState>) -> Option<ParseErrorKind> {
    if state.is_some() {
      None
    } else {
      Some(expected(messages::NGIN_FENCE))
    }
  }
}

impl NginTemplateParseMode {
  fn push_body(&mut self, input: char) -> ParseStep {
    if input == '@' {
      self.pending_at = true;
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    } else {
      self.lit.push(input);
      Ok((ParseStepMutation::Nothing, ParsetStepFlow::Captured))
    }
  }
}
