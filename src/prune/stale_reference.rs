//! Stale reference pruning after declared path and semantic pruning.
//!
//! This module owns cleanup of references that become stale after artifacts or
//! config symbols are removed: kbuild object references and Kconfig source
//! references proven by the removal manifest plus tree index.

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

use crate::edit_reason::{
    ensure_edit_records_for_mutation, sort_edit_records, write_verified_rewrite, EditProofSource,
    EditReason, EditRecord, LineRange,
};
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
    stats
        .edits
        .extend(rewrite_removed_kconfig_helper_assignments(
            root,
            &removed_files,
        )?);

    Ok(stats)
}

const KCONFIG_HELPER_REWRITE_PASS: &str = "prune.rewrite_removed_kconfig_helpers";

#[derive(Debug)]
struct RemovedKconfigHelperAssignment {
    file: PathBuf,
    line: usize,
    variable: String,
    removed_helper: PathBuf,
    before: String,
}

fn rewrite_removed_kconfig_helper_assignments(
    root: &Path,
    removed_files: &[PathBuf],
) -> Result<Vec<EditRecord>> {
    if removed_files.is_empty() {
        return Ok(Vec::new());
    }

    let files = crate::kconfig::kconfig_files(root);
    let contents = files
        .iter()
        .map(|path| Ok((path.clone(), std::fs::read_to_string(path)?)))
        .collect::<Result<Vec<_>>>()?;
    let mut assignments = Vec::new();

    for (path, content) in &contents {
        for (index, raw) in content.split_inclusive('\n').enumerate() {
            let line = raw.strip_suffix('\n').unwrap_or(raw);
            let Some((lhs, rhs)) = line.split_once(":=") else {
                continue;
            };
            let variable = lhs.trim();
            if variable.is_empty()
                || !variable
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
                || !rhs.contains("$(shell,")
            {
                continue;
            }

            let Some(removed_helper) = removed_files.iter().find(|removed| {
                let marker = format!("$(srctree)/{}", removed.display());
                rhs.contains(&marker)
            }) else {
                continue;
            };

            assignments.push(RemovedKconfigHelperAssignment {
                file: path.clone(),
                line: index + 1,
                variable: variable.to_string(),
                removed_helper: removed_helper.clone(),
                before: raw.to_string(),
            });
        }
    }

    for assignment in &assignments {
        let reference = format!("$({})", assignment.variable);
        let still_used = contents.iter().any(|(path, content)| {
            content
                .split_inclusive('\n')
                .enumerate()
                .any(|(index, line)| {
                    !(path == &assignment.file && index + 1 == assignment.line)
                        && line.contains(&reference)
                })
        });
        if still_used {
            bail!(
                "removed Kconfig helper '{}' is still required by live variable '{}'",
                assignment.removed_helper.display(),
                assignment.variable
            );
        }
    }

    let mut edits = assignments
        .iter()
        .map(|assignment| {
            let relative = assignment
                .file
                .strip_prefix(root)
                .unwrap_or(&assignment.file)
                .to_path_buf();
            EditRecord::new(
                relative,
                Some(LineRange {
                    start: assignment.line,
                    end: assignment.line,
                }),
                assignment.before.clone(),
                String::new(),
                EditReason::ManifestPath {
                    path: assignment.removed_helper.clone(),
                },
                EditProofSource::removal_manifest_path(assignment.removed_helper.clone()),
                KCONFIG_HELPER_REWRITE_PASS,
            )
        })
        .collect::<Vec<_>>();

    for (path, content) in &contents {
        let file_assignments = assignments
            .iter()
            .filter(|assignment| assignment.file == *path)
            .collect::<Vec<_>>();
        if file_assignments.is_empty() {
            continue;
        }
        let rewritten = content
            .split_inclusive('\n')
            .enumerate()
            .filter_map(|(index, line)| {
                (!file_assignments
                    .iter()
                    .any(|assignment| assignment.line == index + 1))
                .then_some(line)
            })
            .collect::<String>();
        write_verified_rewrite(root, path, &rewritten, &edits, KCONFIG_HELPER_REWRITE_PASS)?;
    }

    ensure_edit_records_for_mutation(
        KCONFIG_HELPER_REWRITE_PASS,
        assignments.len(),
        &edits,
    )?;
    sort_edit_records(&mut edits);
    Ok(edits)
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
