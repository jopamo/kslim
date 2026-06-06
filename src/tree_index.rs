//! Compatibility facade for read-only kernel tree indexes.
//!
//! Index ownership lives in `crate::index`; this module preserves existing
//! `crate::tree_index::*` call sites while migration proceeds.

pub(crate) use crate::index::*;
