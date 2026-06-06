//! Rejection helpers for path traversal and host-root path escapes.

use anyhow::Result;
use std::path::Path;

use crate::path_policy::{path_contains_parent_traversal, path_is_absolute_like};

pub(crate) fn reject_parent_traversal(label: &str, path: &Path) -> Result<()> {
    if path_contains_parent_traversal(path) {
        return Err(parent_traversal_error(label, path));
    }
    Ok(())
}

pub(crate) fn parent_traversal_error(label: &str, path: &Path) -> anyhow::Error {
    anyhow::anyhow!(
        "{label} must not contain parent components: {}",
        path.display()
    )
}

pub(crate) fn reject_absolute_like_relative_kernel_path(path: &Path) -> Result<()> {
    if path_is_absolute_like(path) {
        anyhow::bail!(
            "relative kernel path must be relative to the kernel tree: {}",
            path.display()
        );
    }
    Ok(())
}
