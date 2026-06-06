//! Path normalization helpers for lifecycle path wrappers.

use anyhow::{Context, Result};
use std::path::{Component, Path, PathBuf};

use crate::path_policy::path_is_empty_like;

use super::traversal::{parent_traversal_error, reject_parent_traversal};

pub(crate) fn reject_empty_path(label: &str, path: &Path) -> Result<()> {
    if path_is_empty_like(path) {
        anyhow::bail!("{label} is empty");
    }
    Ok(())
}

pub(crate) fn normalize_path_without_parent_components(
    label: &str,
    path: &Path,
) -> Result<PathBuf> {
    reject_empty_path(label, path)?;
    reject_parent_traversal(label, path)?;

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                normalized.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                return Err(parent_traversal_error(label, path));
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        Ok(PathBuf::from("."))
    } else {
        Ok(normalized)
    }
}

pub(crate) fn canonicalize_existing_path(label: &str, path: &Path) -> Result<PathBuf> {
    path.canonicalize()
        .with_context(|| format!("failed to normalize {label}: {}", path.display()))
}
