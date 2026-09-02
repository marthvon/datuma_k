use crate::core::value::DatumaFinished;

#[derive(Debug, Default)]
pub struct DatumaState {
  pub value: Option<Box<dyn DatumaFinished>>,
  pub children: Vec<DatumaState>,
}

impl DatumaState {
  pub fn leaf(value: Box<dyn DatumaFinished>) -> Self {
    Self {
      value: Some(value),
      children: Vec::new(),
    }
  }

  pub fn node(value: Option<Box<dyn DatumaFinished>>, children: Vec<DatumaState>) -> Self {
    Self { value, children }
  }

  pub fn adopt(&mut self, child: DatumaState) {
    self.children.push(child);
  }
}
