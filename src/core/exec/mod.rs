pub mod error;
pub mod value;

mod expr;
mod functions;
mod members;
mod ops;
mod scope;
mod stmt;

pub use error::{RuntimeError, RuntimeErrorKind};
pub use scope::Scope;
pub use stmt::Flow;
pub use value::{MemberHost, RuntimeValue};

use crate::core::state::DatumaState;
use crate::core::value::CoreValue;
use functions::FunctionTable;

/// The evaluator recurses on the native stack, so this guard exists to turn
/// runaway recursion into a `StackOverflow` result instead of aborting the
/// process. One interpreted call costs roughly 12KB of native stack in an
/// unoptimized build, so the limit stays well inside a 2MB thread stack.
pub const MAX_CALL_DEPTH: usize = 64;
pub const MAX_LOOP_ITERATIONS: usize = 1_000_000;

#[derive(Debug)]
pub struct Execution {
  pub returned: RuntimeValue,
  pub scope: Scope,
}

/// What a single executed step accomplished. Each variant carries the result
/// that makes the step meaningful on its own, so a reader never has to
/// reconstruct it from the surrounding source.
#[derive(Debug, Clone)]
pub enum StepEvent {
  Assign {
    target: String,
    value: RuntimeValue,
  },
  /// A statement that assigns nothing, such as `1 + 2;` or a bare call.
  Expression {
    value: RuntimeValue,
  },
  Return {
    value: RuntimeValue,
  },
  Break,
  Branch {
    condition: bool,
    /// `then`, `elseif`, `else` or `none`.
    taken: &'static str,
  },
  Iteration {
    index: usize,
    /// Set for `for-in`, along with the element bound this time round.
    variable: Option<String>,
    element: Option<RuntimeValue>,
    /// Set for a classic `for`: the condition that admitted this iteration, or
    /// the failing check that ended the loop.
    condition: Option<bool>,
  },
  Failed {
    error: RuntimeError,
  },
}

#[derive(Debug, Clone)]
pub struct Step {
  pub event: StepEvent,
  /// Innermost active function, `None` at the top level.
  pub function: Option<String>,
  pub stack: Vec<String>,
  /// Names the current scope frame owns, which `pop_frame` would discard.
  pub frame: Vec<String>,
  /// Every visible name with its innermost value, sorted by name.
  pub scope: Vec<(String, RuntimeValue)>,
}

/// A run that keeps its scope and step log even when it fails, which is the
/// whole point: on error those are the only record of how far it got.
#[derive(Debug)]
pub struct TracedRun {
  pub steps: Vec<Step>,
  pub scope: Scope,
  pub outcome: Result<RuntimeValue, RuntimeError>,
}

pub struct Interp<'tree> {
  functions: FunctionTable<'tree>,
  scope: Scope,
  depth: usize,
  pub(super) call_stack: Vec<String>,
  steps: Option<Vec<Step>>,
}

impl<'tree> Interp<'tree> {
  pub(crate) fn from_roots(
    roots: &[&'tree DatumaState],
    scope: Scope,
  ) -> Result<Interp<'tree>, RuntimeErrorKind> {
    Ok(Interp {
      functions: functions::collect_functions(roots)?,
      scope,
      depth: 0,
      call_stack: Vec::new(),
      steps: None,
    })
  }

  #[expect(dead_code)]
  pub(crate) fn with_scope(
    root: &'tree DatumaState,
    scope: Scope,
  ) -> Result<Interp<'tree>, RuntimeErrorKind> {
    Self::from_roots(&[root], scope)
  }

  pub(crate) fn run_tree(&mut self, root: &'tree DatumaState) -> Result<Flow, RuntimeError> {
    self.run_block(&root.children)
  }

  #[expect(dead_code)]
  pub(crate) fn into_scope(self) -> Scope {
    self.scope
  }

  pub(crate) fn scope_mut(&mut self) -> &mut Scope {
    &mut self.scope
  }

  pub(super) fn err(&self, kind: RuntimeErrorKind) -> RuntimeError {
    RuntimeError::from_kind(kind, self.call_stack.clone())
  }

  pub(super) fn lift<T>(&self, result: Result<T, RuntimeErrorKind>) -> Result<T, RuntimeError> {
    result.map_err(|kind| self.err(kind))
  }

  /// Captures the surrounding context with the event. The whole step is built
  /// before `steps` is touched so the `scope` and `steps` borrows stay disjoint.
  pub(super) fn record(&mut self, event: StepEvent) {
    if self.steps.is_none() {
      return;
    }
    let mut scope = self
      .scope
      .iter()
      .map(|(name, value)| (name.to_string(), value.clone()))
      .collect::<Vec<_>>();
    scope.sort_by(|left, right| left.0.cmp(&right.0));
    let step = Step {
      event,
      function: self.call_stack.last().cloned(),
      stack: self.call_stack.clone(),
      frame: self.scope.frame_names().to_vec(),
      scope,
    };
    if let Some(steps) = self.steps.as_mut() {
      steps.push(step);
    }
  }
}

/// Pass 1 hoists every function definition, then pass 2 runs the top-level
/// statements in a single global scope whose frame 0 survives the run.
fn run_program(root: &DatumaState, scope: Scope, steps: Option<Vec<Step>>) -> TracedRun {
  let tracing = steps.is_some();
  let functions = match functions::collect_functions(&[root]) {
    Ok(functions) => functions,
    Err(kind) => {
      let err = RuntimeError::from_kind(kind, Vec::new());
      let mut interp = Interp {
        functions: FunctionTable::default(),
        scope,
        depth: 0,
        call_stack: Vec::new(),
        steps,
      };
      interp.record(StepEvent::Failed { error: err.clone() });
      return TracedRun {
        steps: interp.steps.take().unwrap_or_default(),
        scope: interp.scope,
        outcome: Err(err),
      };
    }
  };
  let mut interp = Interp {
    functions,
    scope,
    depth: 0,
    call_stack: Vec::new(),
    steps,
  };
  let outcome = interp.run_tree(root).map(|flow| match flow {
    Flow::Return(value) => value,
    Flow::Normal | Flow::Break => RuntimeValue::Null,
  });
  if let Err(err) = &outcome {
    let recorded = interp
      .steps
      .as_ref()
      .and_then(|steps| steps.last())
      .is_some_and(|step| matches!(step.event, StepEvent::Failed { .. }));
    if tracing && !recorded {
      interp.record(StepEvent::Failed { error: err.clone() });
    }
  }
  TracedRun {
    steps: interp.steps.take().unwrap_or_default(),
    scope: interp.scope,
    outcome,
  }
}

pub fn execute(root: &DatumaState) -> Result<Execution, RuntimeError> {
  let run = run_program(root, Scope::new(), None);
  Ok(Execution {
    returned: run.outcome?,
    scope: run.scope,
  })
}

pub fn execute_with_scope(root: &DatumaState, scope: Scope) -> Result<Execution, RuntimeError> {
  let run = run_program(root, scope, None);
  Ok(Execution {
    returned: run.outcome?,
    scope: run.scope,
  })
}

pub fn execute_traced(root: &DatumaState) -> TracedRun {
  run_program(root, Scope::new(), Some(Vec::new()))
}

fn core_value(state: &DatumaState) -> Option<&CoreValue> {
  state
    .value
    .as_ref()
    .and_then(|value| value.as_any().downcast_ref::<CoreValue>())
}
