use std::sync::Arc;

use crate::core::exec::error::RuntimeErrorKind;
use crate::core::exec::value::{MemberHost, RuntimeValue};

pub fn property(host: &Arc<dyn MemberHost>, name: &str) -> Result<RuntimeValue, RuntimeErrorKind> {
  host.property(name)
}

pub fn call(
  host: &Arc<dyn MemberHost>,
  name: &str,
  args: Vec<RuntimeValue>,
) -> Result<RuntimeValue, RuntimeErrorKind> {
  host.call(name, args)
}
