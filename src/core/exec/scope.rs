use std::collections::HashMap;

use super::value::RuntimeValue;

#[derive(Debug)]
struct Binding {
  frame: usize,
  value: RuntimeValue,
}

/// Variables live in one flat map keyed by name; each entry is a stack of
/// bindings tagged with the frame that created them. Lookup always takes the
/// innermost binding regardless of frame, but assignment only writes to a
/// binding the current frame owns, so a call shadows an enclosing variable
/// instead of overwriting it.
#[derive(Debug)]
pub struct Scope {
  vars: HashMap<String, Vec<Binding>>,
  frames: Vec<Vec<String>>,
}

impl Default for Scope {
  fn default() -> Self {
    Self::new()
  }
}

impl Scope {
  pub fn new() -> Self {
    Self {
      vars: HashMap::new(),
      frames: vec![Vec::new()],
    }
  }

  pub fn get(&self, name: &str) -> Option<&RuntimeValue> {
    self
      .vars
      .get(name)
      .and_then(|stack| stack.last())
      .map(|binding| &binding.value)
  }

  /// Every visible name paired with its innermost binding, in arbitrary order.
  pub fn iter(&self) -> impl Iterator<Item = (&str, &RuntimeValue)> {
    self
      .vars
      .iter()
      .filter_map(|(name, stack)| stack.last().map(|binding| (name.as_str(), &binding.value)))
  }

  /// Names the innermost frame owns, which `pop_frame` would discard.
  pub fn frame_names(&self) -> &[String] {
    self.frames.last().map(Vec::as_slice).unwrap_or_default()
  }

  pub fn get_mut(&mut self, name: &str) -> Option<&mut RuntimeValue> {
    self
      .vars
      .get_mut(name)
      .and_then(|stack| stack.last_mut())
      .map(|binding| &mut binding.value)
  }

  /// Assignment never writes through to an enclosing frame: an outer binding is
  /// shadowed by a new one that `pop_frame` removes when the call returns.
  pub fn assign(&mut self, name: &str, value: RuntimeValue) {
    if let Some(binding) = self.vars.get_mut(name).and_then(|stack| stack.last_mut()) {
      if binding.frame == self.frames.len() {
        binding.value = value;
        return;
      }
    }
    self.declare(name.to_string(), value);
  }

  pub fn declare(&mut self, name: String, value: RuntimeValue) {
    self.vars.entry(name.clone()).or_default().push(Binding {
      frame: self.frames.len(),
      value,
    });
    if let Some(frame) = self.frames.last_mut() {
      frame.push(name);
    }
  }

  pub fn push_frame(&mut self) {
    self.frames.push(Vec::new());
  }

  pub fn pop_frame(&mut self) {
    if let Some(frame) = self.frames.pop() {
      for name in frame {
        if let Some(stack) = self.vars.get_mut(&name) {
          stack.pop();
          if stack.is_empty() {
            self.vars.remove(&name);
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn repeated_assignment_reuses_the_frames_binding() {
    let mut scope = Scope::new();
    scope.assign("x", RuntimeValue::Integer(1));
    scope.assign("x", RuntimeValue::Integer(2));
    assert_eq!(scope.vars["x"].len(), 1, "binding stack must not grow");
    assert_eq!(scope.frames.last().expect("frame 0").len(), 1);
    assert_eq!(scope.get("x"), Some(&RuntimeValue::Integer(2)));
  }

  #[test]
  fn assignment_in_a_call_shadows_then_restores() {
    let mut scope = Scope::new();
    scope.assign("g", RuntimeValue::Integer(10));
    scope.push_frame();
    scope.assign("g", RuntimeValue::Integer(99));
    assert_eq!(scope.vars["g"].len(), 2, "outer binding must be kept");
    assert_eq!(scope.get("g"), Some(&RuntimeValue::Integer(99)));
    scope.pop_frame();
    assert_eq!(scope.vars["g"].len(), 1);
    assert_eq!(scope.get("g"), Some(&RuntimeValue::Integer(10)));
  }

  #[test]
  fn names_first_bound_in_a_call_do_not_survive_it() {
    let mut scope = Scope::new();
    scope.push_frame();
    scope.assign("local", RuntimeValue::Integer(1));
    scope.pop_frame();
    assert!(scope.get("local").is_none());
    assert!(!scope.vars.contains_key("local"));
  }

  #[test]
  fn default_seeds_the_top_level_frame() {
    let mut scope = Scope::default();
    scope.assign("x", RuntimeValue::Integer(1));
    assert_eq!(scope.frames.last().expect("frame 0").len(), 1);
  }
}
