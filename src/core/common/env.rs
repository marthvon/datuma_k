use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

pub const ROOT_DIRECTORY: &str = "ROOT_DIRECTORY";
pub const DTCT_DIRECTORY: &str = "DTCT_DIRECTORY";
pub const NGIN_DIRECTORY: &str = "NGIN_DIRECTORY";
pub const DEF_DIRECTORY: &str = "DEF_DIRECTORY";

#[derive(Debug, Clone, Default)]
pub struct EnvMap {
  vars: HashMap<String, String>,
}

impl EnvMap {
  pub fn load() -> Self {
    match std::env::current_dir() {
      Ok(cwd) => Self::load_from(&cwd),
      Err(_) => Self::from_vars(std::env::vars()),
    }
  }

  pub fn load_from(dir: &Path) -> Self {
    let mut vars: HashMap<String, String> = std::env::vars().collect();
    if let Ok(iter) = dotenvy::from_path_iter(dir.join(".env")) {
      for item in iter.flatten() {
        vars.insert(item.0, item.1);
      }
    }
    Self::from_vars(vars)
  }

  pub fn empty() -> Self {
    Self::default()
  }

  pub fn from_vars(vars: impl IntoIterator<Item = (String, String)>) -> Self {
    let mut map = HashMap::new();
    for (key, value) in vars {
      let trimmed = value.trim();
      if !trimmed.is_empty() {
        map.insert(key, trimmed.to_string());
      }
    }
    Self { vars: map }
  }

  pub fn get(&self, name: &str) -> Option<&str> {
    self.vars.get(name).map(String::as_str)
  }

  pub fn optional(&self, name: &str) -> Option<&str> {
    self.get(name)
  }

  pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
    self.vars.iter().map(|(key, value)| (key.as_str(), value.as_str()))
  }

  pub fn root_dir(&self, cwd: &Path) -> PathBuf {
    match self.get(ROOT_DIRECTORY) {
      Some(path) => join_base(cwd, Path::new(path)),
      None => cwd.to_path_buf(),
    }
  }

  pub fn resolve_dir(&self, cwd: &Path, key: &str) -> PathBuf {
    match self.get(key) {
      Some(path) => join_base(&self.root_dir(cwd), Path::new(path)),
      None => self.root_dir(cwd),
    }
  }

  pub fn dtct_dir(&self, cwd: &Path) -> PathBuf {
    self.resolve_dir(cwd, DTCT_DIRECTORY)
  }

  pub fn ngin_dir(&self, cwd: &Path) -> PathBuf {
    self.resolve_dir(cwd, NGIN_DIRECTORY)
  }

  pub fn def_dir(&self, cwd: &Path) -> PathBuf {
    self.resolve_dir(cwd, DEF_DIRECTORY)
  }
}

fn join_base(base: &Path, path: &Path) -> PathBuf {
  if path.is_absolute() {
    path.to_path_buf()
  } else if path.as_os_str().is_empty() || path == Path::new(".") {
    base.to_path_buf()
  } else {
    base.join(path)
  }
}

pub fn is_dangerous_dir(path: &Path) -> bool {
  let expanded = expand_tilde(path);
  let canon = match expanded.canonicalize() {
    Ok(path) => path,
    Err(_) => expanded,
  };
  if canon == Path::new("/") {
    true
  } else if let Some(home) = std::env::var_os("HOME") {
    let home = PathBuf::from(home);
    let home = match home.canonicalize() {
      Ok(path) => path,
      Err(_) => home,
    };
    if canon == home {
      true
    } else if let Some(parent) = home.parent() {
      canon == parent
    } else {
      false
    }
  } else {
    canon == Path::new("/Users") || canon == Path::new("/home")
  }
}

fn expand_tilde(path: &Path) -> PathBuf {
  let mut comps = path.components();
  match comps.next() {
    Some(Component::Normal(first)) if first == "~" => match std::env::var_os("HOME") {
      Some(home) => PathBuf::from(home).join(comps.as_path()),
      None => path.to_path_buf(),
    },
    _ => path.to_path_buf(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn trims_and_drops_empty() {
    let env = EnvMap::from_vars([
      ("A".into(), "  x  ".into()),
      ("B".into(), "   ".into()),
      ("C".into(), "y".into()),
    ]);
    assert_eq!(env.get("A"), Some("x"));
    assert_eq!(env.optional("B"), None);
    assert_eq!(env.optional("C"), Some("y"));
  }

  #[test]
  fn unset_dir_falls_back_to_root() {
    let env = EnvMap::from_vars([(ROOT_DIRECTORY.into(), "/tmp/project".into())]);
    let cwd = Path::new("/unused");
    assert_eq!(env.resolve_dir(cwd, DTCT_DIRECTORY), PathBuf::from("/tmp/project"));
    assert_eq!(env.dtct_dir(cwd), PathBuf::from("/tmp/project"));
  }

  #[test]
  fn relative_dir_joins_root() {
    let env = EnvMap::from_vars([
      (ROOT_DIRECTORY.into(), "/tmp/project".into()),
      (DTCT_DIRECTORY.into(), "data".into()),
    ]);
    assert_eq!(env.dtct_dir(Path::new("/unused")), PathBuf::from("/tmp/project/data"));
  }

  #[test]
  fn relative_root_uses_cwd() {
    let env = EnvMap::from_vars([
      (ROOT_DIRECTORY.into(), ".".into()),
      (DTCT_DIRECTORY.into(), "data".into()),
    ]);
    assert_eq!(env.dtct_dir(Path::new("/proj")), PathBuf::from("/proj/data"));
  }

  #[test]
  fn slash_is_dangerous() {
    assert!(is_dangerous_dir(Path::new("/")));
  }

  #[test]
  fn nested_project_is_not_dangerous() {
    assert!(!is_dangerous_dir(Path::new("/tmp/datuma-k-not-home")));
  }

  #[test]
  fn tilde_home_is_dangerous() {
    if std::env::var_os("HOME").is_some() {
      assert!(is_dangerous_dir(Path::new("~")));
    }
  }
}
