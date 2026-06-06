use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

use crate::{manifest, output_repo};

use super::super::plan::GeneratePlan;
use super::super::state::CandidateTreeState;
use super::report::ensure_report_path_is_relative_and_normalized;
use crate::model::TreeFingerprint;

pub(super) const CANDIDATE_METADATA_FILE: &str = "candidate.toml";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CandidateMetadataSummary {
    pub(super) schema_version: u32,
    pub(super) metadata_scope: String,
    pub(super) authoritative: bool,
    pub(super) plan_id: String,
    pub(super) plan_fingerprint: String,
    pub(super) tree_fingerprint: String,
    pub(super) config_content_hash: String,
    pub(super) generated_by: String,
    pub(super) selected_profile: String,
    pub(super) upstream_name: String,
    pub(super) base_ref: String,
    pub(super) base_commit: String,
    pub(super) base_resolved_at: String,
    pub(super) output_branch: String,
    pub(super) output_mode: String,
    pub(super) patch_source_count: usize,
    pub(super) patch_commit_count: usize,
    pub(super) integration_count: usize,
    pub(super) materialized: bool,
    pub(super) integrated: bool,
    pub(super) pruned: bool,
    pub(super) reduced: bool,
    pub(super) selftested: bool,
    pub(super) reducer_ran: bool,
    pub(super) manifest_file: String,
    pub(super) reducer_report_file: Option<String>,
}

pub(super) fn verify_no_host_only_absolute_paths_in_committed_candidate_metadata(
    metadata_dir: &Path,
) -> Result<()> {
    output_repo::validate_committed_metadata_has_no_host_absolute_paths(metadata_dir).with_context(
        || "verification failed: committed candidate metadata contains host-only absolute path",
    )
}

pub(super) fn verify_no_raw_logs_in_committed_candidate_metadata(
    metadata_dir: &Path,
) -> Result<()> {
    output_repo::validate_committed_metadata_has_no_raw_logs(metadata_dir)
        .with_context(|| "verification failed: committed candidate metadata contains raw logs")
}

pub(super) fn verify_no_temporary_paths_in_committed_candidate_metadata(
    metadata_dir: &Path,
    temporary_roots: &[&Path],
) -> Result<()> {
    output_repo::validate_committed_metadata_has_no_temporary_paths(metadata_dir, temporary_roots)
        .with_context(|| {
            "verification failed: committed candidate metadata contains temporary path"
        })
}

pub(super) fn verify_only_reproducible_timestamps_in_committed_candidate_metadata(
    metadata_dir: &Path,
    resolved_base_timestamp: &str,
) -> Result<()> {
    output_repo::validate_committed_metadata_has_only_allowed_reproducible_timestamps(
        metadata_dir,
        &[resolved_base_timestamp],
    )
    .with_context(|| {
        "verification failed: committed candidate metadata contains non-reproducible timestamp"
    })
}

pub(super) fn verify_candidate_metadata_complete(
    plan: &GeneratePlan,
    candidate: &CandidateTreeState,
    metadata: &CandidateMetadataSummary,
    tree_fingerprint: &TreeFingerprint,
) -> Result<()> {
    if metadata.schema_version != 1 {
        anyhow::bail!(
            "verification failed: candidate metadata schema_version must be 1, got {}",
            metadata.schema_version
        );
    }
    ensure_metadata_field_eq("metadata_scope", &metadata.metadata_scope, "candidate")?;
    if metadata.authoritative {
        anyhow::bail!("verification failed: candidate metadata must be non-authoritative");
    }

    ensure_metadata_field_eq("plan_id", &metadata.plan_id, plan.plan_id.as_str())?;
    ensure_metadata_field_eq(
        "plan_fingerprint",
        &metadata.plan_fingerprint,
        plan.fingerprint.as_str(),
    )?;
    ensure_metadata_field_eq(
        "tree_fingerprint",
        &metadata.tree_fingerprint,
        tree_fingerprint.as_str(),
    )?;
    ensure_metadata_field_eq(
        "config_content_hash",
        &metadata.config_content_hash,
        plan.config_content_hash.as_str(),
    )?;
    ensure_metadata_field_eq(
        "generated_by",
        &metadata.generated_by,
        plan.created_with.as_str(),
    )?;
    ensure_metadata_field_eq(
        "selected_profile",
        &metadata.selected_profile,
        plan.requested.selected_profile.as_str(),
    )?;
    ensure_metadata_field_eq(
        "upstream_name",
        &metadata.upstream_name,
        &plan.resolved.base.upstream,
    )?;
    ensure_metadata_field_eq("base_ref", &metadata.base_ref, &plan.resolved.base.r#ref)?;
    ensure_metadata_field_eq(
        "base_commit",
        &metadata.base_commit,
        &plan.resolved.base.commit,
    )?;
    ensure_metadata_field_eq(
        "base_resolved_at",
        &metadata.base_resolved_at,
        &plan.resolved.base.resolved_at,
    )?;
    output_repo::validate_reproducible_metadata_timestamp(
        "candidate metadata field base_resolved_at",
        &metadata.base_resolved_at,
    )?;
    ensure_metadata_field_eq(
        "output_branch",
        &metadata.output_branch,
        &plan.resolved.output_plan.branch,
    )?;
    ensure_metadata_field_eq(
        "output_mode",
        &metadata.output_mode,
        &plan.resolved.output_plan.mode,
    )?;
    ensure_metadata_count_eq(
        "patch_source_count",
        metadata.patch_source_count,
        plan.resolved.patch_plan.sources.len(),
    )?;
    ensure_metadata_count_eq(
        "patch_commit_count",
        metadata.patch_commit_count,
        plan.resolved.patch_plan.total_patch_count,
    )?;
    ensure_metadata_count_eq(
        "integration_count",
        metadata.integration_count,
        plan.resolved.integration_plan.entries.len(),
    )?;

    ensure_metadata_bool_eq(
        "materialized",
        metadata.materialized,
        candidate.materialized,
    )?;
    ensure_metadata_bool_eq("integrated", metadata.integrated, candidate.integrated)?;
    ensure_metadata_bool_eq("pruned", metadata.pruned, candidate.pruned)?;
    ensure_metadata_bool_eq("reduced", metadata.reduced, candidate.reduced)?;
    ensure_metadata_bool_eq("selftested", metadata.selftested, candidate.selftested)?;
    ensure_report_path_is_relative_and_normalized(
        "candidate metadata manifest_file",
        &metadata.manifest_file,
    )?;
    ensure_metadata_field_eq(
        "manifest_file",
        &metadata.manifest_file,
        manifest::OUTPUT_MANIFEST_FILE_NAME,
    )?;
    let manifest_path = candidate
        .metadata_dir
        .as_path()
        .join(&metadata.manifest_file);
    if !manifest_path.is_file() {
        anyhow::bail!(
            "verification failed: candidate metadata manifest file is missing: {}",
            manifest_path.display()
        );
    }

    match metadata.reducer_report_file.as_deref() {
        Some(report_file) => {
            ensure_report_path_is_relative_and_normalized(
                "candidate metadata reducer_report_file",
                report_file,
            )?;
            ensure_metadata_field_eq(
                "reducer_report_file",
                report_file,
                output_repo::REDUCER_REPORT_JSON,
            )?;
            if !metadata.reducer_ran {
                anyhow::bail!(
                    "verification failed: candidate metadata names a reducer report but reducer_ran is false"
                );
            }
        }
        None if metadata.reducer_ran => {
            anyhow::bail!(
                "verification failed: candidate metadata records reducer_ran without reducer_report_file"
            );
        }
        None => {}
    }

    Ok(())
}

fn ensure_metadata_field_eq(label: &str, actual: &str, expected: &str) -> Result<()> {
    if actual.trim().is_empty() {
        anyhow::bail!("verification failed: candidate metadata field {label} is empty");
    }
    if actual != expected {
        anyhow::bail!(
            "verification failed: candidate metadata field {label} mismatch: expected {expected:?}, got {actual:?}"
        );
    }
    Ok(())
}

fn ensure_metadata_count_eq(label: &str, actual: usize, expected: usize) -> Result<()> {
    if actual != expected {
        anyhow::bail!(
            "verification failed: candidate metadata field {label} mismatch: expected {expected}, got {actual}"
        );
    }
    Ok(())
}

fn ensure_metadata_bool_eq(label: &str, actual: bool, expected: bool) -> Result<()> {
    if actual != expected {
        anyhow::bail!(
            "verification failed: candidate metadata field {label} mismatch: expected {expected}, got {actual}"
        );
    }
    Ok(())
}

pub(super) fn read_candidate_metadata_summary(
    metadata_dir: &Path,
) -> Result<CandidateMetadataSummary> {
    let path = metadata_dir.join(CANDIDATE_METADATA_FILE);
    let metadata = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read candidate metadata {}", path.display()))?;
    toml::from_str(&metadata)
        .with_context(|| format!("failed to parse candidate metadata {}", path.display()))
}
