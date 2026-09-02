use std::fs;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;

use crate::dkcache::migrate::{has_fences, strip};
use crate::dkcache::reconcile;
use crate::dkcache::store::{CACHE_FILE, CacheError, DirCache, FileCache, read_cache, write_cache};
use crate::dkcache::vnode::VNode;

const LEGACY_SIDECAR: &str = ".datuma-ngin-paths";

pub fn commit(
  root_directory: &str,
  files: &IndexMap<String, Vec<VNode>>,
) -> Result<(), CacheError> {
  let root = Path::new(root_directory);
  let root_cache_path = root.join(CACHE_FILE);
  let root_cache = read_cache(&root_cache_path)?;
  let sidecar = root.join(LEGACY_SIDECAR);
  let prior_sidecar = if root_cache.files.is_empty() && root_cache.dirs.is_empty() {
    match fs::read_to_string(&sidecar) {
      Ok(text) => text
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>(),
      Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
      Err(err) => return Err(CacheError::Io(err)),
    }
  } else {
    Vec::new()
  };

  let mut groups: IndexMap<PathBuf, IndexMap<String, &Vec<VNode>>> = IndexMap::new();
  for (path_str, tree) in files {
    let path = Path::new(path_str);
    let parent = path
      .parent()
      .map(Path::to_path_buf)
      .unwrap_or_else(|| root.to_path_buf());
    if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
      groups
        .entry(parent)
        .or_default()
        .insert(name.to_string(), tree);
    } else {
      return Err(CacheError::Malformed(format!(
        "invalid output path {path_str}"
      )));
    }
  }

  let mut planned_dirs = groups
    .keys()
    .filter(|dir| dir.as_path() != root)
    .map(|dir| rel_dir(root, dir))
    .collect::<Vec<_>>();
  planned_dirs.sort();

  for path_str in &prior_sidecar {
    if !files.contains_key(path_str) {
      let path = Path::new(path_str);
      if path.exists() {
        fs::remove_file(path)?;
      }
    }
  }
  if sidecar.exists() {
    fs::remove_file(&sidecar)?;
  }

  for rel in &root_cache.dirs {
    let dir = root.join(rel);
    if !groups.contains_key(&dir) {
      unmount_dir(&dir)?;
    }
  }

  let vanished_root: Vec<String> = {
    let planned_root = groups.get(root);
    root_cache
      .files
      .keys()
      .filter(|name| !planned_root.is_some_and(|group| group.contains_key(*name)))
      .cloned()
      .collect()
  };
  for name in &vanished_root {
    let path = root.join(name);
    if path.exists() {
      fs::remove_file(&path)?;
    }
  }

  let mut new_root = DirCache {
    version: 1,
    files: root_cache.files,
    dirs: planned_dirs,
  };
  for name in vanished_root {
    new_root.files.shift_remove(&name);
  }

  for (dir, group) in &groups {
    if dir.as_path() == root {
      apply_group(dir, group, &mut new_root)?;
    } else {
      let cache_path = dir.join(CACHE_FILE);
      let mut dir_cache = read_cache(&cache_path)?;
      let stale = dir_cache
        .files
        .keys()
        .filter(|name| !group.contains_key(*name))
        .cloned()
        .collect::<Vec<_>>();
      for name in stale {
        let path = dir.join(&name);
        if path.exists() {
          fs::remove_file(&path)?;
        }
        dir_cache.files.shift_remove(&name);
      }
      apply_group(dir, group, &mut dir_cache)?;
      dir_cache.dirs.clear();
      write_cache(&cache_path, &dir_cache)?;
    }
  }

  write_cache(&root_cache_path, &new_root)
}

fn unmount_dir(dir: &Path) -> Result<(), CacheError> {
  let cache_path = dir.join(CACHE_FILE);
  let old = read_cache(&cache_path)?;
  for name in old.files.keys() {
    let path = dir.join(name);
    if path.exists() {
      fs::remove_file(path)?;
    }
  }
  if cache_path.exists() {
    fs::remove_file(&cache_path)?;
  }
  Ok(())
}

fn apply_group(
  dir: &Path,
  group: &IndexMap<String, &Vec<VNode>>,
  cache: &mut DirCache,
) -> Result<(), CacheError> {
  for (name, tree) in group {
    let path = dir.join(name);
    if tree.is_empty() {
      if path.exists() {
        fs::remove_file(&path)?;
      }
      cache.files.shift_remove(name);
    } else {
      apply_file(&path, name, tree, cache)?;
    }
  }
  Ok(())
}

fn apply_file(
  path: &Path,
  name: &str,
  desired: &[VNode],
  cache: &mut DirCache,
) -> Result<(), CacheError> {
  let on_disk = match fs::read_to_string(path) {
    Ok(text) => text,
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
    Err(err) => return Err(CacheError::Io(err)),
  };
  let cached = cache
    .files
    .get(name)
    .map(|file| file.tree.clone())
    .unwrap_or_default();
  let (existing, cached) = if cached.is_empty() && has_fences(&on_disk) {
    strip(&on_disk, path)?
  } else {
    (on_disk, cached)
  };
  let (rendered, tree) = reconcile::run(&existing, &cached, desired);
  let unchanged = path.exists()
    && fs::read_to_string(path)
      .ok()
      .is_some_and(|old| old == rendered);
  if !unchanged {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::write(path, &rendered)?;
  }
  cache.files.insert(name.to_string(), FileCache { tree });
  Ok(())
}

fn rel_dir(root: &Path, dir: &Path) -> String {
  match dir.strip_prefix(root) {
    Ok(rel) if rel.as_os_str().is_empty() => ".".into(),
    Ok(rel) => rel.to_string_lossy().into_owned(),
    Err(_) => dir.to_string_lossy().into_owned(),
  }
}
