//! Compatibility facade for immutable generate plans.
//!
//! Plan ownership lives in `crate::plan`; this module preserves existing
//! `super::plan::*` and `crate::generate::plan::*` call sites while migration
//! proceeds.

pub(crate) use crate::plan::*;
