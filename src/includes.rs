//! Compatibility facade for C-family include cleanup.
//!
//! Include indexing, cleanup, public-header policy, and private-header orphaning
//! ownership lives in `crate::source_scan`; this module preserves existing
//! `crate::includes::*` call sites while migration proceeds.

pub(crate) use crate::source_scan::includes::*;
