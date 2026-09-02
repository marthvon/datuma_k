use super::error::{RuntimeError, RuntimeErrorKind};
use super::expr::assign_op;
use super::value::RuntimeValue;
use super::{Interp, MAX_LOOP_ITERATIONS, StepEvent, core_value};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

#[derive(Debug)]
pub enum Flow {
  Normal,
  Return(RuntimeValue),
  Break,
}

impl<'tree> Interp<'tree> {
  pub(crate) fn run_block(
    &mut self,
    statements: &'tree [DatumaState],
  ) -> Result<Flow, RuntimeError> {
    let mut outcome = Ok(Flow::Normal);
    for statement in statements {
      match self.run_statement(statement) {
        Ok(Flow::Normal) => {}
        interrupted => {
          outcome = interrupted;
          break;
        }
      }
    }
    outcome
  }

  fn enrich(&self, kind: RuntimeErrorKind, node: &DatumaState) -> RuntimeError {
    match core_value(node) {
      Some(CoreValue::Instruction {
        file_meta,
        pos_meta,
      }) => RuntimeError::with_span(kind, self.call_stack.clone(), file_meta.clone(), *pos_meta),
      _ => self.err(kind),
    }
  }

  fn fail(&mut self, err: RuntimeError) -> RuntimeError {
    self.record(StepEvent::Failed { error: err.clone() });
    err
  }

  pub(crate) fn run_statement(&mut self, node: &'tree DatumaState) -> Result<Flow, RuntimeError> {
    match core_value(node) {
      Some(CoreValue::FunctionDef(_)) => Ok(Flow::Normal),
      Some(CoreValue::Instruction { .. }) => {
        if node.children.is_empty() {
          return Ok(Flow::Normal);
        }
        let assigns = node.children.iter().any(|token| assign_op(token).is_some());
        match self.eval_tokens(&node.children) {
          Ok(value) => {
            if !assigns {
              self.record(StepEvent::Expression { value });
            }
            Ok(Flow::Normal)
          }
          Err(err) => {
            let err = if err.file_meta.is_none() {
              match core_value(node) {
                Some(CoreValue::Instruction {
                  file_meta,
                  pos_meta,
                }) => RuntimeError {
                  file_meta: Some(file_meta.clone()),
                  pos_meta: Some(*pos_meta),
                  stack: err.stack,
                  kind: err.kind,
                },
                _ => err,
              }
            } else {
              err
            };
            Err(self.fail(err))
          }
        }
      }
      Some(CoreValue::If) => self.run_if(node),
      Some(CoreValue::For) => self.run_for(node),
      Some(CoreValue::Return) => match self.eval_tokens(&node.children) {
        Ok(value) => {
          self.record(StepEvent::Return {
            value: value.clone(),
          });
          Ok(Flow::Return(value))
        }
        Err(err) => Err(self.fail(err)),
      },
      Some(CoreValue::Break) => {
        self.record(StepEvent::Break);
        Ok(Flow::Break)
      }
      Some(CoreValue::Program) => self.run_block(&node.children),
      _ => Err(self.fail(self.enrich(RuntimeErrorKind::MalformedTree("statement"), node))),
    }
  }

  fn run_if(&mut self, node: &'tree DatumaState) -> Result<Flow, RuntimeError> {
    let [condition, then_branch, tail @ ..] = &node.children[..] else {
      return Err(self.fail(self.err(RuntimeErrorKind::MalformedTree(
        "if needs a condition and a branch",
      ))));
    };
    let truthy = match self.eval_tokens(&condition.children) {
      Ok(value) => value.truthy(),
      Err(err) => return Err(self.fail(err)),
    };
    if truthy {
      self.record(StepEvent::Branch {
        condition: true,
        taken: "then",
      });
      return self.run_branch(then_branch);
    }
    match tail.first() {
      None => {
        self.record(StepEvent::Branch {
          condition: false,
          taken: "none",
        });
        Ok(Flow::Normal)
      }
      Some(next) => match core_value(next) {
        Some(CoreValue::ElseIf) => {
          self.record(StepEvent::Branch {
            condition: false,
            taken: "elseif",
          });
          self.run_if(next)
        }
        Some(CoreValue::Else) => {
          self.record(StepEvent::Branch {
            condition: false,
            taken: "else",
          });
          match next.children.first() {
            Some(body) => self.run_branch(body),
            None => Err(self.fail(self.err(RuntimeErrorKind::MalformedTree("else needs a body")))),
          }
        }
        _ => {
          self.record(StepEvent::Branch {
            condition: false,
            taken: "else",
          });
          self.run_branch(next)
        }
      },
    }
  }

  fn run_branch(&mut self, node: &'tree DatumaState) -> Result<Flow, RuntimeError> {
    if matches!(core_value(node), Some(CoreValue::Program)) {
      self.run_block(&node.children)
    } else {
      self.eval_operand(node)?;
      Ok(Flow::Normal)
    }
  }

  fn run_for(&mut self, node: &'tree DatumaState) -> Result<Flow, RuntimeError> {
    let [head, body] = &node.children[..] else {
      return Err(self.fail(self.err(RuntimeErrorKind::MalformedTree(
        "for needs a head and a body",
      ))));
    };
    match head.children.first().and_then(|first| core_value(first)) {
      Some(CoreValue::Ident(name)) => {
        let Some(iterable) = head.children.get(1) else {
          return Err(
            self.fail(self.err(RuntimeErrorKind::MalformedTree("for-in needs an iterable"))),
          );
        };
        let items = match self.eval_operand(iterable) {
          Ok(RuntimeValue::Array(items)) => items,
          Ok(RuntimeValue::Dict(entries)) => {
            entries.keys().cloned().map(RuntimeValue::String).collect()
          }
          Ok(RuntimeValue::String(text)) => text
            .chars()
            .map(|ch| RuntimeValue::String(ch.to_string()))
            .collect(),
          Ok(other) => {
            return Err(self.fail(self.err(RuntimeErrorKind::NotIterable(other.kind()))));
          }
          Err(err) => return Err(self.fail(err)),
        };
        for (index, item) in items.into_iter().enumerate() {
          self.scope.assign(name, item.clone());
          self.record(StepEvent::Iteration {
            index,
            variable: Some(name.clone()),
            element: Some(item),
            condition: None,
          });
          match self.run_block(&body.children)? {
            Flow::Normal => {}
            Flow::Break => break,
            Flow::Return(value) => return Ok(Flow::Return(value)),
          }
        }
        Ok(Flow::Normal)
      }
      _ => {
        let [init, condition, step] = &head.children[..] else {
          return Err(self.fail(self.err(RuntimeErrorKind::MalformedTree(
            "classic for needs three clauses",
          ))));
        };
        self.eval_tokens(&init.children)?;
        for index in 0..MAX_LOOP_ITERATIONS {
          let admitted = if condition.children.is_empty() {
            true
          } else {
            match self.eval_tokens(&condition.children) {
              Ok(value) => value.truthy(),
              Err(err) => return Err(self.fail(err)),
            }
          };
          self.record(StepEvent::Iteration {
            index,
            variable: None,
            element: None,
            condition: Some(admitted),
          });
          if !admitted {
            return Ok(Flow::Normal);
          }
          match self.run_block(&body.children)? {
            Flow::Normal => {}
            Flow::Break => return Ok(Flow::Normal),
            Flow::Return(value) => return Ok(Flow::Return(value)),
          }
          self.eval_tokens(&step.children)?;
        }
        Err(self.fail(self.err(RuntimeErrorKind::LoopLimitExceeded(MAX_LOOP_ITERATIONS))))
      }
    }
  }
}
