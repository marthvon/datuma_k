use std::collections::HashSet;
use std::{vec, write};

use lasso::{Spur, ThreadedRodeo};

use crate::core::state::DatumaState;
use crate::core::value::{CoreValue, DatumaFinished};
use crate::dtct::registry::DtctDb;
use crate::dtct::types::{AttrArg, DtctDbError, DtctFact};
use crate::dtct::value::DtctValue;
use tinyvec::TinyVec;

#[derive(Debug)]
pub enum MaterializeError {
  MissingValue {
    kind: &'static str,
  },
  UnexpectedValue {
    expected: &'static str,
    got: &'static str,
  },
  InvalidArg {
    kind: &'static str,
  },
  Db(DtctDbError),
}

impl std::fmt::Display for MaterializeError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::MissingValue { kind } => write!(f, "missing {kind} value"),
      Self::UnexpectedValue { expected, got } => {
        write!(f, "expected {expected}, got {got}")
      }
      Self::InvalidArg { kind } => write!(f, "invalid {kind} argument"),
      Self::Db(err) => write!(f, "{err}"),
    }
  }
}

impl std::error::Error for MaterializeError {}

pub fn materialize(root: &DatumaState) -> Result<DtctDb, MaterializeError> {
  let pool = ThreadedRodeo::new();
  let facts = materialize_with(root, &pool)?;
  Ok(DtctDb::build(pool, facts))
}

pub fn materialize_with(
  root: &DatumaState,
  pool: &ThreadedRodeo,
) -> Result<Vec<DtctFact>, MaterializeError> {
  let mut facts = Vec::new();
  let mut seen = HashSet::new();
  for child in &root.children {
    expand_model(child, pool, &mut facts, &mut seen)?;
  }
  Ok(facts)
}

fn intern(pool: &ThreadedRodeo, s: &str) -> Spur {
  pool.get_or_intern(s)
}

fn expand_model(
  node: &DatumaState,
  pool: &ThreadedRodeo,
  facts: &mut Vec<DtctFact>,
  seen: &mut HashSet<Spur>,
) -> Result<(), MaterializeError> {
  let value = dtct_value(node, "model")?;
  if let DtctValue::Model { name, traits } = value {
    let model = intern(pool, name);
    if !seen.insert(model) {
      Err(MaterializeError::Db(DtctDbError::DuplicateModel(model)))
    } else {
      let trait_names: Vec<Option<Spur>> = if traits.is_empty() {
        vec![None]
      } else {
        traits.iter().map(|name| Some(intern(pool, name))).collect()
      };
      if node.children.is_empty() {
        for trait_name in trait_names {
          facts.push(DtctFact {
            trait_name,
            model,
            field: None,
            ty: None,
            attribute: None,
            args: TinyVec::new(),
          });
        }
      } else {
        for field_node in &node.children {
          expand_field(field_node, pool, model, &trait_names, facts)?;
        }
      }
      Ok(())
    }
  } else {
    Err(MaterializeError::UnexpectedValue {
      expected: "model",
      got: value.kind(),
    })
  }
}

fn expand_field(
  node: &DatumaState,
  pool: &ThreadedRodeo,
  model: Spur,
  traits: &[Option<Spur>],
  facts: &mut Vec<DtctFact>,
) -> Result<(), MaterializeError> {
  let value = dtct_value(node, "field")?;
  if let DtctValue::Field { name } = value {
    let field = intern(pool, name);
    let ty_node = node
      .children
      .first()
      .ok_or(MaterializeError::MissingValue { kind: "type" })?;
    expand_type(ty_node, pool, model, field, traits, facts)
  } else {
    Err(MaterializeError::UnexpectedValue {
      expected: "field",
      got: value.kind(),
    })
  }
}

fn expand_type(
  node: &DatumaState,
  pool: &ThreadedRodeo,
  model: Spur,
  field: Spur,
  traits: &[Option<Spur>],
  facts: &mut Vec<DtctFact>,
) -> Result<(), MaterializeError> {
  let value = dtct_value(node, "type")?;
  if let DtctValue::Type { name } = value {
    let ty = intern(pool, name);
    let attributes: Vec<(Option<Spur>, TinyVec<[AttrArg; 2]>)> = if node.children.is_empty() {
      vec![(None, TinyVec::new())]
    } else {
      let mut attrs = Vec::new();
      for attr_node in &node.children {
        attrs.push(expand_attribute(attr_node, pool)?);
      }
      attrs
    };
    for trait_name in traits {
      for (attribute, args) in &attributes {
        facts.push(DtctFact {
          trait_name: *trait_name,
          model,
          field: Some(field),
          ty: Some(ty),
          attribute: *attribute,
          args: args.clone(),
        });
      }
    }
    Ok(())
  } else {
    Err(MaterializeError::UnexpectedValue {
      expected: "type",
      got: value.kind(),
    })
  }
}

fn expand_attribute(
  node: &DatumaState,
  pool: &ThreadedRodeo,
) -> Result<(Option<Spur>, TinyVec<[AttrArg; 2]>), MaterializeError> {
  let value = dtct_value(node, "attribute")?;
  if let DtctValue::Attribute { name } = value {
    let mut args = TinyVec::new();
    for arg_node in &node.children {
      args.push(materialize_arg(arg_node, pool)?);
    }
    Ok((Some(intern(pool, name)), args))
  } else {
    Err(MaterializeError::UnexpectedValue {
      expected: "attribute",
      got: value.kind(),
    })
  }
}

fn materialize_arg(node: &DatumaState, pool: &ThreadedRodeo) -> Result<AttrArg, MaterializeError> {
  let value = node
    .value
    .as_ref()
    .ok_or(MaterializeError::MissingValue { kind: "core arg" })?;
  let core =
    value
      .as_any()
      .downcast_ref::<CoreValue>()
      .ok_or(MaterializeError::UnexpectedValue {
        expected: "core arg",
        got: value.kind(),
      })?;
  match core {
    CoreValue::Boolean(value) => Ok(AttrArg::Boolean(*value)),
    CoreValue::Null => Ok(AttrArg::Null),
    CoreValue::Ident(text) => Ok(AttrArg::Ident(intern(pool, text))),
    CoreValue::String(text) => Ok(AttrArg::String(intern(pool, text))),
    CoreValue::Integer(text) => text
      .parse()
      .map(AttrArg::Integer)
      .map_err(|_| MaterializeError::InvalidArg { kind: "integer" }),
    CoreValue::Float(text) | CoreValue::Double(text) => text
      .parse()
      .map(AttrArg::Float)
      .map_err(|_| MaterializeError::InvalidArg { kind: "float" }),
    CoreValue::InvokedFunction(callee) => Ok(AttrArg::Ident(intern(pool, callee))),
    CoreValue::Array
    | CoreValue::Dict
    | CoreValue::Grouped
    | CoreValue::Operator(_)
    | CoreValue::Program
    | CoreValue::Instruction { .. }
    | CoreValue::FunctionDef(_)
    | CoreValue::If
    | CoreValue::Else
    | CoreValue::ElseIf
    | CoreValue::For
    | CoreValue::Accessor
    | CoreValue::Return
    | CoreValue::Break
    | CoreValue::Yield => Err(MaterializeError::UnexpectedValue {
      expected: "core arg",
      got: core.kind(),
    }),
  }
}

fn dtct_value<'a>(
  node: &'a DatumaState,
  kind: &'static str,
) -> Result<&'a DtctValue, MaterializeError> {
  let value = node
    .value
    .as_ref()
    .ok_or(MaterializeError::MissingValue { kind })?;
  value
    .as_any()
    .downcast_ref::<DtctValue>()
    .ok_or(MaterializeError::UnexpectedValue {
      expected: kind,
      got: value.kind(),
    })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::state::DatumaState;
  use crate::core::value::CoreValue;
  use crate::dtct::types::{Dim, Filter, QueryFilter};
  use crate::dtct::value::DtctValue;

  fn ident(text: &str) -> DatumaState {
    DatumaState::leaf(Box::new(CoreValue::Ident(text.to_string())))
  }

  fn string(text: &str) -> DatumaState {
    DatumaState::leaf(Box::new(CoreValue::String(text.to_string())))
  }

  fn attribute(name: &str, args: Vec<DatumaState>) -> DatumaState {
    DatumaState::node(
      Some(Box::new(DtctValue::Attribute {
        name: name.to_string(),
      })),
      args,
    )
  }

  fn ty(name: &str, attributes: Vec<DatumaState>) -> DatumaState {
    DatumaState::node(
      Some(Box::new(DtctValue::Type {
        name: name.to_string(),
      })),
      attributes,
    )
  }

  fn field(name: &str, ty_node: DatumaState) -> DatumaState {
    DatumaState::node(
      Some(Box::new(DtctValue::Field {
        name: name.to_string(),
      })),
      vec![ty_node],
    )
  }

  fn model(name: &str, traits: Vec<&str>, fields: Vec<DatumaState>) -> DatumaState {
    DatumaState::node(
      Some(Box::new(DtctValue::Model {
        name: name.to_string(),
        traits: traits.into_iter().map(str::to_string).collect(),
      })),
      fields,
    )
  }

  fn names(db: &DtctDb, spurs: &[Spur]) -> Vec<String> {
    spurs
      .iter()
      .map(|spur| db.resolve(*spur).to_string())
      .collect()
  }

  #[test]
  fn materializes_sample_tree() {
    let root = DatumaState::node(
      None,
      vec![model(
        "MyModel",
        vec![],
        vec![
          field(
            "my_field",
            ty(
              "my_type",
              vec![
                attribute(
                  "my_attribute",
                  vec![ident("arg1"), ident("arg2"), ident("arg3")],
                ),
                attribute("my_attribute2", vec![]),
              ],
            ),
          ),
          field(
            "other",
            ty("other_type", vec![attribute("flag", vec![string("on")])]),
          ),
        ],
      )],
    );

    let db = materialize(&root).expect("materialize");
    let models = db
      .query(&QueryFilter(vec![]), Dim::Model)
      .expect("models")
      .spurs();
    assert_eq!(names(&db, &models), ["MyModel"]);
    let attrs = db
      .query(
        &QueryFilter(vec![
          Filter::r#in(Dim::Model, vec![db.spur("MyModel").unwrap()]),
          Filter::r#in(Dim::Field, vec![db.spur("my_field").unwrap()]),
        ]),
        Dim::Attribute,
      )
      .expect("attributes")
      .spurs();
    assert_eq!(names(&db, &attrs), ["my_attribute", "my_attribute2"]);
    let fact = db
      .facts()
      .iter()
      .find(|fact| db.resolve(fact.attribute.unwrap()) == "my_attribute")
      .expect("attr fact");
    assert_eq!(fact.args.len(), 3);
    assert_eq!(db.resolve(fact.ty.expect("type")), "my_type");
  }

  #[test]
  fn cartesian_expands_traits_and_attributes() {
    let root = DatumaState::node(
      None,
      vec![model(
        "AuditEvent",
        vec!["Immutable"],
        vec![field(
          "actor",
          ty(
            "email_type",
            vec![attribute("max_length", vec![ident("128")])],
          ),
        )],
      )],
    );
    let db = materialize(&root).expect("materialize");
    assert_eq!(db.facts().len(), 1);
    let models = db
      .query(
        &QueryFilter(vec![Filter::r#in(
          Dim::Trait,
          vec![db.spur("Immutable").unwrap()],
        )]),
        Dim::Model,
      )
      .expect("trait query")
      .spurs();
    assert_eq!(names(&db, &models), ["AuditEvent"]);
  }

  #[test]
  fn empty_models_emit_model_only_rows() {
    let root = DatumaState::node(
      None,
      vec![
        model("Marker", vec![], vec![]),
        model("Tagged", vec!["TagA", "TagB"], vec![]),
      ],
    );
    let db = materialize(&root).expect("materialize");
    assert_eq!(db.facts().len(), 3);
    let models = db
      .query(&QueryFilter(vec![]), Dim::Model)
      .expect("models")
      .spurs();
    assert_eq!(names(&db, &models), ["Marker", "Tagged"]);
    for fact in db.facts() {
      assert!(fact.field.is_none());
      assert!(fact.ty.is_none());
      assert!(fact.attribute.is_none());
      assert!(fact.args.is_empty());
    }
    let marker = db
      .facts()
      .iter()
      .find(|fact| db.resolve(fact.model) == "Marker")
      .expect("marker");
    assert!(marker.trait_name.is_none());
    let tagged_traits = db
      .query(
        &QueryFilter(vec![Filter::r#in(
          Dim::Model,
          vec![db.spur("Tagged").unwrap()],
        )]),
        Dim::Trait,
      )
      .expect("traits")
      .spurs();
    assert_eq!(names(&db, &tagged_traits), ["TagA", "TagB"]);
  }
}
