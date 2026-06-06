//! Compatibility facade for filesystem path security policy.
//!
//! New filesystem trust-boundary ownership lives in `crate::security::filesystem`;
//! this module preserves existing `crate::path_policy::*` call sites while
//! migration proceeds.

pub(crate) use crate::security::{
    contains_parent_traversal, is_absolute_path_like, normalized_relative_path_covers,
    path_contains_parent_traversal, path_is_absolute_like, path_is_empty_like,
    path_is_normalized_tree_root,
};
