//! Verified candidate publication into the output repository.
//!
//! This module consumes candidate-phase verification proof and turns it into a
//! committed output snapshot. It must not resolve upstream refs or mutate
//! requested/candidate state.

mod stage;

use anyhow::{Context, Result};
use std::path::{Component, Path, PathBuf};

use crate::config::{KslimConfig, ProfileConfig};
use crate::lockfile::{self, PublishedLockfileUpdate, ResolvedBase};
use crate::patches;
use crate::paths::{CandidateTreePath, LockfilePath, OutputRepoPath};
use crate::reducer;
use crate::selftest::SelfTestResult;
use crate::{manifest, output_repo};

use super::plan::GeneratePlan;
use super::state::{CandidateTreeState, CommittedOutputSnapshot, PublishedSnapshotState};
use super::verify::{self, CandidateVerification, VerifiedGeneratedOutput};
use super::{
    capture_output_repo_failure_atomic_state, log_generate_stage,
    reducer_manifest_for_profile, rollback_output_repo_failure_atomic_state, set_generate_stage,
    FailureReportContext, GenerateOptions, GenerateResult, GenerateStage, GeneratedArtifacts,
};
#[allow(unused_imports)]
pub(crate) use stage::PublishStage;

#[allow(dead_code)]
pub(crate) const FIXED_PUBLISH_PIPELINE: &[&str] = &[
    PublishStage::CheckOutputPlan.description(),
    PublishStage::CheckOutputSafety.description(),
    PublishStage::ReverifyCandidate.description(),
    PublishStage::StageOutputCandidate.description(),
    PublishStage::ReverifyStagedCandidate.description(),
    PublishStage::BuildPublishedMetadata.description(),
    PublishStage::CaptureOutputRollback.description(),
    PublishStage::CreateOutputBranch.description(),
    PublishStage::SyncOutputTree.description(),
    PublishStage::SyncOutputMetadata.description(),
    PublishStage::WritePublishedMetadata.description(),
    PublishStage::CommitOutput.description(),
    PublishStage::BuildPublishedSnapshot.description(),
    PublishStage::UpdateAuthoritativeLockfile.description(),
];

fn log_publish_stage(stage: PublishStage) {
    log::info!("generate.publish: stage={}", stage.as_str());
}

/// Published metadata coupled to a concrete candidate-verification proof.
///
/// The raw metadata schema is not enough to make publication safe. This wrapper
/// is constructible only inside the generate lifecycle, after
/// `verify::CandidateVerification` exists.
#[derive(Debug)]
pub(crate) struct VerifiedPublishedSnapshotMetadata {
    metadata: output_repo::PublishedSnapshotMetadata,
    proof: PublishedSnapshotMetadataProof,
}

#[derive(Debug)]
enum PublishedSnapshotMetadataProof {
    CandidateVerification {
        metadata_fingerprint: String,
        reducer_ok: bool,
        selftest_ok: bool,
        report_ok: bool,
    },
}

impl VerifiedPublishedSnapshotMetadata {
    pub(in crate::generate) fn from_candidate_verification(
        metadata: output_repo::PublishedSnapshotMetadata,
        verification: &verify::CandidateVerification,
    ) -> Result<Self> {
        if !verification.all_checks_ok() {
            anyhow::bail!("cannot write published metadata from failed candidate verification");
        }
        let metadata_fingerprint = verification.metadata_fingerprint().as_str();
        if metadata.candidate_metadata_fingerprint.as_deref() != Some(metadata_fingerprint) {
            anyhow::bail!(
                "cannot write published metadata: metadata fingerprint does not match candidate verification proof"
            );
        }
        Ok(Self {
            metadata,
            proof: PublishedSnapshotMetadataProof::CandidateVerification {
                metadata_fingerprint: metadata_fingerprint.to_string(),
                reducer_ok: verification.reducer_ok(),
                selftest_ok: verification.selftest_ok(),
                report_ok: verification.report_ok(),
            },
        })
    }

    pub(crate) fn metadata(&self) -> &output_repo::PublishedSnapshotMetadata {
        &self.metadata
    }

    pub(crate) fn proof_summary(&self) -> String {
        match &self.proof {
            PublishedSnapshotMetadataProof::CandidateVerification {
                metadata_fingerprint,
                reducer_ok,
                selftest_ok,
                report_ok,
            } => format!(
                "candidate-verification:{}:reducer={}:selftest={}:report={}",
                metadata_fingerprint, reducer_ok, selftest_ok, report_ok
            ),
        }
    }
}

/// Proof that the output commit phase completed successfully.
///
/// `committed` may be false for an idempotent no-op commit, but this value is
/// produced only after the commit phase has synchronized metadata, selected the
/// output branch, and read a concrete output HEAD.
pub(crate) struct SuccessfulCommitResult {
    pub(crate) committed: bool,
    pub(crate) branch: String,
    pub(crate) tag: String,
    pub(crate) output_commit: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct OutputRepo {
    path: OutputRepoPath,
    force: bool,
}

#[allow(dead_code)]
impl OutputRepo {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Result<Self> {
        Self::with_force(path, false)
    }

    pub(crate) fn with_force(path: impl Into<PathBuf>, force: bool) -> Result<Self> {
        Ok(Self {
            path: OutputRepoPath::new(path)?,
            force,
        })
    }
}

#[allow(dead_code)]
pub(crate) fn commit_verified_candidate(
    plan: &GeneratePlan,
    candidate: CandidateTreeState,
    verification: CandidateVerification,
    output_repo: &mut OutputRepo,
) -> Result<PublishedSnapshotState> {
    log_publish_stage(PublishStage::CheckOutputPlan);
    ensure_output_repo_matches_plan(plan, output_repo)?;
    log_publish_stage(PublishStage::CheckOutputSafety);
    recheck_output_repo_safety(output_repo)?;
    log_publish_stage(PublishStage::ReverifyCandidate);
    ensure_verification_still_matches_candidate(plan, &candidate, &verification)?;

    log_publish_stage(PublishStage::StageOutputCandidate);
    let output_candidate = tempfile::Builder::new()
        .prefix("kslim-final-output-candidate-")
        .tempdir()?;
    let output_candidate_repo = OutputRepoPath::new(output_candidate.path())?;
    output_repo::sync_working_tree(&output_candidate_repo, &candidate.tree)?;
    output_repo::sync_candidate_metadata_dir(&output_candidate_repo, &candidate.tree)?;

    let staged_tree = CandidateTreePath::new(output_candidate.path())?;
    let staged_metadata = output_repo::candidate_metadata_dir(&staged_tree)?;
    let staged_candidate = CandidateTreeState::new(
        staged_tree,
        staged_metadata,
        candidate.materialized,
        candidate.integrated,
        candidate.pruned,
        candidate.reduced,
        candidate.selftested,
    )?;
    log_publish_stage(PublishStage::ReverifyStagedCandidate);
    ensure_verification_still_matches_candidate(plan, &staged_candidate, &verification)?;
    log_publish_stage(PublishStage::BuildPublishedMetadata);
    let published_metadata =
        verified_published_metadata_from_candidate_verification(plan, &verification)?;

    let output_path = output_repo_path_str(output_repo)?;
    log_publish_stage(PublishStage::CaptureOutputRollback);
    let output_transaction =
        capture_output_repo_failure_atomic_state(output_path, &plan.resolved.output_plan.branch)?;
    let publish_result = (|| -> Result<PublishedSnapshotState> {
        log_publish_stage(PublishStage::CreateOutputBranch);
        crate::git::create_branch(output_path, &plan.resolved.output_plan.branch)?;
        log_publish_stage(PublishStage::SyncOutputTree);
        output_repo::sync_working_tree(&output_repo.path, &staged_candidate.tree)?;
        log_publish_stage(PublishStage::SyncOutputMetadata);
        output_repo::sync_candidate_metadata_dir(&output_repo.path, &staged_candidate.tree)?;
        output_repo::sync_candidate_committed_metadata_dir(
            &output_repo.path,
            &staged_candidate.tree,
        )?;
        log_publish_stage(PublishStage::WritePublishedMetadata);
        output_repo::write_verified_published_snapshot_metadata(
            &output_repo.path,
            &published_metadata,
        )?;
        output_repo::write_verified_committed_published_snapshot_metadata(
            &output_repo.path,
            &published_metadata,
            &[candidate.tree.as_path(), staged_candidate.tree.as_path()],
        )?;

        log_publish_stage(PublishStage::CommitOutput);
        let committed =
            crate::git::commit_if_changed(output_path, &commit_message(plan, &verification))?;
        let output_commit = crate::git::head_commit(output_path)?;
        log_publish_stage(PublishStage::BuildPublishedSnapshot);
        // Build published snapshot state only after the output commit phase
        // succeeds and the concrete output HEAD is known.
        let commit = SuccessfulCommitResult {
            committed,
            branch: plan.resolved.output_plan.branch.clone(),
            tag: tag_name(plan),
            output_commit,
        };
        let lockfile_path = lockfile_path_for_plan(plan)?;
        let committed = CommittedOutputSnapshot::from_successful_commit(
            output_repo.path.as_path(),
            lockfile_path,
            &commit,
        )?;
        let snapshot = PublishedSnapshotState::from_committed_output(committed)?;
        write_authoritative_lockfile_from_committed_publish(
            plan,
            output_repo,
            &snapshot,
            &verification,
        )?;
        Ok(snapshot)
    })();

    match publish_result {
        Ok(snapshot) => Ok(snapshot),
        Err(err) => {
            if let Err(rollback_err) =
                rollback_output_repo_failure_atomic_state(output_path, &output_transaction)
            {
                anyhow::bail!(
                    "output publish failed: {:#}; rollback also failed: {:#}",
                    err,
                    rollback_err
                );
            }
            Err(err)
        }
    }
}

fn write_authoritative_lockfile_from_committed_publish(
    plan: &GeneratePlan,
    output_repo: &OutputRepo,
    snapshot: &PublishedSnapshotState,
    verification: &CandidateVerification,
) -> Result<()> {
    log_publish_stage(PublishStage::UpdateAuthoritativeLockfile);
    let project_root = project_root_for_plan(plan);
    let lockfile_path = LockfilePath::new_in_project_root(&project_root)?;
    let rollback = lockfile::capture_lockfile_failure_atomic_state(&lockfile_path)?;
    let result = (|| -> Result<()> {
        let output_path = output_repo_path_str(output_repo)?;
        let current_branch = crate::git::current_branch(output_path)?;
        if current_branch != snapshot.branch().as_str() {
            anyhow::bail!(
                "cannot update authoritative lockfile: output branch '{}' does not match committed snapshot branch '{}'",
                current_branch,
                snapshot.branch().as_str()
            );
        }
        let current_commit = crate::git::head_commit(output_path)?;
        if current_commit != snapshot.commit().as_str() {
            anyhow::bail!(
                "cannot update authoritative lockfile: output HEAD '{}' does not match committed snapshot '{}'",
                current_commit,
                snapshot.commit().as_str()
            );
        }

        let published = output_repo::load_committed_published_snapshot_metadata(
            &output_repo.path,
            snapshot.commit().as_str(),
        )?;
        if published.branch != snapshot.branch().as_str()
            || published.branch != plan.resolved.output_plan.branch
            || published.tag != tag_name(plan)
            || published.base_ref != plan.resolved.base.r#ref
            || published.base_commit != plan.resolved.base.commit
            || published.profile != plan.requested.selected_profile.as_str()
            || published.mode != plan.resolved.output_plan.mode
            || published.generated_at != plan.resolved.base.resolved_at
            || published.candidate_metadata_fingerprint.as_deref()
                != Some(verification.metadata_fingerprint().as_str())
        {
            anyhow::bail!(
                "cannot update authoritative lockfile: committed published metadata does not match verified publish plan"
            );
        }
        if crate::git::is_dirty(output_path)? {
            anyhow::bail!("cannot update authoritative lockfile from dirty output repository");
        }

        let update = lockfile::PublishedLockfileUpdate::new(
            plan.resolved.base.clone(),
            lockfile::PublishedLockfile {
                output_branch: published.branch,
                output_commit: current_commit,
                tag: published.tag,
                base_ref: published.base_ref,
                base_commit: published.base_commit,
                profile: published.profile,
                mode: published.mode,
                generated_at: published.generated_at,
            },
        )?;
        lockfile::write_published_lockfile(&lockfile_path, &update).with_context(|| {
            format!(
                "failed to update authoritative lockfile {}",
                lockfile_path.as_path().display()
            )
        })
    })();

    match result {
        Ok(()) => Ok(()),
        Err(err) => {
            if let Err(rollback_err) = lockfile::rollback_lockfile_failure_atomic_state(&rollback) {
                anyhow::bail!(
                    "authoritative lockfile update failed: {:#}; rollback also failed: {:#}",
                    err,
                    rollback_err
                );
            }
            Err(err)
        }
    }
}

fn ensure_output_repo_matches_plan(plan: &GeneratePlan, output_repo: &OutputRepo) -> Result<()> {
    let expected = normalize_publish_path(plan.resolved.output_plan.output_path.as_path())?;
    let actual = normalize_publish_path(output_repo.path.as_path())?;
    if actual != expected {
        anyhow::bail!(
            "verified candidate output repo mismatch: plan targets {} but entrypoint received {}",
            expected.display(),
            actual.display()
        );
    }
    Ok(())
}

fn verified_published_metadata_from_candidate_verification(
    plan: &GeneratePlan,
    verification: &CandidateVerification,
) -> Result<VerifiedPublishedSnapshotMetadata> {
    VerifiedPublishedSnapshotMetadata::from_candidate_verification(
        output_repo::PublishedSnapshotMetadata {
            branch: plan.resolved.output_plan.branch.clone(),
            tag: tag_name(plan),
            base_ref: plan.resolved.base.r#ref.clone(),
            base_commit: plan.resolved.base.commit.clone(),
            profile: plan.requested.selected_profile.as_str().to_string(),
            mode: plan.resolved.output_plan.mode.clone(),
            generated_at: plan.resolved.base.resolved_at.clone(),
            candidate_metadata_fingerprint: Some(
                verification.metadata_fingerprint().as_str().to_string(),
            ),
            base_metadata_file: output_repo::BASE_METADATA_FILE.to_string(),
            generated_metadata_file: output_repo::GENERATED_METADATA_FILE.to_string(),
            manifest_file: crate::manifest::OUTPUT_MANIFEST_FILE_NAME.to_string(),
            report_file: output_repo::REPORT_FILE.to_string(),
            kslim_version: env!("CARGO_PKG_VERSION").to_string(),
        },
        verification,
    )
}

fn recheck_output_repo_safety(output_repo: &OutputRepo) -> Result<()> {
    let output_path = output_repo_path_str(output_repo)?;
    output_repo::require_managed(output_path)?;
    output_repo::require_clean(output_path, output_repo.force)?;
    output_repo::require_not_detached(output_path, output_repo.force)
}

fn ensure_verification_still_matches_candidate(
    plan: &GeneratePlan,
    candidate: &CandidateTreeState,
    verification: &CandidateVerification,
) -> Result<()> {
    let current = verify::verify_candidate(plan, candidate)?;
    if &current != verification {
        anyhow::bail!("verified candidate proof no longer matches candidate tree or metadata");
    }
    Ok(())
}

fn output_repo_path_str(output_repo: &OutputRepo) -> Result<&str> {
    output_repo
        .path
        .as_path()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("published output repo path is not valid UTF-8"))
}

fn normalize_publish_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("failed to read current directory for output path normalization")?
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    Ok(normalized)
}

fn project_root_for_plan(plan: &GeneratePlan) -> PathBuf {
    plan.requested
        .config_path
        .as_path()
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn lockfile_path_for_plan(plan: &GeneratePlan) -> Result<LockfilePath> {
    LockfilePath::new_in_project_root(project_root_for_plan(plan))
}

fn commit_message(plan: &GeneratePlan, verification: &CandidateVerification) -> String {
    let selected_profile =
        output_repo::sanitize_commit_message_value(plan.requested.selected_profile.as_str());
    let plan_id = output_repo::sanitize_commit_message_value(plan.plan_id.as_str());
    let plan_fingerprint = output_repo::sanitize_commit_message_value(plan.fingerprint.as_str());
    let tree_fingerprint =
        output_repo::sanitize_commit_message_value(verification.tree_fingerprint().as_str());
    let base_ref = output_repo::sanitize_commit_message_value(&plan.resolved.base.r#ref);
    let base_commit = output_repo::sanitize_commit_message_value(&plan.resolved.base.commit);
    let mode = output_repo::sanitize_commit_message_value(&plan.resolved.output_plan.mode);
    format!(
        concat!(
            "kslim: publish verified candidate {}\n\n",
            "Plan: {}\n",
            "Plan-fingerprint: {}\n",
            "Tree-fingerprint: {}\n",
            "Base-ref: {}\n",
            "Base-commit: {}\n",
            "Profile: {}\n",
            "Mode: {}\n",
            "Reducer-summary: ok={} report_ok={}\n",
            "Selftest-summary: ok={}\n"
        ),
        selected_profile,
        plan_id,
        plan_fingerprint,
        tree_fingerprint,
        base_ref,
        base_commit,
        selected_profile,
        mode,
        verification.reducer_ok(),
        verification.report_ok(),
        verification.selftest_ok(),
    )
}

fn tag_name(plan: &GeneratePlan) -> String {
    format!("{}-r1", plan.resolved.output_plan.branch)
}

pub(in crate::generate) fn write_authoritative_lockfile(
    project_root: &std::path::Path,
    config: &KslimConfig,
    profile: &ProfileConfig,
    resolved: &ResolvedBase,
    mode: &str,
    commit_result: &GenerateResult,
) -> Result<()> {
    let lockfile_path = LockfilePath::new_in_project_root(project_root)?;
    let rollback = crate::lockfile::capture_lockfile_failure_atomic_state(&lockfile_path)?;
    let result = (|| -> Result<()> {
        let lock = authoritative_lockfile_from_committed_output(
            config,
            profile,
            resolved,
            mode,
            commit_result,
        )?;
        crate::lockfile::write_published_lockfile(&lockfile_path, &lock)
    })();

    match result {
        Ok(()) => Ok(()),
        Err(err) => {
            if let Err(rollback_err) =
                crate::lockfile::rollback_lockfile_failure_atomic_state(&rollback)
            {
                anyhow::bail!(
                    "authoritative lockfile update failed: {:#}; rollback also failed: {:#}",
                    err,
                    rollback_err
                );
            }
            Err(err)
        }
    }
}

fn authoritative_lockfile_from_committed_output(
    config: &KslimConfig,
    profile: &ProfileConfig,
    resolved: &ResolvedBase,
    mode: &str,
    commit_result: &GenerateResult,
) -> Result<PublishedLockfileUpdate> {
    let output_path = &config.output.path;
    if crate::git::is_dirty(output_path)? {
        anyhow::bail!("cannot write authoritative lockfile from dirty output repository");
    }
    let output_repo = OutputRepoPath::new(output_path.as_str())?;

    let output_commit = commit_result.output_commit.as_deref().ok_or_else(|| {
        anyhow::anyhow!("cannot write authoritative lockfile without output commit")
    })?;
    let tag = commit_result.tag.as_deref().ok_or_else(|| {
        anyhow::anyhow!("cannot write authoritative lockfile without published tag")
    })?;

    let current_branch = crate::git::current_branch(output_path)?;
    if current_branch.trim().is_empty() {
        anyhow::bail!("cannot write authoritative lockfile from detached output repository");
    }
    let current_commit = crate::git::head_commit(output_path)?;
    if current_branch != commit_result.branch {
        anyhow::bail!(
            "cannot write authoritative lockfile: output branch '{}' does not match generated branch '{}'",
            current_branch,
            commit_result.branch
        );
    }
    if current_commit != output_commit {
        anyhow::bail!(
            "cannot write authoritative lockfile: output HEAD '{}' does not match generated commit '{}'",
            current_commit,
            output_commit
        );
    }

    let base = output_repo::load_committed_base_metadata(&output_repo, &current_commit)?;
    let generated = output_repo::load_committed_generated_metadata(&output_repo, &current_commit)?;
    let published =
        output_repo::load_committed_published_snapshot_metadata(&output_repo, &current_commit)?;

    if published.branch != current_branch {
        anyhow::bail!(
            "cannot write authoritative lockfile: committed published metadata branch '{}' does not match output branch '{}'",
            published.branch,
            current_branch
        );
    }
    if published.tag != tag {
        anyhow::bail!(
            "cannot write authoritative lockfile: committed published metadata tag '{}' does not match generated tag '{}'",
            published.tag,
            tag
        );
    }
    if published.base_ref != resolved.r#ref
        || published.base_commit != resolved.commit
        || published.profile != profile.profile.name
        || published.mode != mode
    {
        anyhow::bail!(
            "cannot write authoritative lockfile: committed published metadata does not match resolved generate snapshot"
        );
    }
    if base.base_ref != published.base_ref
        || base.base_commit != published.base_commit
        || base.profile != published.profile
        || base.mode != published.mode
    {
        anyhow::bail!(
            "cannot write authoritative lockfile: committed base metadata does not match published snapshot metadata"
        );
    }
    if generated.generated_at != published.generated_at {
        anyhow::bail!(
            "cannot write authoritative lockfile: committed generated metadata does not match published snapshot metadata"
        );
    }

    crate::lockfile::PublishedLockfileUpdate::new(
        resolved.clone(),
        crate::lockfile::PublishedLockfile {
            output_branch: published.branch,
            output_commit: current_commit,
            tag: published.tag,
            base_ref: published.base_ref,
            base_commit: published.base_commit,
            profile: published.profile,
            mode: published.mode,
            generated_at: published.generated_at,
        },
    )
}

fn render_commit_reducer_summary(stats: &reducer::ReducerStats) -> String {
    format!(
        concat!(
            "ran={} files_removed={} dirs_removed={} edits={} ",
            "unsupported_kconfig={} unsupported_cpp={} skipped_fixups={}"
        ),
        stats.ran,
        stats.files_removed,
        stats.dirs_removed,
        stats.edits.len(),
        stats.unsupported_kconfig_expressions.len(),
        stats.unsupported_cpp_expressions.len(),
        stats.skipped_fixups.len(),
    )
}

fn render_commit_selftest_summary(selftests: Option<&SelfTestResult>) -> String {
    match selftests {
        Some(result) => format!(
            "enabled={} built_in_checks={} kernel_builds={} commands={}",
            result.enabled, result.built_in_checks, result.kernel_builds_run, result.commands_run,
        ),
        None => String::from("enabled=false built_in_checks=0 kernel_builds=0 commands=0"),
    }
}

pub(in crate::generate) fn commit_output_repo_state(
    config: &KslimConfig,
    profile: &ProfileConfig,
    opts: &GenerateOptions,
    resolved: &ResolvedBase,
    plan_fingerprint: &str,
    generated: &GeneratedArtifacts,
    verified: &VerifiedGeneratedOutput,
    verification: &verify::CandidateVerification,
    generated_at: &str,
    patch_infos: Option<&[patches::PatchInfo]>,
    mode: &str,
    branch: &str,
    reducer_stats: &reducer::ReducerStats,
    failure: &mut FailureReportContext,
) -> Result<SuccessfulCommitResult> {
    let output_path = &config.output.path;
    log_generate_stage(GenerateStage::Commit, "commit_output_repo_state");
    let tag = output_repo::tag_name(config, profile, resolved, 1);
    let reducer_summary = render_commit_reducer_summary(reducer_stats);
    let selftest_summary = render_commit_selftest_summary(verified.selftests());
    let commit_details = output_repo::CommitMessageDetails::new(
        plan_fingerprint,
        &reducer_summary,
        &selftest_summary,
    );
    let message = output_repo::commit_message(config, profile, resolved, mode, &commit_details);

    let output_transaction = capture_output_repo_failure_atomic_state(output_path, branch)?;

    let finalize = (|| -> Result<SuccessfulCommitResult> {
        output_repo::init_output_repo(config, profile)?;

        if !opts.force {
            output_repo::require_managed(output_path)?;
            output_repo::require_clean(output_path, false)?;
            output_repo::require_not_detached(output_path, false)?;
        }

        let output_candidate = tempfile::Builder::new()
            .prefix("kslim-output-candidate-")
            .tempdir()?;
        let output_candidate_path = output_candidate.path().to_string_lossy().to_string();
        let output_candidate_repo = OutputRepoPath::new(output_candidate.path())?;
        let verified_tree = CandidateTreePath::new(verified.tree_path())?;
        output_repo::sync_working_tree(&output_candidate_repo, &verified_tree)?;
        output_repo::write_managed_marker(&output_candidate_path, &config.project.name)?;

        set_generate_stage(failure, GenerateStage::Metadata);
        let published_metadata = write_output_metadata_report_and_manifest(
            &output_candidate_path,
            config,
            profile,
            resolved,
            generated,
            generated_at,
            failure.stage,
            patch_infos,
            mode,
            branch,
            &tag,
            Some(reducer_stats),
            verified,
            verification,
        )?;

        set_generate_stage(failure, GenerateStage::Commit);
        let output_repo = OutputRepoPath::new(output_path.as_str())?;
        let output_candidate_tree = CandidateTreePath::new(output_candidate.path())?;
        output_repo::publish_output_candidate(
            &output_repo,
            &output_candidate_tree,
            &verified_tree,
        )?;
        output_repo::write_verified_published_snapshot_metadata(&output_repo, &published_metadata)?;
        output_repo::write_verified_committed_published_snapshot_metadata(
            &output_repo,
            &published_metadata,
            &[output_candidate_tree.as_path(), verified_tree.as_path()],
        )?;

        set_generate_stage(failure, GenerateStage::Commit);
        crate::git::create_branch(output_path, branch)?;

        set_generate_stage(failure, GenerateStage::Commit);
        output_repo::sync_repo_git_config(config, Some(branch))?;

        set_generate_stage(failure, GenerateStage::Commit);
        output_repo::stage_committed_metadata(&output_repo)?;
        set_generate_stage(failure, GenerateStage::Commit);
        let committed = crate::git::commit_if_changed(output_path, &message)?;
        if committed {
            log::info!("generated commit on branch '{}'", branch);
        } else {
            log::info!("no changes detected (idempotent), commit skipped");
        }
        let output_commit = crate::git::head_commit(output_path)?;
        // SuccessfulCommitResult is the publication proof: it is produced only
        // after output commit/no-op and concrete HEAD lookup both succeed.

        Ok(SuccessfulCommitResult {
            committed,
            branch: branch.to_string(),
            tag,
            output_commit,
        })
    })();

    match finalize {
        Ok(result) => {
            failure.output_repo_rollback = Some(output_transaction);
            Ok(result)
        }
        Err(err) => {
            if let Err(rollback_err) =
                rollback_output_repo_failure_atomic_state(output_path, &output_transaction)
            {
                anyhow::bail!(
                    "output publish failed: {:#}; rollback also failed: {:#}",
                    err,
                    rollback_err
                );
            }
            Err(err)
        }
    }
}

pub(in crate::generate) fn write_output_metadata_report_and_manifest(
    output_path: &str,
    config: &KslimConfig,
    profile: &ProfileConfig,
    resolved: &ResolvedBase,
    generated: &GeneratedArtifacts,
    generated_at: &str,
    stage: GenerateStage,
    patch_infos: Option<&[patches::PatchInfo]>,
    mode: &str,
    branch: &str,
    tag: &str,
    reducer_stats: Option<&reducer::ReducerStats>,
    verified: &VerifiedGeneratedOutput,
    verification: &verify::CandidateVerification,
) -> Result<VerifiedPublishedSnapshotMetadata> {
    output_repo::write_base_metadata(output_path, config, profile, resolved, mode)?;
    output_repo::write_generated_metadata(output_path, generated_at)?;
    output_repo::write_patch_metadata(output_path, patch_infos)?;
    output_repo::write_report(
        output_path,
        config,
        profile,
        resolved,
        generated.file_count,
        generated.total_bytes,
        mode,
        stage,
        patch_infos,
        verified.selftests(),
    )?;
    manifest::write_manifest(&generated.entries, output_path)?;
    let published_metadata = VerifiedPublishedSnapshotMetadata::from_candidate_verification(
        output_repo::PublishedSnapshotMetadata {
            branch: branch.to_string(),
            tag: tag.to_string(),
            base_ref: resolved.r#ref.clone(),
            base_commit: resolved.commit.clone(),
            profile: profile.profile.name.clone(),
            mode: mode.to_string(),
            generated_at: generated_at.to_string(),
            candidate_metadata_fingerprint: Some(
                verification.metadata_fingerprint().as_str().to_string(),
            ),
            base_metadata_file: output_repo::BASE_METADATA_FILE.to_string(),
            generated_metadata_file: output_repo::GENERATED_METADATA_FILE.to_string(),
            manifest_file: manifest::OUTPUT_MANIFEST_FILE_NAME.to_string(),
            report_file: output_repo::REPORT_FILE.to_string(),
            kslim_version: env!("CARGO_PKG_VERSION").to_string(),
        },
        verification,
    )?;
    let reducer_manifest =
        reducer_manifest_for_profile(profile, Some(std::path::Path::new(output_path)))?;
    output_repo::write_reducer_metadata_with_context(
        output_path,
        reducer_stats,
        Some(&profile.reducer),
        reducer_manifest.as_ref(),
    )?;

    let verified_tree = CandidateTreePath::new(verified.tree_path())?;
    let candidate_metadata = output_repo::candidate_metadata_dir(&verified_tree)?;
    let output_candidate = OutputRepoPath::new(output_path)?;
    output_repo::copy_candidate_reports_to_output_candidate_metadata(
        &candidate_metadata,
        &output_candidate,
    )?;
    Ok(published_metadata)
}

#[cfg(test)]
mod tests {
    use super::super::state::{
        CliOverrides, ProfileName, RequestedGenerateState, ResolvedCandidateState,
    };
    use super::*;
    use crate::config;
    use crate::lockfile::ResolvedBase;
    use crate::paths::{LockfilePath, RequestedConfigPath};

    fn git_in(dir: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        if !output.status.success() {
            panic!(
                "git {:?} failed in {}: {}",
                args,
                dir.display(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn create_minimal_tree(root: &Path) {
        for dir in &[
            "arch", "drivers", "fs", "include", "kernel", "mm", "net", "scripts",
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
            std::fs::write(root.join(dir).join(".keep"), "").unwrap();
        }
        std::fs::write(root.join("Makefile"), "# test\n").unwrap();
        std::fs::write(root.join("Kconfig"), "# test\n").unwrap();
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

    fn plan_for_tree(config_path: &Path, output: &Path) -> GeneratePlan {
        plan_for_tree_with_branch(config_path, output, "kslim/v1.0/default")
    }

    fn plan_for_tree_with_branch(config_path: &Path, output: &Path, branch: &str) -> GeneratePlan {
        plan_for_tree_with_branch_and_resolved_at(
            config_path,
            output,
            branch,
            "2026-01-01T00:00:00Z",
        )
    }

    fn plan_for_tree_with_branch_and_resolved_at(
        config_path: &Path,
        output: &Path,
        branch: &str,
        resolved_at: &str,
    ) -> GeneratePlan {
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
                resolved_at: resolved_at.to_string(),
            },
            None,
            "unmodified-upstream",
            branch,
        )
        .unwrap();
        GeneratePlan::new(requested_state(config_path), resolved).unwrap()
    }

    fn write_candidate_metadata_for_test(
        metadata_dir: &Path,
        plan: &GeneratePlan,
        candidate: &CandidateTreeState,
    ) {
        std::fs::write(
            metadata_dir.join(crate::manifest::OUTPUT_MANIFEST_FILE_NAME),
            "hash  1  Makefile\n",
        )
        .unwrap();
        let entries =
            crate::manifest::generate_manifest(candidate.tree.as_path().to_str().unwrap()).unwrap();
        let tree_fingerprint = crate::manifest::tree_fingerprint(&entries);
        std::fs::write(
            metadata_dir.join("candidate.toml"),
            format!(
                concat!(
                    "schema_version = 1\n",
                    "metadata_scope = \"candidate\"\n",
                    "authoritative = false\n",
                    "plan_id = \"{}\"\n",
                    "plan_fingerprint = \"{}\"\n",
                    "tree_fingerprint = \"{}\"\n",
                    "config_content_hash = \"{}\"\n",
                    "generated_by = \"{}\"\n",
                    "selected_profile = \"{}\"\n",
                    "upstream_name = \"{}\"\n",
                    "base_ref = \"{}\"\n",
                    "base_commit = \"{}\"\n",
                    "base_resolved_at = \"{}\"\n",
                    "output_branch = \"{}\"\n",
                    "output_mode = \"{}\"\n",
                    "patch_source_count = {}\n",
                    "patch_commit_count = {}\n",
                    "integration_count = {}\n",
                    "materialized = {}\n",
                    "integrated = {}\n",
                    "pruned = {}\n",
                    "reduced = {}\n",
                    "selftested = {}\n",
                    "reducer_ran = false\n",
                    "manifest_file = \"{}\"\n"
                ),
                plan.plan_id.as_str(),
                plan.fingerprint.as_str(),
                tree_fingerprint,
                plan.config_content_hash.as_str(),
                plan.created_with.as_str(),
                plan.requested.selected_profile.as_str(),
                &plan.resolved.base.upstream,
                &plan.resolved.base.r#ref,
                &plan.resolved.base.commit,
                &plan.resolved.base.resolved_at,
                &plan.resolved.output_plan.branch,
                &plan.resolved.output_plan.mode,
                plan.resolved.patch_plan.sources.len(),
                plan.resolved.patch_plan.total_patch_count,
                plan.resolved.integration_plan.entries.len(),
                candidate.materialized,
                candidate.integrated,
                candidate.pruned,
                candidate.reduced,
                candidate.selftested,
                crate::manifest::OUTPUT_MANIFEST_FILE_NAME,
            ),
        )
        .unwrap();
    }

    fn init_managed_output_repo(output: &Path) {
        std::fs::create_dir_all(output).unwrap();
        git_in(output, &["init"]);
        git_in(output, &["config", "user.email", "test@kslim.local"]);
        git_in(output, &["config", "user.name", "kslim test"]);
        std::fs::write(output.join("Makefile"), "# original\n").unwrap();
        std::fs::create_dir_all(output.join(".git/kslim")).unwrap();
        std::fs::write(
            output.join(".git/kslim/managed.toml"),
            "managed_by = \"kslim\"\n",
        )
        .unwrap();
        git_in(output, &["add", "-A"]);
        git_in(output, &["commit", "-m", "initial output"]);
    }

    fn verified_candidate_for_test(
        tmp: &Path,
        output: &Path,
        branch: &str,
    ) -> (
        PathBuf,
        CandidateTreeState,
        GeneratePlan,
        CandidateVerification,
    ) {
        let tree = tmp.join("candidate");
        let config_path = tmp.join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = plan_for_tree_with_branch(&config_path, output, branch);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate);
        let verification = verify::verify_candidate(&plan, &candidate).unwrap();
        (config_path, candidate, plan, verification)
    }

    fn assert_no_publish_side_effects(output: &Path, original_head: &str) {
        assert_eq!(
            crate::git::head_commit(output.to_str().unwrap()).unwrap(),
            original_head
        );
        assert_eq!(
            std::fs::read_to_string(output.join("Makefile")).unwrap(),
            "# original\n"
        );
    }

    #[test]
    fn commit_message_includes_plan_reducer_and_selftest_summary() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let (_config_path, _candidate, plan, verification) =
            verified_candidate_for_test(tmp.path(), &output, "kslim/v1.0/default");

        let message = commit_message(&plan, &verification);

        assert!(message.contains("Profile: default"));
        assert!(message.contains("Base-commit: deadbeef"));
        assert!(message.contains("Plan-fingerprint: fingerprint-"));
        assert!(message.contains("Reducer-summary: ok=true report_ok=true"));
        assert!(message.contains("Selftest-summary: ok=true"));
    }

    #[test]
    fn commit_message_redacts_host_paths_from_publish_message() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let host_path = tmp.path().join("candidate-tree");
        let host_path = host_path.to_str().unwrap();
        let (_config_path, _candidate, mut plan, verification) =
            verified_candidate_for_test(tmp.path(), &output, "kslim/v1.0/default");
        plan.requested.selected_profile = ProfileName::new_unchecked_for_test(host_path);
        plan.resolved.base.r#ref = host_path.to_string();
        plan.resolved.base.commit = format!("commit={host_path}");
        plan.resolved.output_plan.mode = format!("mode={host_path}");

        let message = commit_message(&plan, &verification);

        assert!(!message.contains(host_path));
        assert!(message.contains(output_repo::COMMIT_MESSAGE_HOST_PATH_REDACTION));
        assert!(message.contains("Base-ref: <host-path>"));
        assert!(message.contains("Base-commit: <host-path>"));
        assert!(message.contains("Profile: <host-path>"));
        assert!(message.contains("Mode: <host-path>"));
    }

    #[cfg(unix)]
    fn write_git_hook(repo: &Path, hook: &str, script: &str) {
        use std::os::unix::fs::PermissionsExt;

        let path = repo.join(".git/hooks").join(hook);
        std::fs::write(&path, script).unwrap();
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).unwrap();
    }

    #[test]
    fn test_commit_verified_candidate_rejects_output_repo_mismatch_before_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let expected_output = tmp.path().join("expected-output");
        let actual_output = tmp.path().join("actual-output");
        let (config_path, candidate, plan, verification) =
            verified_candidate_for_test(tmp.path(), &expected_output, "kslim/v1.0/default");
        init_managed_output_repo(&actual_output);
        let original_head = crate::git::head_commit(actual_output.to_str().unwrap()).unwrap();
        let mut output_repo = OutputRepo::new(&actual_output).unwrap();

        let err = commit_verified_candidate(&plan, candidate, verification, &mut output_repo)
            .unwrap_err()
            .to_string();

        assert!(err.contains("verified candidate output repo mismatch"));
        assert_no_publish_side_effects(&actual_output, &original_head);
        assert!(!config_path.parent().unwrap().join("kslim.lock").exists());
    }

    #[test]
    fn test_commit_verified_candidate_rejects_unmanaged_output_before_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let (config_path, candidate, plan, verification) =
            verified_candidate_for_test(tmp.path(), &output, "kslim/v1.0/default");
        std::fs::create_dir_all(&output).unwrap();
        git_in(&output, &["init"]);
        git_in(&output, &["config", "user.email", "test@kslim.local"]);
        git_in(&output, &["config", "user.name", "kslim test"]);
        std::fs::write(output.join("Makefile"), "# original\n").unwrap();
        git_in(&output, &["add", "-A"]);
        git_in(&output, &["commit", "-m", "initial unmanaged output"]);
        let original_head = crate::git::head_commit(output.to_str().unwrap()).unwrap();
        let mut output_repo = OutputRepo::new(&output).unwrap();

        let err = commit_verified_candidate(&plan, candidate, verification, &mut output_repo)
            .unwrap_err()
            .to_string();

        assert!(
            err.contains("not managed") || err.contains("NotManaged"),
            "unexpected error: {err}"
        );
        assert_no_publish_side_effects(&output, &original_head);
        assert!(!config_path.parent().unwrap().join("kslim.lock").exists());
    }

    #[test]
    fn test_commit_verified_candidate_rejects_dirty_output_before_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let (config_path, candidate, plan, verification) =
            verified_candidate_for_test(tmp.path(), &output, "kslim/v1.0/default");
        init_managed_output_repo(&output);
        let original_head = crate::git::head_commit(output.to_str().unwrap()).unwrap();
        std::fs::write(output.join("dirty.txt"), "dirty\n").unwrap();
        let mut output_repo = OutputRepo::new(&output).unwrap();

        let err = commit_verified_candidate(&plan, candidate, verification, &mut output_repo)
            .unwrap_err()
            .to_string();

        assert!(err.contains("uncommitted changes"));
        assert_no_publish_side_effects(&output, &original_head);
        assert!(crate::git::is_dirty(output.to_str().unwrap()).unwrap());
        assert!(!config_path.parent().unwrap().join("kslim.lock").exists());
    }

    #[test]
    fn test_commit_verified_candidate_rejects_detached_output_before_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let (config_path, candidate, plan, verification) =
            verified_candidate_for_test(tmp.path(), &output, "kslim/v1.0/default");
        init_managed_output_repo(&output);
        let original_head = crate::git::head_commit(output.to_str().unwrap()).unwrap();
        git_in(&output, &["checkout", "--detach", "HEAD"]);
        let mut output_repo = OutputRepo::new(&output).unwrap();

        let err = commit_verified_candidate(&plan, candidate, verification, &mut output_repo)
            .unwrap_err()
            .to_string();

        assert!(err.contains("detached HEAD"));
        assert_no_publish_side_effects(&output, &original_head);
        assert_eq!(
            crate::git::current_branch(output.to_str().unwrap()).unwrap(),
            ""
        );
        assert!(!config_path.parent().unwrap().join("kslim.lock").exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_commit_verified_candidate_rejects_private_candidate_sync_failure_before_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let (config_path, candidate, plan, verification) =
            verified_candidate_for_test(tmp.path(), &output, "kslim/v1.0/default");
        init_managed_output_repo(&output);
        let original_head = crate::git::head_commit(output.to_str().unwrap()).unwrap();
        let fifo = candidate.tree.as_path().join("unsupported-fifo");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap();
        assert!(
            status.success(),
            "failed to create fifo at {}",
            fifo.display()
        );
        let mut output_repo = OutputRepo::new(&output).unwrap();

        let err = commit_verified_candidate(&plan, candidate, verification, &mut output_repo)
            .unwrap_err()
            .to_string();

        assert!(err.contains("unsupported file type in snapshot tree"));
        assert_no_publish_side_effects(&output, &original_head);
        assert!(!config_path.parent().unwrap().join("kslim.lock").exists());
    }

    #[test]
    fn test_candidate_verification_rejects_non_reproducible_timestamp_before_publish() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = plan_for_tree_with_branch_and_resolved_at(
            &config_path,
            &output,
            "kslim/v1.0/default",
            "not-a-reproducible-timestamp",
        );
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate);

        let err = format!(
            "{:#}",
            verify::verify_candidate(&plan, &candidate).unwrap_err()
        );

        assert!(err.contains("reproducible RFC3339 timestamp"));
        assert!(!output.exists());
        assert!(!config_path.parent().unwrap().join("kslim.lock").exists());
    }

    #[test]
    fn test_commit_verified_candidate_rolls_back_when_branch_create_fails_before_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let (config_path, candidate, plan, verification) =
            verified_candidate_for_test(tmp.path(), &output, "bad branch name");
        init_managed_output_repo(&output);
        let original_head = crate::git::head_commit(output.to_str().unwrap()).unwrap();
        let original_branch = crate::git::current_branch(output.to_str().unwrap()).unwrap();
        let mut output_repo = OutputRepo::new(&output).unwrap();

        let err = commit_verified_candidate(&plan, candidate, verification, &mut output_repo)
            .unwrap_err()
            .to_string();

        assert!(
            err.contains("branch") || err.contains("git"),
            "unexpected error: {err}"
        );
        assert_no_publish_side_effects(&output, &original_head);
        assert_eq!(
            crate::git::current_branch(output.to_str().unwrap()).unwrap(),
            original_branch
        );
        assert!(!output.join(".kslim/candidate.toml").exists());
        assert!(!config_path.parent().unwrap().join("kslim.lock").exists());
        assert!(!crate::git::is_dirty(output.to_str().unwrap()).unwrap());
    }

    #[test]
    fn test_commit_verified_candidate_entrypoint_commits_verified_candidate() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate);
        let verification = verify::verify_candidate(&plan, &candidate).unwrap();
        let verified_metadata_fingerprint =
            verification.metadata_fingerprint().as_str().to_string();
        init_managed_output_repo(&output);
        let mut output_repo = OutputRepo::new(&output).unwrap();

        let published =
            commit_verified_candidate(&plan, candidate, verification, &mut output_repo).unwrap();

        assert_eq!(published.output_repo().as_path(), output.as_path());
        assert_eq!(published.branch().as_str(), "kslim/v1.0/default");
        assert_eq!(
            std::fs::read_to_string(output.join("Makefile")).unwrap(),
            "# test\n"
        );
        assert!(output.join(".kslim/candidate.toml").is_file());
        assert_eq!(
            crate::git::head_commit(output.to_str().unwrap()).unwrap(),
            published.commit().as_str()
        );
        let committed_published = output_repo::load_committed_published_snapshot_metadata(
            &OutputRepoPath::new(&output).unwrap(),
            published.commit().as_str(),
        )
        .unwrap();
        assert_eq!(committed_published.branch, "kslim/v1.0/default");
        assert_eq!(committed_published.tag, tag_name(&plan));
        assert_eq!(
            committed_published.base_ref.as_str(),
            plan.resolved.base.r#ref.as_str()
        );
        assert_eq!(
            committed_published.base_commit.as_str(),
            plan.resolved.base.commit.as_str()
        );
        assert_eq!(committed_published.profile.as_str(), "default");
        assert_eq!(committed_published.mode.as_str(), "unmodified-upstream");
        assert_eq!(
            committed_published
                .candidate_metadata_fingerprint
                .as_deref(),
            Some(verified_metadata_fingerprint.as_str())
        );
        assert_eq!(
            committed_published.manifest_file,
            crate::manifest::OUTPUT_MANIFEST_FILE_NAME
        );
        let lockfile_path =
            LockfilePath::new_in_project_root(config_path.parent().unwrap()).unwrap();
        let lock = crate::lockfile::load_lockfile(&lockfile_path)
            .unwrap()
            .unwrap();
        let published_lock = lock.published.unwrap();
        assert_eq!(published_lock.output_commit, published.commit().as_str());
        assert_eq!(published_lock.output_branch, committed_published.branch);
        assert_eq!(published_lock.tag, committed_published.tag);
        assert_eq!(published_lock.base_ref, committed_published.base_ref);
        assert_eq!(published_lock.base_commit, committed_published.base_commit);
        assert_eq!(published_lock.profile, committed_published.profile);
        assert_eq!(published_lock.mode, committed_published.mode);
        assert_eq!(
            published_lock.generated_at,
            committed_published.generated_at
        );
        assert!(output.join(".kslim/published.toml").is_file());
        assert!(output.join(".git/kslim/published.toml").is_file());
        assert!(!crate::git::is_dirty(output.to_str().unwrap()).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn test_commit_verified_candidate_rolls_back_when_output_gets_dirty_after_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let (config_path, candidate, plan, verification) =
            verified_candidate_for_test(tmp.path(), &output, "kslim/v1.0/default");
        init_managed_output_repo(&output);
        let original_head = crate::git::head_commit(output.to_str().unwrap()).unwrap();
        write_git_hook(
            &output,
            "post-commit",
            "#!/bin/sh\nprintf dirty > post-commit-dirty.txt\n",
        );
        let mut output_repo = OutputRepo::new(&output).unwrap();

        let err = commit_verified_candidate(&plan, candidate, verification, &mut output_repo)
            .unwrap_err()
            .to_string();

        assert!(err.contains("dirty output repository"));
        assert_no_publish_side_effects(&output, &original_head);
        assert!(!output.join("post-commit-dirty.txt").exists());
        assert!(!config_path.parent().unwrap().join("kslim.lock").exists());
        assert!(!crate::git::is_dirty(output.to_str().unwrap()).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn test_commit_verified_candidate_rolls_back_when_committed_published_metadata_is_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let (config_path, candidate, plan, verification) =
            verified_candidate_for_test(tmp.path(), &output, "kslim/v1.0/default");
        init_managed_output_repo(&output);
        let original_head = crate::git::head_commit(output.to_str().unwrap()).unwrap();
        write_git_hook(
            &output,
            "post-commit",
            concat!(
                "#!/bin/sh\n",
                "git rm -f .kslim/published.toml >/dev/null\n",
                "git -c core.hooksPath=/dev/null commit -m 'remove published metadata' >/dev/null\n"
            ),
        );
        let mut output_repo = OutputRepo::new(&output).unwrap();

        let err = commit_verified_candidate(&plan, candidate, verification, &mut output_repo)
            .unwrap_err()
            .to_string();

        assert!(err.contains("required committed published metadata missing"));
        assert_no_publish_side_effects(&output, &original_head);
        assert!(!config_path.parent().unwrap().join("kslim.lock").exists());
        assert!(!crate::git::is_dirty(output.to_str().unwrap()).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn test_commit_verified_candidate_rolls_back_when_branch_changes_after_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let (config_path, candidate, plan, verification) =
            verified_candidate_for_test(tmp.path(), &output, "kslim/v1.0/default");
        init_managed_output_repo(&output);
        let original_head = crate::git::head_commit(output.to_str().unwrap()).unwrap();
        let original_branch = crate::git::current_branch(output.to_str().unwrap()).unwrap();
        write_git_hook(
            &output,
            "post-commit",
            "#!/bin/sh\ngit checkout -B kslim/post-commit-other >/dev/null 2>&1\n",
        );
        let mut output_repo = OutputRepo::new(&output).unwrap();

        let err = commit_verified_candidate(&plan, candidate, verification, &mut output_repo)
            .unwrap_err()
            .to_string();

        assert!(err.contains("does not match committed snapshot branch"));
        assert_no_publish_side_effects(&output, &original_head);
        assert_eq!(
            crate::git::current_branch(output.to_str().unwrap()).unwrap(),
            original_branch
        );
        assert!(!config_path.parent().unwrap().join("kslim.lock").exists());
        assert!(!crate::git::is_dirty(output.to_str().unwrap()).unwrap());
    }

    #[test]
    fn test_lockfile_update_rejects_published_metadata_fingerprint_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate);
        let verification = verify::verify_candidate(&plan, &candidate).unwrap();
        init_managed_output_repo(&output);
        let mut output_repo = OutputRepo::new(&output).unwrap();
        let published =
            commit_verified_candidate(&plan, candidate, verification.clone(), &mut output_repo)
                .unwrap();
        let lockfile_before =
            std::fs::read_to_string(config_path.parent().unwrap().join("kslim.lock")).unwrap();

        let published_path = output
            .join(output_repo::COMMITTED_METADATA_DIR)
            .join(output_repo::PUBLISHED_SNAPSHOT_FILE);
        let published_metadata = std::fs::read_to_string(&published_path).unwrap();
        std::fs::write(
            &published_path,
            published_metadata.replace(
                verification.metadata_fingerprint().as_str(),
                "metadata-corrupt",
            ),
        )
        .unwrap();
        git_in(&output, &["add", "-A"]);
        git_in(&output, &["commit", "-m", "corrupt published fingerprint"]);
        let corrupt_commit = crate::git::head_commit(output.to_str().unwrap()).unwrap();
        assert_ne!(corrupt_commit, published.commit().as_str());
        let committed = CommittedOutputSnapshot::from_successful_commit(
            &output,
            LockfilePath::new_in_project_root(config_path.parent().unwrap()).unwrap(),
            &SuccessfulCommitResult {
                committed: true,
                branch: plan.resolved.output_plan.branch.clone(),
                tag: tag_name(&plan),
                output_commit: corrupt_commit,
            },
        )
        .unwrap();
        let corrupt_snapshot = PublishedSnapshotState::from_committed_output(committed).unwrap();

        let err = write_authoritative_lockfile_from_committed_publish(
            &plan,
            &output_repo,
            &corrupt_snapshot,
            &verification,
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("committed published metadata does not match"));
        assert_eq!(
            std::fs::read_to_string(config_path.parent().unwrap().join("kslim.lock")).unwrap(),
            lockfile_before
        );
    }

    #[test]
    fn test_lockfile_update_rejects_output_head_mismatch_after_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let (config_path, candidate, plan, verification) =
            verified_candidate_for_test(tmp.path(), &output, "kslim/v1.0/default");
        init_managed_output_repo(&output);
        let mut output_repo = OutputRepo::new(&output).unwrap();
        let published =
            commit_verified_candidate(&plan, candidate, verification.clone(), &mut output_repo)
                .unwrap();
        let lockfile_path = config_path.parent().unwrap().join("kslim.lock");
        let lockfile_before = std::fs::read_to_string(&lockfile_path).unwrap();
        std::fs::write(output.join("after-commit.txt"), "after\n").unwrap();
        git_in(&output, &["add", "-A"]);
        git_in(&output, &["commit", "-m", "advance output head"]);

        let err = write_authoritative_lockfile_from_committed_publish(
            &plan,
            &output_repo,
            &published,
            &verification,
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("output HEAD"));
        assert!(err.contains("does not match committed snapshot"));
        assert_eq!(
            std::fs::read_to_string(&lockfile_path).unwrap(),
            lockfile_before
        );
    }

    #[test]
    fn test_lockfile_update_rejects_invalid_committed_published_metadata_after_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let (config_path, candidate, plan, verification) =
            verified_candidate_for_test(tmp.path(), &output, "kslim/v1.0/default");
        init_managed_output_repo(&output);
        let mut output_repo = OutputRepo::new(&output).unwrap();
        let _published =
            commit_verified_candidate(&plan, candidate, verification.clone(), &mut output_repo)
                .unwrap();
        let lockfile_path = config_path.parent().unwrap().join("kslim.lock");
        let lockfile_before = std::fs::read_to_string(&lockfile_path).unwrap();
        let published_path = output
            .join(output_repo::COMMITTED_METADATA_DIR)
            .join(output_repo::PUBLISHED_SNAPSHOT_FILE);
        std::fs::write(&published_path, "not valid = [\n").unwrap();
        git_in(&output, &["add", "-A"]);
        git_in(
            &output,
            &["commit", "-m", "corrupt published metadata syntax"],
        );
        let corrupt_commit = crate::git::head_commit(output.to_str().unwrap()).unwrap();
        let committed = CommittedOutputSnapshot::from_successful_commit(
            &output,
            LockfilePath::new(&lockfile_path).unwrap(),
            &SuccessfulCommitResult {
                committed: true,
                branch: plan.resolved.output_plan.branch.clone(),
                tag: tag_name(&plan),
                output_commit: corrupt_commit,
            },
        )
        .unwrap();
        let corrupt_snapshot = PublishedSnapshotState::from_committed_output(committed).unwrap();

        let err = write_authoritative_lockfile_from_committed_publish(
            &plan,
            &output_repo,
            &corrupt_snapshot,
            &verification,
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("required committed published metadata is invalid"));
        assert_eq!(
            std::fs::read_to_string(&lockfile_path).unwrap(),
            lockfile_before
        );
    }

    #[test]
    fn test_commit_verified_candidate_rejects_stale_verification_proof() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate);
        let verification = verify::verify_candidate(&plan, &candidate).unwrap();
        std::fs::write(tree.join("stale-after-verification.txt"), "stale\n").unwrap();
        init_managed_output_repo(&output);
        let mut output_repo = OutputRepo::new(&output).unwrap();

        let err = commit_verified_candidate(&plan, candidate, verification, &mut output_repo)
            .unwrap_err()
            .to_string();

        assert!(
            err.contains("verified candidate proof no longer matches")
                || err.contains("candidate metadata field tree_fingerprint mismatch")
        );
        assert_eq!(
            std::fs::read_to_string(output.join("Makefile")).unwrap(),
            "# original\n"
        );
    }

    #[test]
    fn test_published_metadata_write_uses_candidate_verification_proof() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        let (_config_path, _candidate, plan, verification) =
            verified_candidate_for_test(tmp.path(), &output, "kslim/v1.0/default");

        let metadata =
            verified_published_metadata_from_candidate_verification(&plan, &verification).unwrap();

        assert!(metadata
            .proof_summary()
            .starts_with("candidate-verification:metadata-"));
        assert!(metadata
            .proof_summary()
            .contains(":reducer=true:selftest=true:report=true"));
        assert!(!output.join(".kslim/published.toml").exists());
    }

    #[test]
    fn test_committed_published_metadata_write_rejects_host_absolute_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&output).unwrap();
        let (_config_path, _candidate, plan, verification) =
            verified_candidate_for_test(tmp.path(), &output, "kslim/v1.0/default");
        let mut metadata =
            verified_published_metadata_from_candidate_verification(&plan, &verification).unwrap();
        let host_path = tmp.path().join("host-profile-name");
        let host_path = host_path.to_string_lossy().to_string();
        metadata.metadata.profile = host_path.clone();
        let output_repo = OutputRepoPath::new(&output).unwrap();

        let err = output_repo::write_verified_committed_published_snapshot_metadata(
            &output_repo,
            &metadata,
            &[],
        )
        .unwrap_err();
        let err = format!("{err:#}");

        assert!(err.contains("host absolute paths are forbidden"));
        assert!(err.contains("host-only absolute path"));
        assert!(err.contains("published.toml"));
        assert!(err.contains(&host_path));
        assert!(
            !output.join(".kslim/published.toml").exists(),
            "committed published metadata writer must fail before writing host paths"
        );
    }

    #[test]
    fn test_committed_published_metadata_write_rejects_temporary_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&output).unwrap();
        let (_config_path, _candidate, plan, verification) =
            verified_candidate_for_test(tmp.path(), &output, "kslim/v1.0/default");
        let mut metadata =
            verified_published_metadata_from_candidate_verification(&plan, &verification).unwrap();
        let temporary_root = tmp.path().join("staged-candidate");
        let temporary_root = temporary_root.to_string_lossy().to_string();
        metadata.metadata.profile = temporary_root.clone();
        let output_repo = OutputRepoPath::new(&output).unwrap();

        let err = output_repo::write_verified_committed_published_snapshot_metadata(
            &output_repo,
            &metadata,
            &[Path::new(&temporary_root)],
        )
        .unwrap_err();
        let err = format!("{err:#}");

        assert!(err.contains("temporary paths are forbidden"));
        assert!(err.contains("temporary path"));
        assert!(err.contains("published.toml"));
        assert!(err.contains(&temporary_root));
        assert!(
            !output.join(".kslim/published.toml").exists(),
            "committed published metadata writer must fail before writing temporary paths"
        );
    }

    #[test]
    fn test_committed_published_metadata_write_rejects_raw_logs() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        std::fs::create_dir_all(output.join(".kslim")).unwrap();
        std::fs::write(output.join(".kslim/build.log"), "raw compiler output\n").unwrap();
        let (_config_path, _candidate, plan, verification) =
            verified_candidate_for_test(tmp.path(), &output, "kslim/v1.0/default");
        let metadata =
            verified_published_metadata_from_candidate_verification(&plan, &verification).unwrap();
        let output_repo = OutputRepoPath::new(&output).unwrap();

        let err = output_repo::write_verified_committed_published_snapshot_metadata(
            &output_repo,
            &metadata,
            &[],
        )
        .unwrap_err();
        let err = format!("{err:#}");

        assert!(err.contains("raw logs are forbidden"));
        assert!(err.contains("raw log artifact"));
        assert!(err.contains("build.log"));
        assert!(
            !output.join(".kslim/published.toml").exists(),
            "committed published metadata writer must fail before publishing raw logs"
        );
    }

    #[test]
    fn test_commit_verified_candidate_rolls_back_output_when_commit_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate);
        let verification = verify::verify_candidate(&plan, &candidate).unwrap();
        init_managed_output_repo(&output);
        let original_head = crate::git::head_commit(output.to_str().unwrap()).unwrap();
        let hook = output.join(".git/hooks/pre-commit");
        std::fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&hook).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&hook, permissions).unwrap();
        }
        let mut output_repo = OutputRepo::new(&output).unwrap();

        let err = commit_verified_candidate(&plan, candidate, verification, &mut output_repo)
            .unwrap_err()
            .to_string();

        assert!(
            err.contains("pre-commit") || err.contains("git") || err.contains("commit"),
            "unexpected error: {err}"
        );
        assert_eq!(
            crate::git::head_commit(output.to_str().unwrap()).unwrap(),
            original_head
        );
        assert_eq!(
            std::fs::read_to_string(output.join("Makefile")).unwrap(),
            "# original\n"
        );
        assert!(!output.join(".kslim/candidate.toml").exists());
        assert!(!crate::git::is_dirty(output.to_str().unwrap()).unwrap());
    }

    #[test]
    fn test_commit_verified_candidate_rolls_back_output_when_lockfile_update_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate);
        let verification = verify::verify_candidate(&plan, &candidate).unwrap();
        init_managed_output_repo(&output);
        let original_head = crate::git::head_commit(output.to_str().unwrap()).unwrap();
        let lockfile_path = config_path.parent().unwrap().join("kslim.lock");
        std::fs::create_dir(&lockfile_path).unwrap();
        let mut output_repo = OutputRepo::new(&output).unwrap();

        let err = commit_verified_candidate(&plan, candidate, verification, &mut output_repo)
            .unwrap_err()
            .to_string();

        assert!(
            err.contains("failed to update authoritative lockfile")
                || err.contains("Is a directory"),
            "unexpected error: {err}"
        );
        assert_eq!(
            crate::git::head_commit(output.to_str().unwrap()).unwrap(),
            original_head
        );
        assert_eq!(
            std::fs::read_to_string(output.join("Makefile")).unwrap(),
            "# original\n"
        );
        assert!(
            lockfile_path.is_dir(),
            "failed publish must not replace preexisting lockfile path"
        );
        assert!(!crate::git::is_dirty(output.to_str().unwrap()).unwrap());
    }
}
