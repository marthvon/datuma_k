use std::path::Path;

use crate::dkcache::store::{CacheError, CachedNode};
use crate::dkcache::vnode::{fence_token, sanitize_id};

const BEGIN: &str = "/*@dk^";
const END: &str = "/*@dk$";
const MARK_CLOSE: &str = "@*/";
const HASH_BEGIN: &str = "# @dk^";
const HASH_END: &str = "# @dk$";
const LEGACY_BEGIN: &str = "/*@dk:begin ";
const LEGACY_END: &str = "/*@dk:end ";
const LEGACY_HASH_BEGIN: &str = "# @dk:begin ";
const LEGACY_HASH_END: &str = "# @dk:end ";

#[derive(Clone, Copy)]
struct FenceStyle {
  begin: &'static str,
  end: &'static str,
  close: &'static str,
  legacy_begin: &'static str,
  legacy_end: &'static str,
}

const BLOCK: FenceStyle = FenceStyle {
  begin: BEGIN,
  end: END,
  close: MARK_CLOSE,
  legacy_begin: LEGACY_BEGIN,
  legacy_end: LEGACY_END,
};

const HASH: FenceStyle = FenceStyle {
  begin: HASH_BEGIN,
  end: HASH_END,
  close: "",
  legacy_begin: LEGACY_HASH_BEGIN,
  legacy_end: LEGACY_HASH_END,
};

enum Part {
  Unmarked(String),
  Fenced { id: String, body: String },
}

pub fn has_fences(text: &str) -> bool {
  text.contains(BEGIN)
    || text.contains(HASH_BEGIN)
    || text.contains(LEGACY_BEGIN)
    || text.contains(LEGACY_HASH_BEGIN)
}

pub fn strip(text: &str, path: &Path) -> Result<(String, Vec<CachedNode>), CacheError> {
  let style = match path.extension().and_then(|ext| ext.to_str()) {
    Some("py") => HASH,
    _ => BLOCK,
  };
  let parts = parse_parts(text, style)?;
  let mut clean = String::new();
  let mut nodes = Vec::new();
  for part in parts {
    match part {
      Part::Unmarked(chunk) => clean.push_str(&chunk),
      Part::Fenced { id, body } => {
        let (line, col) = super::reconcile::line_col_at(&clean, clean.len());
        clean.push_str(&body);
        nodes.push(CachedNode::host(id, line, col, body));
      }
    }
  }
  Ok((clean, nodes))
}

fn take_id<'a>(after_begin: &'a str, style: FenceStyle) -> Result<(&'a str, &'a str), CacheError> {
  if style.close.is_empty() {
    match after_begin.find('\n') {
      Some(at) => Ok((&after_begin[..at], &after_begin[at + 1..])),
      None => Err(CacheError::Malformed("unclosed begin fence".into())),
    }
  } else {
    match after_begin.find(style.close) {
      Some(at) => {
        let after_id = &after_begin[at + style.close.len()..];
        if after_id.starts_with('\n') {
          Ok((&after_begin[..at], &after_id[1..]))
        } else {
          Ok((&after_begin[..at], after_id))
        }
      }
      None => Err(CacheError::Malformed("unclosed begin fence".into())),
    }
  }
}

fn next_begin(rest: &str, style: FenceStyle) -> Option<(usize, bool)> {
  match (rest.find(style.begin), rest.find(style.legacy_begin)) {
    (None, None) => None,
    (Some(at), None) => Some((at, false)),
    (None, Some(at)) => Some((at, true)),
    (Some(fresh), Some(legacy)) => {
      if fresh <= legacy {
        Some((fresh, false))
      } else {
        Some((legacy, true))
      }
    }
  }
}

fn parse_parts(text: &str, style: FenceStyle) -> Result<Vec<Part>, CacheError> {
  let mut parts = Vec::new();
  let mut rest = text;
  loop {
    match next_begin(rest, style) {
      None => {
        if !rest.is_empty() {
          parts.push(Part::Unmarked(rest.to_string()));
        }
        break Ok(parts);
      }
      Some((at, legacy)) => {
        if at > 0 {
          parts.push(Part::Unmarked(rest[..at].to_string()));
        }
        let begin = if legacy {
          style.legacy_begin
        } else {
          style.begin
        };
        let end = if legacy { style.legacy_end } else { style.end };
        let after_begin = &rest[at + begin.len()..];
        let (raw_id, body_start) = match take_id(after_begin, style) {
          Ok(pair) => pair,
          Err(err) => break Err(err),
        };
        let end_mark = format!("{}{}{}", end, raw_id, style.close);
        let Some(end_at) = body_start.find(&end_mark) else {
          break Err(CacheError::Malformed(format!(
            "missing end fence for {raw_id}"
          )));
        };
        let id = if legacy {
          let identity = raw_id
            .split_once("::")
            .map(|(_, rest)| rest)
            .unwrap_or(raw_id);
          fence_token(&sanitize_id(identity))
        } else {
          sanitize_id(raw_id)
        };
        parts.push(Part::Fenced {
          id,
          body: body_start[..end_at].to_string(),
        });
        rest = &body_start[end_at + end_mark.len()..];
        if rest.starts_with('\n') {
          rest = &rest[1..];
        }
      }
    }
  }
}
