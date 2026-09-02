use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn files_from_dir(path_str: &str, filter_extensions: &[&str]) -> Vec<PathBuf> {
  let path = PathBuf::from(path_str);
  let extensions: HashSet<&str> = filter_extensions.iter().copied().collect();
  WalkDir::new(path)
    .into_iter()
    .filter_map(|result| {
      if let Some(dir_entry) = result.ok()
        && dir_entry.file_type().is_file()
        && let file_path = dir_entry.into_path()
        && let Some(ext_os_str) = file_path.extension()
        && let Some(ext) = ext_os_str.to_str()
        && extensions.contains(ext)
      {
        Some(file_path)
      } else {
        None
      }
    })
    .collect()
}

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
