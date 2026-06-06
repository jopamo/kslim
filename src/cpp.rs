//! Compatibility facade for C-family preprocessor cleanup.
//!
//! Source scanning and CPP folding ownership lives in `crate::source_scan`;
//! this module preserves existing `crate::cpp::*` call sites while migration proceeds.

pub(crate) use crate::source_scan::cpp::*;
