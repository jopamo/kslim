//! Non-authoritative generate failure metadata and failed-run rollback.
//!
//! This module writes attempt-scoped failure state and restores failed-run
//! snapshots. It must not publish authoritative lockfile, committed output
//! metadata, or published snapshot state.

use anyhow::{Context, Error, Result};
use serde::Serialize;
use std::path::{Component, Path, PathBuf};

use crate::path_policy::{path_contains_parent_traversal, path_is_empty_like};
use crate::lockfile::ResolvedBase;
use crate::model::ReportPath;
use crate::output_repo;
use crate::patches;
use crate::paths::{AttemptMetadataDir, OutputRepoPath};
use crate::reducer;
use crate::selftest::{CapturedCommandFailure, SelfTestFailure};

use super::plan::{self, GeneratePlan};
use super::state::{GenerateAttemptFailure, GenerateErrorKind};
use super::{GenerateStage, GenerateStateLedger, normalize_generate_state_path};

const GENERATE_FAILURE_FILE: &str = "generate-failure.toml";
const ATTEMPT_METADATA_DIR: &str = "attempt";
const FAILURE_REPORT_FILE: &str = "report.txt";
const NON_AUTHORITATIVE_ATTEMPT_SCOPE: &str = "non-authoritative-attempt";
const PUBLISHED_SNAPSHOT_IDENTIFIER_KEYS: &[&str] =
    &["published_snapshot_id", "published_snapshot", "snapshot_id"];
const LOCKFILE_UPDATE_CLAIM_KEYS: &[&str] = &[
    "lockfile_update",
    "lockfile_updated",
    "authoritative_lockfile",
    "published_lockfile",
];
const OUTPUT_COMMIT_CLAIM_KEYS: &[&str] = &[
    "output_commit",
    "output_commit_claim",
    "committed_output_commit",
    "output_head",
];
const AUTHORITATIVE_METADATA_CLAIM_KEYS: &[&str] = &[
    "authoritative_metadata",
    "authoritative_metadata_claim",
    "metadata_authoritative",
    "committed_metadata",
    "committed_metadata_file",
    "published_metadata",
    "published_metadata_file",
];

#[derive(Serialize)]
struct GenerateFailureFile {
    schema_version: u32,
    metadata_scope: String,
    authoritative: bool,
    stage: GenerateStage,
    error_kind: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan_fingerprint: Option<String>,
    tool_version: String,
    recorded_at: String,
    report_paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command_context: Option<GenerateFailureCommandContext>,
}

#[derive(Serialize)]
struct GenerateFailureCommandContext {
    kind: String,
    command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    arch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_status: Option<i32>,
    elapsed_ms: u64,
}

#[allow(dead_code)]
pub(crate) fn record_generate_failure(
    plan: Option<&GeneratePlan>,
    stage: GenerateStage,
    error: &Error,
    attempt_dir: &AttemptMetadataDir,
) -> Result<()> {
    ensure_non_authoritative_attempt_dir(attempt_dir.as_path())?;
    crate::fsutil::ensure_dir(attempt_dir.as_path())?;

    let message = format!("{error:#}");
    if message.trim().is_empty() {
        anyhow::bail!("generate failure message is empty");
    }

    let metadata = GenerateFailureFile {
        schema_version: 1,
        metadata_scope: String::from(NON_AUTHORITATIVE_ATTEMPT_SCOPE),
        authoritative: false,
        stage,
        error_kind: GenerateErrorKind::from_stage(stage).as_str().to_string(),
        message,
        plan_id: plan.map(|plan| plan.plan_id.as_str().to_string()),
        plan_fingerprint: plan.map(|plan| plan.fingerprint.as_str().to_string()),
        tool_version: plan
            .map(|plan| plan.created_with.as_str().to_string())
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string()),
        recorded_at: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .context("failed to format generate failure timestamp")?,
        report_paths: report_paths_under_attempt_dir(attempt_dir)?,
        command_context: command_context_from_error(error),
    };

    let serialized = toml::to_string_pretty(&metadata)?;
    ensure_no_published_snapshot_identifiers(&serialized)?;
    ensure_no_lockfile_update_claims(&serialized)?;
    ensure_no_output_commit_claims(&serialized)?;
    ensure_no_authoritative_metadata_claims(&serialized)?;
    std::fs::write(
        attempt_dir.as_path().join(GENERATE_FAILURE_FILE),
        serialized,
    )?;
    Ok(())
}

fn ensure_no_published_snapshot_identifiers(serialized: &str) -> Result<()> {
    let value: toml::Value = toml::from_str(serialized)
        .context("failed to validate generate failure metadata before write")?;
    for key in PUBLISHED_SNAPSHOT_IDENTIFIER_KEYS {
        if toml_value_contains_key(&value, key) {
            anyhow::bail!(
                "generate failure metadata must not include published snapshot identifier '{}'",
                key
            );
        }
    }
    Ok(())
}

fn ensure_no_lockfile_update_claims(serialized: &str) -> Result<()> {
    let value: toml::Value = toml::from_str(serialized)
        .context("failed to validate generate failure metadata before write")?;
    for key in LOCKFILE_UPDATE_CLAIM_KEYS {
        if toml_value_contains_key(&value, key) {
            anyhow::bail!(
                "generate failure metadata must not include lockfile update claim '{}'",
                key
            );
        }
    }
    Ok(())
}

fn ensure_no_output_commit_claims(serialized: &str) -> Result<()> {
    let value: toml::Value = toml::from_str(serialized)
        .context("failed to validate generate failure metadata before write")?;
    for key in OUTPUT_COMMIT_CLAIM_KEYS {
        if toml_value_contains_key(&value, key) {
            anyhow::bail!(
                "generate failure metadata must not include output commit claim '{}'",
                key
            );
        }
    }
    Ok(())
}

fn ensure_no_authoritative_metadata_claims(serialized: &str) -> Result<()> {
    let value: toml::Value = toml::from_str(serialized)
        .context("failed to validate generate failure metadata before write")?;
    for key in AUTHORITATIVE_METADATA_CLAIM_KEYS {
        if toml_value_contains_key(&value, key) {
            anyhow::bail!(
                "generate failure metadata must not include authoritative metadata claim '{}'",
                key
            );
        }
    }
    if toml_value_contains_true_bool_key(&value, "authoritative") {
        anyhow::bail!(
            "generate failure metadata must not include authoritative metadata claim 'authoritative = true'"
        );
    }
    if toml_value_contains_non_attempt_metadata_scope(&value) {
        anyhow::bail!(
            "generate failure metadata must not include authoritative metadata claim through metadata_scope"
        );
    }
    Ok(())
}

fn toml_value_contains_key(value: &toml::Value, needle: &str) -> bool {
    match value {
        toml::Value::Table(table) => table
            .iter()
            .any(|(key, value)| key == needle || toml_value_contains_key(value, needle)),
        toml::Value::Array(values) => values
            .iter()
            .any(|value| toml_value_contains_key(value, needle)),
        _ => false,
    }
}

fn toml_value_contains_true_bool_key(value: &toml::Value, needle: &str) -> bool {
    match value {
        toml::Value::Table(table) => table.iter().any(|(key, value)| {
            (key == needle && value.as_bool() == Some(true))
                || toml_value_contains_true_bool_key(value, needle)
        }),
        toml::Value::Array(values) => values
            .iter()
            .any(|value| toml_value_contains_true_bool_key(value, needle)),
        _ => false,
    }
}

fn toml_value_contains_non_attempt_metadata_scope(value: &toml::Value) -> bool {
    match value {
        toml::Value::Table(table) => table.iter().any(|(key, value)| {
            (key == "metadata_scope" && value.as_str() != Some(NON_AUTHORITATIVE_ATTEMPT_SCOPE))
                || toml_value_contains_non_attempt_metadata_scope(value)
        }),
        toml::Value::Array(values) => values
            .iter()
            .any(toml_value_contains_non_attempt_metadata_scope),
        _ => false,
    }
}

fn report_paths_under_attempt_dir(attempt_dir: &AttemptMetadataDir) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    push_attempt_report_path(
        &mut paths,
        attempt_dir,
        attempt_dir.as_path().join(FAILURE_REPORT_FILE),
    )?;
    push_attempt_report_path(
        &mut paths,
        attempt_dir,
        attempt_dir
            .as_path()
            .join(crate::output_repo::LAST_ATTEMPT_JSON),
    )?;

    for optional_report in [
        crate::output_repo::REDUCER_FAILURE_JSON,
        crate::output_repo::REDUCER_REPORT_MD,
        crate::output_repo::REDUCER_REPORT_JSON,
        crate::output_repo::REDUCER_DIAGNOSTICS_JSON,
        crate::output_repo::REDUCER_EDIT_SUMMARY_JSON,
        crate::output_repo::REDUCER_KCONFIG_SOLVER_REPORT_JSON,
        crate::output_repo::REDUCER_KCONFIG_REWRITE_REPORT_JSON,
        crate::output_repo::REDUCER_SKIPPED_SITES_JSON,
    ] {
        let path = attempt_dir.as_path().join(optional_report);
        if path.exists() {
            push_attempt_report_path(&mut paths, attempt_dir, path)?;
        }
    }

    sort_report_paths(&mut paths);
    Ok(paths)
}

fn sort_report_paths(paths: &mut Vec<String>) {
    paths.sort();
    paths.dedup();
}

fn push_attempt_report_path(
    paths: &mut Vec<String>,
    attempt_dir: &AttemptMetadataDir,
    path: impl AsRef<Path>,
) -> Result<()> {
    let path = path.as_ref();
    let relative = path.strip_prefix(attempt_dir.as_path()).with_context(|| {
        format!(
            "generate failure report path must stay under attempt metadata: {}",
            path.display()
        )
    })?;
    let relative = normalized_relative_report_path(relative)?;
    if !paths.iter().any(|path| path == &relative) {
        paths.push(relative);
    }
    Ok(())
}

fn normalized_relative_report_path(path: &Path) -> Result<String> {
    if path.as_os_str().is_empty() {
        anyhow::bail!("generate failure report path is empty");
    }
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let part = part.to_str().ok_or_else(|| {
                    anyhow::anyhow!(
                        "generate failure report path is not valid UTF-8: {}",
                        path.display()
                    )
                })?;
                parts.push(part.to_string());
            }
            _ => {
                anyhow::bail!(
                    "generate failure report path must be relative and normalized under attempt metadata: {}",
                    path.display()
                );
            }
        }
    }
    if parts.is_empty() {
        anyhow::bail!("generate failure report path is empty");
    }
    Ok(parts.join("/"))
}

fn command_context_from_error(error: &Error) -> Option<GenerateFailureCommandContext> {
    for cause in error.chain() {
        if let Some(selftest) = cause.downcast_ref::<SelfTestFailure>() {
            return command_context_from_selftest_failure(selftest);
        }
    }
    None
}

fn command_context_from_selftest_failure(
    failure: &SelfTestFailure,
) -> Option<GenerateFailureCommandContext> {
    match failure {
        SelfTestFailure::BuiltIn { .. } => None,
        SelfTestFailure::Command { details } => Some(command_context_from_details(
            "selftest-command",
            None,
            None,
            details,
        )),
        SelfTestFailure::KernelBuild {
            label,
            output_dir,
            details,
        } => Some(command_context_from_details(
            "kernel-build",
            Some(label.as_str()),
            Some(output_dir.as_path()),
            details,
        )),
    }
}

fn command_context_from_details(
    kind: &str,
    label: Option<&str>,
    output_dir: Option<&Path>,
    details: &CapturedCommandFailure,
) -> GenerateFailureCommandContext {
    GenerateFailureCommandContext {
        kind: kind.to_string(),
        command: details.command.clone(),
        label: label.map(str::to_string),
        output_dir: output_dir.map(|path| path.display().to_string()),
        target: details.target.clone(),
        arch: details.arch.clone(),
        config: details.config.clone(),
        exit_status: details.exit_status,
        elapsed_ms: duration_millis_u64(details.elapsed),
    }
}

fn duration_millis_u64(duration: std::time::Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn ensure_non_authoritative_attempt_dir(path: &Path) -> Result<()> {
    if path_is_empty_like(path) {
        anyhow::bail!("generate failure attempt metadata dir is empty");
    }
    if path_contains_parent_traversal(path) {
        anyhow::bail!(
            "generate failure attempt metadata dir must not contain parent components: {}",
            path.display()
        );
    }
    if path.file_name().and_then(|name| name.to_str()) != Some("attempt")
        || path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            != Some(".kslim")
    {
        anyhow::bail!(
            "generate failure metadata must be written under non-authoritative attempt metadata: {}",
            path.display()
        );
    }
    Ok(())
}

pub(in crate::generate) struct OutputRepoRollbackState {
    original_branch: Option<String>,
    original_head: String,
    target_branch: String,
    target_branch_head: Option<String>,
    git_config_contents: String,
    metadata_backup: tempfile::TempDir,
}

pub(in crate::generate) enum OutputRepoFailureAtomicState {
    Existing(OutputRepoRollbackState),
    UnmanagedExisting,
    Fresh { output_path: PathBuf },
}

pub(in crate::generate) struct PublishedMetadataFailureAtomicState {
    metadata_dir: PathBuf,
    metadata_backup: Option<tempfile::TempDir>,
}

#[derive(Default)]
pub(in crate::generate) struct FailureReportContext {
    pub(in crate::generate) stage: GenerateStage,
    pub(in crate::generate) attempt_failure: Option<GenerateAttemptFailure>,
    pub(in crate::generate) resolved: Option<ResolvedBase>,
    pub(in crate::generate) mode: Option<String>,
    pub(in crate::generate) patch_infos: Option<Vec<patches::PatchInfo>>,
    pub(in crate::generate) file_count: Option<usize>,
    pub(in crate::generate) total_bytes: Option<u64>,
    pub(in crate::generate) reducer_stats: Option<reducer::ReducerStats>,
    pub(in crate::generate) reducer_failure: Option<reducer::ReducerFailureReport>,
    pub(in crate::generate) generate_plan: Option<plan::GeneratePlan>,
    pub(in crate::generate) states: GenerateStateLedger,
    pub(in crate::generate) output_repo_rollback: Option<OutputRepoFailureAtomicState>,
    pub(in crate::generate) published_metadata_rollback: Option<PublishedMetadataFailureAtomicState>,
    pub(in crate::generate) lockfile_rollback: Option<crate::lockfile::LockfileFailureAtomicState>,
}

pub(in crate::generate) fn set_generate_stage(failure: &mut FailureReportContext, stage: GenerateStage) {
    failure.stage = stage;
    log_generate_stage(stage, "enter");
}

pub(in crate::generate) fn log_generate_stage(stage: GenerateStage, action: &str) {
    log::info!("generate: stage={} action={}", stage.as_str(), action);
}

pub(in crate::generate) fn record_generate_attempt_failure(
    project_root: &std::path::Path,
    failure: &mut FailureReportContext,
    failure_message: &str,
) -> Result<()> {
    let attempt_metadata_dir = AttemptMetadataDir::new(project_attempt_metadata_dir(project_root))?;
    let mut partial_reports = vec![
        ReportPath::new(project_failure_report_path(project_root))?,
        ReportPath::new(project_last_attempt_path(project_root))?,
    ];
    if failure.reducer_failure.is_some() {
        partial_reports.push(ReportPath::new(project_reducer_failure_path(project_root))?);
    }
    failure.attempt_failure = Some(GenerateAttemptFailure::from_stage(
        failure.stage,
        failure_message,
        attempt_metadata_dir,
        partial_reports,
    )?);
    Ok(())
}

pub(in crate::generate) fn ensure_no_attempt_failure_before_publication(failure: &FailureReportContext) -> Result<()> {
    if let Some(attempt) = failure.attempt_failure.as_ref() {
        anyhow::bail!(
            "generate attempt failure at stage {} cannot be converted into published state",
            attempt.stage().as_str()
        );
    }
    Ok(())
}

pub(in crate::generate) fn project_failure_report_path(project_root: &std::path::Path) -> std::path::PathBuf {
    project_attempt_metadata_dir(project_root).join(FAILURE_REPORT_FILE)
}

pub(in crate::generate) fn project_reducer_failure_path(project_root: &std::path::Path) -> std::path::PathBuf {
    project_attempt_metadata_dir(project_root).join(output_repo::REDUCER_FAILURE_JSON)
}

pub(in crate::generate) fn project_last_attempt_path(project_root: &std::path::Path) -> std::path::PathBuf {
    project_attempt_metadata_dir(project_root).join(output_repo::LAST_ATTEMPT_JSON)
}

pub(in crate::generate) fn project_attempt_metadata_dir(project_root: &std::path::Path) -> std::path::PathBuf {
    project_root.join(".kslim").join(ATTEMPT_METADATA_DIR)
}

fn project_attempt_metadata_dir_rel() -> std::path::PathBuf {
    std::path::PathBuf::from(".kslim").join(ATTEMPT_METADATA_DIR)
}

pub(in crate::generate) fn ensure_non_authoritative_attempt_path(
    project_root: &std::path::Path,
    path: &std::path::Path,
) -> Result<()> {
    let attempt_dir = normalize_generate_state_path(&project_attempt_metadata_dir(project_root))?;
    let path = normalize_generate_state_path(path)?;
    if !path.starts_with(&attempt_dir) {
        anyhow::bail!(
            "failure metadata write outside non-authoritative attempt metadata: {}",
            path.display()
        );
    }
    Ok(())
}

pub(in crate::generate) fn remove_optional_dir(path: &std::path::Path) -> Result<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

/// Attempt metadata is non-authoritative and write-only.
///
/// Stale attempt files from older stage names are cleared as a private
/// namespace on success, so no backward-compatible failure reader or migration
/// path is required for authoritative state.
pub(in crate::generate) fn clear_project_failure_artifacts(project_root: &std::path::Path) -> Result<()> {
    remove_optional_dir(&project_attempt_metadata_dir(project_root))?;
    Ok(())
}

pub(in crate::generate) fn write_project_last_attempt(
    project_root: &std::path::Path,
    failure: &FailureReportContext,
    failure_message: &str,
) -> Result<()> {
    let attempt_dir = AttemptMetadataDir::new(project_attempt_metadata_dir(project_root))?;
    let path = output_repo::attempt_last_attempt_report_path(&attempt_dir)?;
    ensure_non_authoritative_attempt_path(project_root, &path)?;
    if let Some(parent) = path.parent() {
        crate::fsutil::ensure_dir(parent)?;
    }
    let content = render_last_attempt_json(failure, failure_message);
    output_repo::validate_last_attempt_json(&content)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub(in crate::generate) fn write_project_reducer_failure_report(
    project_root: &std::path::Path,
    failure: &FailureReportContext,
) -> Result<()> {
    let path = project_reducer_failure_path(project_root);
    ensure_non_authoritative_attempt_path(project_root, &path)?;
    match failure.reducer_failure.as_ref() {
        Some(report) => {
            if let Some(parent) = path.parent() {
                crate::fsutil::ensure_dir(parent)?;
            }
            std::fs::write(&path, render_reducer_failure_json(failure, report))?;
        }
        None => match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        },
    }
    Ok(())
}

fn render_last_attempt_json(failure: &FailureReportContext, failure_message: &str) -> String {
    let written_at = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    let attempt_failure = failure.attempt_failure.as_ref();
    let stage = attempt_failure
        .map(|attempt| attempt.stage())
        .unwrap_or(failure.stage);
    let error_kind = attempt_failure
        .map(|attempt| render_json_string(attempt.error_kind().as_str()))
        .unwrap_or_else(|| String::from("null"));
    let failure_message = attempt_failure
        .map(|attempt| attempt.message())
        .unwrap_or(failure_message);
    let resolved = failure
        .resolved
        .as_ref()
        .map(render_resolved_base_json)
        .unwrap_or_else(|| String::from("null"));
    let patch_sources = failure
        .patch_infos
        .as_ref()
        .map(|infos| infos.len().to_string())
        .unwrap_or_else(|| String::from("null"));
    let patch_count = failure
        .patch_infos
        .as_ref()
        .map(|infos| patches::total_patch_count(infos).to_string())
        .unwrap_or_else(|| String::from("null"));
    let mode = failure
        .mode
        .as_deref()
        .map(render_json_string)
        .unwrap_or_else(|| String::from("null"));
    let file_count = failure
        .file_count
        .map(|count| count.to_string())
        .unwrap_or_else(|| String::from("null"));
    let total_bytes = failure
        .total_bytes
        .map(|bytes| bytes.to_string())
        .unwrap_or_else(|| String::from("null"));
    let reducer = failure
        .reducer_stats
        .as_ref()
        .map(render_last_attempt_reducer_json)
        .unwrap_or_else(|| String::from("null"));
    let reducer_failure_json = if failure.reducer_failure.is_some() {
        render_json_string(output_repo::REDUCER_FAILURE_JSON)
    } else {
        String::from("null")
    };

    format!(
        concat!(
            "{{\n",
            "  \"authoritative\": false,\n",
            "  \"metadata_scope\": \"{}\",\n",
            "  \"metadata_dir\": \"{}\",\n",
            "  \"written_at\": \"{}\",\n",
            "  \"stage\": {},\n",
            "  \"error_kind\": {},\n",
            "  \"mode\": {},\n",
            "  \"failure\": \"{}\",\n",
            "  \"failure_report\": \"report.txt\",\n",
            "  \"resolved_base\": {},\n",
            "  \"patch_sources\": {},\n",
            "  \"patch_count\": {},\n",
            "  \"file_count\": {},\n",
            "  \"total_bytes\": {},\n",
            "  \"authoritative_lockfile\": {{\n",
            "    \"path\": \"kslim.lock\",\n",
            "    \"updated\": false\n",
            "  }},\n",
            "  \"reducer\": {},\n",
            "  \"reducer_artifacts\": {{\n",
            "    \"report_markdown\": \"{}\",\n",
            "    \"report_json\": \"{}\",\n",
            "    \"diagnostics_json\": \"{}\",\n",
            "    \"edit_summary_json\": \"{}\",\n",
            "    \"kconfig_solver_report_json\": \"{}\",\n",
            "    \"kconfig_rewrite_report_json\": \"{}\",\n",
            "    \"failure_json\": {}\n",
            "  }}\n",
            "}}\n"
        ),
        json_escape(output_repo::NON_AUTHORITATIVE_ATTEMPT_SCOPE),
        json_escape(&project_attempt_metadata_dir_rel().display().to_string()),
        json_escape(&written_at),
        render_generate_stage_json(stage),
        error_kind,
        mode,
        json_escape(failure_message),
        resolved,
        patch_sources,
        patch_count,
        file_count,
        total_bytes,
        reducer,
        output_repo::REDUCER_REPORT_MD,
        output_repo::REDUCER_REPORT_JSON,
        output_repo::REDUCER_DIAGNOSTICS_JSON,
        output_repo::REDUCER_EDIT_SUMMARY_JSON,
        output_repo::REDUCER_KCONFIG_SOLVER_REPORT_JSON,
        output_repo::REDUCER_KCONFIG_REWRITE_REPORT_JSON,
        reducer_failure_json,
    )
}

fn render_resolved_base_json(resolved: &ResolvedBase) -> String {
    format!(
        concat!(
            "{{\n",
            "    \"upstream\": \"{}\",\n",
            "    \"url\": \"{}\",\n",
            "    \"ref\": \"{}\",\n",
            "    \"commit\": \"{}\",\n",
            "    \"resolved_at\": \"{}\"\n",
            "  }}"
        ),
        json_escape(&resolved.upstream),
        json_escape(&resolved.url),
        json_escape(&resolved.r#ref),
        json_escape(&resolved.commit),
        json_escape(&resolved.resolved_at),
    )
}

fn render_last_attempt_reducer_json(stats: &reducer::ReducerStats) -> String {
    format!(
        concat!(
            "{{\n",
            "    \"ran\": {},\n",
            "    \"files_removed\": {},\n",
            "    \"dirs_removed\": {},\n",
            "    \"edit_records\": {},\n",
            "    \"unsupported_kconfig_expressions\": {},\n",
            "    \"unsupported_cpp_expressions\": {},\n",
            "    \"skipped_fixup_diagnostics\": {}\n",
            "  }}"
        ),
        if stats.ran { "true" } else { "false" },
        stats.files_removed,
        stats.dirs_removed,
        stats.edits.len(),
        stats.unsupported_kconfig_expressions.len(),
        stats.unsupported_cpp_expressions.len(),
        stats.skipped_fixups.len(),
    )
}

fn render_reducer_failure_json(
    failure: &FailureReportContext,
    report: &reducer::ReducerFailureReport,
) -> String {
    let reducer = failure
        .reducer_stats
        .as_ref()
        .map(render_last_attempt_reducer_json)
        .unwrap_or_else(|| String::from("null"));
    let fixup_passes = report
        .fixup_passes
        .map(|passes| passes.to_string())
        .unwrap_or_else(|| String::from("null"));

    format!(
        concat!(
            "{{\n",
            "  \"authoritative\": false,\n",
            "  \"metadata_scope\": \"{}\",\n",
            "  \"metadata_dir\": \"{}\",\n",
            "  \"kind\": \"reducer_non_convergence\",\n",
            "  \"stage\": {},\n",
            "  \"termination\": \"{}\",\n",
            "  \"termination_description\": \"{}\",\n",
            "  \"fixup_passes\": {},\n",
            "  \"failure\": \"{}\",\n",
            "  \"reducer\": {},\n",
            "  \"artifacts\": {{\n",
            "    \"report_markdown\": \"{}\",\n",
            "    \"report_json\": \"{}\",\n",
            "    \"diagnostics_json\": \"{}\",\n",
            "    \"edit_summary_json\": \"{}\",\n",
            "    \"kconfig_solver_report_json\": \"{}\",\n",
            "    \"kconfig_rewrite_report_json\": \"{}\"\n",
            "  }}\n",
            "}}\n"
        ),
        json_escape(output_repo::NON_AUTHORITATIVE_ATTEMPT_SCOPE),
        json_escape(&project_attempt_metadata_dir_rel().display().to_string()),
        render_generate_stage_json(failure.stage),
        report.termination.json_value(),
        json_escape(report.termination.description()),
        fixup_passes,
        json_escape(&report.failure),
        reducer,
        output_repo::REDUCER_REPORT_MD,
        output_repo::REDUCER_REPORT_JSON,
        output_repo::REDUCER_DIAGNOSTICS_JSON,
        output_repo::REDUCER_EDIT_SUMMARY_JSON,
        output_repo::REDUCER_KCONFIG_SOLVER_REPORT_JSON,
        output_repo::REDUCER_KCONFIG_REWRITE_REPORT_JSON,
    )
}

fn render_json_string(input: &str) -> String {
    format!("\"{}\"", json_escape(input))
}

fn render_generate_stage_json(stage: GenerateStage) -> String {
    serde_json::to_string(&stage).expect("GenerateStage serialization should not fail")
}

fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn capture_output_repo_rollback_state(
    output_path: &str,
    target_branch: &str,
) -> Result<OutputRepoRollbackState> {
    let original_branch = crate::git::current_branch(output_path)
        .ok()
        .filter(|branch| !branch.trim().is_empty());
    let original_head = crate::git::head_commit(output_path)?;
    let target_branch_head = if crate::git::branch_exists(output_path, target_branch)? {
        Some(crate::git::rev_parse(
            output_path,
            &format!("refs/heads/{}", target_branch),
        )?)
    } else {
        None
    };
    let git_config_contents =
        std::fs::read_to_string(std::path::Path::new(output_path).join(".git/config"))?;
    let metadata_backup = tempfile::Builder::new()
        .prefix("kslim-output-metadata-backup-")
        .tempdir()?;
    let output_repo = OutputRepoPath::new(std::path::Path::new(output_path))?;
    let metadata_dir = crate::output_repo::published_metadata_dir(&output_repo)?;
    sync_plain_dir(metadata_dir.as_path(), metadata_backup.path())?;

    Ok(OutputRepoRollbackState {
        original_branch,
        original_head,
        target_branch: target_branch.to_string(),
        target_branch_head,
        git_config_contents,
        metadata_backup,
    })
}

pub(in crate::generate) fn capture_output_repo_failure_atomic_state(
    output_path: &str,
    target_branch: &str,
) -> Result<OutputRepoFailureAtomicState> {
    let output = std::path::Path::new(output_path);
    if output.exists() {
        if output.join(".git").exists() {
            Ok(OutputRepoFailureAtomicState::Existing(
                capture_output_repo_rollback_state(output_path, target_branch)?,
            ))
        } else {
            Ok(OutputRepoFailureAtomicState::UnmanagedExisting)
        }
    } else {
        Ok(OutputRepoFailureAtomicState::Fresh {
            output_path: output.to_path_buf(),
        })
    }
}

pub(in crate::generate) fn capture_published_metadata_failure_atomic_state(
    output_path: &Path,
) -> Result<PublishedMetadataFailureAtomicState> {
    let output_repo = OutputRepoPath::new(output_path)?;
    let metadata_dir = crate::output_repo::published_metadata_dir(&output_repo)?
        .as_path()
        .to_path_buf();
    let metadata_backup = if metadata_dir.exists() {
        let metadata_backup = tempfile::Builder::new()
            .prefix("kslim-published-metadata-backup-")
            .tempdir()?;
        sync_plain_dir(metadata_dir.as_path(), metadata_backup.path()).with_context(|| {
            format!(
                "failed to snapshot published metadata before generate: {}",
                metadata_dir.display()
            )
        })?;
        Some(metadata_backup)
    } else {
        None
    };
    Ok(PublishedMetadataFailureAtomicState {
        metadata_dir,
        metadata_backup,
    })
}

pub(in crate::generate) fn rollback_published_metadata_failure_atomic_state(
    state: &PublishedMetadataFailureAtomicState,
) -> Result<()> {
    match state.metadata_backup.as_ref() {
        Some(metadata_backup) => {
            sync_plain_dir(metadata_backup.path(), state.metadata_dir.as_path())?;
        }
        None => {
            if state.metadata_dir.exists() {
                remove_plain_path(state.metadata_dir.as_path())?;
            }
        }
    }
    ensure_failed_run_published_metadata_snapshot_unmodified(state)
}

pub(in crate::generate) fn ensure_failed_run_published_metadata_snapshot_unmodified(
    state: &PublishedMetadataFailureAtomicState,
) -> Result<()> {
    match state.metadata_backup.as_ref() {
        Some(metadata_backup) => ensure_plain_dir_contents_equal(
            state.metadata_dir.as_path(),
            metadata_backup.path(),
            "published metadata",
        ),
        None => {
            if state.metadata_dir.exists() {
                anyhow::bail!(
                    "failed run created published metadata path: {}",
                    state.metadata_dir.display()
                );
            }
            Ok(())
        }
    }
}

pub(in crate::generate) fn rollback_failed_run_lockfile_state(failure: &FailureReportContext) -> Result<()> {
    if let Some(lockfile_rollback) = failure.lockfile_rollback.as_ref() {
        crate::lockfile::rollback_lockfile_failure_atomic_state(lockfile_rollback)?;
    }
    Ok(())
}

pub(in crate::generate) fn rollback_output_repo_failure_atomic_state(
    output_path: &str,
    state: &OutputRepoFailureAtomicState,
) -> Result<()> {
    match state {
        OutputRepoFailureAtomicState::Existing(state) => {
            rollback_output_repo_state(output_path, state)
        }
        OutputRepoFailureAtomicState::UnmanagedExisting => Ok(()),
        OutputRepoFailureAtomicState::Fresh { output_path } => remove_plain_path(output_path),
    }?;
    ensure_failed_run_output_commits_unmodified(output_path, state)?;
    ensure_failed_run_published_metadata_unmodified(output_path, state)
}

pub(in crate::generate) fn ensure_failed_run_output_commits_unmodified(
    output_path: &str,
    state: &OutputRepoFailureAtomicState,
) -> Result<()> {
    match state {
        OutputRepoFailureAtomicState::Existing(state) => {
            ensure_existing_output_commit_refs_restored(output_path, state)
        }
        OutputRepoFailureAtomicState::UnmanagedExisting => {
            let output = std::path::Path::new(output_path);
            if output.join(".git").exists() {
                anyhow::bail!(
                    "failed run converted unmanaged output path into git repository: {}",
                    output.display()
                );
            }
            Ok(())
        }
        OutputRepoFailureAtomicState::Fresh { output_path } => {
            if output_path.exists() {
                anyhow::bail!(
                    "failed run left fresh output repository behind: {}",
                    output_path.display()
                );
            }
            Ok(())
        }
    }
}

fn ensure_existing_output_commit_refs_restored(
    output_path: &str,
    state: &OutputRepoRollbackState,
) -> Result<()> {
    let current_head = crate::git::head_commit(output_path)?;
    if current_head != state.original_head {
        anyhow::bail!(
            "failed run updated output HEAD: expected {}, found {}",
            state.original_head,
            current_head
        );
    }

    let current_branch = crate::git::current_branch(output_path)
        .ok()
        .filter(|branch| !branch.trim().is_empty());
    if current_branch.as_deref() != state.original_branch.as_deref() {
        anyhow::bail!(
            "failed run left output branch changed: expected {:?}, found {:?}",
            state.original_branch,
            current_branch
        );
    }

    if let Some(branch) = state.original_branch.as_deref() {
        let branch_head = crate::git::rev_parse(output_path, &format!("refs/heads/{}", branch))?;
        if branch_head != state.original_head {
            anyhow::bail!(
                "failed run moved original output branch '{}': expected {}, found {}",
                branch,
                state.original_head,
                branch_head
            );
        }
    }

    match state.target_branch_head.as_deref() {
        Some(expected) => {
            if !crate::git::branch_exists(output_path, &state.target_branch)? {
                anyhow::bail!(
                    "failed run removed existing target output branch '{}'",
                    state.target_branch
                );
            }
            let actual =
                crate::git::rev_parse(output_path, &format!("refs/heads/{}", state.target_branch))?;
            if actual != expected {
                anyhow::bail!(
                    "failed run moved target output branch '{}': expected {}, found {}",
                    state.target_branch,
                    expected,
                    actual
                );
            }
        }
        None => {
            if state.original_branch.as_deref() != Some(state.target_branch.as_str())
                && crate::git::branch_exists(output_path, &state.target_branch)?
            {
                anyhow::bail!(
                    "failed run left new target output branch '{}' published",
                    state.target_branch
                );
            }
        }
    }

    Ok(())
}

pub(in crate::generate) fn ensure_failed_run_published_metadata_unmodified(
    output_path: &str,
    state: &OutputRepoFailureAtomicState,
) -> Result<()> {
    match state {
        OutputRepoFailureAtomicState::Existing(state) => {
            let output_repo = OutputRepoPath::new(std::path::Path::new(output_path))?;
            let metadata_dir = crate::output_repo::published_metadata_dir(&output_repo)?;
            ensure_plain_dir_contents_equal(
                metadata_dir.as_path(),
                state.metadata_backup.path(),
                "published metadata",
            )
        }
        OutputRepoFailureAtomicState::UnmanagedExisting => {
            let output = std::path::Path::new(output_path);
            if output.join(".kslim").exists() || output.join(".git/kslim").exists() {
                anyhow::bail!(
                    "failed run created published metadata under unmanaged output path: {}",
                    output.display()
                );
            }
            Ok(())
        }
        OutputRepoFailureAtomicState::Fresh { output_path } => {
            if output_path.exists() {
                anyhow::bail!(
                    "failed run left fresh published metadata path behind: {}",
                    output_path.display()
                );
            }
            Ok(())
        }
    }
}

fn ensure_plain_dir_contents_equal(
    actual: &std::path::Path,
    expected: &std::path::Path,
    label: &str,
) -> Result<()> {
    let actual_entries = plain_dir_entry_names(actual).with_context(|| {
        format!(
            "failed run updated {label}: cannot read restored directory {}",
            actual.display()
        )
    })?;
    let expected_entries = plain_dir_entry_names(expected).with_context(|| {
        format!(
            "failed run updated {label}: cannot read rollback snapshot {}",
            expected.display()
        )
    })?;

    if let Some(missing) = expected_entries.difference(&actual_entries).next() {
        anyhow::bail!(
            "failed run updated {label}: missing restored entry {}",
            actual.join(missing).display()
        );
    }
    if let Some(extra) = actual_entries.difference(&expected_entries).next() {
        anyhow::bail!(
            "failed run updated {label}: unexpected restored entry {}",
            actual.join(extra).display()
        );
    }

    for name in expected_entries {
        ensure_plain_path_contents_equal(&actual.join(&name), &expected.join(&name), label)?;
    }

    Ok(())
}

fn plain_dir_entry_names(
    path: &std::path::Path,
) -> Result<std::collections::BTreeSet<std::ffi::OsString>> {
    let mut names = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(path)? {
        names.insert(entry?.file_name());
    }
    Ok(names)
}

fn ensure_plain_path_contents_equal(
    actual: &std::path::Path,
    expected: &std::path::Path,
    label: &str,
) -> Result<()> {
    let actual_meta = std::fs::symlink_metadata(actual)?;
    let expected_meta = std::fs::symlink_metadata(expected)?;
    let actual_type = actual_meta.file_type();
    let expected_type = expected_meta.file_type();

    if actual_type.is_dir() || expected_type.is_dir() {
        if actual_type.is_dir() && expected_type.is_dir() {
            return ensure_plain_dir_contents_equal(actual, expected, label);
        }
        anyhow::bail!(
            "failed run updated {label}: entry type changed at {}",
            actual.display()
        );
    }

    if actual_type.is_file() || expected_type.is_file() {
        if actual_type.is_file() && expected_type.is_file() {
            let actual_contents = std::fs::read(actual)?;
            let expected_contents = std::fs::read(expected)?;
            if actual_contents != expected_contents {
                anyhow::bail!(
                    "failed run updated {label}: file contents changed at {}",
                    actual.display()
                );
            }
            return Ok(());
        }
        anyhow::bail!(
            "failed run updated {label}: entry type changed at {}",
            actual.display()
        );
    }

    if actual_type.is_symlink() || expected_type.is_symlink() {
        if actual_type.is_symlink() && expected_type.is_symlink() {
            let actual_target = std::fs::read_link(actual)?;
            let expected_target = std::fs::read_link(expected)?;
            if actual_target != expected_target {
                anyhow::bail!(
                    "failed run updated {label}: symlink target changed at {}",
                    actual.display()
                );
            }
            return Ok(());
        }
        anyhow::bail!(
            "failed run updated {label}: entry type changed at {}",
            actual.display()
        );
    }

    anyhow::bail!(
        "failed run updated {label}: unsupported restored entry type at {}",
        actual.display()
    )
}

fn rollback_output_repo_state(output_path: &str, state: &OutputRepoRollbackState) -> Result<()> {
    let _ = crate::process::run_in_dir(output_path, "git", &["reset", "--hard"]);
    let _ = crate::process::run_in_dir(output_path, "git", &["clean", "-fd"]);

    match state.original_branch.as_deref() {
        Some(branch) => {
            crate::process::run_in_dir(output_path, "git", &["checkout", "-f", branch])?;
            crate::process::run_in_dir(
                output_path,
                "git",
                &["reset", "--hard", &state.original_head],
            )?;
        }
        None => {
            crate::process::run_in_dir(
                output_path,
                "git",
                &["checkout", "--detach", &state.original_head],
            )?;
            crate::process::run_in_dir(
                output_path,
                "git",
                &["reset", "--hard", &state.original_head],
            )?;
        }
    }

    if state.original_branch.as_deref() != Some(state.target_branch.as_str()) {
        match &state.target_branch_head {
            Some(commit) => {
                crate::process::run_in_dir(
                    output_path,
                    "git",
                    &["branch", "-f", &state.target_branch, commit],
                )?;
            }
            None => {
                let _ = crate::process::run_in_dir(
                    output_path,
                    "git",
                    &["branch", "-D", &state.target_branch],
                );
            }
        }
    }

    std::fs::write(
        std::path::Path::new(output_path).join(".git/config"),
        &state.git_config_contents,
    )?;
    let output_repo = OutputRepoPath::new(std::path::Path::new(output_path))?;
    let metadata_dir = crate::output_repo::published_metadata_dir(&output_repo)?;
    sync_plain_dir(state.metadata_backup.path(), metadata_dir.as_path())?;
    crate::process::run_in_dir(output_path, "git", &["clean", "-fd"])?;
    Ok(())
}

fn sync_plain_dir(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    crate::fsutil::ensure_dir(dst)?;

    let mut expected = std::collections::BTreeSet::<std::ffi::OsString>::new();
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        expected.insert(name.clone());
        sync_plain_path(&entry.path(), &dst.join(&name))?;
    }

    for entry in std::fs::read_dir(dst)? {
        let entry = entry?;
        let name = entry.file_name();
        if !expected.contains(&name) {
            remove_plain_path(&entry.path())?;
        }
    }

    Ok(())
}

fn sync_plain_path(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    let meta = std::fs::symlink_metadata(src)?;
    let file_type = meta.file_type();

    if file_type.is_dir() {
        if let Ok(dst_meta) = std::fs::symlink_metadata(dst) {
            if !dst_meta.file_type().is_dir() {
                remove_plain_path(dst)?;
            }
        }
        crate::fsutil::ensure_dir(dst)?;
        sync_plain_dir(src, dst)?;
        return Ok(());
    }

    if let Some(parent) = dst.parent() {
        crate::fsutil::ensure_dir(parent)?;
    }

    if file_type.is_file() {
        if let Ok(dst_meta) = std::fs::symlink_metadata(dst) {
            if !dst_meta.file_type().is_file() {
                remove_plain_path(dst)?;
            }
        }
        std::fs::copy(src, dst)?;
        return Ok(());
    }

    if file_type.is_symlink() {
        if dst.exists() {
            remove_plain_path(dst)?;
        }
        #[cfg(unix)]
        {
            let target = std::fs::read_link(src)?;
            std::os::unix::fs::symlink(target, dst)?;
            return Ok(());
        }
        #[cfg(not(unix))]
        {
            anyhow::bail!(
                "symlink restore is unsupported on this platform: {}",
                src.display()
            );
        }
    }

    anyhow::bail!("unsupported output metadata file type: {}", src.display())
}

fn remove_plain_path(path: &std::path::Path) -> Result<()> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err.into()),
    };
    if meta.file_type().is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::state::{
        CliOverrides, ProfileName, RequestedGenerateState, ResolvedCandidateState,
    };
    use super::*;
    use crate::config;
    use crate::lockfile::ResolvedBase;
    use crate::paths::{AttemptMetadataDir, KernelSourceRoot, RequestedConfigPath};
    use serde::Deserialize;
    use std::time::Duration;

    #[derive(Debug, Deserialize)]
    struct GenerateFailureStageFixture {
        stage: GenerateStage,
    }

    fn requested_state(config_path: &Path) -> RequestedGenerateState {
        RequestedGenerateState::new(
            RequestedConfigPath::new(config_path).unwrap(),
            ProfileName::new("default").unwrap(),
            CliOverrides {
                dry_run: false,
                deep_dry_run: false,
                report_only: false,
                force: false,
                offline: false,
                base_ref: None,
                feature: None,
                remove_feature: None,
                preserve_feature: None,
                arch: None,
                primary_arch: None,
                secondary_arch: None,
                safety: None,
                max_fixup_passes: None,
                matrix: None,
                strict: false,
                no_strict: false,
                run_selftests: false,
            },
        )
    }

    fn plan_for_test(config_path: &Path, output: &Path) -> GeneratePlan {
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let config = config::default_kslim_config("demo", output.to_str().unwrap());
        let profile = config::default_profile_config("v1.0");
        let resolved = ResolvedCandidateState::from_resolved_inputs(
            &config,
            &profile,
            ResolvedBase {
                upstream: config.upstream.name.clone(),
                url: config.upstream.url.clone(),
                r#ref: String::from("v1.0"),
                commit: String::from("deadbeef"),
                resolved_at: String::from("2026-01-01T00:00:00Z"),
            },
            None,
            "unmodified-upstream",
            "kslim/v1.0/default",
        )
        .unwrap();
        GeneratePlan::new(requested_state(config_path), resolved).unwrap()
    }

    fn read_failure(path: &Path) -> toml::Value {
        let contents = std::fs::read_to_string(path.join(GENERATE_FAILURE_FILE)).unwrap();
        toml::from_str(&contents).unwrap()
    }

    fn init_git_repo(path: &Path) {
        crate::git::init_repo(path.to_str().unwrap()).unwrap();
        crate::process::run_in_dir(
            path.to_str().unwrap(),
            "git",
            &["config", "user.email", "test@kslim.local"],
        )
        .unwrap();
        crate::process::run_in_dir(
            path.to_str().unwrap(),
            "git",
            &["config", "user.name", "kslim test"],
        )
        .unwrap();
    }

    fn commit_all(path: &Path, message: &str) -> String {
        crate::git::add_all(path.to_str().unwrap()).unwrap();
        crate::git::commit(path.to_str().unwrap(), message).unwrap();
        crate::git::head_commit(path.to_str().unwrap()).unwrap()
    }

    fn report_paths(value: &toml::Value) -> Vec<&str> {
        value["report_paths"]
            .as_array()
            .unwrap()
            .iter()
            .map(|path| path.as_str().unwrap())
            .collect()
    }

    fn assert_sorted_unique(paths: &[&str]) {
        let mut sorted = paths.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(paths, sorted.as_slice());
    }

    #[test]
    fn test_record_generate_failure_writes_non_authoritative_attempt_metadata_without_plan() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let attempt = AttemptMetadataDir::new(project.join(".kslim/attempt")).unwrap();

        record_generate_failure(
            None,
            GenerateStage::Selftest,
            &anyhow::anyhow!("selftest failed"),
            &attempt,
        )
        .unwrap();

        let failure = read_failure(attempt.as_path());
        let serialized =
            std::fs::read_to_string(attempt.as_path().join(GENERATE_FAILURE_FILE)).unwrap();
        let decoded: GenerateFailureStageFixture = toml::from_str(&serialized).unwrap();
        assert_eq!(decoded.stage, GenerateStage::Selftest);
        assert_eq!(failure["schema_version"].as_integer(), Some(1));
        assert_eq!(
            failure["metadata_scope"].as_str(),
            Some("non-authoritative-attempt")
        );
        assert_eq!(failure["authoritative"].as_bool(), Some(false));
        assert_eq!(failure["stage"].as_str(), Some("selftest"));
        assert_eq!(failure["error_kind"].as_str(), Some("selftest"));
        assert!(failure["message"]
            .as_str()
            .unwrap()
            .contains("selftest failed"));
        assert!(failure.get("plan_id").is_none());
        assert!(failure.get("plan_fingerprint").is_none());
        assert_eq!(
            failure["tool_version"].as_str(),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert!(failure["recorded_at"].as_str().unwrap().contains('T'));
        assert!(failure.get("command_context").is_none());
        let paths = report_paths(&failure);
        assert_eq!(paths, ["last-attempt.json", "report.txt"]);
        assert_sorted_unique(&paths);
        assert!(!project.join("kslim.lock").exists());
    }

    #[test]
    fn test_generate_failure_attempt_metadata_rejects_legacy_stage_aliases() {
        for legacy_stage in [
            "prepare",
            "source",
            "lockfile",
            "reducer",
            "verify",
            "output-commit",
            "output-publish",
        ] {
            let decoded = toml::from_str::<GenerateFailureStageFixture>(&format!(
                "stage = \"{}\"\n",
                legacy_stage
            ));
            assert!(
                decoded.is_err(),
                "generate failure attempt metadata must reject legacy stage alias: {}",
                legacy_stage
            );
        }
    }

    #[test]
    fn test_generate_failure_report_does_not_create_or_update_lockfile() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let attempt = AttemptMetadataDir::new(project.join(".kslim/attempt")).unwrap();
        let lockfile_path = project.join("kslim.lock");

        record_generate_failure(
            None,
            GenerateStage::Reduce,
            &anyhow::anyhow!("reducer failed before publication"),
            &attempt,
        )
        .unwrap();
        assert!(
            !lockfile_path.exists(),
            "failure report must not create an authoritative lockfile"
        );

        let original_lockfile = concat!(
            "[resolved_base]\n",
            "upstream = \"linux\"\n",
            "url = \"/tmp/linux.git\"\n",
            "ref = \"v1.0\"\n",
            "commit = \"deadbeef\"\n",
            "resolved_at = \"2026-01-01T00:00:00Z\"\n",
        );
        std::fs::write(&lockfile_path, original_lockfile).unwrap();

        record_generate_failure(
            None,
            GenerateStage::Publish,
            &anyhow::anyhow!("publish failed before lockfile update"),
            &attempt,
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(&lockfile_path).unwrap(),
            original_lockfile,
            "failure report must not modify an existing authoritative lockfile"
        );
        assert!(attempt.as_path().join(GENERATE_FAILURE_FILE).exists());
    }

    #[test]
    fn test_generate_failure_report_cannot_be_loaded_as_published_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        init_git_repo(&output);
        std::fs::write(output.join("Makefile"), "# test\n").unwrap();
        let attempt = AttemptMetadataDir::new(output.join(".kslim/attempt")).unwrap();

        record_generate_failure(
            None,
            GenerateStage::Publish,
            &anyhow::anyhow!("publish failed before authoritative metadata existed"),
            &attempt,
        )
        .unwrap();
        let failure_report = attempt.as_path().join(GENERATE_FAILURE_FILE);
        assert!(failure_report.exists());

        let commit_with_attempt_report = commit_all(&output, "commit attempt failure report");
        let output_repo = crate::paths::OutputRepoPath::new(&output).unwrap();
        let err = crate::output_repo::load_committed_published_snapshot_metadata(
            &output_repo,
            &commit_with_attempt_report,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("required committed published metadata missing"));
        assert!(err.contains(crate::output_repo::PUBLISHED_SNAPSHOT_FILE));
        assert!(!err.contains(GENERATE_FAILURE_FILE));

        let failure_contents = std::fs::read_to_string(failure_report).unwrap();
        std::fs::write(
            output
                .join(".kslim")
                .join(crate::output_repo::PUBLISHED_SNAPSHOT_FILE),
            failure_contents,
        )
        .unwrap();
        let commit_with_failure_as_published =
            commit_all(&output, "commit failure report as published metadata");

        let err = crate::output_repo::load_committed_published_snapshot_metadata(
            &output_repo,
            &commit_with_failure_as_published,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("required committed published metadata is invalid"));
        assert!(err.contains(crate::output_repo::PUBLISHED_SNAPSHOT_FILE));
    }

    #[test]
    fn test_record_generate_failure_report_paths_are_relative_to_attempt_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let attempt = AttemptMetadataDir::new(project.join(".kslim/attempt")).unwrap();
        std::fs::create_dir_all(attempt.as_path()).unwrap();
        std::fs::write(
            attempt
                .as_path()
                .join(crate::output_repo::REDUCER_FAILURE_JSON),
            "{}",
        )
        .unwrap();
        std::fs::write(
            attempt
                .as_path()
                .join(crate::output_repo::REDUCER_REPORT_JSON),
            "{}",
        )
        .unwrap();
        std::fs::write(project.join(".kslim/report.txt"), "outside attempt").unwrap();

        record_generate_failure(
            None,
            GenerateStage::Reduce,
            &anyhow::anyhow!("reducer stopped"),
            &attempt,
        )
        .unwrap();

        let failure = read_failure(attempt.as_path());
        let paths = report_paths(&failure);
        assert_eq!(
            paths,
            [
                "last-attempt.json",
                "reducer-failure.json",
                "reducer-report.json",
                "report.txt"
            ]
        );
        assert_sorted_unique(&paths);
        for path in paths {
            let path = Path::new(path);
            assert!(!path.is_absolute());
            assert!(!path
                .components()
                .any(|component| !matches!(component, Component::Normal(_))));
            assert!(attempt.as_path().join(path).starts_with(attempt.as_path()));
        }
        assert!(!failure["report_paths"].to_string().contains("kslim.lock"));
        assert!(!failure["report_paths"].to_string().contains("../"));
    }

    #[test]
    fn test_record_generate_failure_omits_authoritative_metadata_claims() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let output = tmp.path().join("output");
        let config_path = project.join("kslim.toml");
        let plan = plan_for_test(&config_path, &output);
        let attempt = AttemptMetadataDir::new(project.join(".kslim/attempt")).unwrap();

        record_generate_failure(
            Some(&plan),
            GenerateStage::Publish,
            &anyhow::anyhow!("metadata publication failed before becoming authoritative"),
            &attempt,
        )
        .unwrap();

        let failure = read_failure(attempt.as_path());
        assert_eq!(failure["authoritative"].as_bool(), Some(false));
        assert_eq!(
            failure["metadata_scope"].as_str(),
            Some(NON_AUTHORITATIVE_ATTEMPT_SCOPE)
        );
        for key in AUTHORITATIVE_METADATA_CLAIM_KEYS {
            assert!(
                !toml_value_contains_key(&failure, key),
                "failure metadata must not include authoritative metadata claim key {key}"
            );
        }
        let serialized =
            std::fs::read_to_string(attempt.as_path().join(GENERATE_FAILURE_FILE)).unwrap();
        assert!(!serialized.contains("authoritative_metadata"));
        assert!(!serialized.contains("metadata_authoritative"));
        assert!(!serialized.contains("committed_metadata"));
        assert!(!serialized.contains("published_metadata"));
    }

    #[test]
    fn test_generate_failure_metadata_guard_rejects_authoritative_metadata_claim() {
        for serialized in [
            r#"
schema_version = 1
authoritative = true
"#,
            r#"
schema_version = 1

[authoritative_metadata]
path = ".git/kslim"
"#,
            r#"
schema_version = 1
metadata_scope = "published"
"#,
        ] {
            let err = ensure_no_authoritative_metadata_claims(serialized)
                .unwrap_err()
                .to_string();
            assert!(
                err.contains("must not include authoritative metadata claim"),
                "unexpected guard error: {err}"
            );
        }
    }

    #[test]
    fn test_record_generate_failure_omits_output_commit_claims() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let output = tmp.path().join("output");
        let config_path = project.join("kslim.toml");
        let plan = plan_for_test(&config_path, &output);
        let attempt = AttemptMetadataDir::new(project.join(".kslim/attempt")).unwrap();

        record_generate_failure(
            Some(&plan),
            GenerateStage::Commit,
            &anyhow::anyhow!("commit phase stopped before publication"),
            &attempt,
        )
        .unwrap();

        let failure = read_failure(attempt.as_path());
        for key in OUTPUT_COMMIT_CLAIM_KEYS {
            assert!(
                !toml_value_contains_key(&failure, key),
                "failure metadata must not include output commit claim key {key}"
            );
        }
        let serialized =
            std::fs::read_to_string(attempt.as_path().join(GENERATE_FAILURE_FILE)).unwrap();
        assert!(!serialized.contains("output_commit"));
        assert!(!serialized.contains("output_commit_claim"));
        assert!(!serialized.contains("committed_output_commit"));
        assert!(!serialized.contains("output_head"));
    }

    #[test]
    fn test_generate_failure_metadata_guard_rejects_output_commit_claim() {
        let err = ensure_no_output_commit_claims(
            r#"
schema_version = 1
output_commit = "deadbeef"
"#,
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("must not include output commit claim"));
    }

    #[test]
    fn test_record_generate_failure_omits_lockfile_update_claims() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let output = tmp.path().join("output");
        let config_path = project.join("kslim.toml");
        let plan = plan_for_test(&config_path, &output);
        let attempt = AttemptMetadataDir::new(project.join(".kslim/attempt")).unwrap();
        std::fs::write(project.join("kslim.lock"), "authoritative lockfile\n").unwrap();

        record_generate_failure(
            Some(&plan),
            GenerateStage::Publish,
            &anyhow::anyhow!("lockfile write failed before publication"),
            &attempt,
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(project.join("kslim.lock")).unwrap(),
            "authoritative lockfile\n"
        );
        let failure = read_failure(attempt.as_path());
        for key in LOCKFILE_UPDATE_CLAIM_KEYS {
            assert!(
                !toml_value_contains_key(&failure, key),
                "failure metadata must not include lockfile update claim key {key}"
            );
        }
        let serialized =
            std::fs::read_to_string(attempt.as_path().join(GENERATE_FAILURE_FILE)).unwrap();
        assert!(!serialized.contains("lockfile_update"));
        assert!(!serialized.contains("authoritative_lockfile"));
        assert!(!serialized.contains("published_lockfile"));
    }

    #[test]
    fn test_generate_failure_metadata_guard_rejects_lockfile_update_claim() {
        let err = ensure_no_lockfile_update_claims(
            r#"
schema_version = 1

[authoritative_lockfile]
updated = false
"#,
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("must not include lockfile update claim"));
    }

    #[test]
    fn test_record_generate_failure_omits_published_snapshot_identifiers() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let output = tmp.path().join("output");
        let config_path = project.join("kslim.toml");
        let plan = plan_for_test(&config_path, &output);
        let attempt = AttemptMetadataDir::new(project.join(".kslim/attempt")).unwrap();

        record_generate_failure(
            Some(&plan),
            GenerateStage::Publish,
            &anyhow::anyhow!("publish failed before snapshot became authoritative"),
            &attempt,
        )
        .unwrap();

        let failure = read_failure(attempt.as_path());
        assert!(failure.get("plan_id").is_some());
        for key in PUBLISHED_SNAPSHOT_IDENTIFIER_KEYS {
            assert!(
                !toml_value_contains_key(&failure, key),
                "failure metadata must not include published snapshot identifier key {key}"
            );
        }
        let serialized =
            std::fs::read_to_string(attempt.as_path().join(GENERATE_FAILURE_FILE)).unwrap();
        assert!(!serialized.contains("published_snapshot_id"));
        assert!(!serialized.contains("snapshot_id"));
    }

    #[test]
    fn test_generate_failure_metadata_guard_rejects_published_snapshot_identifier() {
        let err = ensure_no_published_snapshot_identifiers(
            r#"
schema_version = 1
published_snapshot_id = "deadbeef"
"#,
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("must not include published snapshot identifier"));
    }

    #[test]
    fn test_record_generate_failure_includes_plan_identity_when_plan_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let output = tmp.path().join("output");
        let config_path = project.join("kslim.toml");
        let plan = plan_for_test(&config_path, &output);
        let attempt = AttemptMetadataDir::new(project.join(".kslim/attempt")).unwrap();

        record_generate_failure(
            Some(&plan),
            GenerateStage::Reduce,
            &anyhow::anyhow!("reducer stopped"),
            &attempt,
        )
        .unwrap();

        let failure = read_failure(attempt.as_path());
        assert_eq!(failure["stage"].as_str(), Some("reduce"));
        assert_eq!(failure["error_kind"].as_str(), Some("reduce"));
        assert_eq!(failure["plan_id"].as_str(), Some(plan.plan_id.as_str()));
        assert_eq!(
            failure["plan_fingerprint"].as_str(),
            Some(plan.fingerprint.as_str())
        );
        assert_eq!(
            failure["tool_version"].as_str(),
            Some(plan.created_with.as_str())
        );
    }

    #[test]
    fn test_record_generate_failure_includes_command_context_for_selftest_command_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let attempt = AttemptMetadataDir::new(project.join(".kslim/attempt")).unwrap();
        let error = anyhow::Error::new(SelfTestFailure::Command {
            details: CapturedCommandFailure {
                command: String::from("make test"),
                target: None,
                arch: None,
                config: None,
                stdout: String::from("stdout from failed command"),
                stderr: String::from("stderr from failed command"),
                exit_status: Some(7),
                elapsed: Duration::from_millis(42),
            },
        });

        record_generate_failure(None, GenerateStage::Selftest, &error, &attempt).unwrap();

        let failure = read_failure(attempt.as_path());
        let command_context = failure["command_context"].as_table().unwrap();
        assert_eq!(command_context["kind"].as_str(), Some("selftest-command"));
        assert_eq!(command_context["command"].as_str(), Some("make test"));
        assert_eq!(command_context["exit_status"].as_integer(), Some(7));
        assert_eq!(command_context["elapsed_ms"].as_integer(), Some(42));
        assert!(command_context.get("stdout").is_none());
        assert!(command_context.get("stderr").is_none());
    }

    #[test]
    fn test_record_generate_failure_finds_command_context_through_fixed_point_error_chain() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let tree = tmp.path().join("tree");
        std::fs::create_dir_all(&tree).unwrap();
        let attempt = AttemptMetadataDir::new(project.join(".kslim/attempt")).unwrap();
        let mut profile = config::default_profile_config("v1.0");
        profile.selftests.check_kconfig_sources = false;
        profile.selftests.check_makefiles = false;
        profile.selftests.commands = vec![String::from("printf out; printf err >&2; exit 9")];
        profile.reducer.max_fixup_passes = 0;
        let mut reducer_stats = crate::reducer::ReducerStats::default();
        let kernel_root = KernelSourceRoot::new(&tree).unwrap();
        let error =
            crate::reducer::run_selftests_with_fixups(&kernel_root, &profile, &mut reducer_stats)
                .unwrap_err();

        record_generate_failure(None, GenerateStage::Selftest, &error, &attempt).unwrap();

        let failure = read_failure(attempt.as_path());
        let command_context = failure["command_context"].as_table().unwrap();
        assert_eq!(command_context["kind"].as_str(), Some("selftest-command"));
        assert_eq!(
            command_context["command"].as_str(),
            Some("printf out; printf err >&2; exit 9")
        );
        assert_eq!(command_context["exit_status"].as_integer(), Some(9));
    }

    #[test]
    fn test_record_generate_failure_includes_kernel_build_command_context() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let output_dir = tmp.path().join("build-output");
        let attempt = AttemptMetadataDir::new(project.join(".kslim/attempt")).unwrap();
        let error = anyhow::Error::new(SelfTestFailure::KernelBuild {
            label: String::from("x86-defconfig"),
            output_dir: output_dir.clone(),
            details: CapturedCommandFailure {
                command: String::from("make"),
                target: Some(String::from("olddefconfig vmlinux")),
                arch: Some(String::from("x86")),
                config: Some(String::from("olddefconfig")),
                stdout: String::new(),
                stderr: String::from("build failed"),
                exit_status: None,
                elapsed: Duration::from_millis(5),
            },
        });

        record_generate_failure(None, GenerateStage::Selftest, &error, &attempt).unwrap();

        let failure = read_failure(attempt.as_path());
        let command_context = failure["command_context"].as_table().unwrap();
        assert_eq!(command_context["kind"].as_str(), Some("kernel-build"));
        assert_eq!(command_context["command"].as_str(), Some("make"));
        assert_eq!(command_context["label"].as_str(), Some("x86-defconfig"));
        assert_eq!(
            command_context["output_dir"].as_str(),
            Some(output_dir.to_string_lossy().as_ref())
        );
        assert_eq!(
            command_context["target"].as_str(),
            Some("olddefconfig vmlinux")
        );
        assert_eq!(command_context["arch"].as_str(), Some("x86"));
        assert_eq!(command_context["config"].as_str(), Some("olddefconfig"));
        assert!(command_context.get("exit_status").is_none());
    }

    #[test]
    fn test_record_generate_failure_requires_attempt_metadata_dir_type() {
        let tmp = tempfile::tempdir().unwrap();
        let metadata_dir = tmp.path().join(".kslim");

        let err = AttemptMetadataDir::new(&metadata_dir)
            .unwrap_err()
            .to_string();

        assert!(err.contains("non-authoritative attempt dir"));
        assert!(!tmp.path().join(".kslim/generate-failure.toml").exists());
    }
}
