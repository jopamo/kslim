//! Declared path pruning and removal accounting.
//!
//! This module owns manifest-backed filesystem removal, preservation checks,
//! failed-removal accounting, and edit records emitted for path deletion.
//! Orphan cleanup removes empty parent directories after declared paths vanish.

use anyhow::Result;
use std::io;
use std::path::{Path, PathBuf};

use crate::edit_reason::{
    ensure_edit_records_for_mutation, sort_edit_records, EditProofSource, EditReason, EditRecord,
};
use crate::path_policy::{
    path_contains_parent_traversal, path_is_absolute_like, path_is_normalized_tree_root,
};
use crate::removal_manifest::RemovalManifest;

use super::{cleanup_empty_parent_chain, effective_removed_config_symbols_for_abi_policy};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RemovalAccounting {
    pub removed_files: Vec<PathBuf>,
    pub removed_dirs: Vec<PathBuf>,
    pub removed_config_symbols: Vec<String>,
    pub empty_parents_cleaned: Vec<PathBuf>,
    pub missing_paths: Vec<PathBuf>,
}

impl RemovalAccounting {
    fn normalize_and_sort_lists(&mut self) {
        normalize_and_sort_paths(&mut self.removed_files);
        normalize_and_sort_paths(&mut self.removed_dirs);
        normalize_and_sort_paths(&mut self.empty_parents_cleaned);
        normalize_and_sort_paths(&mut self.missing_paths);
        normalize_and_sort_symbols(&mut self.removed_config_symbols);
    }
}

#[derive(Debug, Clone)]
pub struct DeclaredPathPruneResult {
    pub files_removed: usize,
    pub dirs_removed: usize,
    pub(crate) removed_artifacts: Vec<RemovedArtifact>,
    pub removal: RemovalAccounting,
    pub edits: Vec<EditRecord>,
    pub result: PruneResult,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PruneResult {
    pub removed: Vec<PrunedPath>,
    pub failed: Vec<FailedRemoval>,
    pub edits: Vec<EditRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PrunedPath {
    pub path: PathBuf,
    pub is_dir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FailedRemoval {
    pub path: PathBuf,
    pub kind: FailedRemovalKind,
    pub reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailedRemovalKind {
    MissingPath,
    PermissionDenied,
    EscapedRoot,
    UnsupportedSpecialFile,
    IoError,
}

impl FailedRemovalKind {
    pub(in crate::prune) fn stable_name(self) -> &'static str {
        match self {
            Self::MissingPath => "missing_path",
            Self::PermissionDenied => "permission_denied",
            Self::EscapedRoot => "escaped_root",
            Self::UnsupportedSpecialFile => "unsupported_special_file",
            Self::IoError => "io_error",
        }
    }
}

impl FailedRemoval {
    pub(in crate::prune) fn new(
        path: PathBuf,
        kind: FailedRemovalKind,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            path,
            kind,
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemovalFailurePolicy {
    pub fail_on_missing_path: bool,
    pub strict_permission_failures: bool,
    pub ignore_unsupported_special_files: bool,
}

impl Default for RemovalFailurePolicy {
    fn default() -> Self {
        Self {
            fail_on_missing_path: false,
            strict_permission_failures: true,
            ignore_unsupported_special_files: false,
        }
    }
}

impl RemovalFailurePolicy {
    pub fn from_reducer_config(config: &crate::config::ReducerConfig) -> Self {
        Self {
            fail_on_missing_path: config.fail_on_missing_prune_paths,
            strict_permission_failures: config.strict_mode(),
            ignore_unsupported_special_files: config.ignore_unsupported_special_removals,
        }
    }

    pub(in crate::prune) fn is_fatal(self, kind: FailedRemovalKind) -> bool {
        match kind {
            FailedRemovalKind::MissingPath => self.fail_on_missing_path,
            FailedRemovalKind::PermissionDenied => self.strict_permission_failures,
            FailedRemovalKind::EscapedRoot => true,
            FailedRemovalKind::UnsupportedSpecialFile => !self.ignore_unsupported_special_files,
            FailedRemovalKind::IoError => true,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RemovedArtifact {
    pub relative: PathBuf,
    pub is_dir: bool,
}

#[allow(dead_code)]
pub(crate) fn prune_declared_paths_from_manifest(
    root: &Path,
    manifest: &RemovalManifest,
) -> Result<DeclaredPathPruneResult> {
    prune_declared_paths_from_manifest_with_policy(root, manifest, RemovalFailurePolicy::default())
}

pub(crate) fn prune_declared_paths_from_manifest_with_policy(
    root: &Path,
    manifest: &RemovalManifest,
    policy: RemovalFailurePolicy,
) -> Result<DeclaredPathPruneResult> {
    let removed_paths = manifest.removed_paths_vec();
    if removed_paths
        .iter()
        .any(|path| path_is_normalized_tree_root(path))
        && !manifest.unsafe_allow_root_path_removal
    {
        anyhow::bail!(
            "declared removal paths must not resolve to the tree root without an explicit unsafe override"
        );
    }
    for path in &removed_paths {
        crate::abi::validate_declared_removal(path, &manifest.abi_policy)?;
    }
    let effective_removed_config_symbols =
        effective_removed_config_symbols_for_abi_policy(root, manifest)?;
    let preserved_paths = manifest.preserved_paths_vec();
    let mut declared =
        prune_declared_paths_with_preservation(root, &removed_paths, &preserved_paths, policy)?;
    for symbol in &effective_removed_config_symbols {
        if !declared.removal.removed_config_symbols.contains(symbol) {
            declared.removal.removed_config_symbols.push(symbol.clone());
        }
    }
    declared.removal.normalize_and_sort_lists();
    Ok(declared)
}


#[allow(dead_code)]
pub(in crate::prune) fn prune_declared_paths(
    root: &Path,
    remove_paths: &[PathBuf],
    policy: RemovalFailurePolicy,
) -> Result<DeclaredPathPruneResult> {
    prune_declared_paths_with_preservation(root, remove_paths, &[], policy)
}

fn prune_declared_paths_with_preservation(
    root: &Path,
    remove_paths: &[PathBuf],
    preserved_paths: &[PathBuf],
    policy: RemovalFailurePolicy,
) -> Result<DeclaredPathPruneResult> {
    let mut files_removed = 0usize;
    let mut dirs_removed = 0usize;
    let mut direct_dirs_removed = 0usize;
    let mut removed_artifacts = Vec::new();
    let mut removal = RemovalAccounting::default();
    let mut edits = Vec::new();
    let mut failed = Vec::new();

    for manifest_path in remove_paths {
        if removal_path_escapes_root(manifest_path) {
            let failed_removal = FailedRemoval::new(
                manifest_path.clone(),
                FailedRemovalKind::EscapedRoot,
                "declared removal path escapes tree root",
            );
            failed.push(failed_removal.clone());
            reject_fatal_failed_removal(&failed_removal, policy)?;
            continue;
        }

        let target = root.join(manifest_path);
        match std::fs::symlink_metadata(&target) {
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                if removal
                    .removed_files
                    .iter()
                    .any(|path| path == manifest_path)
                    || removal
                        .removed_dirs
                        .iter()
                        .any(|path| path == manifest_path)
                {
                    continue;
                }
                log::warn!(
                    "prune: path '{}' not found in tree, skipping",
                    manifest_path.display()
                );
                removal.missing_paths.push(manifest_path.clone());
                let failed_removal = FailedRemoval::new(
                    normalize_relative_path(manifest_path),
                    FailedRemovalKind::MissingPath,
                    "path not found in tree",
                );
                failed.push(failed_removal.clone());
                reject_fatal_failed_removal(&failed_removal, policy)?;
                continue;
            }
            Err(err) => {
                let kind = failed_removal_kind_from_io_error(&err);
                record_failed_removal(
                    &mut failed,
                    normalize_relative_path(manifest_path),
                    kind,
                    err.to_string(),
                    policy,
                )?;
                continue;
            }
        }

        let (files, dirs) = remove_path(
            &target,
            root,
            manifest_path,
            remove_paths,
            &mut removed_artifacts,
            &mut removal,
            &mut edits,
            &mut failed,
            preserved_paths,
            policy,
        )?;
        files_removed += files;
        dirs_removed += dirs;
        direct_dirs_removed += dirs;

        let parent = target.parent().unwrap_or(root);
        let cleanup =
            cleanup_empty_parent_chain(parent, root, manifest_path, remove_paths, preserved_paths)?;
        dirs_removed += cleanup.dirs_removed;
        removed_artifacts.extend(
            cleanup
                .empty_parents_cleaned
                .iter()
                .cloned()
                .map(|relative| RemovedArtifact {
                    relative,
                    is_dir: true,
                }),
        );
        removal
            .empty_parents_cleaned
            .extend(cleanup.empty_parents_cleaned);
        edits.extend(cleanup.edits);

        log::info!(
            "prune: removed '{}' ({files} files, {dirs} dirs)",
            manifest_path.display()
        );
    }

    ensure_edit_records_for_mutation(
        "prune.remove_path",
        files_removed + direct_dirs_removed,
        &edits,
    )?;
    sort_edit_records(&mut edits);

    sort_removed_artifacts(&mut removed_artifacts);
    removal.normalize_and_sort_lists();
    failed.sort();
    failed.dedup();
    let result = PruneResult {
        removed: pruned_paths_from_removed_artifacts(&removed_artifacts),
        failed,
        edits: edits.clone(),
    };

    Ok(DeclaredPathPruneResult {
        files_removed,
        dirs_removed,
        removed_artifacts,
        removal,
        edits,
        result,
    })
}

fn pruned_paths_from_removed_artifacts(artifacts: &[RemovedArtifact]) -> Vec<PrunedPath> {
    artifacts
        .iter()
        .map(|artifact| PrunedPath {
            path: artifact.relative.clone(),
            is_dir: artifact.is_dir,
        })
        .collect()
}

fn sort_removed_artifacts(artifacts: &mut Vec<RemovedArtifact>) {
    for artifact in artifacts.iter_mut() {
        artifact.relative = normalize_relative_path(&artifact.relative);
    }
    artifacts.sort_by(|left, right| {
        left.relative
            .cmp(&right.relative)
            .then(left.is_dir.cmp(&right.is_dir))
    });
    artifacts
        .dedup_by(|left, right| left.relative == right.relative && left.is_dir == right.is_dir);
}

fn normalize_and_sort_paths(paths: &mut Vec<PathBuf>) {
    for path in paths.iter_mut() {
        *path = normalize_relative_path(path);
    }
    paths.sort();
    paths.dedup();
}

pub(in crate::prune) fn normalize_and_sort_symbols(symbols: &mut Vec<String>) {
    symbols.sort();
    symbols.dedup();
}

pub(in crate::prune) fn normalize_relative_path(path: &Path) -> PathBuf {
    crate::kbuild::normalize_relative(path)
}

fn removal_path_escapes_root(path: &Path) -> bool {
    path_is_absolute_like(path)
        || path_contains_parent_traversal(path)
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::RootDir | std::path::Component::Prefix(_)
            )
        })
}

pub(in crate::prune) fn failed_removal_kind_from_io_error(error: &io::Error) -> FailedRemovalKind {
    match error.kind() {
        io::ErrorKind::NotFound => FailedRemovalKind::MissingPath,
        io::ErrorKind::PermissionDenied => FailedRemovalKind::PermissionDenied,
        _ => FailedRemovalKind::IoError,
    }
}

fn reject_fatal_failed_removal(failed: &FailedRemoval, policy: RemovalFailurePolicy) -> Result<()> {
    if policy.is_fatal(failed.kind) {
        anyhow::bail!(
            "failed to remove '{}': {}: {}",
            failed.path.display(),
            failed.kind.stable_name(),
            failed.reason
        );
    }
    Ok(())
}

fn record_failed_removal(
    failed: &mut Vec<FailedRemoval>,
    path: PathBuf,
    kind: FailedRemovalKind,
    reason: impl Into<String>,
    policy: RemovalFailurePolicy,
) -> Result<()> {
    let failed_removal = FailedRemoval::new(path, kind, reason);
    failed.push(failed_removal.clone());
    reject_fatal_failed_removal(&failed_removal, policy)
}

fn failed_path_for(path: &Path, root: &Path, manifest_path: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map(normalize_relative_path)
        .unwrap_or_else(|_| normalize_relative_path(manifest_path))
}

fn path_is_preserved(path: &Path, preserved_paths: &[PathBuf]) -> bool {
    preserved_paths
        .iter()
        .any(|preserved| crate::path_policy::normalized_relative_path_covers(preserved, path))
}

fn remove_path(
    path: &Path,
    root: &Path,
    manifest_path: &Path,
    manifest_paths: &[PathBuf],
    removed: &mut Vec<RemovedArtifact>,
    removal: &mut RemovalAccounting,
    edits: &mut Vec<EditRecord>,
    failed: &mut Vec<FailedRemoval>,
    preserved_paths: &[PathBuf],
    policy: RemovalFailurePolicy,
) -> Result<(usize, usize)> {
    let relative = failed_path_for(path, root, manifest_path);
    if path_is_preserved(&relative, preserved_paths) {
        log::debug!(
            "prune: preserving path '{}' due to named feature preservation intent",
            relative.display()
        );
        return Ok((0, 0));
    }
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) => {
            let kind = failed_removal_kind_from_io_error(&err);
            record_failed_removal(
                failed,
                failed_path_for(path, root, manifest_path),
                kind,
                err.to_string(),
                policy,
            )?;
            return Ok((0, 0));
        }
    };
    if abi_sensitive_path_requires_exact_manifest_truth(&relative, manifest_paths) {
        log::warn!(
            "prune: preserving ABI-sensitive path '{}' because it was not explicitly declared",
            relative.display()
        );
        return Ok((0, 0));
    }

    if meta.file_type().is_dir() {
        let mut file_count = 0usize;
        let mut dir_count = 0usize;

        let entries = match std::fs::read_dir(path) {
            Ok(entries) => entries,
            Err(err) => {
                let kind = failed_removal_kind_from_io_error(&err);
                record_failed_removal(failed, relative, kind, err.to_string(), policy)?;
                return Ok((0, 0));
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    let kind = failed_removal_kind_from_io_error(&err);
                    record_failed_removal(failed, relative.clone(), kind, err.to_string(), policy)?;
                    continue;
                }
            };
            let (sub_files, sub_dirs) = remove_path(
                &entry.path(),
                root,
                manifest_path,
                manifest_paths,
                removed,
                removal,
                edits,
                failed,
                preserved_paths,
                policy,
            )?;
            file_count += sub_files;
            dir_count += sub_dirs;
        }

        let empty = match directory_is_empty(path) {
            Ok(empty) => empty,
            Err(err) => {
                let kind = failed_removal_kind_from_io_error(&err);
                record_failed_removal(failed, relative, kind, err.to_string(), policy)?;
                return Ok((file_count, dir_count));
            }
        };
        if !empty {
            return Ok((file_count, dir_count));
        }

        let before = String::from("<directory>");
        if let Err(err) = std::fs::remove_dir(path) {
            let kind = failed_removal_kind_from_io_error(&err);
            record_failed_removal(failed, relative, kind, err.to_string(), policy)?;
            return Ok((file_count, dir_count));
        }
        dir_count += 1;
        removed.push(RemovedArtifact {
            relative: relative.clone(),
            is_dir: true,
        });
        removal.removed_dirs.push(relative.clone());
        edits.push(EditRecord::new(
            relative,
            None,
            before,
            String::new(),
            EditReason::ManifestPath {
                path: manifest_path.to_path_buf(),
            },
            EditProofSource::removal_manifest_path(manifest_path.to_path_buf()),
            "prune.remove_path",
        ));

        Ok((file_count, dir_count))
    } else if meta.file_type().is_symlink() {
        let before = std::fs::read_link(path)
            .map(|target| format!("<symlink -> {}>", target.display()))
            .unwrap_or_else(|_| String::from("<symlink>"));
        if let Err(err) = std::fs::remove_file(path) {
            let kind = failed_removal_kind_from_io_error(&err);
            record_failed_removal(failed, relative, kind, err.to_string(), policy)?;
            return Ok((0, 0));
        }
        removed.push(RemovedArtifact {
            relative: relative.clone(),
            is_dir: false,
        });
        removal.removed_files.push(relative.clone());
        edits.push(EditRecord::new(
            relative,
            None,
            before,
            String::new(),
            EditReason::ManifestPath {
                path: manifest_path.to_path_buf(),
            },
            EditProofSource::removal_manifest_path(manifest_path.to_path_buf()),
            "prune.remove_path",
        ));
        Ok((1, 0))
    } else if meta.file_type().is_file() {
        for symbol in crate::kconfig::defined_symbols_in_file(path)? {
            if !removal.removed_config_symbols.contains(&symbol) {
                removal.removed_config_symbols.push(symbol);
            }
        }
        let mut before = snippet_for_path(path)?;
        if before.is_empty() {
            before = String::from("<empty file>");
        }
        if let Err(err) = std::fs::remove_file(path) {
            let kind = failed_removal_kind_from_io_error(&err);
            record_failed_removal(failed, relative, kind, err.to_string(), policy)?;
            return Ok((0, 0));
        }
        removed.push(RemovedArtifact {
            relative: relative.clone(),
            is_dir: false,
        });
        removal.removed_files.push(relative.clone());
        edits.push(EditRecord::new(
            relative,
            None,
            before,
            String::new(),
            EditReason::ManifestPath {
                path: manifest_path.to_path_buf(),
            },
            EditProofSource::removal_manifest_path(manifest_path.to_path_buf()),
            "prune.remove_path",
        ));
        Ok((1, 0))
    } else {
        record_failed_removal(
            failed,
            relative,
            FailedRemovalKind::UnsupportedSpecialFile,
            "path is not a regular file or directory",
            policy,
        )?;
        Ok((0, 0))
    }
}

pub(in crate::prune) fn abi_sensitive_path_requires_exact_manifest_truth(
    path: &Path,
    manifest_paths: &[PathBuf],
) -> bool {
    (crate::removal_manifest::is_public_header_path(path) || crate::abi::is_uapi_path(path))
        && !manifest_paths
            .iter()
            .any(|manifest_path| manifest_path.as_path() == path)
}

fn directory_is_empty(path: &Path) -> io::Result<bool> {
    Ok(std::fs::read_dir(path)?.next().is_none())
}

pub(in crate::prune) fn snippet_for_path(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
