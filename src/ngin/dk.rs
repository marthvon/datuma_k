use std::collections::HashSet;
use std::sync::Arc;

use lasso::Spur;

use crate::core::exec::{MemberHost, RuntimeErrorKind, RuntimeValue};
use crate::dtct::registry::DtctDb;
use crate::dtct::types::{AttrArg, Dim, FactId, QueryError};

#[derive(Debug, Clone)]
struct DkHost {
  db: Arc<DtctDb>,
  include_dims: HashSet<Dim>,
  exclude_dims: HashSet<Dim>,
  ids: Vec<FactId>,
  dim: Dim,
  row: Option<Spur>,
}

pub fn dk_host(db: Arc<DtctDb>) -> RuntimeValue {
  RuntimeValue::Host(Arc::new(DkHost {
    ids: db.all_ids(),
    db,
    include_dims: HashSet::new(),
    exclude_dims: HashSet::new(),
    dim: Dim::Model,
    row: None,
  }))
}

impl DkHost {
  fn wrap(self) -> RuntimeValue {
    RuntimeValue::Host(Arc::new(self))
  }

  fn with_include(&self, dim: Dim, name: &str) -> Result<RuntimeValue, RuntimeErrorKind> {
    if self.include_dims.contains(&dim) {
      Err(RuntimeErrorKind::MalformedTree(
        "duplicate filter dimension",
      ))
    } else {
      let mut include_dims = self.include_dims.clone();
      include_dims.insert(dim);
      Ok(
        DkHost {
          db: Arc::clone(&self.db),
          include_dims,
          exclude_dims: self.exclude_dims.clone(),
          ids: self.narrow(dim, name, false)?,
          dim: self.dim,
          row: self.row,
        }
        .wrap(),
      )
    }
  }

  fn with_exclude(&self, dim: Dim, name: &str) -> Result<RuntimeValue, RuntimeErrorKind> {
    if self.exclude_dims.contains(&dim) {
      Err(RuntimeErrorKind::MalformedTree(
        "duplicate filter dimension",
      ))
    } else {
      let mut exclude_dims = self.exclude_dims.clone();
      exclude_dims.insert(dim);
      Ok(
        DkHost {
          db: Arc::clone(&self.db),
          include_dims: self.include_dims.clone(),
          exclude_dims,
          ids: self.narrow(dim, name, true)?,
          dim: self.dim,
          row: self.row,
        }
        .wrap(),
      )
    }
  }

  fn narrow(&self, dim: Dim, name: &str, exclude: bool) -> Result<Vec<FactId>, RuntimeErrorKind> {
    match self.db.spur(name) {
      Some(spur) => {
        let view = self.db.view(self.dim, self.ids.clone());
        let next = if exclude {
          view.exclude(dim, &[spur])
        } else {
          view.include(dim, &[spur])
        };
        match next {
          Ok(view) => Ok(view.ids().to_vec()),
          Err(err) => Err(query_err(err)),
        }
      }
      None => Ok(Vec::new()),
    }
  }

  fn project_rows(&self, dim: Dim) -> RuntimeValue {
    let view = self.db.view(dim, self.ids.clone());
    RuntimeValue::Array(
      view
        .spurs()
        .into_iter()
        .map(|spur| {
          DkHost {
            db: Arc::clone(&self.db),
            include_dims: self.include_dims.clone(),
            exclude_dims: self.exclude_dims.clone(),
            ids: view.of(spur).ids().to_vec(),
            dim,
            row: Some(spur),
          }
          .wrap()
        })
        .collect(),
    )
  }

  fn field_type(&self) -> Result<RuntimeValue, RuntimeErrorKind> {
    if self.dim != Dim::Field || self.row.is_none() {
      Err(RuntimeErrorKind::UnknownMember {
        kind: "dk",
        member: "type".to_string(),
      })
    } else {
      let mut found = None;
      for &id in &self.ids {
        if let Some(ty) = self.db.facts()[id as usize].ty {
          found = Some(self.db.resolve(ty).to_string());
          break;
        }
      }
      match found {
        Some(name) => Ok(RuntimeValue::String(name)),
        None => Ok(RuntimeValue::Null),
      }
    }
  }

  fn first_args(&self) -> RuntimeValue {
    match self.ids.first() {
      Some(&id) => RuntimeValue::Array(
        self.db.facts()[id as usize]
          .args
          .iter()
          .copied()
          .map(|arg| match arg {
            AttrArg::Ident(spur) => RuntimeValue::String(self.db.resolve(spur).to_string()),
            AttrArg::String(spur) => RuntimeValue::String(self.db.resolve(spur).to_string()),
            AttrArg::Integer(n) => RuntimeValue::Integer(n),
            AttrArg::Float(n) => RuntimeValue::Double(n),
            AttrArg::Boolean(flag) => RuntimeValue::Boolean(flag),
            AttrArg::Null => RuntimeValue::Null,
          })
          .collect(),
      ),
      None => RuntimeValue::Array(Vec::new()),
    }
  }
}

fn query_err(err: QueryError) -> RuntimeErrorKind {
  match err {
    QueryError::DuplicateFilterDim(_) => {
      RuntimeErrorKind::MalformedTree("duplicate filter dimension")
    }
    QueryError::EmptyFilter(_) => RuntimeErrorKind::MalformedTree("empty filter"),
  }
}

fn one_name<'a>(args: &'a [RuntimeValue], function: &str) -> Result<&'a str, RuntimeErrorKind> {
  match args {
    [RuntimeValue::String(name)] => Ok(name.as_str()),
    _ => Err(RuntimeErrorKind::ArityMismatch {
      function: function.to_string(),
      expected: 1,
      got: args.len(),
    }),
  }
}

impl MemberHost for DkHost {
  fn kind(&self) -> &'static str {
    "dk"
  }

  fn display(&self) -> String {
    match self.row {
      Some(spur) => self.db.resolve(spur).to_string(),
      None => String::new(),
    }
  }

  fn property(&self, name: &str) -> Result<RuntimeValue, RuntimeErrorKind> {
    match name {
      "models" => Ok(self.project_rows(Dim::Model)),
      "fields" => Ok(self.project_rows(Dim::Field)),
      "traits" => Ok(self.project_rows(Dim::Trait)),
      "types" => Ok(self.project_rows(Dim::Type)),
      "attributes" => Ok(self.project_rows(Dim::Attribute)),
      "length" => Ok(RuntimeValue::Integer(
        self.db.view(self.dim, self.ids.clone()).spurs().len() as i64,
      )),
      "type" => self.field_type(),
      "args" => Ok(self.first_args()),
      _ => Err(RuntimeErrorKind::UnknownMember {
        kind: "dk",
        member: name.to_string(),
      }),
    }
  }

  fn call(&self, name: &str, args: Vec<RuntimeValue>) -> Result<RuntimeValue, RuntimeErrorKind> {
    match name {
      "model" | "models" => self.with_include(Dim::Model, one_name(&args, "dk.model")?),
      "trait" | "traits" => self.with_include(Dim::Trait, one_name(&args, "dk.trait")?),
      "field" | "fields" => self.with_include(Dim::Field, one_name(&args, "dk.field")?),
      "attribute" | "attributes" => {
        self.with_include(Dim::Attribute, one_name(&args, "dk.attribute")?)
      }
      "type" | "types" => self.with_include(Dim::Type, one_name(&args, "dk.type")?),
      "not_model" => self.with_exclude(Dim::Model, one_name(&args, "dk.not_model")?),
      "not_trait" => self.with_exclude(Dim::Trait, one_name(&args, "dk.not_trait")?),
      "not_field" => self.with_exclude(Dim::Field, one_name(&args, "dk.not_field")?),
      "not_attribute" => self.with_exclude(Dim::Attribute, one_name(&args, "dk.not_attribute")?),
      "not_type" => self.with_exclude(Dim::Type, one_name(&args, "dk.not_type")?),
      _ => Err(RuntimeErrorKind::UnknownMember {
        kind: "dk",
        member: format!("{name}()"),
      }),
    }
  }

  fn truthy(&self) -> bool {
    !self.ids.is_empty()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::dtct::types::{AttrArg, DtctFact};
  use lasso::ThreadedRodeo;
  use tinyvec::TinyVec;

  fn sample_db() -> Arc<DtctDb> {
    let pool = ThreadedRodeo::new();
    let user = pool.get_or_intern("User");
    let post = pool.get_or_intern("Post");
    let email = pool.get_or_intern("email");
    let title = pool.get_or_intern("title");
    let email_type = pool.get_or_intern("email_type");
    let text_type = pool.get_or_intern("text_type");
    let filterable = pool.get_or_intern("filterable");
    let bootstrap = pool.get_or_intern("Bootstrap");
    Arc::new(DtctDb::build(
      pool,
      vec![
        DtctFact {
          trait_name: Some(bootstrap),
          model: user,
          field: Some(email),
          ty: Some(email_type),
          attribute: Some(filterable),
          args: TinyVec::new(),
        },
        DtctFact {
          trait_name: None,
          model: post,
          field: Some(title),
          ty: Some(text_type),
          attribute: None,
          args: TinyVec::<[AttrArg; 2]>::new(),
        },
      ],
    ))
  }

  fn names(value: RuntimeValue) -> Vec<String> {
    match value {
      RuntimeValue::Array(items) => items.into_iter().map(|item| item.stringify()).collect(),
      _ => panic!("expected array"),
    }
  }

  #[test]
  fn models_projection_and_attribute_filter() {
    let db = sample_db();
    let dk = dk_host(Arc::clone(&db));
    let RuntimeValue::Host(host) = dk else {
      panic!("dk host");
    };
    let models = host.property("models").expect("models");
    assert_eq!(names(models), ["User", "Post"]);
    let filtered = host
      .call("model", vec![RuntimeValue::String("User".into())])
      .expect("model");
    let RuntimeValue::Host(user) = filtered else {
      panic!("user host");
    };
    let attributed = user
      .call("attribute", vec![RuntimeValue::String("filterable".into())])
      .expect("attribute");
    let RuntimeValue::Host(attributed) = attributed else {
      panic!("attr host");
    };
    let fields = attributed.property("fields").expect("fields");
    assert_eq!(names(fields), ["email"]);
  }

  #[test]
  fn duplicate_include_dim_errors() {
    let db = sample_db();
    let RuntimeValue::Host(host) = dk_host(db) else {
      panic!("dk host");
    };
    let once = host
      .call("model", vec![RuntimeValue::String("User".into())])
      .expect("first");
    let RuntimeValue::Host(once) = once else {
      panic!("host");
    };
    let err = once
      .call("model", vec![RuntimeValue::String("Post".into())])
      .expect_err("duplicate");
    assert!(matches!(
      err,
      RuntimeErrorKind::MalformedTree("duplicate filter dimension")
    ));
  }

  #[test]
  fn field_type_property() {
    let db = sample_db();
    let RuntimeValue::Host(host) = dk_host(db) else {
      panic!("dk host");
    };
    let RuntimeValue::Array(fields) = host.property("fields").expect("fields") else {
      panic!("fields");
    };
    let email = fields
      .iter()
      .find(|field| field.stringify() == "email")
      .expect("email");
    let RuntimeValue::Host(email) = email else {
      panic!("email host");
    };
    assert_eq!(
      email.property("type").expect("type"),
      RuntimeValue::String("email_type".into())
    );
  }

  #[test]
  fn attribute_args_from_nested_field_view() {
    let pool = ThreadedRodeo::new();
    let event = pool.get_or_intern("Event");
    let capacity = pool.get_or_intern("capacity");
    let int_type = pool.get_or_intern("int_type");
    let min = pool.get_or_intern("min");
    let max = pool.get_or_intern("max");
    let mut min_args = TinyVec::new();
    min_args.push(AttrArg::Integer(1));
    let mut max_args = TinyVec::new();
    max_args.push(AttrArg::Integer(500));
    let db = Arc::new(DtctDb::build(
      pool,
      vec![
        DtctFact {
          trait_name: None,
          model: event,
          field: Some(capacity),
          ty: Some(int_type),
          attribute: Some(min),
          args: min_args,
        },
        DtctFact {
          trait_name: None,
          model: event,
          field: Some(capacity),
          ty: Some(int_type),
          attribute: Some(max),
          args: max_args,
        },
      ],
    ));
    let RuntimeValue::Host(host) = dk_host(db) else {
      panic!("dk host");
    };
    let RuntimeValue::Array(fields) = host.property("fields").expect("fields") else {
      panic!("fields");
    };
    let RuntimeValue::Host(field) = fields
      .iter()
      .find(|item| item.stringify() == "capacity")
      .expect("capacity")
    else {
      panic!("field host");
    };
    let RuntimeValue::Array(attrs) = field.property("attributes").expect("attributes") else {
      panic!("attributes");
    };
    let RuntimeValue::Host(min_attr) = attrs
      .iter()
      .find(|item| item.stringify() == "min")
      .expect("min")
    else {
      panic!("min host");
    };
    assert_eq!(
      min_attr.property("args").expect("args"),
      RuntimeValue::Array(vec![RuntimeValue::Integer(1)])
    );
    let RuntimeValue::Host(max_attr) = attrs
      .iter()
      .find(|item| item.stringify() == "max")
      .expect("max")
    else {
      panic!("max host");
    };
    assert_eq!(
      max_attr.property("args").expect("args"),
      RuntimeValue::Array(vec![RuntimeValue::Integer(500)])
    );
  }

  #[test]
  fn attribute_call_filters_args_and_missing_is_falsy() {
    let pool = ThreadedRodeo::new();
    let event = pool.get_or_intern("Event");
    let capacity = pool.get_or_intern("capacity");
    let int_type = pool.get_or_intern("int_type");
    let min = pool.get_or_intern("min");
    let max = pool.get_or_intern("max");
    let mut min_args = TinyVec::new();
    min_args.push(AttrArg::Integer(1));
    let mut max_args = TinyVec::new();
    max_args.push(AttrArg::Integer(500));
    let db = Arc::new(DtctDb::build(
      pool,
      vec![
        DtctFact {
          trait_name: None,
          model: event,
          field: Some(capacity),
          ty: Some(int_type),
          attribute: Some(min),
          args: min_args,
        },
        DtctFact {
          trait_name: None,
          model: event,
          field: Some(capacity),
          ty: Some(int_type),
          attribute: Some(max),
          args: max_args,
        },
      ],
    ));
    let RuntimeValue::Host(host) = dk_host(db) else {
      panic!("dk host");
    };
    let RuntimeValue::Array(fields) = host.property("fields").expect("fields") else {
      panic!("fields");
    };
    let RuntimeValue::Host(field) = fields
      .iter()
      .find(|item| item.stringify() == "capacity")
      .expect("capacity")
    else {
      panic!("field host");
    };
    let RuntimeValue::Host(min_attr) = field
      .call("attribute", vec![RuntimeValue::String("min".into())])
      .expect("min")
    else {
      panic!("min host");
    };
    assert!(min_attr.truthy());
    assert_eq!(
      min_attr.property("args").expect("args"),
      RuntimeValue::Array(vec![RuntimeValue::Integer(1)])
    );
    let RuntimeValue::Host(missing) = field
      .call("attribute", vec![RuntimeValue::String("required".into())])
      .expect("missing")
    else {
      panic!("missing host");
    };
    assert!(!missing.truthy());
    assert_eq!(
      missing.property("args").expect("args"),
      RuntimeValue::Array(Vec::new())
    );
  }

  #[test]
  fn model_type_is_unknown() {
    let db = sample_db();
    let RuntimeValue::Host(host) = dk_host(db) else {
      panic!("dk host");
    };
    let RuntimeValue::Array(models) = host.property("models").expect("models") else {
      panic!("models");
    };
    let RuntimeValue::Host(user) = models
      .iter()
      .find(|model| model.stringify() == "User")
      .expect("User")
    else {
      panic!("user host");
    };
    let err = user.property("type").expect_err("model.type");
    assert!(matches!(
      err,
      RuntimeErrorKind::UnknownMember {
        kind: "dk",
        member
      } if member == "type"
    ));
  }

  #[test]
  fn not_model_excludes_rows() {
    let db = sample_db();
    let RuntimeValue::Host(host) = dk_host(db) else {
      panic!("dk host");
    };
    let filtered = host
      .call("not_model", vec![RuntimeValue::String("Post".into())])
      .expect("not_model");
    let RuntimeValue::Host(filtered) = filtered else {
      panic!("host");
    };
    assert_eq!(
      names(filtered.property("models").expect("models")),
      ["User"]
    );
  }

  #[test]
  fn include_and_exclude_same_dim() {
    let db = sample_db();
    let RuntimeValue::Host(host) = dk_host(db) else {
      panic!("dk host");
    };
    let included = host
      .call("model", vec![RuntimeValue::String("User".into())])
      .expect("model");
    let RuntimeValue::Host(included) = included else {
      panic!("host");
    };
    let both = included
      .call("not_model", vec![RuntimeValue::String("Post".into())])
      .expect("include+exclude");
    let RuntimeValue::Host(both) = both else {
      panic!("host");
    };
    assert_eq!(names(both.property("models").expect("models")), ["User"]);
  }

  #[test]
  fn duplicate_exclude_dim_errors() {
    let db = sample_db();
    let RuntimeValue::Host(host) = dk_host(db) else {
      panic!("dk host");
    };
    let once = host
      .call("not_model", vec![RuntimeValue::String("Post".into())])
      .expect("first");
    let RuntimeValue::Host(once) = once else {
      panic!("host");
    };
    let err = once
      .call("not_model", vec![RuntimeValue::String("User".into())])
      .expect_err("duplicate");
    assert!(matches!(
      err,
      RuntimeErrorKind::MalformedTree("duplicate filter dimension")
    ));
  }

  #[test]
  fn traits_call_alias_matches_trait() {
    let db = sample_db();
    let RuntimeValue::Host(host) = dk_host(db) else {
      panic!("dk host");
    };
    let via_plural = host
      .call("traits", vec![RuntimeValue::String("Bootstrap".into())])
      .expect("traits");
    let RuntimeValue::Host(via_plural) = via_plural else {
      panic!("host");
    };
    assert_eq!(
      names(via_plural.property("models").expect("models")),
      ["User"]
    );
  }
}
