use super::cursor::ParseErrorSource;
use super::mode::ParseMode;
use super::step::{ParseErrorKind, ParseResolveMutation, ParseStepMutation, ParsetStepFlow};
use crate::core::common::{expected_root_close, internal_invariant};
use crate::core::state::DatumaState;
use std::ptr::NonNull;

struct ParseFrame {
  mode: Box<dyn ParseMode>,
  resolved: bool,
}

impl From<Box<dyn ParseMode>> for ParseFrame {
  fn from(mode: Box<dyn ParseMode>) -> Self {
    Self {
      mode,
      resolved: false,
    }
  }
}

pub struct ParseStack {
  frames: Vec<ParseFrame>,
  /// Bound cursor for `note_source`. Always `None` outside `set_source`…`clear_source`.
  /// See SAFETY on those methods — do not replace with a plain `&` field (that reintroduces
  /// lifetime churn on this long-lived stack).
  source: Option<NonNull<dyn ParseErrorSource>>,
  #[cfg(feature = "parse-trace")]
  on_change: Option<Box<dyn Fn(&ParseStack)>>,
}

impl ParseStack {
  pub fn with_root(root: Box<dyn ParseMode>) -> Self {
    let mut frames = Vec::with_capacity(16);
    frames.push(root.into());
    Self {
      frames,
      source: None,
      #[cfg(feature = "parse-trace")]
      on_change: None,
    }
  }

  /// Temporarily attach `source` for subsequent `parse` / `note_source` calls.
  ///
  /// # Safety (caller contract — enforced by `parse_line`, not the type system)
  /// - `source` must outlive every use until `clear_source`.
  /// - Must not drop the referent while this stack still holds the bind.
  /// - Must `clear_source` before any exclusive use of the referent (e.g. `chars.next()`)
  ///   and before the stack is dropped or the referent goes out of scope.
  /// - Do not nest binds; clear before set again.
  pub fn set_source(&mut self, source: &dyn ParseErrorSource) {
    // SAFETY: caller upholds the contract above. We only ever dereference
    // through `step_parse` between set and clear, as a shared borrow.
    // The cast drops the input lifetime — that is intentional; correctness is
    // the caller's `clear_source` discipline, not the type system.
    self.source = Some(unsafe {
      NonNull::new_unchecked(source as *const dyn ParseErrorSource as *mut dyn ParseErrorSource)
    });
  }

  /// Drop the bind from `set_source`. Must run before exclusive use of the referent.
  pub fn clear_source(&mut self) {
    self.source = None;
  }

  #[cfg(feature = "parse-trace")]
  pub fn on_change<F>(&mut self, f: F)
  where
    F: Fn(&ParseStack) + 'static,
  {
    self.on_change = Some(Box::new(f));
  }

  #[cfg(feature = "parse-trace")]
  pub fn path(&self) -> String {
    self
      .frames
      .iter()
      .map(|frame| frame.mode.to_string())
      .collect()
  }

  pub fn len(&self) -> usize {
    self.frames.len()
  }

  pub fn is_empty(&self) -> bool {
    self.frames.is_empty()
  }

  pub fn dismiss_resolved(&mut self) {
    self.frames.retain(|frame| !frame.resolved);
  }

  pub fn has_active_frames(&self) -> bool {
    self.frames.iter().skip(1).any(|frame| !frame.resolved)
  }

  pub fn parse(&mut self, input: char) -> Result<(), ParseErrorKind> {
    'parse: loop {
      let mut j = self.frames.len();
      while j > 0 {
        j -= 1;
        let flow = if self.frames[j].resolved {
          self.step_resolved(j, input)?
        } else {
          let flow = self.step_parse(j, input)?;
          if flow == ParsetStepFlow::Propagate
            && self.frames[j].resolved
            && self.frames.len() == j + 1
          {
            ParsetStepFlow::Repropagate
          } else {
            flow
          }
        };
        if flow == ParsetStepFlow::Repropagate {
          continue 'parse;
        } else if flow == ParsetStepFlow::Captured {
          break 'parse Ok(());
        }
      }
      break Err(ParseErrorKind::UnexpectedChar(input));
    }
  }

  pub fn into_root(mut self) -> Box<dyn ParseMode> {
    self.frames.pop().expect("parse stack retains root").mode
  }

  fn emit_change(&mut self) {
    #[cfg(feature = "parse-trace")]
    if let Some(on_change) = self.on_change.as_deref() {
      on_change(self);
    }
  }

  fn step_resolved(&mut self, j: usize, input: char) -> Result<ParsetStepFlow, ParseErrorKind> {
    let (mutation, flow) = self.frames[j].mode.on_parse_resolved(input)?;
    match mutation {
      ParseResolveMutation::Dismiss => {
        self.frames.remove(j);
        self.emit_change();
      }
      ParseResolveMutation::StartMode(mode) => {
        self.frames.remove(j);
        self.frames.push(mode.into());
        self.emit_change();
      }
      ParseResolveMutation::NoDismissStartMode(mode) => {
        self.frames.push(mode.into());
        self.emit_change();
      }
      ParseResolveMutation::ParentForceDismissMode => {
        self.parent_force_dismiss(j)?;
      }
      ParseResolveMutation::ParentForceDismissAndStartMode(mode) => {
        self.parent_force_dismiss(j)?;
        self.frames.insert(j - 1, mode.into());
        self.emit_change();
      }
      ParseResolveMutation::Nothing => {}
    }
    Ok(flow)
  }

  fn step_parse(&mut self, j: usize, input: char) -> Result<ParsetStepFlow, ParseErrorKind> {
    if let Some(ptr) = self.source {
      // SAFETY: `set_source` contract — valid shared borrow until `clear_source`.
      self.frames[j].mode.note_source(unsafe { ptr.as_ref() });
    }
    let (mutation, flow) = self.frames[j].mode.on_parse(input)?;
    match mutation {
      ParseStepMutation::StartMode(mode) => {
        self.frames.push(mode.into());
        self.emit_change();
      }
      ParseStepMutation::ReplaceMode(mode) => {
        let replaced = std::mem::replace(&mut self.frames[j].mode, mode);
        self.frames[j].mode.on_replace(replaced);
        self.emit_change();
      }
      ParseStepMutation::CloseAndStartMode(state, mode) if j > 0 => {
        self.close_and_adopt_to(j, state)?;
        self.frames.push(mode.into());
        self.emit_change();
      }
      ParseStepMutation::CloseMode(state) if j > 0 => {
        self.close_and_adopt_to(j, state)?;
      }
      ParseStepMutation::ParentForceDismissMode if j >= 2 => {
        self.parent_force_dismiss(j)?;
      }
      ParseStepMutation::ParentForceDismissAndStartMode(mode) if j >= 2 => {
        self.parent_force_dismiss(j)?;
        self.frames.insert(j - 1, mode.into());
        self.emit_change();
      }
      ParseStepMutation::CloseMode(_)
      | ParseStepMutation::CloseAndStartMode(_, _)
      | ParseStepMutation::ParentForceDismissMode
      | ParseStepMutation::ParentForceDismissAndStartMode(_) => {
        return Err(expected_root_close());
      }
      ParseStepMutation::Nothing => {}
    }
    Ok(flow)
  }

  fn parent_force_dismiss(&mut self, j: usize) -> Result<usize, ParseErrorKind> {
    if j < 2 {
      Err(internal_invariant(
        "parent force dismiss requires grandparent",
      ))
    } else {
      let parent = j - 1;
      let state = self.frames[parent].mode.close_state();
      if let Some(err) = self.frames[parent].mode.incomplete_close_error(&state) {
        Err(err)
      } else {
        if let Some(child) = state {
          self.adopt_into_parent(parent, child)?;
        }
        self.frames.remove(parent);
        self.emit_change();
        Ok(j - 1)
      }
    }
  }

  fn close_and_adopt_to(
    &mut self,
    j: usize,
    state: Option<DatumaState>,
  ) -> Result<(), ParseErrorKind> {
    self.force_close_above(j)?;
    if let Some(err) = self.frames[j].mode.incomplete_close_error(&state) {
      Err(err)
    } else {
      if let Some(child) = state {
        self.adopt_into_parent(j, child)?;
      }
      self.frames[j].resolved = true;
      self.emit_change();
      Ok(())
    }
  }

  fn force_close_above(&mut self, j: usize) -> Result<(), ParseErrorKind> {
    let mut i = self.frames.len();
    while i > j + 1 {
      i -= 1;
      if self.frames[i].resolved {
        continue;
      } else {
        let state = self.frames[i].mode.close_state();
        if let Some(err) = self.frames[i].mode.incomplete_close_error(&state) {
          return Err(err);
        } else {
          if let Some(child) = state {
            self.adopt_into_parent(i, child)?;
          }
          self.frames[i].resolved = true;
          self.emit_change();
        }
      }
    }
    Ok(())
  }

  fn adopt_into_parent(
    &mut self,
    child_index: usize,
    child: DatumaState,
  ) -> Result<(), ParseErrorKind> {
    let mut parent = child_index - 1;
    while parent > 0
      && self.frames[parent].resolved
      && !self.frames[parent].mode.accepts_resolved_child()
    {
      parent -= 1;
    }
    self.frames[parent].mode.adopt(child);
    if self.frames[parent].mode.reactivate_after_child_close() {
      self.frames[parent].resolved = false;
      self.emit_change();
    }
    if let Some(state) = self.frames[parent].mode.close_after_adopt() {
      let state = Some(state);
      if let Some(err) = self.frames[parent].mode.incomplete_close_error(&state) {
        Err(err)
      } else {
        self.adopt_into_parent(parent, state.expect("close_after_adopt state"))?;
        self.frames[parent].resolved = true;
        self.emit_change();
        Ok(())
      }
    } else {
      Ok(())
    }
  }
}
