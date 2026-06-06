use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use crate::abi::{self, AbiPolicyConfig};
use crate::path_policy::{contains_parent_traversal, is_absolute_path_like};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NormalizedDeclaredPath {
    pub(super) path: PathBuf,
    pub(super) declared_directory: bool,
}

#[allow(dead_code)]
pub(crate) fn is_uapi_header_path(path: &Path) -> bool {
    abi::is_uapi_header_path(path)
}

pub(crate) fn is_uapi_path(path: &Path) -> bool {
    abi::is_uapi_path(path)
}

pub(crate) fn is_public_header_path(path: &Path) -> bool {
    abi::is_public_header_path(path)
}

pub(super) fn validate_declared_abi_removal_policy(
    path: &Path,
    abi_policy: &AbiPolicyConfig,
) -> Result<()> {
    abi::validate_declared_removal(path, abi_policy)
}

pub(super) fn abi_sensitive_path_requires_own_manifest_truth(parent: &Path, path: &Path) -> bool {
    path != parent && (is_public_header_path(path) || is_uapi_path(path))
}

pub(super) fn normalize_declared_path(
    raw_path: &str,
    unsafe_allow_root_path_removal: bool,
) -> Result<NormalizedDeclaredPath> {
    if raw_path.trim().is_empty() {
        anyhow::bail!("slim.remove_paths must not contain empty values");
    }
    let mut declared_directory = raw_path.ends_with('/');

    let path = Path::new(raw_path);
    if is_absolute_path_like(raw_path) {
        anyhow::bail!("declared removal paths must be relative: {raw_path}");
    }
    if contains_parent_traversal(raw_path) {
        anyhow::bail!("declared removal paths must not contain '..': {raw_path}");
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                anyhow::bail!("declared removal paths must not contain '..': {raw_path}")
            }
            Component::RootDir | Component::Prefix(_) => {
                anyhow::bail!("declared removal paths must be relative: {raw_path}")
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        if !unsafe_allow_root_path_removal {
            anyhow::bail!(
                "declared removal paths must not resolve to the tree root without an explicit unsafe override; set slim.unsafe_allow_root_path_removal = true: {raw_path}"
            );
        }
        normalized.push(".");
        declared_directory = true;
    }

    Ok(NormalizedDeclaredPath {
        path: normalized,
        declared_directory,
    })
}

pub(super) fn derive_removed_path_categories(
    root: Option<&Path>,
    removed_paths: &BTreeSet<PathBuf>,
    declared_dirs: &BTreeSet<PathBuf>,
) -> Result<(BTreeSet<PathBuf>, BTreeSet<PathBuf>)> {
    let mut removed_dirs = BTreeSet::new();
    let mut removed_files = BTreeSet::new();

    for path in removed_paths {
        let declared_directory = declared_dirs.contains(path);
        match root {
            Some(root) => {
                let target = root.join(path);
                match std::fs::symlink_metadata(&target) {
                    Ok(metadata) if metadata.file_type().is_dir() => {
                        removed_dirs.insert(path.clone());
                    }
                    Ok(_) if declared_directory => {
                        anyhow::bail!(
                            "declared directory removal path exists but is not a directory: {}",
                            path.display()
                        );
                    }
                    Ok(_) => {
                        removed_files.insert(path.clone());
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                        if declared_directory {
                            removed_dirs.insert(path.clone());
                        }
                    }
                    Err(err) => {
                        return Err(err).with_context(|| {
                            format!(
                                "failed to classify declared removal path {}",
                                path.display()
                            )
                        });
                    }
                }
            }
            None if declared_directory => {
                removed_dirs.insert(path.clone());
            }
            None => {}
        }
    }

    Ok((removed_dirs, removed_files))
}
