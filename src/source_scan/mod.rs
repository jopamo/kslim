//! C-family source scanning, CPP folding, and include cleanup.
//!
//! This module owns source-file scanning surfaces: preprocessor branch folding,
//! include-site indexing, include cleanup, public-header policy, and
//! private-header orphaning. Legacy `cpp.rs` and `includes.rs` modules are
//! compatibility facades over this ownership root.

pub(crate) mod cpp;
pub(crate) mod includes;
