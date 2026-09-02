use indexmap::IndexMap;

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VNode {
  Host { id: String, text: String },
  Frame { id: String, children: Vec<VNode> },
}

impl VNode {
  pub fn host(id: impl Into<String>, text: impl Into<String>) -> Self {
    Self::Host {
      id: id.into(),
      text: text.into(),
    }
  }

  pub fn frame(id: impl Into<String>, children: Vec<VNode>) -> Self {
    Self::Frame {
      id: id.into(),
      children,
    }
  }

  pub fn id(&self) -> &str {
    match self {
      Self::Host { id, .. } | Self::Frame { id, .. } => id,
    }
  }

  pub fn flatten(&self) -> String {
    let mut out = String::new();
    flatten_into(self, &mut out);
    out
  }

  pub fn flatten_all(nodes: &[VNode]) -> String {
    let mut out = String::new();
    for node in nodes {
      flatten_into(node, &mut out);
    }
    out
  }
}

pub fn sanitize_id(id: &str) -> String {
  id.replace("*/", "").replace(['\n', '\r'], " ")
}

pub fn fence_token(identity: &str) -> String {
  let mut hash = FNV_OFFSET;
  for &byte in identity.as_bytes() {
    hash ^= u64::from(byte);
    hash = hash.wrapping_mul(FNV_PRIME);
  }
  format!("{:016x}", hash)
}

pub fn merge_planned(into: &mut IndexMap<String, Vec<VNode>>, extra: IndexMap<String, Vec<VNode>>) {
  for (path, nodes) in extra {
    match into.get_mut(&path) {
      Some(existing) => merge_nodes(existing, nodes),
      None => {
        into.insert(path, nodes);
      }
    }
  }
}

fn flatten_into(node: &VNode, out: &mut String) {
  match node {
    VNode::Host { text, .. } => out.push_str(text),
    VNode::Frame { children, .. } => {
      for child in children {
        flatten_into(child, out);
      }
    }
  }
}

fn merge_nodes(into: &mut Vec<VNode>, extra: Vec<VNode>) {
  for node in extra {
    let id = node.id().to_string();
    match into.iter().position(|item| item.id() == id) {
      Some(at) => merge_node(&mut into[at], node),
      None => into.push(node),
    }
  }
}

fn merge_node(into: &mut VNode, extra: VNode) {
  match (into, extra) {
    (VNode::Frame { children, .. }, VNode::Frame { children: more, .. }) => {
      merge_nodes(children, more);
    }
    (slot, extra) => *slot = extra,
  }
}
