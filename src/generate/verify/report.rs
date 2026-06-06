use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Component, Path};

use crate::output_repo;

use super::super::plan::GeneratePlan;
use super::super::state::CandidateTreeState;
use super::metadata::{CandidateMetadataSummary, CANDIDATE_METADATA_FILE};

#[derive(Debug, Deserialize)]
pub(super) struct ReducerReportFile {
    pub(super) summary: ReducerReportSummary,
    pub(super) unsupported_fallout: ReducerUnsupportedFallout,
    pub(super) artifacts: ReducerReportArtifacts,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReducerReportSummary {
    pub(super) edit_records: usize,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReducerUnsupportedFallout {
    pub(super) unsupported_kconfig_expressions: usize,
    pub(super) unsupported_cpp_expressions: usize,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReducerReportArtifacts {
    pub(super) markdown: String,
    pub(super) summary_json: String,
    pub(super) diagnostics_json: String,
    pub(super) edit_summary_json: String,
    pub(super) kconfig_solver_report_json: String,
    pub(super) kconfig_rewrite_report_json: String,
    pub(super) skipped_sites_json: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReducerEditSummaryFile {
    pub(super) edit_records: usize,
    pub(super) edit_record_details: Vec<ReducerEditRecord>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReducerDiagnosticsFile {
    #[serde(default)]
    pub(super) classified_diagnostics: Vec<ReducerClassifiedDiagnostic>,
    #[serde(default)]
    pub(super) unknown_diagnostics: Vec<ReducerClassifiedDiagnostic>,
    #[serde(default)]
    pub(super) consumed_diagnostics: Vec<ReducerConsumedDiagnostic>,
    #[serde(default)]
    pub(super) skipped_diagnostics: Vec<ReducerSkippedFixupDiagnostic>,
    pub(super) unsupported_kconfig_expressions: Vec<ReducerUnsupportedSyntaxSite>,
    pub(super) unsupported_cpp_expressions: Vec<ReducerUnsupportedSyntaxSite>,
    pub(super) skipped_cpp_nested_edge_cases: Vec<ReducerPathSite>,
    pub(super) ambiguous_makefile_lines: Vec<ReducerPathSite>,
    pub(super) skipped_fixup_diagnostics: Vec<ReducerSkippedFixupDiagnostic>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReducerUnsupportedSyntaxSite {
    pub(super) kind: String,
    pub(super) file: String,
    pub(super) line: usize,
    pub(super) directive: String,
    pub(super) expression: String,
    pub(super) reason: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReducerPathSite {
    pub(super) file: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReducerSkippedFixupDiagnostic {
    pub(super) reason: String,
    pub(super) diagnostic: ReducerClassifiedDiagnostic,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReducerConsumedDiagnostic {
    pub(super) diagnostic: ReducerClassifiedDiagnostic,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReducerClassifiedDiagnostic {
    pub(super) class: String,
    pub(super) file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReducerEditRecord {
    pub(super) file: String,
    pub(super) pass_name: String,
    pub(super) edit_kind: String,
    pub(super) edit_reason: ReducerEditTruth,
    pub(super) proof_source: ReducerEditTruth,
    pub(super) old: ReducerEditOld,
    #[serde(rename = "new")]
    pub(super) new_value: ReducerEditNew,
    pub(super) idempotence_marker: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReducerEditTruth {
    pub(super) kind: String,
    pub(super) payload: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReducerEditOld {
    pub(super) line_start: Option<usize>,
    pub(super) line_end: Option<usize>,
    pub(super) logical_item: String,
    pub(super) byte_len: usize,
    pub(super) sha256: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReducerEditNew {
    pub(super) logical_item: String,
    pub(super) byte_len: usize,
    pub(super) sha256: String,
}

pub(super) fn verify_candidate_reports(
    plan: &GeneratePlan,
    candidate: &CandidateTreeState,
    metadata: &CandidateMetadataSummary,
) -> Result<bool> {
    let metadata_file = candidate
        .metadata_dir
        .as_path()
        .join(CANDIDATE_METADATA_FILE);
    if !metadata_file.is_file() {
        anyhow::bail!(
            "verification failed: candidate metadata file is missing: {}",
            metadata_file.display()
        );
    }

    let reducer_report_required = super::reducer_artifacts_required(plan, metadata);
    if !reducer_report_required {
        return Ok(true);
    }
    let Some(report_file) = metadata.reducer_report_file.as_deref() else {
        anyhow::bail!(
            "verification failed: reducer report is required but candidate metadata does not name it"
        );
    };
    if report_file != output_repo::REDUCER_REPORT_JSON {
        anyhow::bail!(
            "verification failed: candidate reducer report must be {}",
            output_repo::REDUCER_REPORT_JSON
        );
    }

    let report_path = candidate.metadata_dir.as_path().join(report_file);
    if !report_path.is_file() {
        anyhow::bail!(
            "verification failed: reducer report is missing: {}",
            report_path.display()
        );
    }
    Ok(true)
}

pub(super) fn verify_report_paths_are_relative_and_normalized(
    plan: &GeneratePlan,
    candidate: &CandidateTreeState,
    metadata: &CandidateMetadataSummary,
) -> Result<()> {
    ensure_report_path_is_relative_and_normalized(
        "candidate metadata manifest_file",
        &metadata.manifest_file,
    )?;
    if let Some(report_file) = metadata.reducer_report_file.as_deref() {
        ensure_report_path_is_relative_and_normalized(
            "candidate metadata reducer_report_file",
            report_file,
        )?;
    }

    if !super::reducer_artifacts_required(plan, metadata) {
        return Ok(());
    }

    let reducer_report = read_reducer_report(candidate.metadata_dir.as_path())?;
    verify_report_artifact_path(
        "reducer report artifact markdown",
        &reducer_report.artifacts.markdown,
        output_repo::REDUCER_REPORT_MD,
    )?;
    verify_report_artifact_path(
        "reducer report artifact summary_json",
        &reducer_report.artifacts.summary_json,
        output_repo::REDUCER_REPORT_JSON,
    )?;
    verify_report_artifact_path(
        "reducer report artifact diagnostics_json",
        &reducer_report.artifacts.diagnostics_json,
        output_repo::REDUCER_DIAGNOSTICS_JSON,
    )?;
    verify_report_artifact_path(
        "reducer report artifact edit_summary_json",
        &reducer_report.artifacts.edit_summary_json,
        output_repo::REDUCER_EDIT_SUMMARY_JSON,
    )?;
    verify_report_artifact_path(
        "reducer report artifact kconfig_solver_report_json",
        &reducer_report.artifacts.kconfig_solver_report_json,
        output_repo::REDUCER_KCONFIG_SOLVER_REPORT_JSON,
    )?;
    verify_report_artifact_path(
        "reducer report artifact kconfig_rewrite_report_json",
        &reducer_report.artifacts.kconfig_rewrite_report_json,
        output_repo::REDUCER_KCONFIG_REWRITE_REPORT_JSON,
    )?;
    if let Some(skipped_sites_json) = reducer_report.artifacts.skipped_sites_json.as_deref() {
        verify_report_artifact_path(
            "reducer report artifact skipped_sites_json",
            skipped_sites_json,
            output_repo::REDUCER_SKIPPED_SITES_JSON,
        )?;
    }

    let edit_summary = read_reducer_edit_summary(candidate.metadata_dir.as_path())?;
    for edit in &edit_summary.edit_record_details {
        ensure_report_path_is_relative_and_normalized("reducer edit record file", &edit.file)?;
    }

    let diagnostics = read_reducer_diagnostics(candidate.metadata_dir.as_path())?;
    let _solver_report = read_kconfig_solver_report(candidate.metadata_dir.as_path())?;
    let _rewrite_report = read_kconfig_rewrite_report(candidate.metadata_dir.as_path())?;
    for diagnostic in &diagnostics.classified_diagnostics {
        verify_diagnostic_file_path("reducer classified diagnostic file", diagnostic)?;
    }
    for diagnostic in &diagnostics.unknown_diagnostics {
        verify_diagnostic_file_path("reducer unknown diagnostic file", diagnostic)?;
    }
    for consumed in &diagnostics.consumed_diagnostics {
        verify_diagnostic_file_path("reducer consumed diagnostic file", &consumed.diagnostic)?;
    }
    for skipped in &diagnostics.skipped_diagnostics {
        verify_diagnostic_file_path("reducer skipped diagnostic file", &skipped.diagnostic)?;
    }
    for site in &diagnostics.unsupported_kconfig_expressions {
        ensure_report_path_is_relative_and_normalized(
            "reducer unsupported Kconfig expression file",
            &site.file,
        )?;
    }
    for site in &diagnostics.unsupported_cpp_expressions {
        ensure_report_path_is_relative_and_normalized(
            "reducer unsupported preprocessor expression file",
            &site.file,
        )?;
    }
    for site in &diagnostics.skipped_cpp_nested_edge_cases {
        ensure_report_path_is_relative_and_normalized(
            "reducer skipped preprocessor nested edge case file",
            &site.file,
        )?;
    }
    for site in &diagnostics.ambiguous_makefile_lines {
        ensure_report_path_is_relative_and_normalized(
            "reducer ambiguous Makefile line file",
            &site.file,
        )?;
    }
    for skipped in &diagnostics.skipped_fixup_diagnostics {
        if let Some(file) = skipped.diagnostic.file.as_deref() {
            ensure_report_path_is_relative_and_normalized(
                "reducer skipped fixup diagnostic file",
                file,
            )?;
        }
    }

    Ok(())
}

fn verify_diagnostic_file_path(
    label: &str,
    diagnostic: &ReducerClassifiedDiagnostic,
) -> Result<()> {
    if let Some(file) = diagnostic.file.as_deref() {
        ensure_report_path_is_relative_and_normalized(label, file)?;
    }
    Ok(())
}

fn verify_report_artifact_path(label: &str, actual: &str, expected: &str) -> Result<()> {
    ensure_report_path_is_relative_and_normalized(label, actual)?;
    if actual != expected {
        anyhow::bail!("verification failed: {label} must be {expected:?}, got {actual:?}");
    }
    Ok(())
}

pub(super) fn ensure_report_path_is_relative_and_normalized(
    label: &str,
    value: &str,
) -> Result<()> {
    if value.is_empty() || value.trim() != value {
        anyhow::bail!("verification failed: {label} must be a relative normalized path");
    }
    if value.contains('\\') || has_windows_drive_prefix(value) {
        anyhow::bail!(
            "verification failed: {label} must be a relative normalized path, got {value:?}"
        );
    }
    let path = Path::new(value);
    if path.is_absolute() {
        anyhow::bail!(
            "verification failed: {label} must be a relative normalized path, got {value:?}"
        );
    }

    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let part = part.to_str().ok_or_else(|| {
                    anyhow::anyhow!(
                        "verification failed: {label} must be valid UTF-8 relative path"
                    )
                })?;
                parts.push(part);
            }
            _ => {
                anyhow::bail!(
                    "verification failed: {label} must be a relative normalized path, got {value:?}"
                );
            }
        }
    }

    if parts.is_empty() || parts.join("/") != value {
        anyhow::bail!(
            "verification failed: {label} must be a relative normalized path, got {value:?}"
        );
    }
    Ok(())
}

fn has_windows_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic()
}

pub(super) fn read_reducer_report(metadata_dir: &Path) -> Result<ReducerReportFile> {
    read_json_file(metadata_dir, output_repo::REDUCER_REPORT_JSON)
}

pub(super) fn read_reducer_edit_summary(metadata_dir: &Path) -> Result<ReducerEditSummaryFile> {
    read_json_file(metadata_dir, output_repo::REDUCER_EDIT_SUMMARY_JSON)
}

pub(super) fn read_reducer_diagnostics(metadata_dir: &Path) -> Result<ReducerDiagnosticsFile> {
    read_json_file(metadata_dir, output_repo::REDUCER_DIAGNOSTICS_JSON)
}

pub(super) fn read_kconfig_solver_report(metadata_dir: &Path) -> Result<serde_json::Value> {
    read_json_file(metadata_dir, output_repo::REDUCER_KCONFIG_SOLVER_REPORT_JSON)
}

pub(super) fn read_kconfig_rewrite_report(metadata_dir: &Path) -> Result<serde_json::Value> {
    read_json_file(metadata_dir, output_repo::REDUCER_KCONFIG_REWRITE_REPORT_JSON)
}

fn read_json_file<T: for<'de> Deserialize<'de>>(metadata_dir: &Path, file_name: &str) -> Result<T> {
    let path = metadata_dir.join(file_name);
    let contents = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "failed to read candidate reducer artifact {}",
            path.display()
        )
    })?;
    serde_json::from_str(&contents).with_context(|| {
        format!(
            "failed to parse candidate reducer artifact {}",
            path.display()
        )
    })
}
