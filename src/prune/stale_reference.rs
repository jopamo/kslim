//! Stale reference pruning after declared path and semantic pruning.
//!
//! This module owns cleanup of references that become stale after artifacts or
//! config symbols are removed: kbuild object references and Kconfig source
//! references proven by the removal manifest plus tree index.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::edit_reason::EditRecord;
use crate::kbuild::KbuildSkippedLine;
use crate::removal_manifest::RemovalManifest;

use super::{normalize_relative_path, RemovedArtifact};

#[derive(Default)]
pub(in crate::prune) struct StaleReferenceRewriteStats {
    pub(in crate::prune) makefile_refs_removed: usize,
    pub(in crate::prune) skipped_makefile_lines: Vec<KbuildSkippedLine>,
    pub(in crate::prune) edits: Vec<EditRecord>,
}

pub(in crate::prune) fn rewrite_build_graph(
    root: &Path,
    removed_artifacts: &[RemovedArtifact],
    removed_configs: &[String],
) -> Result<StaleReferenceRewriteStats> {
    let mut stats = StaleReferenceRewriteStats::default();

    let removed_files = removed_artifacts
        .iter()
        .filter(|artifact| !artifact.is_dir)
        .map(|artifact| artifact.relative.clone())
        .collect::<Vec<_>>();
    let removed_dirs = removed_artifacts
        .iter()
        .filter(|artifact| artifact.is_dir)
        .map(|artifact| artifact.relative.clone())
        .collect::<Vec<_>>();

    let makefile_report = rewrite_makefiles(root, &removed_files, &removed_dirs, removed_configs)?;
    stats.makefile_refs_removed = makefile_report.removed_refs;
    stats.skipped_makefile_lines = makefile_report.skipped_ambiguous_lines;
    stats.edits.extend(makefile_report.edits);

    Ok(stats)
}

pub(in crate::prune) fn rewrite_kconfig_sources(
    root: &Path,
    manifest: &RemovalManifest,
) -> Result<(usize, Vec<EditRecord>)> {
    let index = crate::tree_index::TreeIndex::build(root, manifest)?;
    let proofs = kconfig_source_removal_proofs(root, manifest, &index);
    crate::kconfig::rewrite_kconfig_sources(root, &proofs)
}

fn kconfig_source_removal_proofs(
    root: &Path,
    manifest: &RemovalManifest,
    index: &crate::tree_index::TreeIndex,
) -> Vec<crate::kconfig::KconfigSourceRemovalProof> {
    let removed_sources = manifest.removed_kconfig_sources_vec();
    let mut proofs = Vec::new();

    for source_ref in &index.kconfig_sources {
        if source_ref.optional || source_ref.source.contains('$') {
            continue;
        }
        let Some(removed_target) =
            manifest_removed_kconfig_source_target(root, source_ref, &removed_sources)
        else {
            continue;
        };
        proofs.push(crate::kconfig::KconfigSourceRemovalProof {
            file: source_ref.file.clone(),
            line: source_ref.line,
            source: source_ref.source.clone(),
            optional: source_ref.optional,
            relative: source_ref.relative,
            removed_target,
        });
    }

    proofs.sort();
    proofs.dedup();
    proofs
}

fn manifest_removed_kconfig_source_target(
    root: &Path,
    source_ref: &crate::tree_index::KconfigSourceReference,
    removed_sources: &[PathBuf],
) -> Option<PathBuf> {
    let kconfig_dir = root.join(source_ref.file.parent().unwrap_or(Path::new("")));
    let primary = if source_ref.relative {
        normalize_relative_path(&kconfig_dir.join(&source_ref.source))
    } else {
        normalize_relative_path(&root.join(&source_ref.source))
    };
    let fallback = if source_ref.relative {
        normalize_relative_path(&root.join(&source_ref.source))
    } else {
        normalize_relative_path(&kconfig_dir.join(&source_ref.source))
    };

    [primary, fallback].into_iter().find_map(|candidate| {
        let relative = candidate.strip_prefix(root).unwrap_or(candidate.as_path());
        let normalized = normalize_relative_path(relative);
        removed_sources
            .iter()
            .find(|source| source.as_path() == normalized.as_path())
            .cloned()
    })
}

fn rewrite_makefiles(
    root: &Path,
    removed_files: &[PathBuf],
    removed_dirs: &[PathBuf],
    removed_configs: &[String],
) -> Result<crate::kbuild::KbuildRewriteReport> {
    crate::kbuild::rewrite_makefiles_report(root, removed_files, removed_dirs, removed_configs)
}
