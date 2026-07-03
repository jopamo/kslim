//! Report artifact file names and metadata-relative report paths.
//!
//! This module owns names, path validation, and report authority boundaries.
//! Report rendering stays with the reducer/generate code that owns the data
//! being rendered.

use anyhow::{Context, Result};
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

use crate::paths::{
    AttemptMetadataDir, CandidateMetadataDir, OutputRepoPath, PublishedMetadataDir,
};

use super::metadata;

#[allow(dead_code)]
pub const REDUCER_REMOVAL_MANIFEST: &str = "removal-manifest.toml";
#[allow(dead_code)]
pub const REDUCER_REPORT_JSON: &str = "reducer-report.json";
#[allow(dead_code)]
pub const REDUCER_REPORT_MD: &str = "reducer-report.md";
#[allow(dead_code)]
pub const REDUCER_DIAGNOSTICS_JSON: &str = "diagnostics.json";
#[allow(dead_code)]
pub const REDUCER_EDIT_SUMMARY_JSON: &str = "edit-summary.json";
#[allow(dead_code)]
pub const REDUCER_KCONFIG_SOLVER_REPORT_JSON: &str = "kconfig-solver-report.json";
#[allow(dead_code)]
pub const REDUCER_KCONFIG_REWRITE_REPORT_JSON: &str = "kconfig-rewrite-report.json";
#[allow(dead_code)]
pub const REDUCER_SKIPPED_SITES_JSON: &str = "skipped-sites.json";
#[allow(dead_code)]
pub const MATRIX_REPORT_JSON: &str = "matrix-report.json";
#[allow(dead_code)]
pub const GENERATE_REPORT_JSON: &str = "generate-report.json";
#[allow(dead_code)]
pub const LAST_ATTEMPT_JSON: &str = "last-attempt.json";
#[allow(dead_code)]
pub const REDUCER_FAILURE_JSON: &str = "reducer-failure.json";
#[allow(dead_code)]
pub const NON_AUTHORITATIVE_ATTEMPT_SCOPE: &str = "non-authoritative-attempt";

const CANDIDATE_REPORT_FILES: &[&str] = &[
    REDUCER_REMOVAL_MANIFEST,
    REDUCER_REPORT_MD,
    REDUCER_REPORT_JSON,
    REDUCER_DIAGNOSTICS_JSON,
    REDUCER_EDIT_SUMMARY_JSON,
    REDUCER_KCONFIG_SOLVER_REPORT_JSON,
    REDUCER_KCONFIG_REWRITE_REPORT_JSON,
    REDUCER_SKIPPED_SITES_JSON,
    MATRIX_REPORT_JSON,
    GENERATE_REPORT_JSON,
];

const COMMITTED_REPORT_FILES: &[&str] = &[
    metadata::REPORT_FILE,
];

#[allow(dead_code)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CandidateReportCopySummary {
    pub(crate) files_copied: usize,
    pub(crate) files_missing: usize,
}

pub(crate) fn copy_candidate_reports_to_output_candidate_metadata(
    candidate_metadata: &CandidateMetadataDir,
    output_candidate: &OutputRepoPath,
) -> Result<CandidateReportCopySummary> {
    let output_metadata = metadata::published_metadata_dir(output_candidate)?;
    copy_candidate_reports_to_metadata_dir(candidate_metadata, &output_metadata)
}

fn copy_candidate_reports_to_metadata_dir(
    candidate_metadata: &CandidateMetadataDir,
    output_metadata: &PublishedMetadataDir,
) -> Result<CandidateReportCopySummary> {
    crate::fsutil::ensure_dir(output_metadata.as_path())?;

    let mut summary = CandidateReportCopySummary::default();
    for artifact in CANDIDATE_REPORT_FILES {
        let source = candidate_report_path(candidate_metadata, artifact)?;
        let destination = published_report_path(output_metadata, artifact)?;
        match std::fs::symlink_metadata(&source) {
            Ok(metadata) if metadata.file_type().is_file() => {
                std::fs::copy(&source, &destination).with_context(|| {
                    format!(
                        "failed to copy candidate report {} to output candidate metadata {}",
                        source.display(),
                        destination.display()
                    )
                })?;
                summary.files_copied += 1;
            }
            Ok(_) => {
                anyhow::bail!(
                    "candidate report is not a regular file: {}",
                    source.display()
                );
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                summary.files_missing += 1;
            }
            Err(err) => {
                return Err(err).with_context(|| {
                    format!("failed to inspect candidate report {}", source.display())
                })
            }
        }
    }

    Ok(summary)
}

pub(crate) fn validate_candidate_committed_reports_temporary_paths(
    metadata_dir: &CandidateMetadataDir,
    temporary_roots: &[&Path],
) -> Result<()> {
    validate_committed_report_artifacts_temporary_paths(metadata_dir.as_path(), temporary_roots)
}

fn validate_committed_report_artifacts_temporary_paths(
    metadata_dir: &Path,
    temporary_roots: &[&Path],
) -> Result<()> {
    metadata::validate_committed_metadata_named_files_have_no_temporary_paths(
        metadata_dir,
        COMMITTED_REPORT_FILES,
        temporary_roots,
        "committed report",
    )
}

#[allow(dead_code)]
pub(crate) fn candidate_report_path(
    metadata_dir: &CandidateMetadataDir,
    artifact_name: &str,
) -> Result<PathBuf> {
    metadata_report_path(metadata_dir.as_path(), artifact_name)
}

#[allow(dead_code)]
pub(crate) fn published_report_path(
    metadata_dir: &PublishedMetadataDir,
    artifact_name: &str,
) -> Result<PathBuf> {
    validate_published_report_file_name(artifact_name)?;
    Ok(metadata_dir.as_path().join(artifact_name))
}

#[allow(dead_code)]
pub(crate) fn attempt_last_attempt_report_path(
    metadata_dir: &AttemptMetadataDir,
) -> Result<PathBuf> {
    validate_report_file_name(LAST_ATTEMPT_JSON)?;
    Ok(metadata_dir.as_path().join(LAST_ATTEMPT_JSON))
}

pub(crate) fn output_report_path(
    output_repo: &OutputRepoPath,
    artifact_name: &str,
) -> Result<PathBuf> {
    let metadata_dir = metadata::published_metadata_dir(output_repo)?;
    published_report_path(&metadata_dir, artifact_name)
}

pub(crate) fn metadata_report_path(metadata_dir: &Path, artifact_name: &str) -> Result<PathBuf> {
    validate_regular_report_file_name(artifact_name)?;
    Ok(metadata_dir.join(artifact_name))
}

fn validate_published_report_file_name(artifact_name: &str) -> Result<()> {
    validate_regular_report_file_name(artifact_name)?;
    Ok(())
}

fn validate_regular_report_file_name(artifact_name: &str) -> Result<()> {
    validate_report_file_name(artifact_name)?;
    if artifact_name == LAST_ATTEMPT_JSON {
        anyhow::bail!(
            "report artifact '{}' is non-authoritative attempt metadata; \
             use the explicit attempt last-attempt path helper",
            artifact_name
        );
    }
    Ok(())
}

pub(crate) fn validate_last_attempt_json(serialized: &str) -> Result<()> {
    let value: Value = serde_json::from_str(serialized)
        .context("failed to validate last-attempt metadata before write")?;
    if value.get("authoritative").and_then(Value::as_bool) != Some(false) {
        anyhow::bail!("last-attempt metadata must declare authoritative=false");
    }
    if value.get("metadata_scope").and_then(Value::as_str) != Some(NON_AUTHORITATIVE_ATTEMPT_SCOPE)
    {
        anyhow::bail!(
            "last-attempt metadata must declare metadata_scope={}",
            NON_AUTHORITATIVE_ATTEMPT_SCOPE
        );
    }
    ensure_no_authoritative_json_claims(&value)?;
    Ok(())
}

fn ensure_no_authoritative_json_claims(value: &Value) -> Result<()> {
    if json_value_contains_true_bool_key(value, "authoritative") {
        anyhow::bail!("last-attempt metadata must not contain authoritative=true");
    }
    if json_value_contains_non_attempt_metadata_scope(value) {
        anyhow::bail!("last-attempt metadata must not claim an authoritative metadata scope");
    }
    for key in [
        "published_snapshot_id",
        "published_snapshot",
        "snapshot_id",
        "output_commit",
        "output_head",
        "committed_output_commit",
        "published_metadata",
        "published_metadata_file",
    ] {
        if json_value_contains_key(value, key) {
            anyhow::bail!(
                "last-attempt metadata must not include authoritative claim '{}'",
                key
            );
        }
    }
    if json_value_contains_true_updated_authoritative_lockfile(value) {
        anyhow::bail!("last-attempt metadata must not claim authoritative lockfile update");
    }
    Ok(())
}

fn json_value_contains_key(value: &Value, needle: &str) -> bool {
    match value {
        Value::Object(object) => object
            .iter()
            .any(|(key, value)| key == needle || json_value_contains_key(value, needle)),
        Value::Array(values) => values
            .iter()
            .any(|value| json_value_contains_key(value, needle)),
        _ => false,
    }
}

fn json_value_contains_true_bool_key(value: &Value, needle: &str) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            (key == needle && value.as_bool() == Some(true))
                || json_value_contains_true_bool_key(value, needle)
        }),
        Value::Array(values) => values
            .iter()
            .any(|value| json_value_contains_true_bool_key(value, needle)),
        _ => false,
    }
}

fn json_value_contains_non_attempt_metadata_scope(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            (key == "metadata_scope" && value.as_str() != Some(NON_AUTHORITATIVE_ATTEMPT_SCOPE))
                || json_value_contains_non_attempt_metadata_scope(value)
        }),
        Value::Array(values) => values
            .iter()
            .any(json_value_contains_non_attempt_metadata_scope),
        _ => false,
    }
}

fn json_value_contains_true_updated_authoritative_lockfile(value: &Value) -> bool {
    match value {
        Value::Object(object) => {
            let direct_claim = object
                .get("authoritative_lockfile")
                .and_then(|lockfile| lockfile.get("updated"))
                .and_then(Value::as_bool)
                == Some(true);
            direct_claim
                || object
                    .values()
                    .any(json_value_contains_true_updated_authoritative_lockfile)
        }
        Value::Array(values) => values
            .iter()
            .any(json_value_contains_true_updated_authoritative_lockfile),
        _ => false,
    }
}

pub(crate) fn validate_report_file_name(artifact_name: &str) -> Result<()> {
    if artifact_name.trim().is_empty() {
        anyhow::bail!("report artifact name must not be empty");
    }
    if artifact_name.contains('/') || artifact_name.contains('\\') {
        anyhow::bail!(
            "report artifact '{}' must be a single file name under the kslim metadata directory",
            artifact_name
        );
    }
    let path = Path::new(artifact_name);
    if path.is_absolute() {
        anyhow::bail!(
            "report artifact '{}' must be a single file name under the kslim metadata directory",
            artifact_name
        );
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        anyhow::bail!(
            "report artifact '{}' must be a normalized file name",
            artifact_name
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_report_file_names_match_metadata_layout() {
        assert_eq!(REDUCER_REMOVAL_MANIFEST, "removal-manifest.toml");
        assert_eq!(REDUCER_REPORT_JSON, "reducer-report.json");
        assert_eq!(REDUCER_DIAGNOSTICS_JSON, "diagnostics.json");
        assert_eq!(
            REDUCER_KCONFIG_SOLVER_REPORT_JSON,
            "kconfig-solver-report.json"
        );
        assert_eq!(
            REDUCER_KCONFIG_REWRITE_REPORT_JSON,
            "kconfig-rewrite-report.json"
        );
        assert_eq!(REDUCER_SKIPPED_SITES_JSON, "skipped-sites.json");
        assert_eq!(MATRIX_REPORT_JSON, "matrix-report.json");
        assert_eq!(GENERATE_REPORT_JSON, "generate-report.json");
        assert_eq!(LAST_ATTEMPT_JSON, "last-attempt.json");
    }

    #[test]
    fn committed_report_validation_rejects_temporary_workspace_paths() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("candidate");
        let metadata_dir = candidate.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        std::fs::write(
            metadata_dir.join(metadata::REPORT_FILE),
            format!("candidate: {}\n", candidate.display()),
        )
        .unwrap();
        let metadata_dir = CandidateMetadataDir::new(&metadata_dir).unwrap();

        let err = validate_candidate_committed_reports_temporary_paths(
            &metadata_dir,
            &[candidate.as_path()],
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("committed report"));
        assert!(err.contains("temporary path"));
        assert!(err.contains("non-authoritative attempt metadata"));
    }

    #[test]
    fn committed_report_validation_allows_attempt_only_last_attempt_paths() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("candidate");
        let metadata_dir = candidate.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        std::fs::write(
            metadata_dir.join(LAST_ATTEMPT_JSON),
            format!("{{\"candidate\":\"{}\"}}", candidate.display()),
        )
        .unwrap();
        let metadata_dir = CandidateMetadataDir::new(&metadata_dir).unwrap();

        validate_candidate_committed_reports_temporary_paths(&metadata_dir, &[candidate.as_path()])
            .unwrap();
    }

    #[test]
    fn copy_candidate_reports_to_output_candidate_metadata_copies_known_reports_only() {
        let temp = tempfile::tempdir().unwrap();
        let candidate_metadata = temp.path().join("candidate/.kslim");
        let output_candidate = temp.path().join("output");
        std::fs::create_dir_all(&candidate_metadata).unwrap();
        std::fs::write(
            candidate_metadata.join(REDUCER_REPORT_JSON),
            "{\"source\":\"candidate\"}\n",
        )
        .unwrap();
        std::fs::write(
            candidate_metadata.join(REDUCER_DIAGNOSTICS_JSON),
            "{\"diagnostics\":[]}\n",
        )
        .unwrap();
        std::fs::write(
            candidate_metadata.join(LAST_ATTEMPT_JSON),
            "{\"authoritative\":false}\n",
        )
        .unwrap();
        let candidate_metadata = CandidateMetadataDir::new(&candidate_metadata).unwrap();
        let output_candidate = OutputRepoPath::new(&output_candidate).unwrap();

        let summary = copy_candidate_reports_to_output_candidate_metadata(
            &candidate_metadata,
            &output_candidate,
        )
        .unwrap();

        assert_eq!(summary.files_copied, 2);
        assert_eq!(summary.files_missing, CANDIDATE_REPORT_FILES.len() - 2);
        assert_eq!(
            std::fs::read_to_string(
                output_candidate
                    .as_path()
                    .join(".kslim")
                    .join(REDUCER_REPORT_JSON),
            )
            .unwrap(),
            "{\"source\":\"candidate\"}\n"
        );
        assert!(
            !output_candidate
                .as_path()
                .join(".kslim")
                .join(LAST_ATTEMPT_JSON)
                .exists(),
            "non-authoritative attempt metadata must not be copied into output metadata"
        );
    }

    #[test]
    fn copy_candidate_reports_to_output_candidate_metadata_rejects_non_files() {
        let temp = tempfile::tempdir().unwrap();
        let candidate_metadata = temp.path().join("candidate/.kslim");
        let output_candidate = temp.path().join("output");
        std::fs::create_dir_all(candidate_metadata.join(REDUCER_REPORT_JSON)).unwrap();
        let candidate_metadata = CandidateMetadataDir::new(&candidate_metadata).unwrap();
        let output_candidate = OutputRepoPath::new(&output_candidate).unwrap();

        let err = copy_candidate_reports_to_output_candidate_metadata(
            &candidate_metadata,
            &output_candidate,
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("not a regular file"));
    }

    #[test]
    fn metadata_report_path_accepts_single_file_names_only() {
        let metadata_dir = Path::new("/tmp/output/.kslim");

        assert_eq!(
            metadata_report_path(metadata_dir, REDUCER_REPORT_JSON).unwrap(),
            metadata_dir.join(REDUCER_REPORT_JSON)
        );

        for invalid in [
            "",
            "   ",
            ".",
            "..",
            "/tmp/report.json",
            "reports/reducer-report.json",
            "reports\\reducer-report.json",
        ] {
            let err = metadata_report_path(metadata_dir, invalid)
                .unwrap_err()
                .to_string();
            assert!(
                err.contains("single file name")
                    || err.contains("must not be empty")
                    || err.contains("normalized"),
                "unexpected error for {invalid:?}: {err}"
            );
        }
    }

    #[test]
    fn metadata_report_path_rejects_implicit_last_attempt_metadata() {
        let metadata_dir = Path::new("/tmp/output/.kslim");

        let err = metadata_report_path(metadata_dir, LAST_ATTEMPT_JSON)
            .unwrap_err()
            .to_string();

        assert!(err.contains("explicit attempt last-attempt path helper"));
    }

    #[test]
    fn published_report_path_rejects_last_attempt_metadata() {
        let output_repo = OutputRepoPath::new("/tmp/output").unwrap();
        let metadata_dir =
            PublishedMetadataDir::new_in_output_repo(&output_repo, "/tmp/output/.kslim").unwrap();

        let err = published_report_path(&metadata_dir, LAST_ATTEMPT_JSON)
            .unwrap_err()
            .to_string();

        assert!(err.contains("non-authoritative attempt metadata"));
    }

    #[test]
    fn attempt_last_attempt_report_path_requires_attempt_metadata_dir() {
        let attempt_dir = AttemptMetadataDir::new("/tmp/project/.kslim/attempt").unwrap();
        assert_eq!(
            attempt_last_attempt_report_path(&attempt_dir).unwrap(),
            Path::new("/tmp/project/.kslim/attempt").join(LAST_ATTEMPT_JSON)
        );

        let err = AttemptMetadataDir::new("/tmp/candidate/.kslim")
            .unwrap_err()
            .to_string();

        assert!(err.contains("non-authoritative attempt dir"));
    }

    #[test]
    fn validate_last_attempt_json_accepts_attempt_scope_only() {
        validate_last_attempt_json(
            r#"{
              "authoritative": false,
              "metadata_scope": "non-authoritative-attempt",
              "authoritative_lockfile": {"path": "kslim.lock", "updated": false}
            }"#,
        )
        .unwrap();

        for (json, expected) in [
            (
                r#"{"authoritative":true,"metadata_scope":"non-authoritative-attempt"}"#,
                "authoritative=false",
            ),
            (
                r#"{"authoritative":false,"metadata_scope":"published"}"#,
                "metadata_scope=non-authoritative-attempt",
            ),
            (
                concat!(
                    r#"{"authoritative":false,"#,
                    r#""metadata_scope":"non-authoritative-attempt","#,
                    r#""authoritative_lockfile":{"updated":true}}"#,
                ),
                "authoritative lockfile update",
            ),
            (
                concat!(
                    r#"{"authoritative":false,"#,
                    r#""metadata_scope":"non-authoritative-attempt","#,
                    r#""output_commit":"abc"}"#,
                ),
                "output_commit",
            ),
        ] {
            let err = validate_last_attempt_json(json).unwrap_err().to_string();
            assert!(
                err.contains(expected),
                "expected {expected:?} in validation error {err:?}"
            );
        }
    }

    #[test]
    fn output_report_path_uses_published_metadata_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let output_repo = OutputRepoPath::new(&output).unwrap();

        assert_eq!(
            output_report_path(&output_repo, REDUCER_REMOVAL_MANIFEST).unwrap(),
            output.join(".kslim").join(REDUCER_REMOVAL_MANIFEST)
        );

        std::fs::create_dir_all(output.join(".git")).unwrap();
        assert_eq!(
            output_report_path(&output_repo, REDUCER_REPORT_JSON).unwrap(),
            output.join(".git/kslim").join(REDUCER_REPORT_JSON)
        );
    }
}
