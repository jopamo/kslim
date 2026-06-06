//! Compatibility facade for generate lifecycle state.
//!
//! State ownership lives in `crate::state`; this module preserves existing
//! `super::state::*` and `crate::generate::state::*` call sites while migration
//! proceeds.

pub(crate) use crate::state::*;
