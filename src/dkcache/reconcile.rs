use std::collections::HashMap;

use crate::dkcache::store::CachedNode;
use crate::dkcache::vnode::VNode;

struct Located {
  id: String,
  start: usize,
  end: usize,
  frame: bool,
  children: Vec<Located>,
}

enum Part<'a> {
  Unmarked(&'a str),
  Node(&'a Located),
}

pub fn run(existing: &str, cached: &[CachedNode], desired: &[VNode]) -> (String, Vec<CachedNode>) {
  let located = locate_children(existing, cached, 0, existing.len());
  let mut out = String::new();
  let mut tree = Vec::new();
  splice_level(
    existing,
    0,
    existing.len(),
    &located,
    desired,
    &mut out,
    &mut tree,
  );
  (out, tree)
}

pub fn line_col_at(text: &str, offset: usize) -> (usize, usize) {
  let at = offset.min(text.len());
  let prefix = &text[..at];
  let line = prefix.bytes().filter(|&byte| byte == b'\n').count() + 1;
  let col = match prefix.rfind('\n') {
    Some(index) => at - index,
    None => at + 1,
  };
  (line, col)
}

fn offset_at(text: &str, line: usize, col: usize) -> Option<usize> {
  if line < 1 || col < 1 {
    None
  } else {
    let mut rest = text;
    let mut at = 0usize;
    for _ in 1..line {
      match rest.find('\n') {
        Some(index) => {
          at += index + 1;
          rest = &rest[index + 1..];
        }
        None => return None,
      }
    }
    let col_idx = col - 1;
    let line_len = rest.find('\n').unwrap_or(rest.len());
    if col_idx <= line_len {
      Some(at + col_idx)
    } else {
      None
    }
  }
}

fn locate_children(existing: &str, cached: &[CachedNode], lo: usize, hi: usize) -> Vec<Located> {
  let mut cursor = lo;
  let mut found = Vec::new();
  for node in cached {
    if cursor > hi {
      break;
    } else {
      match locate_one(existing, node, cursor, hi) {
        Some(loc) => {
          cursor = loc.end;
          found.push(loc);
        }
        None => {}
      }
    }
  }
  found
}

fn locate_one(existing: &str, node: &CachedNode, cursor: usize, hi: usize) -> Option<Located> {
  if node.is_frame() {
    let children = node.children.as_deref().unwrap_or(&[]);
    let hint = offset_at(existing, node.line, node.col);
    let child_lo = match hint {
      Some(at) if at >= cursor && at <= hi => at,
      _ => cursor,
    };
    let located_children = locate_children(existing, children, child_lo, hi);
    match (located_children.first(), located_children.last()) {
      (Some(first), Some(last)) => Some(Located {
        id: node.id.clone(),
        start: first.start,
        end: last.end,
        frame: true,
        children: located_children,
      }),
      _ => None,
    }
  } else {
    let text = node.text.as_deref().unwrap_or("");
    if text.is_empty() {
      Some(Located {
        id: node.id.clone(),
        start: cursor,
        end: cursor,
        frame: false,
        children: Vec::new(),
      })
    } else {
      let hinted = match offset_at(existing, node.line, node.col) {
        Some(at)
          if at >= cursor
            && at + text.len() <= hi
            && existing
              .get(at..)
              .is_some_and(|slice| slice.starts_with(text)) =>
        {
          Some(at)
        }
        _ => None,
      };
      let start = hinted.or_else(|| {
        existing
          .get(cursor..hi)
          .and_then(|slice| slice.find(text).map(|index| cursor + index))
      })?;
      if start + text.len() > hi {
        None
      } else {
        Some(Located {
          id: node.id.clone(),
          start,
          end: start + text.len(),
          frame: false,
          children: Vec::new(),
        })
      }
    }
  }
}

fn splice_level(
  existing: &str,
  lo: usize,
  hi: usize,
  located: &[Located],
  desired: &[VNode],
  out: &mut String,
  cached: &mut Vec<CachedNode>,
) {
  let mut parts = Vec::new();
  let mut prev = lo;
  for loc in located {
    if loc.start > prev && loc.start <= existing.len() {
      let end = loc.start.min(existing.len());
      if prev < end {
        parts.push(Part::Unmarked(&existing[prev..end]));
      }
    }
    parts.push(Part::Node(loc));
    prev = loc.end;
  }
  if hi > prev && prev < existing.len() {
    parts.push(Part::Unmarked(&existing[prev..hi.min(existing.len())]));
  }

  let mut order = HashMap::new();
  for (index, node) in desired.iter().enumerate() {
    order.insert(node.id().to_string(), index);
  }
  let mut written = vec![false; desired.len()];
  let mut pending = String::new();
  let mut next_new = 0usize;
  for part in parts {
    match part {
      Part::Unmarked(text) => pending.push_str(text),
      Part::Node(loc) => match order.get(&loc.id).copied() {
        Some(ord) => {
          while next_new < ord {
            if !written[next_new] {
              cached.push(emit_new(&desired[next_new], out));
              written[next_new] = true;
            }
            next_new += 1;
          }
          out.push_str(&pending);
          pending.clear();
          cached.push(emit_update(&desired[ord], loc, existing, out));
          written[ord] = true;
          next_new = ord + 1;
        }
        None => {}
      },
    }
  }
  while next_new < desired.len() {
    if !written[next_new] {
      cached.push(emit_new(&desired[next_new], out));
      written[next_new] = true;
    }
    next_new += 1;
  }
  out.push_str(&pending);
}

fn emit_new(node: &VNode, out: &mut String) -> CachedNode {
  let (line, col) = line_col_at(out, out.len());
  match node {
    VNode::Host { id, text } => {
      out.push_str(text);
      CachedNode::host(id.clone(), line, col, text.clone())
    }
    VNode::Frame { id, children } => {
      let mut inner = Vec::new();
      for child in children {
        inner.push(emit_new(child, out));
      }
      CachedNode::frame(id.clone(), line, col, inner)
    }
  }
}

fn emit_update(desired: &VNode, loc: &Located, existing: &str, out: &mut String) -> CachedNode {
  match desired {
    VNode::Host { id, text } if !loc.frame => {
      let (line, col) = line_col_at(out, out.len());
      out.push_str(text);
      CachedNode::host(id.clone(), line, col, text.clone())
    }
    VNode::Frame { id, children } if loc.frame => {
      let (line, col) = line_col_at(out, out.len());
      let mut inner = Vec::new();
      splice_level(
        existing,
        loc.start,
        loc.end,
        &loc.children,
        children,
        out,
        &mut inner,
      );
      CachedNode::frame(id.clone(), line, col, inner)
    }
    _ => emit_new(desired, out),
  }
}
