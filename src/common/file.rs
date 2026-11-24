use std::collections::HashSet;
use std::path::{Path, PathBuf};

use mvparser::common::files_from_dir;

#[derive(Debug, Default)]
pub struct DirFiles {
  pub dtct: Vec<PathBuf>,
  pub def_ngin: Vec<PathBuf>,
  pub ngin: Vec<PathBuf>,
}

pub fn partition_dir_files(paths: impl IntoIterator<Item = PathBuf>) -> DirFiles {
  let mut files = DirFiles::default();
  for path in paths {
    if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
      if name.ends_with(".def.ngin") {
        files.def_ngin.push(path);
      } else if name.ends_with(".ngin") {
        files.ngin.push(path);
      } else if name.ends_with(".dtct") {
        files.dtct.push(path);
      }
    }
  }
  files.dtct.sort();
  files.def_ngin.sort();
  files.ngin.sort();
  files
}

pub fn collect_unique_dir_files(dirs: &[&Path]) -> DirFiles {
  let mut seen = HashSet::new();
  let mut paths = Vec::new();
  for dir in dirs {
    if dir.is_dir() {
      let key = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
      if seen.insert(key) {
        paths.extend(files_from_dir(
          dir.to_str().unwrap_or_default(),
          &["dtct", "ngin"],
        ));
      }
    }
  }
  partition_dir_files(paths)
}
