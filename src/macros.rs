#[macro_export]
macro_rules! mod_use {
  ($name:ident) => {
    mod $name;
    #[allow(unused_imports)]
    pub use $name::*;
  };
}

#[macro_export]
macro_rules! mod_use_in {
  ($in:ident, $name:ident) => {
    mod $name;
    #[allow(unused_imports)]
    pub(in crate::$in) use $name::*;
  };
}

#[macro_export]
macro_rules! pub_mod_use {
  ($name:ident) => {
    pub mod $name;
    pub use $name::*;
  };
}

#[macro_export]
macro_rules! pub_mod_use_in {
  ($in:ident, $name:ident) => {
    pub(in crate::$in) mod $name;
    pub(in crate::$in) use $name::*;
  };
}
