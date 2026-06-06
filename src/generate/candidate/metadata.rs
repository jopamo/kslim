use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

use crate::config::ReducerConfig;
use crate::model::ReportPath;
use crate::paths::AttemptMetadataDir;
use crate::removal_manifest::RemovalManifest;
use crate::{manifest, output_repo, reducer};

use super::super::plan::GeneratePlan;
use super::super::state::CandidateTreeState;
use super::super::GenerateStage;
use super::model::{
    ensure_path_inside_candidate_tree, normalize_candidate_boundary_path,
    path_aliases_across_lifecycle, project_root_for_requested_config,
};

pub(super) const CANDIDATE_METADATA_FILE: &str = "candidate.toml";
pub(super) const CANDIDATE_FAILURE_ATTEMPT_FILE: &str = "candidate-failure.toml";
pub(super) const KSLIM_METADATA_DIR: &str = ".kslim";
pub(super) const ATTEMPT_METADATA_DIR: &str = "attempt";

#[derive(Serialize)]
struct CandidateMetadataFile {
    schema_version: u32,
    metadata_scope: String,
    authoritative: bool,
    plan_id: String,
    plan_fingerprint: String,
    tree_fingerprint: String,
    config_content_hash: String,
    generated_by: String,
    selected_profile: String,
    upstream_name: String,
    base_ref: String,
    base_commit: String,
    base_resolved_at: String,
    output_branch: String,
    output_mode: String,
    patch_source_count: usize,
    patch_commit_count: usize,
    integration_count: usize,
    materialized: bool,
    integrated: bool,
    pruned: bool,
    reduced: bool,
    selftested: bool,
    reducer_ran: bool,
    manifest_file: String,
    reducer_report_file: Option<String>,
}

#[derive(Serialize)]
struct CandidateFailureAttemptFile {
    schema_version: u32,
    metadata_scope: String,
    authoritative: bool,
    stage: GenerateStage,
    plan_id: String,
    plan_fingerprint: String,
    selected_profile: String,
    message: String,
    partial_reports: Vec<String>,
}

pub(super) fn write_candidate_failure_attempt_metadata(
    plan: &GeneratePlan,
    stage: GenerateStage,
    message: &str,
    existing_partial_reports: &[ReportPath],
) -> Result<Vec<ReportPath>> {
    let attempt_metadata_dir = candidate_attempt_metadata_dir(plan)?;
    ensure_candidate_attempt_metadata_dir(&attempt_metadata_dir)?;
    crate::fsutil::ensure_dir(attempt_metadata_dir.as_path())?;

    let mut partial_reports = existing_partial_reports.to_vec();
    sort_report_paths(&mut partial_reports);
    let partial_report_names = partial_reports
        .iter()
        .map(|report| attempt_report_name(&attempt_metadata_dir, report.as_path()))
        .collect::<Result<Vec<_>>>()?;

    let metadata = CandidateFailureAttemptFile {
        schema_version: 1,
        metadata_scope: String::from("non-authoritative-attempt"),
        authoritative: false,
        stage,
        plan_id: plan.plan_id.as_str().to_string(),
        plan_fingerprint: plan.fingerprint.as_str().to_string(),
        selected_profile: plan.requested.selected_profile.as_str().to_string(),
        message: message.to_string(),
        partial_reports: partial_report_names,
    };
    let path = attempt_metadata_dir
        .as_path()
        .join(CANDIDATE_FAILURE_ATTEMPT_FILE);
    std::fs::write(&path, toml::to_string_pretty(&metadata)?)?;
    partial_reports.push(report_path_under_attempt_metadata(
        &attempt_metadata_dir,
        path.as_path(),
    )?);
    sort_report_paths(&mut partial_reports);
    Ok(partial_reports)
}

fn sort_report_paths(paths: &mut Vec<ReportPath>) {
    paths.sort();
    paths.dedup();
}

fn attempt_report_name(attempt_metadata_dir: &AttemptMetadataDir, path: &Path) -> Result<String> {
    let attempt_metadata_dir = normalize_candidate_boundary_path(attempt_metadata_dir.as_path())?;
    let path = normalize_candidate_boundary_path(path)?;
    if !path.starts_with(&attempt_metadata_dir) {
        anyhow::bail!(
            "candidate attempt report outside attempt metadata: {}",
            path.display()
        );
    }
    let relative = path.strip_prefix(&attempt_metadata_dir).with_context(|| {
        format!(
            "failed to relativize candidate attempt report {} under {}",
            path.display(),
            attempt_metadata_dir.display()
        )
    })?;
    Ok(relative.to_string_lossy().to_string())
}

pub(super) fn record_partial_candidate_reducer_reports(
    plan: &GeneratePlan,
    stats: &reducer::ReducerStats,
    reducer_config: &ReducerConfig,
    manifest: &RemovalManifest,
) -> Result<Vec<ReportPath>> {
    let attempt_metadata_dir = candidate_attempt_metadata_dir(plan)?;
    record_partial_candidate_reducer_reports_at_dir(
        &attempt_metadata_dir,
        stats,
        reducer_config,
        manifest,
    )
}

pub(super) fn record_partial_candidate_reducer_reports_at_dir(
    attempt_metadata_dir: &AttemptMetadataDir,
    stats: &reducer::ReducerStats,
    reducer_config: &ReducerConfig,
    manifest: &RemovalManifest,
) -> Result<Vec<ReportPath>> {
    ensure_candidate_attempt_metadata_dir(attempt_metadata_dir)?;
    output_repo::write_reducer_metadata_at_dir_with_context(
        attempt_metadata_dir.as_path(),
        Some(stats),
        Some(reducer_config),
        Some(manifest),
    )?;
    collect_partial_candidate_report_paths(attempt_metadata_dir)
}

fn candidate_attempt_metadata_dir(plan: &GeneratePlan) -> Result<AttemptMetadataDir> {
    let project_root = project_root_for_requested_config(plan.requested.config_path.as_path());
    let attempt_dir = normalize_candidate_boundary_path(
        &project_root
            .join(KSLIM_METADATA_DIR)
            .join(ATTEMPT_METADATA_DIR),
    )?;
    if path_aliases_across_lifecycle(
        attempt_dir.as_path(),
        plan.resolved.output_plan.output_path.as_path(),
    )? {
        anyhow::bail!(
            "candidate attempt metadata aliases resolved output path: attempt={} output={}",
            attempt_dir.display(),
            plan.resolved.output_plan.output_path.as_path().display()
        );
    }
    AttemptMetadataDir::new(attempt_dir)
}

fn ensure_candidate_attempt_metadata_dir(attempt_metadata_dir: &AttemptMetadataDir) -> Result<()> {
    let path = normalize_candidate_boundary_path(attempt_metadata_dir.as_path())?;
    if path.file_name().and_then(|name| name.to_str()) != Some(ATTEMPT_METADATA_DIR)
        || path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            != Some(KSLIM_METADATA_DIR)
    {
        anyhow::bail!(
            "partial candidate reports must be recorded under attempt metadata: {}",
            path.display()
        );
    }
    Ok(())
}

fn collect_partial_candidate_report_paths(
    attempt_metadata_dir: &AttemptMetadataDir,
) -> Result<Vec<ReportPath>> {
    let mut reports = Vec::new();
    for artifact in partial_candidate_report_artifacts() {
        let path = attempt_metadata_dir.as_path().join(artifact);
        if path.exists() {
            reports.push(report_path_under_attempt_metadata(
                attempt_metadata_dir,
                path.as_path(),
            )?);
        }
    }
    sort_report_paths(&mut reports);
    Ok(reports)
}

fn partial_candidate_report_artifacts() -> [&'static str; 7] {
    [
        output_repo::REDUCER_REPORT_MD,
        output_repo::REDUCER_REPORT_JSON,
        output_repo::REDUCER_DIAGNOSTICS_JSON,
        output_repo::REDUCER_EDIT_SUMMARY_JSON,
        output_repo::REDUCER_KCONFIG_SOLVER_REPORT_JSON,
        output_repo::REDUCER_KCONFIG_REWRITE_REPORT_JSON,
        output_repo::REDUCER_SKIPPED_SITES_JSON,
    ]
}

fn report_path_under_attempt_metadata(
    attempt_metadata_dir: &AttemptMetadataDir,
    path: &Path,
) -> Result<ReportPath> {
    let attempt_metadata_dir = normalize_candidate_boundary_path(attempt_metadata_dir.as_path())?;
    let path = normalize_candidate_boundary_path(path)?;
    if !path.starts_with(&attempt_metadata_dir) {
        anyhow::bail!(
            "partial candidate report outside attempt metadata: {}",
            path.display()
        );
    }
    ReportPath::new(path)
}

pub(in crate::generate) fn write_candidate_metadata_for_verified_generate(
    plan: &GeneratePlan,
    state: &CandidateTreeState,
    reducer_stats: Option<&reducer::ReducerStats>,
    reducer_config: &ReducerConfig,
    removal_manifest: &RemovalManifest,
) -> Result<()> {
    write_candidate_metadata(
        plan,
        state,
        reducer_stats,
        reducer_config,
        removal_manifest,
    )
}

pub(super) fn write_candidate_metadata(
    plan: &GeneratePlan,
    state: &CandidateTreeState,
    reducer_stats: Option<&reducer::ReducerStats>,
    reducer_config: &ReducerConfig,
    removal_manifest: &RemovalManifest,
) -> Result<()> {
    let tree_path = state.tree.as_path();
    let tree_path_str = tree_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("candidate tree path is not valid UTF-8"))?;
    output_repo::validate_reproducible_metadata_timestamp(
        "candidate.base_resolved_at",
        &plan.resolved.base.resolved_at,
    )?;
    ensure_path_inside_candidate_tree(
        tree_path,
        state.metadata_dir.as_path(),
        "candidate metadata directory",
    )?;
    crate::fsutil::ensure_dir(state.metadata_dir.as_path())?;

    let entries = manifest::generate_manifest(tree_path_str)?;
    manifest::write_manifest(&entries, tree_path_str)?;
    output_repo::write_reducer_metadata_at_dir_with_context(
        state.metadata_dir.as_path(),
        reducer_stats,
        Some(reducer_config),
        Some(removal_manifest),
    )?;

    let metadata = CandidateMetadataFile {
        schema_version: 1,
        metadata_scope: String::from("candidate"),
        authoritative: false,
        plan_id: plan.plan_id.as_str().to_string(),
        plan_fingerprint: plan.fingerprint.as_str().to_string(),
        tree_fingerprint: manifest::tree_fingerprint(&entries),
        config_content_hash: plan.config_content_hash.as_str().to_string(),
        generated_by: plan.created_with.as_str().to_string(),
        selected_profile: plan.requested.selected_profile.as_str().to_string(),
        upstream_name: plan.resolved.base.upstream.clone(),
        base_ref: plan.resolved.base.r#ref.clone(),
        base_commit: plan.resolved.base.commit.clone(),
        base_resolved_at: plan.resolved.base.resolved_at.clone(),
        output_branch: plan.resolved.output_plan.branch.clone(),
        output_mode: plan.resolved.output_plan.mode.clone(),
        patch_source_count: plan.resolved.patch_plan.sources.len(),
        patch_commit_count: plan.resolved.patch_plan.total_patch_count,
        integration_count: plan.resolved.integration_plan.entries.len(),
        materialized: state.materialized,
        integrated: state.integrated,
        pruned: state.pruned,
        reduced: state.reduced,
        selftested: state.selftested,
        reducer_ran: reducer_stats.map(|stats| stats.ran).unwrap_or(false),
        manifest_file: manifest::OUTPUT_MANIFEST_FILE_NAME.to_string(),
        reducer_report_file: reducer_stats
            .filter(|stats| stats.ran)
            .map(|_| output_repo::REDUCER_REPORT_JSON.to_string()),
    };
    std::fs::write(
        state.metadata_dir.as_path().join(CANDIDATE_METADATA_FILE),
        toml::to_string_pretty(&metadata)?,
    )?;
    Ok(())
}
