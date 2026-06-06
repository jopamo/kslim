//! Orphan cleanup after declared path pruning.
//!
//! This module removes empty parent directories orphaned by manifest-backed path
//! deletion while preserving named-feature roots and ABI-sensitive paths without
//! exact manifest truth.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::edit_reason::{
    ensure_edit_records_for_mutation, EditProofSource, EditReason, EditRecord,
};

use super::path::abi_sensitive_path_requires_exact_manifest_truth;

pub(in crate::prune) struct EmptyParentCleanup {
    pub(in crate::prune) dirs_removed: usize,
    pub(in crate::prune) empty_parents_cleaned: Vec<PathBuf>,
    pub(in crate::prune) edits: Vec<EditRecord>,
}

fn path_intersects_preserved_roots(path: &Path, preserved_paths: &[PathBuf]) -> bool {
    preserved_paths.iter().any(|preserved| {
        crate::path_policy::normalized_relative_path_covers(preserved, path)
            || crate::path_policy::normalized_relative_path_covers(path, preserved)
    })
}

pub(in crate::prune) fn cleanup_empty_parent_chain(
    dir: &Path,
    root: &Path,
    manifest_path: &Path,
    manifest_paths: &[PathBuf],
    preserved_paths: &[PathBuf],
) -> Result<EmptyParentCleanup> {
    let mut current = dir.to_path_buf();
    let mut dirs_removed = 0usize;
    let mut empty_parents_cleaned = Vec::new();
    let mut edits = Vec::new();

    while current.starts_with(root) && current != root {
        if !current.exists() {
            current = current.parent().unwrap_or(root).to_path_buf();
            continue;
        }
        let mut entries = std::fs::read_dir(&current)?;
        if entries.next().is_none() {
            let relative = current.strip_prefix(root).unwrap_or(&current).to_path_buf();
            if path_intersects_preserved_roots(&relative, preserved_paths) {
                break;
            }
            if abi_sensitive_path_requires_exact_manifest_truth(&relative, manifest_paths) {
                break;
            }
            std::fs::remove_dir(&current)?;
            dirs_removed += 1;
            log::debug!("prune: cleaned up empty parent dir: {}", current.display());
            empty_parents_cleaned.push(relative.clone());
            edits.push(EditRecord::new(
                relative,
                None,
                String::from("<empty directory>"),
                String::new(),
                EditReason::ManifestPath {
                    path: manifest_path.to_path_buf(),
                },
                EditProofSource::removal_manifest_path(manifest_path.to_path_buf()),
                "prune.cleanup_empty_parents",
            ));
            current = current.parent().unwrap_or(root).to_path_buf();
        } else {
            break;
        }
    }

    ensure_edit_records_for_mutation("prune.cleanup_empty_parents", dirs_removed, &edits)?;

    Ok(EmptyParentCleanup {
        dirs_removed,
        empty_parents_cleaned,
        edits,
    })
}
