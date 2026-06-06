//! File and path indexing primitives for the read-only tree index.
//!
//! This module owns deterministic file discovery, header classification, and
//! normalized relative candidate-tree paths. Domain indexes consume these paths
//! instead of repeating traversal or path-boundary checks.

use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

pub type FileIndex = BTreeSet<PathBuf>;
pub type HeaderIndex = BTreeSet<PathBuf>;

pub(in crate::index) fn indexed_tree_files(root: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut files = BTreeMap::new();
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let absolute = entry.path().to_path_buf();
        let relative = relative_path_under_root(root, &absolute)?;
        files.insert(relative, absolute);
    }
    Ok(files.into_iter().collect())
}

pub(in crate::index) fn is_header_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "h")
}

pub(in crate::index) fn normalize_touched_paths(
    root: &Path,
    touched: &[PathBuf],
) -> Result<Vec<PathBuf>> {
    let mut out = BTreeSet::new();
    for path in touched {
        let relative = if path.is_absolute() {
            relative_path_under_root(root, path)?
        } else {
            ensure_relative_input_path(path)?;
            let relative = normalize_index_path(path);
            ensure_relative_index_path(&relative)?;
            relative
        };
        out.insert(relative);
    }
    Ok(out.into_iter().collect())
}

pub(in crate::index) fn existing_touched_files(
    root: &Path,
    touched: &[PathBuf],
) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut files = BTreeMap::new();
    for relative in touched {
        let path = root.join(relative);
        if path.is_file() {
            files.insert(relative.clone(), path);
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&path) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let absolute = entry.path().to_path_buf();
            let relative = relative_path_under_root(root, &absolute)?;
            files.insert(relative, absolute);
        }
    }
    Ok(files.into_iter().collect())
}

pub(in crate::index) fn relative_path_under_root(root: &Path, path: &Path) -> Result<PathBuf> {
    let root = normalize_index_path(root);
    let path = normalize_index_path(path);
    let Ok(relative) = path.strip_prefix(&root) else {
        anyhow::bail!("tree index path escaped candidate root");
    };
    ensure_relative_index_path(relative)?;
    Ok(relative.to_path_buf())
}

pub(in crate::index) fn normalize_relative_to_root(root: &Path, path: &Path) -> Option<PathBuf> {
    let root = normalize_index_path(root);
    let path = normalize_index_path(path);
    let relative = path.strip_prefix(&root).ok()?;
    is_relative_index_path(relative).then(|| relative.to_path_buf())
}

fn normalize_index_path(path: &Path) -> PathBuf {
    crate::kbuild::normalize_relative(path)
}

pub(in crate::index) fn ensure_relative_index_path(path: &Path) -> Result<()> {
    if !is_relative_index_path(path) {
        anyhow::bail!("tree index contains a non-relative candidate path");
    }
    Ok(())
}

fn ensure_relative_input_path(path: &Path) -> Result<()> {
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        anyhow::bail!("tree index touched path is not relative to candidate root");
    }
    Ok(())
}

pub(in crate::index) fn is_relative_index_path(path: &Path) -> bool {
    !path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

pub(in crate::index) fn index_path_is_under(path: &Path, base: &Path) -> bool {
    base.as_os_str().is_empty() || path == base || path.starts_with(base)
}

pub(in crate::index) fn ensure_index_text_not_host_absolute_path(value: &str) -> Result<()> {
    if is_host_absolute_path_like(value) {
        anyhow::bail!("tree index contains a host absolute path literal");
    }
    Ok(())
}

pub(in crate::index) fn is_host_absolute_path_like(value: &str) -> bool {
    Path::new(value).is_absolute()
        || value.starts_with("file:")
        || is_windows_absolute_path_like(value)
}

fn is_windows_absolute_path_like(value: &str) -> bool {
    let bytes = value.as_bytes();
    if value.starts_with("\\\\") {
        return true;
    }
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}
