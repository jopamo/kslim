//! Compatibility facade for process execution helpers.
//!
//! New process execution ownership lives in `crate::execution`; this module
//! preserves existing `crate::process::*` call sites while migration proceeds.

pub(crate) use crate::execution::{run, run_in_dir};
#[allow(unused_imports)]
pub(crate) use crate::execution::run_quiet;
