use anyhow::{Context, Result};
use std::path::{Component, Path, PathBuf};

use crate::config::{KslimConfig, ProfileConfig};
use crate::lockfile::ResolvedBase;
use crate::manifest;
use crate::model::OutputBranchName;
#[cfg(test)]
use crate::model::ReportPath;
use crate::output_repo;
use crate::patches;
#[cfg(test)]
use crate::paths::AttemptMetadataDir;
use crate::paths::{LockfilePath, OutputRepoPath, PublishedMetadataDir};
use crate::reducer;
use crate::selftest::SelfTestResult;
use crate::upstream;

mod candidate;
mod failure;
mod frozen_plan;
mod options;
mod orchestration;
mod plan;
mod plan_report;
mod plan_summary;
mod publish;
mod stage;
mod state;
mod verify;

pub(in crate::generate) use failure::{
    capture_output_repo_failure_atomic_state, capture_published_metadata_failure_atomic_state,
    clear_project_failure_artifacts, ensure_no_attempt_failure_before_publication,
    ensure_non_authoritative_attempt_path, log_generate_stage, project_attempt_metadata_dir,
    project_failure_report_path, project_last_attempt_path, project_reducer_failure_path,
    record_generate_attempt_failure, remove_optional_dir, rollback_failed_run_lockfile_state,
    rollback_output_repo_failure_atomic_state, rollback_published_metadata_failure_atomic_state,
    set_generate_stage, write_project_last_attempt, write_project_reducer_failure_report,
    FailureReportContext,
};
pub(crate) use frozen_plan::{
    ensure_tree_matches_frozen_base, load_frozen_plan, write_frozen_plan_for_request,
    FrozenPlanInputs,
};
pub use options::GenerateOptions;
#[allow(unused_imports)]
pub use orchestration::generate;
pub(crate) use orchestration::generate_with_source_maps;
pub(crate) use plan::GeneratePlanSourceMaps;
pub(crate) use plan_summary::{resolve_plan_summary, GeneratePlanSummary};
pub(in crate::generate) use publish::{
    commit_output_repo_state, write_authoritative_lockfile,
};
pub(crate) use publish::SuccessfulCommitResult;
#[allow(unused_imports)]
pub(crate) use publish::PublishStage;
pub(crate) use publish::VerifiedPublishedSnapshotMetadata;
pub(crate) use stage::GenerateStage;
use state::{
    CandidateTreeState, CommittedOutputSnapshot, GenerateStateIdentity, GenerateStatePhase,
    PublishedSnapshotState, RequestedGenerateState,
};
#[cfg(test)]
use state::GenerateAttemptFailure;
use plan_report::{
    deep_dry_run_result_from_candidate, dry_run_result_from_plan, report_only_result_from_plan,
};
#[allow(unused_imports)]
pub(crate) use state::{OutputNamingPlan, OutputPlan};
use verify::VerifiedGeneratedOutput;
#[allow(unused_imports)]
pub(crate) use verify::VerificationStage;

pub struct GenerateResult {
    pub committed: bool,
    pub stage: GenerateStage,
    pub branch: String,
    pub tag: Option<String>,
    pub output_commit: Option<String>,
    pub file_count: usize,
    pub total_bytes: u64,
    pub patch_count: usize,
    pub selftests_enabled: bool,
    pub built_in_selftests: usize,
    pub selftest_commands: usize,
}

struct GeneratedArtifacts {
    entries: Vec<manifest::FileEntry>,
    file_count: usize,
    total_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GenerateStatePathClaim {
    phase: GenerateStatePhase,
    label: &'static str,
    path: PathBuf,
}

impl GenerateStatePathClaim {
    fn new(phase: GenerateStatePhase, label: &'static str, path: &Path) -> Result<Self> {
        if path.as_os_str().is_empty() {
            anyhow::bail!(
                "generate state path claim for {}/{} is empty",
                phase.as_str(),
                label
            );
        }
        Ok(Self {
            phase,
            label,
            path: normalize_generate_state_path(path)?,
        })
    }

    fn aliases(
        &self,
        other: &Self,
        allowed_phase_overlaps: &[(GenerateStatePhase, GenerateStatePhase)],
    ) -> bool {
        self.phase != other.phase
            && !phase_overlap_is_allowed(self.phase, other.phase, allowed_phase_overlaps)
            && (self.path == other.path
                || self.path.starts_with(&other.path)
                || other.path.starts_with(&self.path))
    }
}

fn phase_overlap_is_allowed(
    left: GenerateStatePhase,
    right: GenerateStatePhase,
    allowed_phase_overlaps: &[(GenerateStatePhase, GenerateStatePhase)],
) -> bool {
    allowed_phase_overlaps
        .iter()
        .any(|(allowed_left, allowed_right)| {
            (*allowed_left == left && *allowed_right == right)
                || (*allowed_left == right && *allowed_right == left)
        })
}

fn normalize_generate_state_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("failed to read current directory for generate state path normalization")?
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

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CandidateGenerateState {
    identity: GenerateStateIdentity,
    tree: CandidateTreeState,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedGenerateState {
    identity: GenerateStateIdentity,
    output_repo: OutputRepoPath,
    metadata_dir: PublishedMetadataDir,
    branch: OutputBranchName,
}

impl ResolvedGenerateState {
    fn from_plan(plan: &plan::GeneratePlan) -> Result<Self> {
        let output_repo = plan.resolved.output_plan.output_path.clone();
        let metadata_dir = output_repo::published_metadata_dir(&output_repo)?;
        let branch = OutputBranchName::new(plan.resolved.output_plan.branch.clone())?;
        Ok(Self {
            identity: GenerateStateIdentity::new(
                GenerateStatePhase::Resolved,
                format!(
                    "resolved:plan={}:fingerprint={}:output={}:branch={}",
                    plan.plan_id.as_str(),
                    plan.fingerprint.as_str(),
                    output_repo.as_path().display(),
                    branch.as_str()
                ),
            )?,
            output_repo,
            metadata_dir,
            branch,
        })
    }

    fn path_claims(&self) -> Result<Vec<GenerateStatePathClaim>> {
        Ok(vec![
            GenerateStatePathClaim::new(
                GenerateStatePhase::Resolved,
                "resolved output target",
                self.output_repo.as_path(),
            )?,
            GenerateStatePathClaim::new(
                GenerateStatePhase::Resolved,
                "resolved output metadata",
                self.metadata_dir.as_path(),
            )?,
        ])
    }
}

impl CandidateGenerateState {
    fn from_tree_path(tree_path: &Path) -> Result<Self> {
        let tree = CandidateTreeState::from_materialized_tree(tree_path)?;
        Ok(Self {
            identity: GenerateStateIdentity::new(
                GenerateStatePhase::Candidate,
                format!("candidate:tree={}", tree_path.display()),
            )?,
            tree,
        })
    }

    fn path_claims(&self) -> Result<Vec<GenerateStatePathClaim>> {
        Ok(vec![
            GenerateStatePathClaim::new(
                GenerateStatePhase::Candidate,
                "candidate tree",
                self.tree.tree.as_path(),
            )?,
            GenerateStatePathClaim::new(
                GenerateStatePhase::Candidate,
                "candidate metadata",
                self.tree.metadata_dir.as_path(),
            )?,
        ])
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct OutputTargetReservation {
    identity: GenerateStateIdentity,
    output_repo: OutputRepoPath,
    metadata_dir: PublishedMetadataDir,
    branch: OutputBranchName,
}

impl OutputTargetReservation {
    fn from_output_target(output_path: &Path, branch: &str) -> Result<Self> {
        let output_repo = OutputRepoPath::new(output_path)?;
        let metadata_dir = output_repo::published_metadata_dir(&output_repo)?;
        let branch = OutputBranchName::new(branch)?;
        Ok(Self {
            identity: GenerateStateIdentity::new(
                GenerateStatePhase::OutputTarget,
                format!(
                    "output-target:output={}:branch={}",
                    output_repo.as_path().display(),
                    branch.as_str()
                ),
            )?,
            output_repo,
            metadata_dir,
            branch,
        })
    }

    fn path_claims(&self) -> Result<Vec<GenerateStatePathClaim>> {
        Ok(vec![
            GenerateStatePathClaim::new(
                GenerateStatePhase::OutputTarget,
                "output target",
                self.output_repo.as_path(),
            )?,
            GenerateStatePathClaim::new(
                GenerateStatePhase::OutputTarget,
                "output target metadata",
                self.metadata_dir.as_path(),
            )?,
        ])
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PublishedGenerateState {
    identity: GenerateStateIdentity,
    output_repo: OutputRepoPath,
    metadata_dir: PublishedMetadataDir,
    branch: OutputBranchName,
    snapshot: PublishedSnapshotState,
}

impl PublishedGenerateState {
    fn from_successful_commit(
        output_path: &Path,
        lockfile_path: LockfilePath,
        commit: &SuccessfulCommitResult,
    ) -> Result<Self> {
        let committed =
            CommittedOutputSnapshot::from_successful_commit(output_path, lockfile_path, commit)?;
        let snapshot = PublishedSnapshotState::from_committed_output(committed)?;
        Ok(Self {
            identity: GenerateStateIdentity::new(
                GenerateStatePhase::Published,
                format!(
                    "published:output={}:branch={}:commit={}",
                    snapshot.output_repo().as_path().display(),
                    snapshot.branch().as_str(),
                    snapshot.commit().as_str()
                ),
            )?,
            output_repo: snapshot.output_repo().clone(),
            metadata_dir: snapshot.metadata_dir().clone(),
            branch: snapshot.branch().clone(),
            snapshot,
        })
    }

    fn path_claims(&self) -> Result<Vec<GenerateStatePathClaim>> {
        Ok(vec![
            GenerateStatePathClaim::new(
                GenerateStatePhase::Published,
                "published output",
                self.output_repo.as_path(),
            )?,
            GenerateStatePathClaim::new(
                GenerateStatePhase::Published,
                "published metadata",
                self.metadata_dir.as_path(),
            )?,
        ])
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct FailureGenerateState {
    identity: GenerateStateIdentity,
    attempt_metadata_dir: PathBuf,
    report_path: PathBuf,
    reducer_failure_path: PathBuf,
    last_attempt_path: PathBuf,
}

impl FailureGenerateState {
    fn from_project_root(project_root: &Path) -> Result<Self> {
        if project_root.as_os_str().is_empty() {
            anyhow::bail!("failure project root path is empty");
        }
        let attempt_metadata_dir = project_attempt_metadata_dir(project_root);
        let report_path = project_failure_report_path(project_root);
        let reducer_failure_path = project_reducer_failure_path(project_root);
        let last_attempt_path = project_last_attempt_path(project_root);
        Ok(Self {
            identity: GenerateStateIdentity::new(
                GenerateStatePhase::Failure,
                format!("failure:last-attempt={}", last_attempt_path.display()),
            )?,
            attempt_metadata_dir,
            report_path,
            reducer_failure_path,
            last_attempt_path,
        })
    }

    fn path_claims(&self) -> Result<Vec<GenerateStatePathClaim>> {
        Ok(vec![
            GenerateStatePathClaim::new(
                GenerateStatePhase::Failure,
                "failure attempt metadata",
                &self.attempt_metadata_dir,
            )?,
            GenerateStatePathClaim::new(
                GenerateStatePhase::Failure,
                "failure report",
                &self.report_path,
            )?,
            GenerateStatePathClaim::new(
                GenerateStatePhase::Failure,
                "failure reducer report",
                &self.reducer_failure_path,
            )?,
            GenerateStatePathClaim::new(
                GenerateStatePhase::Failure,
                "failure last attempt",
                &self.last_attempt_path,
            )?,
        ])
    }
}

#[derive(Debug, Default, Clone)]
struct GenerateStateLedger {
    requested: Option<RequestedGenerateState>,
    resolved: Option<ResolvedGenerateState>,
    candidate: Option<CandidateGenerateState>,
    output_target: Option<OutputTargetReservation>,
    published: Option<PublishedGenerateState>,
    failure: Option<FailureGenerateState>,
}

impl GenerateStateLedger {
    fn record_requested(&mut self, state: RequestedGenerateState) -> Result<()> {
        if self.requested.is_some() {
            anyhow::bail!("requested generate state was recorded more than once");
        }
        let mut next = self.clone();
        next.requested = Some(state);
        next.ensure_no_aliases()?;
        *self = next;
        Ok(())
    }

    fn record_resolved(&mut self, state: ResolvedGenerateState) -> Result<()> {
        if self.resolved.is_some() {
            anyhow::bail!("resolved generate state was recorded more than once");
        }
        let mut next = self.clone();
        next.resolved = Some(state);
        next.ensure_no_aliases()?;
        *self = next;
        Ok(())
    }

    fn record_candidate(&mut self, state: CandidateGenerateState) -> Result<()> {
        if self.candidate.is_some() {
            anyhow::bail!("candidate generate state was recorded more than once");
        }
        let mut next = self.clone();
        next.candidate = Some(state);
        next.ensure_no_aliases()?;
        *self = next;
        Ok(())
    }

    fn record_output_target(&mut self, state: OutputTargetReservation) -> Result<()> {
        if self.output_target.is_some() {
            anyhow::bail!("output target reservation was recorded more than once");
        }
        let mut next = self.clone();
        next.output_target = Some(state);
        next.ensure_no_aliases()?;
        *self = next;
        Ok(())
    }

    fn record_published(&mut self, state: PublishedGenerateState) -> Result<()> {
        if self.published.is_some() {
            anyhow::bail!("published generate state was recorded more than once");
        }
        if let Some(target) = self.output_target.as_ref() {
            if target.output_repo != state.output_repo || target.branch != state.branch {
                anyhow::bail!(
                    "published generate state commit does not match reserved output target"
                );
            }
        }
        let mut next = self.clone();
        next.output_target = None;
        next.published = Some(state);
        next.ensure_no_aliases()?;
        *self = next;
        Ok(())
    }

    fn record_failure(&mut self, state: FailureGenerateState) -> Result<()> {
        if self.failure.is_some() {
            anyhow::bail!("failure generate state was recorded more than once");
        }
        let mut next = self.clone();
        next.failure = Some(state);
        next.ensure_no_aliases()?;
        *self = next;
        Ok(())
    }

    fn allowed_path_overlaps(&self) -> Vec<(GenerateStatePhase, GenerateStatePhase)> {
        let mut allowed = Vec::new();
        if let (Some(resolved), Some(output_target)) =
            (self.resolved.as_ref(), self.output_target.as_ref())
        {
            if resolved.output_repo == output_target.output_repo
                && resolved.branch == output_target.branch
            {
                allowed.push((
                    GenerateStatePhase::Resolved,
                    GenerateStatePhase::OutputTarget,
                ));
            }
        }
        if let (Some(resolved), Some(published)) = (self.resolved.as_ref(), self.published.as_ref())
        {
            if resolved.output_repo == published.output_repo && resolved.branch == published.branch
            {
                allowed.push((GenerateStatePhase::Resolved, GenerateStatePhase::Published));
            }
        }
        allowed
    }

    fn ensure_no_aliases(&self) -> Result<()> {
        let mut identities = Vec::new();
        if let Some(state) = self.requested.as_ref() {
            identities.push((GenerateStatePhase::Requested, state.identity()?));
        }
        if let Some(state) = self.resolved.as_ref() {
            identities.push((GenerateStatePhase::Resolved, state.identity.clone()));
        }
        if let Some(state) = self.candidate.as_ref() {
            identities.push((GenerateStatePhase::Candidate, state.identity.clone()));
        }
        if let Some(state) = self.output_target.as_ref() {
            identities.push((GenerateStatePhase::OutputTarget, state.identity.clone()));
        }
        if let Some(state) = self.published.as_ref() {
            identities.push((GenerateStatePhase::Published, state.identity.clone()));
        }
        if let Some(state) = self.failure.as_ref() {
            identities.push((GenerateStatePhase::Failure, state.identity.clone()));
        }

        for (slot_phase, identity) in identities.iter() {
            if *slot_phase != identity.phase {
                anyhow::bail!(
                    "generate state phase mismatch: {} state stored in {} slot",
                    identity.phase.as_str(),
                    slot_phase.as_str()
                );
            }
        }

        for (idx, (_, left)) in identities.iter().enumerate() {
            for (_, right) in identities.iter().skip(idx + 1) {
                if left.phase == right.phase || left.key == right.key {
                    anyhow::bail!(
                        "generate state alias detected between {} and {}",
                        left.phase.as_str(),
                        right.phase.as_str()
                    );
                }
            }
        }

        let mut path_claims = Vec::new();
        if let Some(state) = &self.requested {
            path_claims.push(GenerateStatePathClaim::new(
                GenerateStatePhase::Requested,
                "requested config",
                state.config_path.as_path(),
            )?);
        }
        if let Some(state) = &self.resolved {
            path_claims.extend(state.path_claims()?);
        }
        if let Some(state) = &self.candidate {
            path_claims.extend(state.path_claims()?);
        }
        if let Some(state) = &self.output_target {
            path_claims.extend(state.path_claims()?);
        }
        if let Some(state) = &self.published {
            path_claims.extend(state.path_claims()?);
        }
        if let Some(state) = &self.failure {
            path_claims.extend(state.path_claims()?);
        }
        let allowed_phase_overlaps = self.allowed_path_overlaps();
        for (idx, left) in path_claims.iter().enumerate() {
            for right in path_claims.iter().skip(idx + 1) {
                if left.aliases(right, &allowed_phase_overlaps) {
                    anyhow::bail!(
                        "generate state path alias detected between {} {} ({}) and {} {} ({})",
                        left.phase.as_str(),
                        left.label,
                        left.path.display(),
                        right.phase.as_str(),
                        right.label,
                        right.path.display()
                    );
                }
            }
        }

        Ok(())
    }
}


fn generate_inner(
    config: &KslimConfig,
    profile: &ProfileConfig,
    opts: &GenerateOptions,
    requested: RequestedGenerateState,
    source_maps: Option<GeneratePlanSourceMaps>,
    project_root: Option<&Path>,
    failure: &mut FailureReportContext,
) -> Result<GenerateResult> {
    let upstream_name = &config.upstream.name;
    let upstream_url = &config.upstream.url;

    // ── prepare ─────────────────────────────────────────────────────────
    log_generate_stage(GenerateStage::Resolve, "prepare");

    // ── source ──────────────────────────────────────────────────────────
    set_generate_stage(failure, GenerateStage::Resolve);
    if opts.dry_run {
        log::info!(
            "[dry-run] would verify direct read-only upstream '{}'",
            upstream_name
        );
    } else if opts.frozen_plan.is_some() {
        crate::network_policy::require_local_upstream_url(upstream_url)?;
        log::info!(
            "[frozen-plan] using resolved upstream commit without refreshing '{}'",
            upstream_name
        );
    } else {
        log_generate_stage(GenerateStage::Resolve, "source");
        upstream::sync(upstream_name, upstream_url)?;
    }

    // ── resolve ─────────────────────────────────────────────────────────
    set_generate_stage(failure, GenerateStage::Resolve);
    log_generate_stage(GenerateStage::Resolve, "resolve");
    let plan = plan::resolve_candidate_plan_with_source_maps(
        config,
        profile,
        opts,
        requested,
        source_maps,
    )?;
    let generate_plan = &plan.generate_plan;
    let resolved = &generate_plan.resolved;
    failure.generate_plan = Some(generate_plan.clone());
    failure
        .states
        .record_resolved(ResolvedGenerateState::from_plan(generate_plan)?)?;
    failure.published_metadata_rollback = Some(
        capture_published_metadata_failure_atomic_state(
            resolved.output_plan.output_path.as_path(),
        )?,
    );
    failure.resolved = Some(resolved.base.clone());
    failure.patch_infos = plan.patch_infos.clone();
    failure.mode = Some(resolved.output_plan.mode.clone());

    log::info!(
        "resolved base: {} -> {}",
        resolved.base.r#ref,
        resolved.base.commit
    );

    if opts.dry_run {
        return Ok(dry_run_result_from_plan(generate_plan));
    }
    if opts.report_only {
        return report_only_result_from_plan(generate_plan, project_root);
    }
    resolved.reject_unresolved_feature_conflicts_in_strict_mode()?;

    // ── materialize ─────────────────────────────────────────────────────
    let materialized = candidate::materialize_integrate_and_reduce_candidate_tree(
        generate_plan,
        profile,
        plan.patch_infos.as_deref(),
        opts.keep_temp,
        |event| match event {
            candidate::CandidateMaterializationEvent::Stage(stage) => {
                set_generate_stage(failure, stage);
                Ok(())
            }
            candidate::CandidateMaterializationEvent::Materialized(path) => {
                failure
                    .states
                    .record_candidate(CandidateGenerateState::from_tree_path(path.as_path())?)
            }
        },
    )?;
    let candidate::CandidateMaterialization {
        temp_dir,
        path: temp_path,
        patch_infos,
        mode,
        mut reducer_stats,
    } = materialized;
    failure.patch_infos = patch_infos.clone();
    failure.mode = Some(mode.clone());
    failure.reducer_stats = Some(reducer_stats.clone());
    if let Err(err) = reducer::ensure_supported_fallout(&reducer_stats, &profile.reducer) {
        failure.reducer_failure = Some(reducer::ReducerFailureReport::unsupported_syntax(
            &format!("{:#}", err),
        ));
        return Err(err);
    }
    failure.reducer_stats = Some(reducer_stats.clone());

    set_generate_stage(failure, GenerateStage::Metadata);
    let generated = write_tree_metadata_and_manifest(
        &temp_path,
        config,
        profile,
        &resolved.base,
        &mode,
        &resolved.base.resolved_at,
        patch_infos.as_ref().map(Vec::as_slice),
    )?;
    failure.file_count = Some(generated.file_count);
    failure.total_bytes = Some(generated.total_bytes);

    // ── verify ──────────────────────────────────────────────────────────
    set_generate_stage(failure, GenerateStage::Metadata);
    let verified = verify::verify_generated_output(
        &temp_path,
        profile,
        generate_plan.requested.cli_overrides.run_selftests,
        &mut reducer_stats,
        failure,
    )?;

    // Write report into temp tree
    set_generate_stage(failure, GenerateStage::Metadata);
    write_tree_report(
        &temp_path,
        config,
        profile,
        &resolved.base,
        &generated,
        &mode,
        failure.stage,
        patch_infos.as_ref().map(Vec::as_slice),
        verified.selftests(),
    )?;
    let candidate_verification = verify::write_candidate_metadata_and_verify(
        generate_plan,
        Path::new(&temp_path),
        patch_infos.as_ref().is_some_and(|infos| !infos.is_empty())
            || !resolved.integration_plan.entries.is_empty(),
        reducer_stats.ran,
        verified.selftests().is_some(),
        &reducer_stats,
        profile,
    )?;

    if opts.deep_dry_run {
        return Ok(deep_dry_run_result_from_candidate(
            generate_plan,
            &generated,
            &verified,
        ));
    }

    // ── commit ──────────────────────────────────────────────────────────
    set_generate_stage(failure, GenerateStage::Commit);
    failure
        .states
        .record_output_target(OutputTargetReservation::from_output_target(
            resolved.output_plan.output_path.as_path(),
            &resolved.output_plan.branch,
        )?)?;
    let commit_result = commit_output_repo_state(
        config,
        profile,
        opts,
        &resolved.base,
        generate_plan.fingerprint.as_str(),
        &generated,
        &verified,
        &candidate_verification,
        &resolved.base.resolved_at,
        patch_infos.as_ref().map(Vec::as_slice),
        &mode,
        &resolved.output_plan.branch,
        &reducer_stats,
        failure,
    )?;
    let lockfile_path = project_root
        .map(LockfilePath::new_in_project_root)
        .transpose()?
        .unwrap_or_else(|| LockfilePath::new("kslim.lock").expect("valid default lockfile path"));
    ensure_no_attempt_failure_before_publication(failure)?;
    // Published state may only be materialized from the successful commit
    // phase proof returned after output commit/no-op and HEAD lookup succeed.
    failure
        .states
        .record_published(PublishedGenerateState::from_successful_commit(
            resolved.output_plan.output_path.as_path(),
            lockfile_path,
            &commit_result,
        )?)?;

    drop(temp_dir);

    let (selftests_enabled, built_in_selftests, selftest_commands) = verified
        .selftests()
        .map(|result| (result.enabled, result.built_in_checks, result.commands_run))
        .unwrap_or((false, 0, 0));

    Ok(GenerateResult {
        committed: commit_result.committed,
        branch: commit_result.branch,
        tag: Some(commit_result.tag),
        output_commit: Some(commit_result.output_commit),
        stage: GenerateStage::Commit,
        file_count: generated.file_count,
        total_bytes: generated.total_bytes,
        patch_count: generate_plan.resolved.patch_plan.total_patch_count,
        selftests_enabled,
        built_in_selftests,
        selftest_commands,
    })
}

fn write_tree_metadata_and_manifest(
    tree_path: &str,
    config: &KslimConfig,
    profile: &ProfileConfig,
    resolved: &ResolvedBase,
    mode: &str,
    generated_at: &str,
    patch_infos: Option<&[patches::PatchInfo]>,
) -> Result<GeneratedArtifacts> {
    output_repo::write_base_metadata(tree_path, config, profile, resolved, mode)?;
    output_repo::write_generated_metadata(tree_path, generated_at)?;
    output_repo::write_patch_metadata(tree_path, patch_infos)?;

    let entries = manifest::generate_manifest(tree_path)?;
    manifest::write_manifest(&entries, tree_path)?;

    let file_count = entries.len();
    let total_bytes = entries.iter().map(|entry| entry.size).sum();

    Ok(GeneratedArtifacts {
        entries,
        file_count,
        total_bytes,
    })
}

fn write_tree_report(
    tree_path: &str,
    config: &KslimConfig,
    profile: &ProfileConfig,
    resolved: &ResolvedBase,
    generated: &GeneratedArtifacts,
    mode: &str,
    stage: GenerateStage,
    patch_infos: Option<&[patches::PatchInfo]>,
    selftests: Option<&SelfTestResult>,
) -> Result<()> {
    output_repo::write_report(
        tree_path,
        config,
        profile,
        resolved,
        generated.file_count,
        generated.total_bytes,
        mode,
        stage,
        patch_infos,
        selftests,
    )
}

fn reducer_manifest_for_profile(
    profile: &ProfileConfig,
    root: Option<&std::path::Path>,
) -> Result<Option<crate::removal_manifest::RemovalManifest>> {
    let removal_input = profile.effective_removal_input();
    let preservation_input = profile.effective_preservation_input();
    let abi_policy = profile.effective_abi_policy();
    if removal_input.is_none() && preservation_input.is_none() {
        return Ok(None);
    }
    let removal_input = removal_input.unwrap_or_default();
    match root {
        Some(root) => {
            crate::removal_manifest::RemovalManifest::from_slim_config_for_tree_with_abi_policy_and_preservation(
                root,
                &removal_input,
                preservation_input.as_ref(),
                &abi_policy,
            )
        }
        None => crate::removal_manifest::RemovalManifest::from_slim_config_with_abi_policy_and_preservation(
            &removal_input,
            preservation_input.as_ref(),
            &abi_policy,
        ),
    }
    .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::patches::PatchInfo;
    use crate::paths::RequestedConfigPath;
    use serde::Deserialize;
    use std::path::Path;
    use std::process::Command;

    #[derive(Debug, Deserialize)]
    struct LastAttemptStageFixture {
        stage: GenerateStage,
    }

    fn git_in(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
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

    fn test_resolved_base() -> ResolvedBase {
        ResolvedBase {
            upstream: "linux".to_string(),
            url: "/tmp/linux.git".to_string(),
            r#ref: "v1.0".to_string(),
            commit: "deadbeef".to_string(),
            resolved_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_generate_state_phase_stable_names_cover_all_variants() {
        let cases = [
            (GenerateStatePhase::Requested, "requested"),
            (GenerateStatePhase::Resolved, "resolved"),
            (GenerateStatePhase::Candidate, "candidate"),
            (GenerateStatePhase::OutputTarget, "output_target"),
            (GenerateStatePhase::Published, "published"),
            (GenerateStatePhase::Failure, "failure"),
        ];

        for (phase, stable_name) in cases {
            assert_eq!(phase.as_str(), stable_name);
        }
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

    fn test_requested_state(opts: &GenerateOptions) -> RequestedGenerateState {
        RequestedGenerateState::new(
            RequestedConfigPath::new("/tmp/project/kslim.toml").unwrap(),
            state::ProfileName::new("default").unwrap(),
            state::CliOverrides::from_options(opts),
        )
    }

    fn test_generate_plan_for_output(
        project: &Path,
        output: &Path,
        profile: &ProfileConfig,
        opts: &GenerateOptions,
        branch: &str,
    ) -> plan::GeneratePlan {
        let config = config::default_kslim_config("demo", output.to_str().unwrap());
        let requested =
            RequestedGenerateState::from_inputs(project.join("kslim.toml"), profile, opts).unwrap();
        let resolved = state::ResolvedCandidateState::from_resolved_inputs(
            &config,
            profile,
            test_resolved_base(),
            None,
            "slimmed",
            branch,
        )
        .unwrap();
        plan::GeneratePlan::new(requested, resolved).unwrap()
    }

    fn generate_options_for_test() -> GenerateOptions {
        GenerateOptions {
            dry_run: false,
            deep_dry_run: false,
            report_only: false,
            keep_temp: false,
            max_fixup_passes: None,
            matrix: None,
            offline: false,
            frozen_plan: None,
            force: false,
            base_ref: None,
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            strict: false,
            no_strict: false,
            run_selftests: false,
        }
    }

    fn candidate_verification_for_test(
        root: &Path,
        candidate: &Path,
        output: &Path,
        profile: &ProfileConfig,
        reducer_stats: &reducer::ReducerStats,
    ) -> verify::CandidateVerification {
        let opts = generate_options_for_test();
        let plan = test_generate_plan_for_output(
            &root.join("project"),
            output,
            profile,
            &opts,
            "kslim/v1.0/default",
        );
        verify::write_candidate_metadata_and_verify(
            &plan,
            candidate,
            false,
            reducer_stats.ran,
            false,
            reducer_stats,
            profile,
        )
        .unwrap()
    }

    #[test]
    fn test_dry_run_result_is_derived_from_generate_plan() {
        let config = config::default_kslim_config("demo", "/tmp/output");
        let mut profile = config::default_profile_config("v1.0");
        profile.selftests.enabled = false;
        let opts = GenerateOptions {
            dry_run: true,
            deep_dry_run: false,
            report_only: false,
            keep_temp: false,
            max_fixup_passes: None,
            matrix: None,
            offline: false,
            frozen_plan: None,
            force: false,
            base_ref: Some("HEAD".to_string()),
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            strict: false,
            no_strict: false,
            run_selftests: true,
        };
        let mut resolved = test_resolved_base();
        resolved.r#ref = "HEAD".to_string();
        resolved.commit = "feedface".to_string();
        let resolved = state::ResolvedCandidateState::from_resolved_inputs(
            &config,
            &profile,
            resolved,
            None,
            "unmodified-upstream",
            "kslim/HEAD/default",
        )
        .unwrap();
        let plan = plan::GeneratePlan::new(test_requested_state(&opts), resolved).unwrap();

        let result = dry_run_result_from_plan(&plan);

        assert!(!result.committed);
        assert_eq!(result.stage, GenerateStage::Resolve);
        assert_eq!(result.branch, "kslim/HEAD/default");
        assert_eq!(result.patch_count, 0);
        assert!(result.selftests_enabled);
    }

    #[test]
    fn test_report_only_result_is_derived_from_generate_plan_and_attempt_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&project).unwrap();

        let config = config::default_kslim_config("demo", output.to_str().unwrap());
        let mut profile = config::default_profile_config("v1.0");
        profile.selftests.enabled = false;
        let opts = GenerateOptions {
            dry_run: false,
            deep_dry_run: false,
            report_only: true,
            keep_temp: false,
            max_fixup_passes: None,
            matrix: None,
            offline: false,
            frozen_plan: None,
            force: false,
            base_ref: Some("HEAD".to_string()),
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            strict: false,
            no_strict: false,
            run_selftests: false,
        };
        let mut resolved = test_resolved_base();
        resolved.r#ref = "HEAD".to_string();
        resolved.commit = "feedface".to_string();
        let resolved = state::ResolvedCandidateState::from_resolved_inputs(
            &config,
            &profile,
            resolved,
            None,
            "unmodified-upstream",
            "kslim/HEAD/default",
        )
        .unwrap();
        let plan = plan::GeneratePlan::new(test_requested_state(&opts), resolved).unwrap();

        let result = report_only_result_from_plan(&plan, Some(&project)).unwrap();

        assert!(!result.committed);
        assert_eq!(result.stage, GenerateStage::Resolve);
        assert_eq!(result.branch, "kslim/HEAD/default");
        assert_eq!(result.patch_count, 0);
        assert!(!result.selftests_enabled);
        assert!(!output.exists());

        let report = std::fs::read_to_string(project_failure_report_path(&project)).unwrap();
        assert!(report.contains("Status: report-only"));
        assert!(report.contains("Authoritative: false"));
        assert!(report.contains(plan.plan_id.as_str()));
        assert!(report.contains(plan.fingerprint.as_str()));
        assert!(report.contains(plan.config_content_hash.as_str()));
        assert!(report.contains("Base ref: HEAD"));
        assert!(report.contains("Base commit: feedface"));
        assert!(report.contains("Source map: unavailable"));
    }

    #[test]
    fn test_report_only_report_includes_source_map_when_available() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&project).unwrap();

        let config = config::default_kslim_config("demo", output.to_str().unwrap());
        let mut profile = config::default_profile_config("v1.0");
        profile.selftests.enabled = false;
        let opts = GenerateOptions {
            dry_run: false,
            deep_dry_run: false,
            report_only: true,
            keep_temp: false,
            max_fixup_passes: None,
            matrix: None,
            offline: false,
            frozen_plan: None,
            force: false,
            base_ref: Some("HEAD".to_string()),
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            strict: false,
            no_strict: false,
            run_selftests: false,
        };
        let mut resolved = test_resolved_base();
        resolved.r#ref = "HEAD".to_string();
        resolved.commit = "feedface".to_string();
        let resolved = state::ResolvedCandidateState::from_resolved_inputs(
            &config,
            &profile,
            resolved,
            None,
            "unmodified-upstream",
            "kslim/HEAD/default",
        )
        .unwrap();

        let mut config_source_map = config::ConfigSourceMap::default();
        config_source_map.insert(
            "project.name",
            config::ConfigSourceKind::ConfigFile,
            project.join("kslim.toml").display().to_string(),
        );
        config_source_map.insert(
            "output.branch_prefix",
            config::ConfigSourceKind::Default,
            "built-in default",
        );
        let mut profile_source_map = config::ConfigSourceMap::default();
        profile_source_map.insert(
            "base.ref",
            config::ConfigSourceKind::Profile,
            project.join("profiles/default.toml").display().to_string(),
        );
        let mut override_source_map = config::ConfigSourceMap::default();
        override_source_map.insert_cli_override("base.ref", "cli --base");

        let plan = plan::GeneratePlan::new(test_requested_state(&opts), resolved)
            .unwrap()
            .with_source_maps(plan::GeneratePlanSourceMaps::new(
                config_source_map,
                profile_source_map,
                override_source_map,
            ))
            .unwrap();

        report_only_result_from_plan(&plan, Some(&project)).unwrap();

        let report = std::fs::read_to_string(project_failure_report_path(&project)).unwrap();
        assert!(report.contains("Source map:"));
        assert!(report.contains("  Config:"));
        assert!(report.contains("    project.name: config_file ("));
        assert!(report.contains("    output.branch_prefix: default (built-in default)"));
        assert!(report.contains("  Profile:"));
        assert!(report.contains("    base.ref: profile ("));
        assert!(report.contains("  Overrides:"));
        assert!(report.contains("    base.ref: cli (cli --base)"));
    }

    #[test]
    fn test_execute_patch_phase_must_match_resolved_plan() {
        let planned = PatchInfo {
            source: "worktree".to_string(),
            worktree_path: "/tmp/patches".to_string(),
            branch: "topic".to_string(),
            head_commit: "abc123".to_string(),
            merge_base: "base123".to_string(),
            base_remote: "origin".to_string(),
            base_ref: "main".to_string(),
            patch_count: 1,
        };
        let mut changed = planned.clone();
        changed.head_commit = "changed".to_string();

        candidate::ensure_patch_application_matches_plan(Some(&[planned.clone()]), Some(&[planned]))
            .unwrap();
        let err = candidate::ensure_patch_application_matches_plan(Some(&[changed]), None)
            .unwrap_err()
            .to_string();
        assert!(err.contains("no longer match the resolved generate plan"));
    }

    #[test]
    fn test_attempt_failure_cannot_pass_publication_guard() {
        let tmp = tempfile::tempdir().unwrap();
        let attempt_dir =
            AttemptMetadataDir::new(tmp.path().join(".kslim").join("attempt")).unwrap();
        let report_path = ReportPath::new(attempt_dir.as_path().join("report.txt")).unwrap();
        let failure = FailureReportContext {
            attempt_failure: Some(
                GenerateAttemptFailure::from_stage(
                    GenerateStage::Publish,
                    "publish failed",
                    attempt_dir,
                    vec![report_path],
                )
                .unwrap(),
            ),
            ..FailureReportContext::default()
        };

        let err = ensure_no_attempt_failure_before_publication(&failure)
            .unwrap_err()
            .to_string();

        assert!(err.contains("cannot be converted into published state"));
        assert!(err.contains("publish"));
    }

    fn create_committed_output_fixture(
        root: &Path,
    ) -> (
        KslimConfig,
        ProfileConfig,
        ResolvedBase,
        GeneratedArtifacts,
        SuccessfulCommitResult,
    ) {
        let snapshot = root.join("snapshot");
        create_minimal_tree(&snapshot);

        let output = root.join("output");
        let mut config = config::default_kslim_config("demo", output.to_str().unwrap());
        config.upstream.url = "/tmp/linux.git".to_string();
        let mut profile = config::default_profile_config("v1.0");
        profile.selftests.enabled = false;
        let resolved = test_resolved_base();
        let generated = write_tree_metadata_and_manifest(
            snapshot.to_str().unwrap(),
            &config,
            &profile,
            &resolved,
            "unmodified-upstream",
            &resolved.resolved_at,
            None,
        )
        .unwrap();
        let mut failure = FailureReportContext {
            stage: GenerateStage::Commit,
            ..FailureReportContext::default()
        };
        let mut reducer_stats = reducer::ReducerStats::default();
        let verified = verify::verify_generated_output(
            snapshot.to_str().unwrap(),
            &profile,
            false,
            &mut reducer_stats,
            &mut failure,
        )
        .unwrap();
        let verification = candidate_verification_for_test(
            root,
            snapshot.as_path(),
            output.as_path(),
            &profile,
            &reducer_stats,
        );
        let commit = commit_output_repo_state(
            &config,
            &profile,
            &GenerateOptions {
                dry_run: false,
                deep_dry_run: false,
                report_only: false,
                keep_temp: false,
                max_fixup_passes: None,
                matrix: None,
                offline: false,
                frozen_plan: None,
                force: false,
                base_ref: None,
                feature: None,
                remove_feature: None,
                preserve_feature: None,
                arch: None,
                primary_arch: None,
                secondary_arch: None,
                safety: None,
                strict: false,
                no_strict: false,
                run_selftests: false,
            },
            &resolved,
            "fingerprint-test",
            &generated,
            &verified,
            &verification,
            &resolved.resolved_at,
            None,
            "unmodified-upstream",
            &output_repo::branch_name(&config, &profile, &resolved),
            &reducer_stats,
            &mut failure,
        )
        .unwrap();

        (config, profile, resolved, generated, commit)
    }

    #[test]
    fn test_project_failure_paths_are_non_authoritative_attempt_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let attempt_dir = project_attempt_metadata_dir(tmp.path());
        let report_path = project_failure_report_path(tmp.path());
        let reducer_failure_path = project_reducer_failure_path(tmp.path());
        let last_attempt_path = project_last_attempt_path(tmp.path());

        assert_eq!(attempt_dir, tmp.path().join(".kslim").join("attempt"));
        assert_eq!(report_path, attempt_dir.join("report.txt"));
        assert_eq!(
            reducer_failure_path,
            attempt_dir.join(output_repo::REDUCER_FAILURE_JSON)
        );
        assert_eq!(last_attempt_path, attempt_dir.join("last-attempt.json"));
        ensure_non_authoritative_attempt_path(tmp.path(), &report_path).unwrap();
        ensure_non_authoritative_attempt_path(tmp.path(), &reducer_failure_path).unwrap();
        ensure_non_authoritative_attempt_path(tmp.path(), &last_attempt_path).unwrap();
    }

    #[test]
    fn test_write_project_last_attempt_writes_non_authoritative_attempt_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let failure = FailureReportContext {
            stage: GenerateStage::Selftest,
            ..Default::default()
        };

        write_project_last_attempt(tmp.path(), &failure, "selftest failed").unwrap();

        let path = project_last_attempt_path(tmp.path());
        let content = std::fs::read_to_string(&path).unwrap();
        output_repo::validate_last_attempt_json(&content).unwrap();
        let decoded: LastAttemptStageFixture = serde_json::from_str(&content).unwrap();
        assert_eq!(decoded.stage, GenerateStage::Selftest);
        assert!(content.contains("\"authoritative\": false"));
        assert!(content.contains("\"metadata_scope\": \"non-authoritative-attempt\""));
        assert!(content.contains("\"stage\": \"selftest\""));
        assert!(content.contains("\"updated\": false"));
        assert!(
            !tmp.path().join(".kslim/last-attempt.json").exists(),
            "last-attempt metadata must stay inside the attempt namespace"
        );
    }

    #[test]
    fn test_failed_generate_final_guard_restores_existing_lockfile() {
        let tmp = tempfile::tempdir().unwrap();
        let lockfile_path = LockfilePath::new_in_project_root(tmp.path()).unwrap();
        std::fs::write(lockfile_path.as_path(), "authoritative lockfile\n").unwrap();
        let failure = FailureReportContext {
            stage: GenerateStage::Selftest,
            lockfile_rollback: Some(
                crate::lockfile::capture_lockfile_failure_atomic_state(&lockfile_path).unwrap(),
            ),
            ..Default::default()
        };

        std::fs::write(lockfile_path.as_path(), "failed run lockfile\n").unwrap();

        rollback_failed_run_lockfile_state(&failure).unwrap();

        assert_eq!(
            std::fs::read_to_string(lockfile_path.as_path()).unwrap(),
            "authoritative lockfile\n"
        );
    }

    #[test]
    fn test_failed_generate_final_guard_removes_created_lockfile() {
        let tmp = tempfile::tempdir().unwrap();
        let lockfile_path = LockfilePath::new_in_project_root(tmp.path()).unwrap();
        let failure = FailureReportContext {
            stage: GenerateStage::Selftest,
            lockfile_rollback: Some(
                crate::lockfile::capture_lockfile_failure_atomic_state(&lockfile_path).unwrap(),
            ),
            ..Default::default()
        };

        std::fs::write(lockfile_path.as_path(), "failed run lockfile\n").unwrap();

        rollback_failed_run_lockfile_state(&failure).unwrap();

        assert!(!lockfile_path.as_path().exists());
    }

    #[test]
    fn test_last_attempt_metadata_rejects_legacy_stage_aliases() {
        for legacy_stage in [
            "prepare",
            "source",
            "lockfile",
            "reducer",
            "verify",
            "output-commit",
            "output-publish",
        ] {
            let decoded = serde_json::from_str::<LastAttemptStageFixture>(&format!(
                "{{\"stage\":\"{}\"}}",
                legacy_stage
            ));
            assert!(
                decoded.is_err(),
                "last-attempt metadata must reject legacy stage alias: {}",
                legacy_stage
            );
        }
    }

    #[test]
    fn test_non_authoritative_attempt_path_guard_rejects_authoritative_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let err = ensure_non_authoritative_attempt_path(tmp.path(), &tmp.path().join("kslim.lock"))
            .unwrap_err()
            .to_string();

        assert!(err.contains("outside non-authoritative attempt metadata"));
    }

    #[test]
    fn test_clear_project_failure_artifacts_removes_legacy_attempt_metadata_without_reader() {
        let tmp = tempfile::tempdir().unwrap();
        let attempt_dir = project_attempt_metadata_dir(tmp.path());
        std::fs::create_dir_all(&attempt_dir).unwrap();
        std::fs::write(
            attempt_dir.join("last-attempt.json"),
            r#"{"authoritative":false,"stage":"lockfile"}"#,
        )
        .unwrap();
        std::fs::write(
            attempt_dir.join(output_repo::REDUCER_FAILURE_JSON),
            r#"{"stage":"reducer"}"#,
        )
        .unwrap();
        std::fs::write(attempt_dir.join("report.txt"), "Stage: output-commit\n").unwrap();

        clear_project_failure_artifacts(tmp.path()).unwrap();

        assert!(
            !attempt_dir.exists(),
            "legacy attempt metadata should be discarded without parsing or migration"
        );
    }

    #[test]
    fn test_generate_state_ledger_records_distinct_lifecycle_phases() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let candidate = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&candidate).unwrap();
        std::fs::create_dir_all(&output).unwrap();

        let profile = config::default_profile_config("v1.0");
        let opts = GenerateOptions {
            dry_run: false,
            deep_dry_run: false,
            report_only: false,
            keep_temp: false,
            max_fixup_passes: None,
            matrix: None,
            offline: false,
            frozen_plan: None,
            force: false,
            base_ref: Some(String::from("v1.0-test")),
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            strict: false,
            no_strict: false,
            run_selftests: false,
        };
        let commit = SuccessfulCommitResult {
            committed: true,
            branch: String::from("kslim/v1.0/default"),
            tag: String::from("kslim-v1.0-default-1"),
            output_commit: String::from("deadbeef"),
        };

        let mut states = GenerateStateLedger::default();
        states
            .record_requested(
                RequestedGenerateState::from_inputs(project.join("kslim.toml"), &profile, &opts)
                    .unwrap(),
            )
            .unwrap();
        let generate_plan =
            test_generate_plan_for_output(&project, &output, &profile, &opts, &commit.branch);
        states
            .record_resolved(ResolvedGenerateState::from_plan(&generate_plan).unwrap())
            .unwrap();
        states
            .record_candidate(CandidateGenerateState::from_tree_path(&candidate).unwrap())
            .unwrap();
        states
            .record_failure(FailureGenerateState::from_project_root(&project).unwrap())
            .unwrap();
        states
            .record_output_target(
                OutputTargetReservation::from_output_target(&output, &commit.branch).unwrap(),
            )
            .unwrap();
        assert!(states.output_target.is_some());
        assert!(states.published.is_none());
        states
            .record_published(
                PublishedGenerateState::from_successful_commit(
                    &output,
                    LockfilePath::new_in_project_root(&project).unwrap(),
                    &commit,
                )
                .unwrap(),
            )
            .unwrap();

        states.ensure_no_aliases().unwrap();
        assert!(states.resolved.is_some());
        assert!(states.output_target.is_none());
        assert_eq!(
            states
                .published
                .as_ref()
                .unwrap()
                .snapshot
                .commit()
                .as_str(),
            "deadbeef"
        );
    }

    #[test]
    fn test_generate_state_ledger_rejects_cross_phase_identity_alias() {
        let mut states = GenerateStateLedger::default();
        let profile = config::default_profile_config("v1.0");
        let opts = GenerateOptions {
            dry_run: false,
            deep_dry_run: false,
            report_only: false,
            keep_temp: false,
            max_fixup_passes: None,
            matrix: None,
            offline: false,
            frozen_plan: None,
            force: false,
            base_ref: None,
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            strict: false,
            no_strict: false,
            run_selftests: false,
        };
        let requested =
            RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
                .unwrap();
        let requested_identity = requested.identity().unwrap();
        states.requested = Some(requested);
        states.candidate = Some(CandidateGenerateState {
            identity: GenerateStateIdentity::new(
                GenerateStatePhase::Candidate,
                requested_identity.key,
            )
            .unwrap(),
            tree: CandidateTreeState::from_materialized_tree("/tmp/candidate").unwrap(),
        });

        let err = states.ensure_no_aliases().unwrap_err().to_string();
        assert!(err.contains("generate state alias detected"));
    }

    #[test]
    fn test_generate_state_ledger_rejects_cross_phase_path_alias() {
        let tmp = tempfile::tempdir().unwrap();
        let mut states = GenerateStateLedger::default();
        states
            .record_candidate(CandidateGenerateState::from_tree_path(tmp.path()).unwrap())
            .unwrap();

        let err = states
            .record_failure(FailureGenerateState::from_project_root(tmp.path()).unwrap())
            .unwrap_err()
            .to_string();

        assert!(err.contains("generate state path alias detected"));
        assert!(states.failure.is_none());
    }

    #[test]
    fn test_generate_state_ledger_rejects_requested_resolved_path_alias() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let profile = config::default_profile_config("v1.0");
        let opts = GenerateOptions {
            dry_run: false,
            deep_dry_run: false,
            report_only: false,
            keep_temp: false,
            max_fixup_passes: None,
            matrix: None,
            offline: false,
            frozen_plan: None,
            force: false,
            base_ref: None,
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            strict: false,
            no_strict: false,
            run_selftests: false,
        };
        let generate_plan =
            test_generate_plan_for_output(&project, &project, &profile, &opts, "kslim/test");
        let mut states = GenerateStateLedger::default();
        states
            .record_requested(
                RequestedGenerateState::from_inputs(project.join("kslim.toml"), &profile, &opts)
                    .unwrap(),
            )
            .unwrap();

        let err = states
            .record_resolved(ResolvedGenerateState::from_plan(&generate_plan).unwrap())
            .unwrap_err()
            .to_string();

        assert!(err.contains("generate state path alias detected"));
        assert!(states.resolved.is_none());
    }

    #[test]
    fn test_generate_state_ledger_rejects_resolved_candidate_path_alias() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let output = tmp.path().join("candidate-output");
        let profile = config::default_profile_config("v1.0");
        let opts = GenerateOptions {
            dry_run: false,
            deep_dry_run: false,
            report_only: false,
            keep_temp: false,
            max_fixup_passes: None,
            matrix: None,
            offline: false,
            frozen_plan: None,
            force: false,
            base_ref: None,
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            strict: false,
            no_strict: false,
            run_selftests: false,
        };
        let generate_plan =
            test_generate_plan_for_output(&project, &output, &profile, &opts, "kslim/test");
        let mut states = GenerateStateLedger::default();
        states
            .record_resolved(ResolvedGenerateState::from_plan(&generate_plan).unwrap())
            .unwrap();

        let err = states
            .record_candidate(CandidateGenerateState::from_tree_path(&output).unwrap())
            .unwrap_err()
            .to_string();

        assert!(err.contains("generate state path alias detected"));
        assert!(states.candidate.is_none());
    }

    #[test]
    fn test_generate_state_ledger_rejects_resolved_failure_path_alias() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let profile = config::default_profile_config("v1.0");
        let opts = GenerateOptions {
            dry_run: false,
            deep_dry_run: false,
            report_only: false,
            keep_temp: false,
            max_fixup_passes: None,
            matrix: None,
            offline: false,
            frozen_plan: None,
            force: false,
            base_ref: None,
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            strict: false,
            no_strict: false,
            run_selftests: false,
        };
        let generate_plan =
            test_generate_plan_for_output(&project, &project, &profile, &opts, "kslim/test");
        let mut states = GenerateStateLedger::default();
        states
            .record_failure(FailureGenerateState::from_project_root(&project).unwrap())
            .unwrap();

        let err = states
            .record_resolved(ResolvedGenerateState::from_plan(&generate_plan).unwrap())
            .unwrap_err()
            .to_string();

        assert!(err.contains("generate state path alias detected"));
        assert!(states.resolved.is_none());
    }

    #[test]
    fn test_generate_state_ledger_rejects_resolved_published_path_alias_when_target_differs() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let output = tmp.path().join("output");
        let profile = config::default_profile_config("v1.0");
        let opts = GenerateOptions {
            dry_run: false,
            deep_dry_run: false,
            report_only: false,
            keep_temp: false,
            max_fixup_passes: None,
            matrix: None,
            offline: false,
            frozen_plan: None,
            force: false,
            base_ref: None,
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            strict: false,
            no_strict: false,
            run_selftests: false,
        };
        let generate_plan =
            test_generate_plan_for_output(&project, &output, &profile, &opts, "kslim/resolved");
        let commit = SuccessfulCommitResult {
            committed: true,
            branch: String::from("kslim/published"),
            tag: String::from("kslim-published-1"),
            output_commit: String::from("deadbeef"),
        };
        let mut states = GenerateStateLedger::default();
        states
            .record_resolved(ResolvedGenerateState::from_plan(&generate_plan).unwrap())
            .unwrap();

        let err = states
            .record_published(
                PublishedGenerateState::from_successful_commit(
                    &output,
                    LockfilePath::new_in_project_root(&project).unwrap(),
                    &commit,
                )
                .unwrap(),
            )
            .unwrap_err()
            .to_string();

        assert!(err.contains("generate state path alias detected"));
        assert!(states.published.is_none());
    }

    #[test]
    fn test_generate_state_ledger_rejects_duplicate_resolved_state() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let output = tmp.path().join("output");
        let profile = config::default_profile_config("v1.0");
        let opts = GenerateOptions {
            dry_run: false,
            deep_dry_run: false,
            report_only: false,
            keep_temp: false,
            max_fixup_passes: None,
            matrix: None,
            offline: false,
            frozen_plan: None,
            force: false,
            base_ref: None,
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            strict: false,
            no_strict: false,
            run_selftests: false,
        };
        let generate_plan =
            test_generate_plan_for_output(&project, &output, &profile, &opts, "kslim/test");
        let mut states = GenerateStateLedger::default();
        states
            .record_resolved(ResolvedGenerateState::from_plan(&generate_plan).unwrap())
            .unwrap();

        let err = states
            .record_resolved(ResolvedGenerateState::from_plan(&generate_plan).unwrap())
            .unwrap_err()
            .to_string();

        assert!(err.contains("resolved generate state was recorded more than once"));
    }

    #[test]
    fn test_generate_state_ledger_rejects_published_target_alias_before_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let mut states = GenerateStateLedger::default();
        states
            .record_failure(FailureGenerateState::from_project_root(tmp.path()).unwrap())
            .unwrap();

        let err = states
            .record_output_target(
                OutputTargetReservation::from_output_target(tmp.path(), "kslim/test").unwrap(),
            )
            .unwrap_err()
            .to_string();

        assert!(err.contains("generate state path alias detected"));
        assert!(states.output_target.is_none());
        assert!(states.published.is_none());
    }

    #[test]
    fn test_generate_state_ledger_rejects_duplicate_phase_recording() {
        let profile = config::default_profile_config("v1.0");
        let opts = GenerateOptions {
            dry_run: false,
            deep_dry_run: false,
            report_only: false,
            keep_temp: false,
            max_fixup_passes: None,
            matrix: None,
            offline: false,
            frozen_plan: None,
            force: false,
            base_ref: None,
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            strict: false,
            no_strict: false,
            run_selftests: false,
        };

        let mut states = GenerateStateLedger::default();
        states
            .record_requested(
                RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
                    .unwrap(),
            )
            .unwrap();
        let err = states
            .record_requested(
                RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
                    .unwrap(),
            )
            .unwrap_err()
            .to_string();

        assert!(err.contains("requested generate state was recorded more than once"));
    }

    #[test]
    fn test_write_tree_metadata_and_manifest_writes_expected_artifacts() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("Makefile"), "# test\n").unwrap();
        std::fs::create_dir_all(tmp.path().join("include")).unwrap();
        std::fs::write(tmp.path().join("include/test.h"), "#define TEST 1\n").unwrap();

        let config = config::default_kslim_config("demo", "/tmp/output");
        let profile = config::default_profile_config("v1.0");
        let resolved = test_resolved_base();

        let generated = write_tree_metadata_and_manifest(
            tmp.path().to_str().unwrap(),
            &config,
            &profile,
            &resolved,
            "slimmed",
            &resolved.resolved_at,
            None,
        )
        .unwrap();

        assert!(tmp.path().join(".kslim/base.toml").exists());
        assert!(tmp.path().join(".kslim/generated.toml").exists());
        assert!(tmp.path().join(".kslim/manifest.txt").exists());
        assert_eq!(generated.file_count, generated.entries.len());
        assert!(generated
            .entries
            .iter()
            .any(|entry| entry.path == "Makefile"));

        write_tree_report(
            tmp.path().to_str().unwrap(),
            &config,
            &profile,
            &resolved,
            &generated,
            "slimmed",
            GenerateStage::Metadata,
            None,
            Some(&SelfTestResult {
                enabled: true,
                built_in_checks: 2,
                kernel_builds_run: 1,
                commands_run: 0,
            }),
        )
        .unwrap();

        let report = std::fs::read_to_string(tmp.path().join(".kslim/report.txt")).unwrap();
        assert!(report.contains("Profile: default"));
        assert!(report.contains("Mode: slimmed"));
        assert!(report.contains("Stage: metadata"));
        assert!(report.contains("Kernel build checks: 1"));
    }

    #[test]
    fn test_write_output_metadata_report_and_manifest_writes_git_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        std::fs::create_dir_all(output.join(".git")).unwrap();
        let verified_tree = tmp.path().join("verified-tree");
        create_minimal_tree(&verified_tree);

        let config = config::default_kslim_config("demo", output.to_str().unwrap());
        let mut profile = config::default_profile_config("v1.0");
        profile.selftests.enabled = false;
        let resolved = test_resolved_base();
        let generated = write_tree_metadata_and_manifest(
            verified_tree.to_str().unwrap(),
            &config,
            &profile,
            &resolved,
            "unmodified-upstream",
            &resolved.resolved_at,
            None,
        )
        .unwrap();
        let mut failure = FailureReportContext {
            stage: GenerateStage::Metadata,
            ..FailureReportContext::default()
        };
        let mut verify_stats = reducer::ReducerStats::default();
        let verified = verify::verify_generated_output(
            verified_tree.to_str().unwrap(),
            &profile,
            false,
            &mut verify_stats,
            &mut failure,
        )
        .unwrap();
        let reducer_stats = reducer::ReducerStats {
            ran: true,
            files_removed: 1,
            dirs_removed: 2,
            ..reducer::ReducerStats::default()
        };
        let verification = candidate_verification_for_test(
            tmp.path(),
            verified_tree.as_path(),
            output.as_path(),
            &profile,
            &reducer_stats,
        );

        let published_metadata = publish::write_output_metadata_report_and_manifest(
            output.to_str().unwrap(),
            &config,
            &profile,
            &resolved,
            &generated,
            &resolved.resolved_at,
            GenerateStage::Metadata,
            None,
            "unmodified-upstream",
            "kslim/v1.0/default",
            "kslim-v1.0-default-r1",
            Some(&reducer_stats),
            &verified,
            &verification,
        )
        .unwrap();

        let metadata_dir = output.join(".git/kslim");
        assert!(metadata_dir.join("base.toml").exists());
        assert!(metadata_dir.join("generated.toml").exists());
        assert!(metadata_dir.join("report.txt").exists());
        assert!(metadata_dir.join("manifest.txt").exists());
        assert!(!metadata_dir.join("published.toml").exists());
        assert!(metadata_dir.join("reducer-report.md").exists());
        assert!(metadata_dir.join("reducer-report.json").exists());
        assert!(metadata_dir.join("diagnostics.json").exists());
        assert!(metadata_dir.join("edit-summary.json").exists());
        assert!(metadata_dir.join("kconfig-solver-report.json").exists());
        assert!(metadata_dir.join("kconfig-rewrite-report.json").exists());
        assert!(std::fs::read_to_string(metadata_dir.join(output_repo::REDUCER_REPORT_JSON))
            .unwrap()
            .contains("\"schema_version\""));

        let manifest = std::fs::read_to_string(metadata_dir.join("manifest.txt")).unwrap();
        assert!(manifest.contains("Makefile"));

        let report = std::fs::read_to_string(metadata_dir.join("report.txt")).unwrap();
        assert!(report.contains("Mode: unmodified-upstream"));
        assert!(report.contains("Stage: metadata"));
        assert!(report.contains("Files: 1"));

        let output_repo = OutputRepoPath::new(&output).unwrap();
        output_repo::write_verified_published_snapshot_metadata(&output_repo, &published_metadata)
            .unwrap();
        output_repo::write_verified_committed_published_snapshot_metadata(
            &output_repo,
            &published_metadata,
            &[],
        )
        .unwrap();
        let published = std::fs::read_to_string(metadata_dir.join("published.toml")).unwrap();
        assert!(published.contains("branch = \"kslim/v1.0/default\""));
        assert!(published.contains("tag = \"kslim-v1.0-default-r1\""));
        assert!(published.contains("candidate_metadata_fingerprint = \"metadata-"));
        assert!(output.join(".kslim/published.toml").exists());
    }

    #[test]
    fn test_failed_run_output_rollback_restores_visible_commit_refs() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&output).unwrap();
        git_in(&output, &["init"]);
        git_in(&output, &["config", "user.email", "test@kslim.local"]);
        git_in(&output, &["config", "user.name", "kslim test"]);
        std::fs::write(output.join("Makefile"), "# original\n").unwrap();
        git_in(&output, &["add", "-A"]);
        git_in(&output, &["commit", "-m", "original"]);
        std::fs::create_dir_all(output.join(".git/kslim")).unwrap();
        std::fs::write(
            output.join(".git/kslim/managed.toml"),
            "managed_by = \"kslim\"\n",
        )
        .unwrap();
        let output_path = output.to_str().unwrap();
        let original_head = git_in(&output, &["rev-parse", "HEAD"]);
        let original_branch = git_in(&output, &["branch", "--show-current"]);
        let rollback_state =
            capture_output_repo_failure_atomic_state(output_path, "kslim/test").unwrap();

        git_in(&output, &["checkout", "-B", "kslim/test"]);
        std::fs::write(output.join("Makefile"), "# failed candidate\n").unwrap();
        git_in(&output, &["add", "-A"]);
        git_in(&output, &["commit", "-m", "failed candidate"]);

        let err = failure::ensure_failed_run_output_commits_unmodified(output_path, &rollback_state)
            .unwrap_err()
            .to_string();
        assert!(err.contains("failed run updated output HEAD"));

        rollback_output_repo_failure_atomic_state(output_path, &rollback_state).unwrap();

        assert_eq!(git_in(&output, &["rev-parse", "HEAD"]), original_head);
        assert_eq!(
            git_in(&output, &["branch", "--show-current"]),
            original_branch
        );
        assert!(
            git_in(&output, &["branch", "--list", "kslim/test"])
                .trim()
                .is_empty(),
            "rollback must remove the failed target branch"
        );
    }

    #[test]
    fn test_failed_run_output_rollback_restores_published_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&output).unwrap();
        git_in(&output, &["init"]);
        git_in(&output, &["config", "user.email", "test@kslim.local"]);
        git_in(&output, &["config", "user.name", "kslim test"]);
        std::fs::write(output.join("Makefile"), "# original\n").unwrap();
        git_in(&output, &["add", "-A"]);
        git_in(&output, &["commit", "-m", "original"]);

        let metadata_dir = output.join(".git/kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        std::fs::write(
            metadata_dir.join("managed.toml"),
            "managed_by = \"kslim\"\n",
        )
        .unwrap();
        std::fs::write(metadata_dir.join("published.toml"), "branch = \"old\"\n").unwrap();

        let output_path = output.to_str().unwrap();
        let rollback_state =
            capture_output_repo_failure_atomic_state(output_path, "kslim/test").unwrap();

        std::fs::write(metadata_dir.join("published.toml"), "branch = \"failed\"\n").unwrap();
        std::fs::write(
            metadata_dir.join("failed-extra.toml"),
            "authoritative = true\n",
        )
        .unwrap();

        let err = failure::ensure_failed_run_published_metadata_unmodified(output_path, &rollback_state)
            .unwrap_err()
            .to_string();
        assert!(err.contains("failed run updated published metadata"));

        rollback_output_repo_failure_atomic_state(output_path, &rollback_state).unwrap();

        assert_eq!(
            std::fs::read_to_string(metadata_dir.join("managed.toml")).unwrap(),
            "managed_by = \"kslim\"\n"
        );
        assert_eq!(
            std::fs::read_to_string(metadata_dir.join("published.toml")).unwrap(),
            "branch = \"old\"\n"
        );
        assert!(!metadata_dir.join("failed-extra.toml").exists());
    }

    #[test]
    fn test_failed_run_published_metadata_guard_restores_without_output_transaction() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&output).unwrap();
        git_in(&output, &["init"]);
        git_in(&output, &["config", "user.email", "test@kslim.local"]);
        git_in(&output, &["config", "user.name", "kslim test"]);

        let metadata_dir = output.join(".git/kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        std::fs::write(
            metadata_dir.join("managed.toml"),
            "managed_by = \"kslim\"\n",
        )
        .unwrap();
        std::fs::write(metadata_dir.join("published.toml"), "branch = \"old\"\n").unwrap();

        let rollback_state =
            capture_published_metadata_failure_atomic_state(output.as_path()).unwrap();
        std::fs::write(metadata_dir.join("published.toml"), "branch = \"failed\"\n").unwrap();
        std::fs::write(
            metadata_dir.join("failed-extra.toml"),
            "authoritative = true\n",
        )
        .unwrap();

        let err = failure::ensure_failed_run_published_metadata_snapshot_unmodified(&rollback_state)
            .unwrap_err()
            .to_string();
        assert!(err.contains("failed run updated published metadata"));

        rollback_published_metadata_failure_atomic_state(&rollback_state).unwrap();

        assert_eq!(
            std::fs::read_to_string(metadata_dir.join("managed.toml")).unwrap(),
            "managed_by = \"kslim\"\n"
        );
        assert_eq!(
            std::fs::read_to_string(metadata_dir.join("published.toml")).unwrap(),
            "branch = \"old\"\n"
        );
        assert!(!metadata_dir.join("failed-extra.toml").exists());
    }

    #[test]
    fn test_failed_run_published_metadata_guard_removes_new_metadata_without_output_transaction() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&output).unwrap();

        let rollback_state =
            capture_published_metadata_failure_atomic_state(output.as_path()).unwrap();
        let metadata_dir = output.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        std::fs::write(metadata_dir.join("published.toml"), "branch = \"failed\"\n").unwrap();

        let err = failure::ensure_failed_run_published_metadata_snapshot_unmodified(&rollback_state)
            .unwrap_err()
            .to_string();
        assert!(err.contains("failed run created published metadata path"));

        rollback_published_metadata_failure_atomic_state(&rollback_state).unwrap();

        assert!(!metadata_dir.exists());
        assert!(output.exists());
    }

    #[test]
    fn test_authoritative_lockfile_is_derived_from_published_output_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        let (config, profile, resolved, generated, commit) =
            create_committed_output_fixture(tmp.path());
        let generate_result = GenerateResult {
            committed: commit.committed,
            stage: GenerateStage::Publish,
            branch: commit.branch.clone(),
            tag: Some(commit.tag.clone()),
            output_commit: Some(commit.output_commit.clone()),
            file_count: generated.file_count,
            total_bytes: generated.total_bytes,
            patch_count: 0,
            selftests_enabled: false,
            built_in_selftests: 0,
            selftest_commands: 0,
        };

        write_authoritative_lockfile(
            &project,
            &config,
            &profile,
            &resolved,
            "unmodified-upstream",
            &generate_result,
        )
        .unwrap();

        let lockfile_path = LockfilePath::new_in_project_root(&project).unwrap();
        let lock = crate::lockfile::load_lockfile(&lockfile_path)
            .unwrap()
            .unwrap();
        let published_lock = lock.published.unwrap();
        let output_repo = OutputRepoPath::new(config.output.path.as_str()).unwrap();
        let published_metadata = output_repo::load_committed_published_snapshot_metadata(
            &output_repo,
            &commit.output_commit,
        )
        .unwrap();
        assert_eq!(published_lock.output_commit, commit.output_commit);
        assert_eq!(published_lock.output_branch, published_metadata.branch);
        assert_eq!(published_lock.tag, published_metadata.tag);
        assert_eq!(published_lock.base_ref, published_metadata.base_ref);
        assert_eq!(published_lock.base_commit, published_metadata.base_commit);
        assert_eq!(published_lock.profile, published_metadata.profile);
        assert_eq!(published_lock.mode, published_metadata.mode);
        assert_eq!(published_lock.generated_at, published_metadata.generated_at);
        assert_eq!(lock.resolved_base.commit, resolved.commit);
    }

    #[test]
    fn test_authoritative_lockfile_rejects_mismatched_published_output_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        let original_lockfile = concat!(
            "[resolved_base]\n",
            "upstream = \"linux\"\n",
            "url = \"/tmp/linux.git\"\n",
            "ref = \"old\"\n",
            "commit = \"oldcommit\"\n",
            "resolved_at = \"2026-01-01T00:00:00Z\"\n",
        );
        std::fs::write(project.join("kslim.lock"), original_lockfile).unwrap();
        let (config, profile, resolved, generated, mut commit) =
            create_committed_output_fixture(tmp.path());
        let published_path = Path::new(&config.output.path)
            .join(output_repo::COMMITTED_METADATA_DIR)
            .join(output_repo::PUBLISHED_SNAPSHOT_FILE);
        let published = std::fs::read_to_string(&published_path).unwrap();
        std::fs::write(
            &published_path,
            published.replace("profile = \"default\"", "profile = \"candidate\""),
        )
        .unwrap();
        git_in(Path::new(&config.output.path), &["add", "-A"]);
        git_in(
            Path::new(&config.output.path),
            &["commit", "-m", "corrupt committed metadata"],
        );
        commit.output_commit = git_in(Path::new(&config.output.path), &["rev-parse", "HEAD"]);
        let generate_result = GenerateResult {
            committed: true,
            stage: GenerateStage::Publish,
            branch: commit.branch.clone(),
            tag: Some(commit.tag.clone()),
            output_commit: Some(commit.output_commit.clone()),
            file_count: generated.file_count,
            total_bytes: generated.total_bytes,
            patch_count: 0,
            selftests_enabled: false,
            built_in_selftests: 0,
            selftest_commands: 0,
        };

        let err = write_authoritative_lockfile(
            &project,
            &config,
            &profile,
            &resolved,
            "unmodified-upstream",
            &generate_result,
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("committed published metadata does not match"));
        assert_eq!(
            std::fs::read_to_string(project.join("kslim.lock")).unwrap(),
            original_lockfile
        );
    }

    #[test]
    fn test_verify_generated_output_accepts_valid_tree_without_selftests() {
        let tmp = tempfile::tempdir().unwrap();
        for dir in &[
            "arch", "drivers", "fs", "include", "kernel", "mm", "net", "scripts", ".kslim",
        ] {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        std::fs::write(tmp.path().join("Makefile"), "# test\n").unwrap();
        std::fs::write(tmp.path().join("Kconfig"), "# test\n").unwrap();
        std::fs::write(tmp.path().join(".kslim/base.toml"), "base = true\n").unwrap();
        std::fs::write(
            tmp.path().join(".kslim/generated.toml"),
            "generated_by = \"kslim\"\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join(".kslim/manifest.txt"),
            "hash  1  Makefile\n",
        )
        .unwrap();

        let profile = config::default_profile_config("v1.0");
        let mut failure = FailureReportContext {
            stage: GenerateStage::Metadata,
            ..FailureReportContext::default()
        };
        let result = verify::verify_generated_output(
            tmp.path().to_str().unwrap(),
            &profile,
            false,
            &mut reducer::ReducerStats::default(),
            &mut failure,
        )
        .unwrap();

        assert_eq!(result.tree_path(), tmp.path().to_str().unwrap());
        assert!(result.selftests().is_none());
        assert_eq!(failure.stage, GenerateStage::Metadata);
    }

    #[test]
    fn test_verify_generated_output_rejects_missing_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        for dir in &[
            "arch", "drivers", "fs", "include", "kernel", "mm", "net", "scripts", ".kslim",
        ] {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        std::fs::write(tmp.path().join("Makefile"), "# test\n").unwrap();
        std::fs::write(tmp.path().join("Kconfig"), "# test\n").unwrap();
        std::fs::write(tmp.path().join(".kslim/base.toml"), "base = true\n").unwrap();
        std::fs::write(
            tmp.path().join(".kslim/manifest.txt"),
            "hash  1  Makefile\n",
        )
        .unwrap();

        let mut failure = FailureReportContext {
            stage: GenerateStage::Metadata,
            ..FailureReportContext::default()
        };
        let err = verify::verify_generated_output(
            tmp.path().to_str().unwrap(),
            &config::default_profile_config("v1.0"),
            false,
            &mut reducer::ReducerStats::default(),
            &mut failure,
        )
        .unwrap_err();

        assert!(format!("{:#}", err).contains(".kslim/generated.toml missing"));
    }

    #[test]
    fn test_commit_output_repo_state_commits_snapshot_to_output_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let snapshot = tmp.path().join("snapshot");
        create_minimal_tree(&snapshot);
        std::fs::write(snapshot.join(".gitignore"), ".*\n").unwrap();

        let output = tmp.path().join("output");
        let mut config = config::default_kslim_config("demo", output.to_str().unwrap());
        config.upstream.url = "/tmp/linux.git".to_string();
        let mut profile = config::default_profile_config("v1.0");
        profile.selftests.enabled = false;
        let resolved = test_resolved_base();
        let generated = write_tree_metadata_and_manifest(
            snapshot.to_str().unwrap(),
            &config,
            &profile,
            &resolved,
            "unmodified-upstream",
            &resolved.resolved_at,
            None,
        )
        .unwrap();
        let mut failure = FailureReportContext {
            stage: GenerateStage::Commit,
            ..FailureReportContext::default()
        };
        let mut reducer_stats = reducer::ReducerStats::default();
        let verified = verify::verify_generated_output(
            snapshot.to_str().unwrap(),
            &profile,
            false,
            &mut reducer_stats,
            &mut failure,
        )
        .unwrap();
        let verification = candidate_verification_for_test(
            tmp.path(),
            snapshot.as_path(),
            output.as_path(),
            &profile,
            &reducer_stats,
        );

        let result = commit_output_repo_state(
            &config,
            &profile,
            &GenerateOptions {
                dry_run: false,
                deep_dry_run: false,
                report_only: false,
                keep_temp: false,
                max_fixup_passes: None,
                matrix: None,
                offline: false,
                frozen_plan: None,
                force: false,
                base_ref: None,
                feature: None,
                remove_feature: None,
                preserve_feature: None,
                arch: None,
                primary_arch: None,
                secondary_arch: None,
                safety: None,
                strict: false,
                no_strict: false,
                run_selftests: false,
            },
            &resolved,
            "fingerprint-test",
            &generated,
            &verified,
            &verification,
            &resolved.resolved_at,
            None,
            "unmodified-upstream",
            &output_repo::branch_name(&config, &profile, &resolved),
            &reducer_stats,
            &mut failure,
        )
        .unwrap();

        assert!(result.committed);
        assert_eq!(result.branch, "kslim/v1.0/default");
        assert_eq!(
            git_in(&output, &["branch", "--show-current"]),
            "kslim/v1.0/default"
        );
        assert!(output.join(".git/kslim/managed.toml").exists());
        assert!(output.join(".git/kslim/base.toml").exists());
        assert!(output.join(".git/kslim/report.txt").exists());
        assert!(output.join(".kslim/base.toml").exists());
        assert!(git_in(&output, &["show", "HEAD:.kslim/base.toml"])
            .contains("base_commit = \"deadbeef\""));
        let commit_message = git_in(&output, &["log", "-1", "--pretty=%B"]);
        assert!(commit_message.contains("Profile: default"));
        assert!(commit_message.contains("Base-commit: deadbeef"));
        assert!(commit_message.contains("Plan-fingerprint: fingerprint-test"));
        assert!(commit_message.contains("Reducer-summary: ran=false"));
        assert!(commit_message.contains("Selftest-summary: enabled=false"));
        assert_eq!(failure.stage, GenerateStage::Commit);
    }

    #[test]
    fn test_commit_output_repo_state_is_idempotent_for_unchanged_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let snapshot = tmp.path().join("snapshot");
        create_minimal_tree(&snapshot);

        let output = tmp.path().join("output");
        let mut config = config::default_kslim_config("demo", output.to_str().unwrap());
        config.upstream.url = "/tmp/linux.git".to_string();
        let mut profile = config::default_profile_config("v1.0");
        profile.selftests.enabled = false;
        let resolved = test_resolved_base();
        let generated = write_tree_metadata_and_manifest(
            snapshot.to_str().unwrap(),
            &config,
            &profile,
            &resolved,
            "unmodified-upstream",
            &resolved.resolved_at,
            None,
        )
        .unwrap();

        let opts = GenerateOptions {
            dry_run: false,
            deep_dry_run: false,
            report_only: false,
            keep_temp: false,
            max_fixup_passes: None,
            matrix: None,
            offline: false,
            frozen_plan: None,
            force: false,
            base_ref: None,
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            strict: false,
            no_strict: false,
            run_selftests: false,
        };
        let mut reducer_stats = reducer::ReducerStats::default();

        let mut failure = FailureReportContext {
            stage: GenerateStage::Commit,
            ..FailureReportContext::default()
        };
        let verified = verify::verify_generated_output(
            snapshot.to_str().unwrap(),
            &profile,
            false,
            &mut reducer_stats,
            &mut failure,
        )
        .unwrap();
        let verification = candidate_verification_for_test(
            tmp.path(),
            snapshot.as_path(),
            output.as_path(),
            &profile,
            &reducer_stats,
        );
        let first = commit_output_repo_state(
            &config,
            &profile,
            &opts,
            &resolved,
            "fingerprint-test",
            &generated,
            &verified,
            &verification,
            &resolved.resolved_at,
            None,
            "unmodified-upstream",
            &output_repo::branch_name(&config, &profile, &resolved),
            &reducer_stats,
            &mut failure,
        )
        .unwrap();
        let second = commit_output_repo_state(
            &config,
            &profile,
            &opts,
            &resolved,
            "fingerprint-test",
            &generated,
            &verified,
            &verification,
            &resolved.resolved_at,
            None,
            "unmodified-upstream",
            &output_repo::branch_name(&config, &profile, &resolved),
            &reducer_stats,
            &mut failure,
        )
        .unwrap();

        assert!(first.committed);
        assert!(!second.committed);
        assert_eq!(git_in(&output, &["rev-list", "--count", "HEAD"]), "2");
    }
}
