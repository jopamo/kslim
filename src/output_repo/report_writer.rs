//! Output repository report writers.
//!
//! This module owns writing reducer report artifacts and generate/failure
//! report text into output metadata locations. Report names and path authority
//! stay in `report.rs`; reducer report rendering stays in `reducer/report/*`.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::config::{KslimConfig, ProfileConfig};
use crate::generate::GenerateStage;
use crate::lockfile::ResolvedBase;
use crate::patches::PatchInfo;
use crate::paths::OutputRepoPath;
use crate::reducer::ReducerStats;
use crate::selftest::SelfTestResult;

use super::naming::branch_name;
use super::report::{
    REDUCER_DIAGNOSTICS_JSON, REDUCER_EDIT_SUMMARY_JSON,
    REDUCER_KCONFIG_REWRITE_REPORT_JSON, REDUCER_KCONFIG_SOLVER_REPORT_JSON, REDUCER_REPORT_JSON,
    REDUCER_REPORT_MD, REDUCER_SKIPPED_SITES_JSON,
};
use super::{metadata, report};

fn published_metadata_dir_path(output_path: &Path) -> Result<PathBuf> {
    let output_repo = OutputRepoPath::new(output_path)?;
    Ok(metadata::published_metadata_dir(&output_repo)?
        .as_path()
        .to_path_buf())
}

#[allow(dead_code)]
pub fn reducer_artifact_path(output_path: &Path, artifact_name: &str) -> Result<PathBuf> {
    let output_repo = OutputRepoPath::new(output_path)?;
    report::output_report_path(&output_repo, artifact_name)
}

#[allow(dead_code)]
pub fn write_reducer_artifact(output_path: &str, artifact_name: &str, content: &str) -> Result<()> {
    let path = reducer_artifact_path(Path::new(output_path), artifact_name)?;
    write_reducer_artifact_at_path(path.as_path(), artifact_name, content)
}

fn write_reducer_artifact_at_path(path: &Path, artifact_name: &str, content: &str) -> Result<()> {
    report::validate_report_file_name(artifact_name)?;
    crate::fsutil::ensure_dir(path.parent().unwrap())?;
    std::fs::write(path, content)?;
    Ok(())
}

#[allow(dead_code)]
pub fn write_reducer_metadata(output_path: &str, stats: Option<&ReducerStats>) -> Result<()> {
    write_reducer_metadata_with_config(output_path, stats, None)
}

pub fn write_reducer_metadata_with_config(
    output_path: &str,
    stats: Option<&ReducerStats>,
    reducer_config: Option<&crate::config::ReducerConfig>,
) -> Result<()> {
    write_reducer_metadata_with_context(output_path, stats, reducer_config, None)
}

pub fn write_reducer_metadata_with_context(
    output_path: &str,
    stats: Option<&ReducerStats>,
    reducer_config: Option<&crate::config::ReducerConfig>,
    manifest: Option<&crate::removal_manifest::RemovalManifest>,
) -> Result<()> {
    let metadata_dir = published_metadata_dir_path(Path::new(output_path))?;
    write_reducer_metadata_at_dir_with_context(
        metadata_dir.as_path(),
        stats,
        reducer_config,
        manifest,
    )
}

#[allow(dead_code)]
pub fn write_reducer_metadata_at_dir(
    metadata_dir: &Path,
    stats: Option<&ReducerStats>,
) -> Result<()> {
    write_reducer_metadata_at_dir_with_config(metadata_dir, stats, None)
}

pub fn write_reducer_metadata_at_dir_with_config(
    metadata_dir: &Path,
    stats: Option<&ReducerStats>,
    reducer_config: Option<&crate::config::ReducerConfig>,
) -> Result<()> {
    write_reducer_metadata_at_dir_with_context(metadata_dir, stats, reducer_config, None)
}

pub fn write_reducer_metadata_at_dir_with_context(
    metadata_dir: &Path,
    stats: Option<&ReducerStats>,
    reducer_config: Option<&crate::config::ReducerConfig>,
    manifest: Option<&crate::removal_manifest::RemovalManifest>,
) -> Result<()> {
    match stats {
        Some(stats) if stats.ran => {
            let artifacts = crate::reducer::render_reducer_stats_report_artifacts_with_manifest(
                stats,
                reducer_config,
                manifest,
                crate::reducer::ReducerReportArtifactNames {
                    markdown: REDUCER_REPORT_MD,
                    summary_json: REDUCER_REPORT_JSON,
                    diagnostics_json: REDUCER_DIAGNOSTICS_JSON,
                    edit_summary_json: REDUCER_EDIT_SUMMARY_JSON,
                    kconfig_solver_report_json: REDUCER_KCONFIG_SOLVER_REPORT_JSON,
                    kconfig_rewrite_report_json: REDUCER_KCONFIG_REWRITE_REPORT_JSON,
                    skipped_sites_json: REDUCER_SKIPPED_SITES_JSON,
                },
            )?;
            write_reducer_artifact_at_dir(metadata_dir, REDUCER_REPORT_MD, &artifacts.markdown)?;
            write_reducer_artifact_at_dir(
                metadata_dir,
                REDUCER_REPORT_JSON,
                &artifacts.summary_json,
            )?;
            write_reducer_artifact_at_dir(
                metadata_dir,
                REDUCER_DIAGNOSTICS_JSON,
                &artifacts.diagnostics_json,
            )?;
            write_reducer_artifact_at_dir(
                metadata_dir,
                REDUCER_EDIT_SUMMARY_JSON,
                &artifacts.edit_summary_json,
            )?;
            write_reducer_artifact_at_dir(
                metadata_dir,
                REDUCER_KCONFIG_SOLVER_REPORT_JSON,
                &artifacts.kconfig_solver_report_json,
            )?;
            write_reducer_artifact_at_dir(
                metadata_dir,
                REDUCER_KCONFIG_REWRITE_REPORT_JSON,
                &artifacts.kconfig_rewrite_report_json,
            )?;
            match artifacts.skipped_sites_json {
                Some(skipped_sites_json) => write_reducer_artifact_at_dir(
                    metadata_dir,
                    REDUCER_SKIPPED_SITES_JSON,
                    &skipped_sites_json,
                )?,
                None => remove_reducer_artifact_at_dir(metadata_dir, REDUCER_SKIPPED_SITES_JSON)?,
            }
        }
        _ => {
            remove_reducer_artifact_at_dir(metadata_dir, REDUCER_REPORT_MD)?;
            remove_reducer_artifact_at_dir(metadata_dir, REDUCER_REPORT_JSON)?;
            remove_reducer_artifact_at_dir(metadata_dir, REDUCER_DIAGNOSTICS_JSON)?;
            remove_reducer_artifact_at_dir(metadata_dir, REDUCER_EDIT_SUMMARY_JSON)?;
            remove_reducer_artifact_at_dir(metadata_dir, REDUCER_KCONFIG_SOLVER_REPORT_JSON)?;
            remove_reducer_artifact_at_dir(metadata_dir, REDUCER_KCONFIG_REWRITE_REPORT_JSON)?;
            remove_reducer_artifact_at_dir(metadata_dir, REDUCER_SKIPPED_SITES_JSON)?;
        }
    }
    Ok(())
}

/// Write .kslim/report.txt
pub fn write_report(
    output_path: &str,
    config: &KslimConfig,
    profile: &ProfileConfig,
    resolved: &ResolvedBase,
    file_count: usize,
    total_bytes: u64,
    mode: &str,
    stage: GenerateStage,
    patch_infos: Option<&[PatchInfo]>,
    selftests: Option<&SelfTestResult>,
) -> Result<()> {
    let kslim_dir = published_metadata_dir_path(Path::new(output_path))?;
    crate::fsutil::ensure_dir(&kslim_dir)?;

    let patch_section =
        metadata::render_patch_section(patch_infos, metadata::MetadataPathPolicy::Committed);

    let selftest_section = match selftests {
        Some(result) if result.enabled => format!(
            "\nSelftests:\n  Built-in checks: {}\n  Kernel build checks: {}\n  Custom commands: {}\n",
            result.built_in_checks, result.kernel_builds_run, result.commands_run
        ),
        Some(_) => "\nSelftests:\n  Disabled: yes\n".to_string(),
        None => String::new(),
    };
    let stage = render_generate_stage_for_report(stage);

    let content = format!(
        r#"kslim report
============

Profile: {}
Mode: {}
Stage: {}
Upstream: {}
Base ref: {}
Base commit: {}
Output branch: {}

Files: {}
Bytes: {}
{}
{}"#,
        profile.profile.name,
        mode,
        stage,
        metadata::committed_upstream_label(config),
        resolved.r#ref,
        resolved.commit,
        branch_name(config, profile, resolved),
        file_count,
        total_bytes,
        patch_section,
        selftest_section,
    );
    std::fs::write(kslim_dir.join("report.txt"), &content)?;
    Ok(())
}

/// Write a generate failure report to a private report path.
pub fn write_failure_report(
    report_path: &Path,
    config: &KslimConfig,
    profile: &ProfileConfig,
    resolved: Option<&ResolvedBase>,
    mode: Option<&str>,
    patch_infos: Option<&[PatchInfo]>,
    stage: GenerateStage,
    failure: &str,
    file_count: Option<usize>,
    total_bytes: Option<u64>,
    reducer_stats: Option<&ReducerStats>,
) -> Result<()> {
    if let Some(parent) = report_path.parent() {
        crate::fsutil::ensure_dir(parent)?;
    }

    let patch_section =
        metadata::render_patch_section(patch_infos, metadata::MetadataPathPolicy::Attempt);

    let base_ref = resolved.map(|r| r.r#ref.as_str()).unwrap_or("<unresolved>");
    let base_commit = resolved
        .map(|r| r.commit.as_str())
        .unwrap_or("<unresolved>");
    let mode = mode.unwrap_or("<unknown>");
    let file_count = file_count
        .map(|count| count.to_string())
        .unwrap_or_else(|| "<unknown>".to_string());
    let total_bytes = total_bytes
        .map(|bytes| bytes.to_string())
        .unwrap_or_else(|| "<unknown>".to_string());
    let stage = render_generate_stage_for_report(stage);
    let reducer_section = match reducer_stats {
        Some(stats) if stats.ran => format!(
            concat!(
                "\nReducer artifacts:\n",
                "  Markdown: {}\n",
                "  Summary JSON: {}\n",
                "  Diagnostics JSON: {}\n",
                "  Edit summary JSON: {}\n",
                "  Kconfig solver JSON: {}\n",
                "  Kconfig rewrite JSON: {}\n"
            ),
            REDUCER_REPORT_MD,
            REDUCER_REPORT_JSON,
            REDUCER_DIAGNOSTICS_JSON,
            REDUCER_EDIT_SUMMARY_JSON,
            REDUCER_KCONFIG_SOLVER_REPORT_JSON,
            REDUCER_KCONFIG_REWRITE_REPORT_JSON,
        ),
        _ => String::new(),
    };

    let content = format!(
        r#"kslim report
============

Status: failure
Authoritative: false
Metadata scope: non-authoritative-attempt
Profile: {}
Mode: {}
Upstream: {}
Base ref: {}
Base commit: {}
Stage: {}

Files: {}
Bytes: {}

Failure:
{}
{}{}"#,
        profile.profile.name,
        mode,
        config.upstream.url,
        base_ref,
        base_commit,
        stage,
        file_count,
        total_bytes,
        failure,
        patch_section,
        reducer_section,
    );

    std::fs::write(report_path, &content)?;
    Ok(())
}

fn render_generate_stage_for_report(stage: GenerateStage) -> &'static str {
    stage.as_str()
}

fn write_reducer_artifact_at_dir(
    metadata_dir: &Path,
    artifact_name: &str,
    content: &str,
) -> Result<()> {
    let path = report::metadata_report_path(metadata_dir, artifact_name)?;
    crate::fsutil::ensure_dir(metadata_dir)?;
    std::fs::write(path, content)?;
    Ok(())
}

fn remove_reducer_artifact_at_dir(metadata_dir: &Path, artifact_name: &str) -> Result<()> {
    let path = report::metadata_report_path(metadata_dir, artifact_name)?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
