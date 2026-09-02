use std::collections::HashMap;

use super::error::{RuntimeError, RuntimeErrorKind};
use super::stmt::Flow;
use super::value::RuntimeValue;
use super::{Interp, MAX_CALL_DEPTH, core_value};
use crate::core::state::DatumaState;
use crate::core::value::CoreValue;

#[derive(Debug, Clone, Copy)]
pub struct FunctionDef<'tree> {
  pub params: &'tree DatumaState,
  pub body: &'tree DatumaState,
}

#[derive(Debug, Default)]
pub struct FunctionTable<'tree> {
  defs: HashMap<&'tree str, FunctionDef<'tree>>,
}

impl<'tree> FunctionTable<'tree> {
  pub fn get(&self, name: &str) -> Option<FunctionDef<'tree>> {
    self.defs.get(name).copied()
  }
}

/// Pass 1: every `function_def` in the trees, including nested ones, becomes
/// globally callable. A later definition of the same name wins.
pub fn collect_functions<'tree>(
  roots: &[&'tree DatumaState],
) -> Result<FunctionTable<'tree>, RuntimeErrorKind> {
  let mut table = FunctionTable::default();
  for root in roots {
    collect_into(root, &mut table)?;
  }
  Ok(table)
}

fn collect_into<'tree>(
  node: &'tree DatumaState,
  table: &mut FunctionTable<'tree>,
) -> Result<(), RuntimeErrorKind> {
  if let Some(CoreValue::FunctionDef(name)) = core_value(node) {
    let [params, body] = &node.children[..] else {
      return Err(RuntimeErrorKind::MalformedTree(
        "function_def needs params and body",
      ));
    };
    table
      .defs
      .insert(name.as_str(), FunctionDef { params, body });
  }
  for child in &node.children {
    collect_into(child, table)?;
  }
  Ok(())
}

impl<'tree> Interp<'tree> {
  pub(super) fn call_function(
    &mut self,
    name: &str,
    args: Vec<RuntimeValue>,
  ) -> Result<RuntimeValue, RuntimeError> {
    if let Some(def) = self.functions.get(name) {
      if def.params.children.len() != args.len() {
        Err(self.err(RuntimeErrorKind::ArityMismatch {
          function: name.to_string(),
          expected: def.params.children.len(),
          got: args.len(),
        }))
      } else if self.depth >= MAX_CALL_DEPTH {
        Err(self.err(RuntimeErrorKind::StackOverflow {
          depth: MAX_CALL_DEPTH,
        }))
      } else {
        self.call_stack.push(name.to_string());
        self.scope.push_frame();
        let result = match def
          .params
          .children
          .iter()
          .zip(args)
          .try_for_each(|(param, arg)| match core_value(param) {
            Some(CoreValue::Ident(param_name)) => {
              self.scope.declare(param_name.clone(), arg);
              Ok(())
            }
            _ => Err(self.err(RuntimeErrorKind::MalformedTree("function parameter"))),
          }) {
          Err(err) => Err(err),
          Ok(()) => {
            self.depth += 1;
            let flow = self.run_block(&def.body.children);
            self.depth -= 1;
            flow.map(|flow| match flow {
              Flow::Return(value) => value,
              Flow::Normal | Flow::Break => RuntimeValue::Null,
            })
          }
        };
        self.scope.pop_frame();
        self.call_stack.pop();
        result
      }
    } else {
      Err(self.err(RuntimeErrorKind::UndefinedFunction(name.to_string())))
    }
  }
}
