use indexmap::IndexMap;

use super::error::{RuntimeError, RuntimeErrorKind};
use super::stmt::Flow;
use super::value::RuntimeValue;
use super::{Interp, StepEvent, core_value, members, ops};
use crate::core::state::DatumaState;
use crate::core::value::{CoreOperator, CoreValue};

const PREFIX_BP: u8 = 21;

#[derive(Debug)]
pub(super) struct LValue {
  name: String,
  path: Vec<PathStep>,
}

#[derive(Debug)]
enum PathStep {
  Index(i64),
  Key(String),
}

impl<'tree> Interp<'tree> {
  /// Instructions, accessors, groups and argument slots are all flat token
  /// streams, so every expression entry point funnels through here.
  pub(crate) fn eval_tokens(
    &mut self,
    tokens: &'tree [DatumaState],
  ) -> Result<RuntimeValue, RuntimeError> {
    if tokens.is_empty() {
      Ok(RuntimeValue::Null)
    } else {
      match tokens.iter().position(|token| assign_op(token).is_some()) {
        Some(at) => self.eval_assign(tokens, at),
        None => {
          let (value, next) = self.parse_expr(tokens, 0, 0)?;
          if next == tokens.len() {
            Ok(value)
          } else {
            Err(self.err(RuntimeErrorKind::MalformedTree(
              "unconsumed expression tokens",
            )))
          }
        }
      }
    }
  }

  pub(crate) fn eval_operand(
    &mut self,
    node: &'tree DatumaState,
  ) -> Result<RuntimeValue, RuntimeError> {
    match core_value(node) {
      None | Some(CoreValue::Grouped | CoreValue::Instruction { .. }) => {
        self.eval_tokens(&node.children)
      }
      Some(CoreValue::Ident(name)) => self
        .scope
        .get(name)
        .cloned()
        .ok_or_else(|| self.err(RuntimeErrorKind::UndefinedVariable(name.clone()))),
      Some(CoreValue::String(text)) => Ok(RuntimeValue::String(text.clone())),
      Some(CoreValue::Integer(literal)) => literal
        .parse()
        .map(RuntimeValue::Integer)
        .map_err(|_| self.err(RuntimeErrorKind::MalformedTree("integer literal"))),
      Some(CoreValue::Float(literal)) => literal
        .parse()
        .map(RuntimeValue::Float)
        .map_err(|_| self.err(RuntimeErrorKind::MalformedTree("float literal"))),
      Some(CoreValue::Double(literal)) => literal
        .parse()
        .map(RuntimeValue::Double)
        .map_err(|_| self.err(RuntimeErrorKind::MalformedTree("double literal"))),
      Some(CoreValue::Boolean(flag)) => Ok(RuntimeValue::Boolean(*flag)),
      Some(CoreValue::Null) => Ok(RuntimeValue::Null),
      Some(CoreValue::Array) => self.eval_groups(&node.children).map(RuntimeValue::Array),
      Some(CoreValue::Dict) => {
        let mut entries = IndexMap::with_capacity(node.children.len());
        for entry in &node.children {
          let Some(key) = entry.children.first().and_then(dict_key) else {
            return Err(self.err(RuntimeErrorKind::MalformedTree("dict entry key")));
          };
          entries.insert(key, self.eval_tokens(&entry.children[1..])?);
        }
        Ok(RuntimeValue::Dict(entries))
      }
      Some(CoreValue::InvokedFunction(name)) => {
        let args = self.eval_groups(&node.children)?;
        self.call_function(name, args)
      }
      Some(CoreValue::If) => self.eval_if(node),
      Some(CoreValue::Program) => self.branch_value(node),
      Some(_) => Err(self.err(RuntimeErrorKind::MalformedTree("operand"))),
    }
  }

  /// Argument lists and collection literals lose their commas during parsing;
  /// a new element starts wherever two operands sit next to each other.
  pub(super) fn eval_groups(
    &mut self,
    tokens: &'tree [DatumaState],
  ) -> Result<Vec<RuntimeValue>, RuntimeError> {
    let mut values = Vec::new();
    let mut start = 0;
    let mut expecting_operand = true;
    for (at, token) in tokens.iter().enumerate() {
      if merge_wrapper_op(token).is_some() {
        continue;
      }
      match core_value(token) {
        Some(CoreValue::Accessor) => {}
        Some(CoreValue::Operator(CoreOperator::Increment | CoreOperator::Decrement)) => {}
        Some(CoreValue::Operator(_)) => expecting_operand = true,
        _ if expecting_operand => expecting_operand = false,
        _ => {
          values.push(self.eval_tokens(&tokens[start..at])?);
          start = at;
        }
      }
    }
    if start < tokens.len() {
      values.push(self.eval_tokens(&tokens[start..])?);
    }
    Ok(values)
  }

  pub(super) fn eval_if(&mut self, node: &'tree DatumaState) -> Result<RuntimeValue, RuntimeError> {
    let [condition, then_branch, tail @ ..] = &node.children[..] else {
      return Err(self.err(RuntimeErrorKind::MalformedTree(
        "if needs a condition and a branch",
      )));
    };
    if self.eval_tokens(&condition.children)?.truthy() {
      return self.branch_value(then_branch);
    }
    match tail.first() {
      None => Ok(RuntimeValue::Null),
      Some(next) => match core_value(next) {
        Some(CoreValue::ElseIf) => self.eval_if(next),
        Some(CoreValue::Else) => match next.children.first() {
          Some(body) => self.branch_value(body),
          None => Err(self.err(RuntimeErrorKind::MalformedTree("else needs a body"))),
        },
        _ => self.branch_value(next),
      },
    }
  }

  pub(super) fn branch_value(
    &mut self,
    node: &'tree DatumaState,
  ) -> Result<RuntimeValue, RuntimeError> {
    if matches!(core_value(node), Some(CoreValue::Program)) {
      self.run_block(&node.children).map(|flow| match flow {
        Flow::Return(value) => value,
        Flow::Normal | Flow::Break => RuntimeValue::Null,
      })
    } else {
      self.eval_operand(node)
    }
  }

  fn eval_assign(
    &mut self,
    tokens: &'tree [DatumaState],
    at: usize,
  ) -> Result<RuntimeValue, RuntimeError> {
    let Some(op) = assign_op(&tokens[at]) else {
      return Err(self.err(RuntimeErrorKind::MalformedTree("assignment operator")));
    };
    let (lvalue, consumed) = self.take_lvalue(tokens, 0)?;
    if consumed != at {
      return Err(self.err(RuntimeErrorKind::NotAssignable));
    }
    let rhs = self.eval_tokens(&tokens[at + 1..])?;
    let value = match compound_base(op) {
      None => rhs,
      Some(base) => self.lift(ops::binary(base, self.read_lvalue(&lvalue)?, rhs))?,
    };
    self.write_lvalue(&lvalue, value.clone())?;
    self.record(StepEvent::Assign {
      target: lvalue.name.clone(),
      value: value.clone(),
    });
    Ok(value)
  }

  fn parse_expr(
    &mut self,
    tokens: &'tree [DatumaState],
    pos: usize,
    min_bp: u8,
  ) -> Result<(RuntimeValue, usize), RuntimeError> {
    let (mut lhs, mut pos) = self.parse_prefix(tokens, pos)?;
    while let Some(CoreValue::Operator(op)) = tokens.get(pos).and_then(|token| core_value(token)) {
      let Some((lbp, rbp)) = binary_bp(*op) else {
        break;
      };
      if lbp < min_bp {
        break;
      }
      let (rhs, next) = self.parse_expr(tokens, pos + 1, rbp)?;
      lhs = self.lift(ops::binary(*op, lhs, rhs))?;
      pos = next;
    }
    Ok((lhs, pos))
  }

  fn parse_prefix(
    &mut self,
    tokens: &'tree [DatumaState],
    pos: usize,
  ) -> Result<(RuntimeValue, usize), RuntimeError> {
    let Some(node) = tokens.get(pos) else {
      return Err(self.err(RuntimeErrorKind::MalformedTree("expression ended early")));
    };
    match core_value(node) {
      Some(CoreValue::Operator(CoreOperator::Not)) => {
        let (operand, next) = self.parse_expr(tokens, pos + 1, PREFIX_BP)?;
        Ok((self.lift(ops::unary(CoreOperator::Not, operand))?, next))
      }
      Some(CoreValue::Operator(op @ (CoreOperator::Increment | CoreOperator::Decrement))) => {
        let (lvalue, next) = self.take_lvalue(tokens, pos + 1)?;
        let updated = self.lift(ops::step(*op, self.read_lvalue(&lvalue)?))?;
        self.write_lvalue(&lvalue, updated.clone())?;
        Ok((updated, next))
      }
      Some(CoreValue::Operator(op)) => Err(self.err(RuntimeErrorKind::InvalidUnaryOperation {
        op: *op,
        operand: "nothing",
      })),
      Some(CoreValue::Ident(name)) => {
        let value = self.eval_operand(node)?;
        let origin = LValue {
          name: name.clone(),
          path: Vec::new(),
        };
        self.apply_postfix(value, Some(origin), tokens, pos + 1)
      }
      _ => {
        let value = self.eval_operand(node)?;
        self.apply_postfix(value, None, tokens, pos + 1)
      }
    }
  }

  /// Postfix binds tighter than any binary operator. `origin` tracks the
  /// variable path the value came from so mutations can be written back.
  fn apply_postfix(
    &mut self,
    mut value: RuntimeValue,
    mut origin: Option<LValue>,
    tokens: &'tree [DatumaState],
    mut pos: usize,
  ) -> Result<(RuntimeValue, usize), RuntimeError> {
    while pos < tokens.len() {
      let node = &tokens[pos];
      if let Some(op) = merge_wrapper_op(node) {
        let rhs = self.eval_tokens(&node.children[1..])?;
        value = self.lift(ops::binary(op, value, rhs))?;
        origin = None;
        pos += 1;
        continue;
      }
      match core_value(node) {
        Some(CoreValue::Accessor) => {
          let key = self.eval_tokens(&node.children)?;
          value = self.lift(members::index(&value, &key))?;
          match (origin.as_mut(), path_step(&key)) {
            (Some(lvalue), Some(step)) => lvalue.path.push(step),
            (Some(_), None) => origin = None,
            (None, _) => {}
          }
          pos += 1;
        }
        Some(CoreValue::Operator(CoreOperator::Dot)) => {
          let Some(member) = tokens.get(pos + 1) else {
            return Err(self.err(RuntimeErrorKind::MalformedTree("dot without a member")));
          };
          match core_value(member) {
            Some(CoreValue::Ident(name)) => {
              // Only dicts have real keys; `.length` on other kinds is synthetic
              // and must not extend an assignable path.
              let addressable = matches!(value, RuntimeValue::Dict(_));
              value = self.lift(members::property(&value, name))?;
              match origin.as_mut() {
                Some(lvalue) if addressable => lvalue.path.push(PathStep::Key(name.clone())),
                _ => origin = None,
              }
            }
            Some(CoreValue::InvokedFunction(name)) => {
              let args = self.eval_groups(&member.children)?;
              value = match origin.take() {
                Some(lvalue) => self.call_on_lvalue(&lvalue, name, args)?,
                None => self.lift(members::call(&mut value, name, args))?,
              };
            }
            _ => return Err(self.err(RuntimeErrorKind::MalformedTree("dot member"))),
          }
          pos += 2;
        }
        Some(CoreValue::Operator(op @ (CoreOperator::Increment | CoreOperator::Decrement))) => {
          let Some(lvalue) = origin.take() else {
            return Err(self.err(RuntimeErrorKind::NotAssignable));
          };
          self.write_lvalue(&lvalue, self.lift(ops::step(*op, value.clone()))?)?;
          pos += 1;
        }
        _ => break,
      }
    }
    Ok((value, pos))
  }

  fn call_on_lvalue(
    &mut self,
    lvalue: &LValue,
    name: &str,
    args: Vec<RuntimeValue>,
  ) -> Result<RuntimeValue, RuntimeError> {
    let result = match self.scope.get_mut(&lvalue.name) {
      None => Err(RuntimeErrorKind::UndefinedVariable(lvalue.name.clone())),
      Some(root) => match path_mut(root, &lvalue.path, false) {
        Ok(target) => members::call(target, name, args),
        Err(kind) => Err(kind),
      },
    };
    self.lift(result)
  }

  fn take_lvalue(
    &mut self,
    tokens: &'tree [DatumaState],
    pos: usize,
  ) -> Result<(LValue, usize), RuntimeError> {
    let Some(CoreValue::Ident(name)) = tokens.get(pos).and_then(|token| core_value(token)) else {
      return Err(self.err(RuntimeErrorKind::NotAssignable));
    };
    let mut lvalue = LValue {
      name: name.clone(),
      path: Vec::new(),
    };
    let mut cursor = pos + 1;
    while cursor < tokens.len() {
      match core_value(&tokens[cursor]) {
        Some(CoreValue::Accessor) => {
          let key = self.eval_tokens(&tokens[cursor].children)?;
          let Some(step) = path_step(&key) else {
            return Err(self.err(RuntimeErrorKind::InvalidIndexType {
              base: "collection",
              index: key.kind(),
            }));
          };
          lvalue.path.push(step);
          cursor += 1;
        }
        Some(CoreValue::Operator(CoreOperator::Dot)) => {
          let Some(CoreValue::Ident(key)) =
            tokens.get(cursor + 1).and_then(|token| core_value(token))
          else {
            break;
          };
          lvalue.path.push(PathStep::Key(key.clone()));
          cursor += 2;
        }
        _ => break,
      }
    }
    Ok((lvalue, cursor))
  }

  fn read_lvalue(&self, lvalue: &LValue) -> Result<RuntimeValue, RuntimeError> {
    let Some(root) = self.scope.get(&lvalue.name) else {
      return Err(self.err(RuntimeErrorKind::UndefinedVariable(lvalue.name.clone())));
    };
    let mut current = root;
    for step in &lvalue.path {
      current = match step {
        PathStep::Index(index) => match current {
          RuntimeValue::Array(items) => {
            match usize::try_from(*index).ok().and_then(|at| items.get(at)) {
              Some(item) => item,
              None => {
                return Err(self.err(RuntimeErrorKind::IndexOutOfBounds {
                  index: *index,
                  len: items.len(),
                }));
              }
            }
          }
          RuntimeValue::Null => return Err(self.err(RuntimeErrorKind::NullReference("index"))),
          other => {
            return Err(self.err(RuntimeErrorKind::InvalidIndexType {
              base: other.kind(),
              index: "integer",
            }));
          }
        },
        PathStep::Key(key) => match current {
          RuntimeValue::Dict(entries) => match entries.get(key) {
            Some(value) => value,
            None => {
              return Err(self.err(RuntimeErrorKind::UnknownMember {
                kind: "dict",
                member: key.clone(),
              }));
            }
          },
          RuntimeValue::Null => {
            return Err(self.err(RuntimeErrorKind::NullReference("member access")));
          }
          other => {
            return Err(self.err(RuntimeErrorKind::InvalidIndexType {
              base: other.kind(),
              index: "string",
            }));
          }
        },
      };
    }
    Ok(current.clone())
  }

  fn write_lvalue(&mut self, lvalue: &LValue, value: RuntimeValue) -> Result<(), RuntimeError> {
    if lvalue.path.is_empty() {
      self.scope.assign(&lvalue.name, value);
      Ok(())
    } else {
      let result = match self.scope.get_mut(&lvalue.name) {
        None => Err(RuntimeErrorKind::UndefinedVariable(lvalue.name.clone())),
        Some(root) => path_mut(root, &lvalue.path, true).map(|slot| {
          *slot = value;
        }),
      };
      self.lift(result)
    }
  }
}

/// `create_last` lets `d["new"] = 1` introduce a key while intermediate hops
/// still require an existing slot.
fn path_mut<'value>(
  root: &'value mut RuntimeValue,
  path: &[PathStep],
  create_last: bool,
) -> Result<&'value mut RuntimeValue, RuntimeErrorKind> {
  let mut current = root;
  for (at, step) in path.iter().enumerate() {
    current = match (current, step) {
      (RuntimeValue::Array(items), PathStep::Index(index)) => {
        let len = items.len();
        match usize::try_from(*index)
          .ok()
          .and_then(|at| items.get_mut(at))
        {
          Some(slot) => slot,
          None => return Err(RuntimeErrorKind::IndexOutOfBounds { index: *index, len }),
        }
      }
      (RuntimeValue::Dict(entries), PathStep::Key(key)) => {
        if create_last && at + 1 == path.len() && !entries.contains_key(key) {
          entries.insert(key.clone(), RuntimeValue::Null);
        }
        match entries.get_mut(key) {
          Some(slot) => slot,
          None => {
            return Err(RuntimeErrorKind::UnknownMember {
              kind: "dict",
              member: key.clone(),
            });
          }
        }
      }
      (RuntimeValue::Null, _) => return Err(RuntimeErrorKind::NullReference("assignment path")),
      (other, PathStep::Index(_)) => {
        return Err(RuntimeErrorKind::InvalidIndexType {
          base: other.kind(),
          index: "integer",
        });
      }
      (other, PathStep::Key(_)) => {
        return Err(RuntimeErrorKind::InvalidIndexType {
          base: other.kind(),
          index: "string",
        });
      }
    };
  }
  Ok(current)
}

fn path_step(key: &RuntimeValue) -> Option<PathStep> {
  match key {
    RuntimeValue::Integer(index) => Some(PathStep::Index(*index)),
    RuntimeValue::String(key) => Some(PathStep::Key(key.clone())),
    _ => None,
  }
}

fn dict_key(node: &DatumaState) -> Option<String> {
  match core_value(node) {
    Some(CoreValue::Ident(key) | CoreValue::String(key) | CoreValue::Integer(key)) => {
      Some(key.clone())
    }
    _ => None,
  }
}

/// A literal left operand of a collection merge becomes a sibling wrapper node
/// holding `[operator, rhs]` instead of staying flat.
fn merge_wrapper_op(node: &DatumaState) -> Option<CoreOperator> {
  if !matches!(core_value(node), Some(CoreValue::Array | CoreValue::Dict))
    || node.children.len() < 2
  {
    return None;
  }
  match core_value(&node.children[0]) {
    Some(CoreValue::Operator(op)) => binary_bp(*op).map(|_| *op),
    _ => None,
  }
}

pub(super) fn assign_op(node: &DatumaState) -> Option<CoreOperator> {
  let Some(CoreValue::Operator(op)) = core_value(node) else {
    return None;
  };
  match op {
    CoreOperator::Assign => Some(*op),
    _ => compound_base(*op).map(|_| *op),
  }
}

fn compound_base(op: CoreOperator) -> Option<CoreOperator> {
  match op {
    CoreOperator::AddAssign => Some(CoreOperator::Add),
    CoreOperator::SubAssign => Some(CoreOperator::Sub),
    CoreOperator::MulAssign => Some(CoreOperator::Mul),
    CoreOperator::DivAssign => Some(CoreOperator::Div),
    CoreOperator::ModAssign => Some(CoreOperator::Mod),
    CoreOperator::PowAssign => Some(CoreOperator::Pow),
    CoreOperator::XorAssign => Some(CoreOperator::Xor),
    CoreOperator::AndAssign => Some(CoreOperator::BitAnd),
    CoreOperator::OrAssign => Some(CoreOperator::BitOr),
    CoreOperator::AndAndAssign => Some(CoreOperator::And),
    CoreOperator::OrOrAssign => Some(CoreOperator::Or),
    CoreOperator::RightDiffAssign => Some(CoreOperator::RightDiff),
    CoreOperator::LeftDiffAssign => Some(CoreOperator::LeftDiff),
    _ => None,
  }
}

/// `(left, right)` binding powers; a right power below the left one makes the
/// operator right-associative.
fn binary_bp(op: CoreOperator) -> Option<(u8, u8)> {
  match op {
    CoreOperator::Or => Some((1, 2)),
    CoreOperator::And => Some((3, 4)),
    CoreOperator::BitOr => Some((5, 6)),
    CoreOperator::Xor
    | CoreOperator::SymmetricDiff
    | CoreOperator::RightDiff
    | CoreOperator::LeftDiff => Some((7, 8)),
    CoreOperator::BitAnd | CoreOperator::Intersect => Some((9, 10)),
    CoreOperator::Equal | CoreOperator::NotEqual => Some((11, 12)),
    CoreOperator::Lt | CoreOperator::Gt | CoreOperator::LessEqual | CoreOperator::GreaterEqual => {
      Some((13, 14))
    }
    CoreOperator::Add | CoreOperator::Sub => Some((15, 16)),
    CoreOperator::Mul | CoreOperator::Div | CoreOperator::Mod => Some((17, 18)),
    CoreOperator::Pow => Some((20, 19)),
    _ => None,
  }
}
