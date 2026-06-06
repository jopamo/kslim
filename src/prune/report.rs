//! Prune reporting and final statistics assembly.
//!
//! This module owns the public prune statistics model and merges declared path,
//! semantic, stale-reference, and orphan cleanup outputs into the final
//! deterministic prune result.

use anyhow::Result;
use std::path::Path;

use crate::edit_reason::{sort_edit_records, EditRecord};
use crate::kbuild::KbuildSkippedLine;
use crate::kconfig::{
    KconfigReportCounts, KconfigSolverReport, UnsupportedKconfigExpression,
};
use crate::removal_manifest::RemovalManifest;

use super::{
    prune_declared_paths_from_manifest, rewrite_build_graph, rewrite_kconfig_stage,
    DeclaredPathPruneResult, KconfigPruneStageResult, PruneResult, RemovalAccounting,
};

#[derive(Debug)]
pub struct PruneStats {
    pub files_removed: usize,
    pub dirs_removed: usize,
    pub configs_disabled: usize,
    pub defaults_overridden: usize,
    pub kconfig_refs_removed: usize,
    pub makefile_refs_removed: usize,
    pub kconfig_report: KconfigReportCounts,
    pub kconfig_solver_report: KconfigSolverReport,
    pub unsupported_kconfig_expressions: Vec<UnsupportedKconfigExpression>,
    pub skipped_makefile_lines: Vec<KbuildSkippedLine>,
    pub removal: RemovalAccounting,
    pub edits: Vec<EditRecord>,
    #[allow(dead_code)]
    pub result: PruneResult,
}

#[allow(dead_code)]
pub fn prune_tree_from_manifest(root: &str, manifest: &RemovalManifest) -> Result<PruneStats> {
    let root = Path::new(root);
    let declared = prune_declared_paths_from_manifest(root, manifest)?;
    continue_prune_from_declared(root, manifest, &declared)
}

#[allow(dead_code)]
pub(crate) fn continue_prune_from_declared(
    root: &Path,
    manifest: &RemovalManifest,
    declared: &DeclaredPathPruneResult,
) -> Result<PruneStats> {
    let kconfig_stage = rewrite_kconfig_stage(root, manifest)?;
    continue_prune_after_kconfig(root, manifest, declared, kconfig_stage)
}

pub(crate) fn continue_prune_after_kconfig(
    root: &Path,
    _manifest: &RemovalManifest,
    declared: &DeclaredPathPruneResult,
    kconfig_stage: KconfigPruneStageResult,
) -> Result<PruneStats> {
    let mut edits = declared.edits.clone();
    edits.extend(kconfig_stage.edits.clone());

    let rewrite_stats = rewrite_build_graph(
        root,
        &declared.removed_artifacts,
        &kconfig_stage.removed_config_symbols,
    )?;
    edits.extend(rewrite_stats.edits.clone());
    sort_edit_records(&mut edits);
    let result = PruneResult {
        removed: declared.result.removed.clone(),
        failed: declared.result.failed.clone(),
        edits: edits.clone(),
    };

    Ok(PruneStats {
        files_removed: declared.files_removed,
        dirs_removed: declared.dirs_removed,
        configs_disabled: kconfig_stage.configs_disabled,
        defaults_overridden: kconfig_stage.defaults_overridden,
        kconfig_refs_removed: kconfig_stage.kconfig_report.removed_sources,
        makefile_refs_removed: rewrite_stats.makefile_refs_removed,
        kconfig_report: kconfig_stage.kconfig_report,
        kconfig_solver_report: kconfig_stage.kconfig_solver_report,
        unsupported_kconfig_expressions: kconfig_stage.unsupported_kconfig_expressions,
        skipped_makefile_lines: rewrite_stats.skipped_makefile_lines,
        removal: declared.removal.clone(),
        edits,
        result,
    })
}
