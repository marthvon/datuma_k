use std::fs;
use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub const CACHE_FILE: &str = ".dkcache";

#[derive(Debug)]
pub enum CacheError {
  Io(std::io::Error),
  Malformed(String),
}

impl std::fmt::Display for CacheError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Io(err) => write!(f, "{err}"),
      Self::Malformed(text) => write!(f, "{text}"),
    }
  }
}

impl std::error::Error for CacheError {}

impl From<std::io::Error> for CacheError {
  fn from(err: std::io::Error) -> Self {
    Self::Io(err)
  }
}

impl From<serde_json::Error> for CacheError {
  fn from(err: serde_json::Error) -> Self {
    Self::Malformed(err.to_string())
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirCache {
  pub version: u32,
  #[serde(default)]
  pub files: IndexMap<String, FileCache>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub dirs: Vec<String>,
}

impl DirCache {
  pub fn empty() -> Self {
    Self {
      version: 1,
      files: IndexMap::new(),
      dirs: Vec::new(),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileCache {
  pub tree: Vec<CachedNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedNode {
  pub id: String,
  pub line: usize,
  pub col: usize,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub text: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub children: Option<Vec<CachedNode>>,
}

impl CachedNode {
  pub fn host(id: String, line: usize, col: usize, text: String) -> Self {
    Self {
      id,
      line,
      col,
      text: Some(text),
      children: None,
    }
  }

  pub fn frame(id: String, line: usize, col: usize, children: Vec<CachedNode>) -> Self {
    Self {
      id,
      line,
      col,
      text: None,
      children: Some(children),
    }
  }

  pub fn is_frame(&self) -> bool {
    self.children.is_some()
  }
}

pub fn read_cache(path: &Path) -> Result<DirCache, CacheError> {
  match fs::read_to_string(path) {
    Ok(text) => {
      let cache: DirCache = serde_json::from_str(&text)?;
      if cache.version != 1 {
        Err(CacheError::Malformed(format!(
          "unsupported .dkcache version {}",
          cache.version
        )))
      } else {
        Ok(cache)
      }
    }
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(DirCache::empty()),
    Err(err) => Err(CacheError::Io(err)),
  }
}

pub fn write_cache(path: &Path, cache: &DirCache) -> Result<(), CacheError> {
  if cache.files.is_empty() && cache.dirs.is_empty() {
    if path.exists() {
      fs::remove_file(path)?;
    }
    Ok(())
  } else if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)?;
    let mut encoded = serde_json::to_string_pretty(cache)?;
    encoded.push('\n');
    fs::write(path, encoded)?;
    Ok(())
  } else {
    let mut encoded = serde_json::to_string_pretty(cache)?;
    encoded.push('\n');
    fs::write(path, encoded)?;
    Ok(())
  }
}
