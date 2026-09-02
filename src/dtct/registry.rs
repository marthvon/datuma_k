use lasso::{RodeoReader, Spur, ThreadedRodeo};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::fs;
use std::io;
use std::path::Path;

use super::ids::{difference_sorted, intersect_sorted, union_sorted};
use super::types::{AttrArg, Dim, DtctDbError, DtctFact, FactId, Filter, QueryError, QueryFilter};

#[derive(Debug)]
pub struct DtctDb {
  pool: RodeoReader,
  facts: Vec<DtctFact>,
  by_trait: HashMap<Spur, Vec<FactId>>,
  by_model: HashMap<Spur, Vec<FactId>>,
  by_field: HashMap<Spur, Vec<FactId>>,
  by_type: HashMap<Spur, Vec<FactId>>,
  by_attribute: HashMap<Spur, Vec<FactId>>,
}

impl DtctDb {
  pub fn empty() -> Self {
    Self {
      pool: ThreadedRodeo::new().into_reader(),
      facts: Vec::new(),
      by_trait: HashMap::new(),
      by_model: HashMap::new(),
      by_field: HashMap::new(),
      by_type: HashMap::new(),
      by_attribute: HashMap::new(),
    }
  }

  pub fn build(pool: ThreadedRodeo, facts: Vec<DtctFact>) -> Self {
    let mut db = Self {
      pool: pool.into_reader(),
      facts,
      by_trait: HashMap::new(),
      by_model: HashMap::new(),
      by_field: HashMap::new(),
      by_type: HashMap::new(),
      by_attribute: HashMap::new(),
    };
    db.rebuild_indexes();
    db
  }

  fn rebuild_indexes(&mut self) {
    self.by_trait.clear();
    self.by_model.clear();
    self.by_field.clear();
    self.by_type.clear();
    self.by_attribute.clear();
    for (id, fact) in self.facts.iter().enumerate() {
      let id = id as FactId;
      if let Some(trait_name) = fact.trait_name {
        self.by_trait.entry(trait_name).or_default().push(id);
      }
      self.by_model.entry(fact.model).or_default().push(id);
      if let Some(field) = fact.field {
        self.by_field.entry(field).or_default().push(id);
      }
      if let Some(ty) = fact.ty {
        self.by_type.entry(ty).or_default().push(id);
      }
      if let Some(attribute) = fact.attribute {
        self.by_attribute.entry(attribute).or_default().push(id);
      }
    }
  }

  pub fn spur(&self, s: &str) -> Option<Spur> {
    self.pool.get(s)
  }

  pub fn resolve(&self, key: Spur) -> &str {
    self.pool.resolve(&key)
  }

  pub fn facts(&self) -> &[DtctFact] {
    &self.facts
  }

  pub fn query(&self, filter: &QueryFilter, dim: Dim) -> Result<QueryView<'_>, QueryError> {
    let mut seen_include = HashSet::new();
    let mut seen_exclude = HashSet::new();
    for item in &filter.0 {
      if item.names.is_empty() {
        return Err(QueryError::EmptyFilter(item.dim));
      }
      let seen = if item.exclude {
        &mut seen_exclude
      } else {
        &mut seen_include
      };
      if !seen.insert(item.dim) {
        return Err(QueryError::DuplicateFilterDim(item.dim));
      }
    }
    Ok(QueryView {
      db: self,
      dim,
      ids: if filter.0.is_empty() {
        self.all_ids()
      } else {
        self.matching_ids(&filter.0)
      },
    })
  }

  pub fn view(&self, dim: Dim, ids: Vec<FactId>) -> QueryView<'_> {
    QueryView { db: self, dim, ids }
  }

  pub fn all_ids(&self) -> Vec<FactId> {
    (0..self.facts.len() as FactId).collect()
  }

  fn matching_ids(&self, filters: &[Filter]) -> Vec<FactId> {
    let mut ids: Option<Vec<FactId>> = None;
    for filter in filters.iter().filter(|filter| !filter.exclude) {
      let posting = self.union_posting(filter.dim, &filter.names);
      ids = Some(match ids {
        None => posting,
        Some(cur) => intersect_sorted(&cur, &posting),
      });
    }
    let mut ids = ids.unwrap_or_else(|| self.all_ids());
    for filter in filters.iter().filter(|filter| filter.exclude) {
      ids = difference_sorted(&ids, &self.union_posting(filter.dim, &filter.names));
    }
    ids
  }

  fn union_posting(&self, dim: Dim, names: &[Spur]) -> Vec<FactId> {
    let index = match dim {
      Dim::Trait => &self.by_trait,
      Dim::Model => &self.by_model,
      Dim::Field => &self.by_field,
      Dim::Type => &self.by_type,
      Dim::Attribute => &self.by_attribute,
    };
    let mut ids = Vec::new();
    for spur in names {
      ids = union_sorted(&ids, index.get(spur).map(Vec::as_slice).unwrap_or(&[]));
    }
    ids
  }

  pub fn dump(&self, path: &Path) -> io::Result<()> {
    fs::write(path, self.dump_string())
  }

  pub fn dump_string(&self) -> String {
    let mut out = String::new();
    write!(out, "# facts {}\n", self.facts.len()).expect("write dump");
    for (id, fact) in self.facts.iter().enumerate() {
      write!(
        out,
        "[{id}] trait={} model={} field={} type={} attribute={} args=[{}]\n",
        self.opt_name(fact.trait_name),
        self.resolve(fact.model),
        self.opt_name(fact.field),
        self.opt_name(fact.ty),
        self.opt_name(fact.attribute),
        fact
          .args
          .iter()
          .map(|arg| match *arg {
            AttrArg::Ident(spur) | AttrArg::String(spur) => self.resolve(spur).to_string(),
            AttrArg::Integer(value) => value.to_string(),
            AttrArg::Float(value) => value.to_string(),
            AttrArg::Boolean(value) => value.to_string(),
            AttrArg::Null => "null".to_string(),
          })
          .collect::<Vec<_>>()
          .join(", "),
      )
      .expect("write dump");
    }
    self.dump_index(&mut out, "by_trait", &self.by_trait);
    self.dump_index(&mut out, "by_model", &self.by_model);
    self.dump_index(&mut out, "by_field", &self.by_field);
    self.dump_index(&mut out, "by_type", &self.by_type);
    self.dump_index(&mut out, "by_attribute", &self.by_attribute);
    out
  }

  fn opt_name(&self, spur: Option<Spur>) -> &str {
    match spur {
      Some(spur) => self.resolve(spur),
      None => "-",
    }
  }

  fn dump_index(&self, out: &mut String, label: &str, index: &HashMap<Spur, Vec<FactId>>) {
    write!(out, "# index {label}\n").expect("write dump");
    let mut keys: Vec<Spur> = index.keys().copied().collect();
    keys.sort_by_key(|spur| self.resolve(*spur));
    for key in keys {
      let list = index[&key]
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
      write!(out, "{} -> {{{list}}}\n", self.resolve(key)).expect("write dump");
    }
  }
}

#[derive(Debug, Clone)]
pub struct QueryView<'db> {
  db: &'db DtctDb,
  dim: Dim,
  ids: Vec<FactId>,
}

impl<'db> QueryView<'db> {
  pub fn spurs(&self) -> Vec<Spur> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for &id in &self.ids {
      if let Some(spur) = project(&self.db.facts[id as usize], self.dim) {
        if seen.insert(spur) {
          out.push(spur);
        }
      }
    }
    out
  }

  pub fn project(&self, dim: Dim) -> QueryView<'db> {
    QueryView {
      db: self.db,
      dim,
      ids: self.ids.clone(),
    }
  }

  pub fn include(&self, dim: Dim, names: &[Spur]) -> Result<QueryView<'db>, QueryError> {
    if names.is_empty() {
      Err(QueryError::EmptyFilter(dim))
    } else {
      Ok(QueryView {
        db: self.db,
        dim: self.dim,
        ids: intersect_sorted(&self.ids, &self.db.union_posting(dim, names)),
      })
    }
  }

  pub fn exclude(&self, dim: Dim, names: &[Spur]) -> Result<QueryView<'db>, QueryError> {
    if names.is_empty() {
      Err(QueryError::EmptyFilter(dim))
    } else {
      Ok(QueryView {
        db: self.db,
        dim: self.dim,
        ids: difference_sorted(&self.ids, &self.db.union_posting(dim, names)),
      })
    }
  }

  pub fn of(&self, name: Spur) -> QueryView<'db> {
    QueryView {
      db: self.db,
      dim: self.dim,
      ids: intersect_sorted(&self.ids, &self.db.union_posting(self.dim, &[name])),
    }
  }

  pub fn ids(&self) -> &[FactId] {
    &self.ids
  }

  pub fn dim(&self) -> Dim {
    self.dim
  }
}

fn project(fact: &DtctFact, dim: Dim) -> Option<Spur> {
  match dim {
    Dim::Trait => fact.trait_name,
    Dim::Model => Some(fact.model),
    Dim::Field => fact.field,
    Dim::Type => fact.ty,
    Dim::Attribute => fact.attribute,
  }
}

pub fn merge_model_names(
  seen: &mut HashSet<Spur>,
  incoming: impl IntoIterator<Item = Spur>,
) -> Result<(), DtctDbError> {
  for model in incoming {
    if !seen.insert(model) {
      return Err(DtctDbError::DuplicateModel(model));
    }
  }
  Ok(())
}
