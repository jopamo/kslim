use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::config::{
    FeaturePreservationInput, IntegrationsConfig, ProfileConfig, ReducerConfig,
    RtlmqIntegrationConfig, SlimConfig,
};
use crate::feature::FeatureConflictReport;
use crate::removal_manifest::RemovalManifest;
use crate::{integrations, patches, prune, reducer, upstream};

use super::super::plan::GeneratePlan;
use super::super::state::{CandidateTreeState, PatchSourcePlan, PrunePlan, ReducerPlan};
use super::super::GenerateStage;
use super::errors::{
    record_candidate_failure_attempt, record_candidate_stage, CandidateBuildStageFailure,
};
use super::metadata::{record_partial_candidate_reducer_reports, write_candidate_metadata};
use super::model::{
    ensure_candidate_mutation_target, project_root_for_requested_config, CandidateMutationTarget,
    MaterializedTree, WorkspacePaths,
};

struct CandidatePrunePreparation {
    manifest: RemovalManifest,
    declared_prune: Option<prune::DeclaredPathPruneResult>,
}

fn log_candidate_generate_stage(stage: GenerateStage, action: &str) {
    log::info!("generate: stage={} action={}", stage.as_str(), action);
}

#[allow(dead_code)]
pub(super) fn build_candidate_tree(
    plan: &GeneratePlan,
    workspace: &WorkspacePaths,
) -> Result<CandidateTreeState> {
    match build_candidate_tree_inner(plan, workspace) {
        Ok(state) => Ok(state),
        Err(err) => Err(record_candidate_failure_attempt(plan, err)),
    }
}

fn build_candidate_tree_inner(
    plan: &GeneratePlan,
    workspace: &WorkspacePaths,
) -> Result<CandidateTreeState> {
    plan.resolved
        .reject_unresolved_feature_conflicts_in_strict_mode()?;
    let tree_path = workspace.candidate_tree();
    let mutation_target = record_candidate_stage(GenerateStage::Materialize, || {
        let mutation_target = ensure_candidate_mutation_target(plan, tree_path)?;
        ensure_candidate_workspace_is_empty(mutation_target.as_path())?;
        let tree_path_str = mutation_target.path_str()?;

        upstream::archive_tree(
            &plan.resolved.base.url,
            &plan.resolved.base.commit,
            tree_path_str,
        )
        .with_context(|| {
            format!(
                "failed to materialize upstream commit {} into candidate workspace {}",
                plan.resolved.base.commit,
                tree_path.display()
            )
        })?;
        upstream::validate_tree(tree_path_str).with_context(|| {
            format!(
                "failed to validate materialized candidate tree at {}",
                tree_path.display()
            )
        })?;
        Ok(mutation_target)
    })?;
    let integrated = record_candidate_stage(GenerateStage::Integrate, || {
        apply_patch_plan(plan, &mutation_target)?;
        apply_integration_plan(plan, &mutation_target)
    })?;
    let prune_preparation = record_candidate_stage(GenerateStage::Prune, || {
        prepare_candidate_prune(plan, &mutation_target)
    })?;
    let pruned = prune_preparation.declared_prune.is_some();
    let reducer_stats = run_candidate_reducer(
        plan,
        &mutation_target,
        &prune_preparation.manifest,
        prune_preparation.declared_prune,
    )?;
    let reduced = reducer_stats.is_some();

    record_candidate_stage(GenerateStage::Metadata, || {
        let mut state =
            CandidateTreeState::from_materialized_tree(mutation_target.as_path().to_path_buf())?;
        if integrated {
            state.mark_integrated()?;
        }
        if pruned {
            state.mark_pruned()?;
        }
        if reduced {
            state.mark_reduced()?;
        }
        let reducer_config = reducer_config_from_plan(&plan.resolved.reducer_plan);
        write_candidate_metadata(
            plan,
            &state,
            reducer_stats.as_ref(),
            &reducer_config,
            &prune_preparation.manifest,
        )?;
        Ok(state)
    })
}

pub(in crate::generate) fn materialize_resolved_candidate_tree(
    plan: &GeneratePlan,
    keep_temp: bool,
) -> Result<MaterializedTree> {
    let mut temp_dir = tempfile::Builder::new().prefix("kslim-gen-").tempdir()?;
    if keep_temp {
        temp_dir.disable_cleanup(true);
        println!("  kept temp:            {}", temp_dir.path().display());
    }
    let mutation_target = ensure_candidate_mutation_target(plan, temp_dir.path())?;
    let path = mutation_target.path_str()?.to_string();
    upstream::archive_tree(&plan.resolved.base.url, &plan.resolved.base.commit, &path)?;
    Ok(MaterializedTree {
        temp_dir,
        path,
        mutation_target,
    })
}

pub(in crate::generate) struct CandidateMaterialization {
    pub(in crate::generate) temp_dir: tempfile::TempDir,
    pub(in crate::generate) path: String,
    pub(in crate::generate) patch_infos: Option<Vec<patches::PatchInfo>>,
    pub(in crate::generate) mode: String,
    pub(in crate::generate) reducer_stats: reducer::ReducerStats,
}

pub(in crate::generate) enum CandidateMaterializationEvent {
    Stage(GenerateStage),
    Materialized(PathBuf),
}

pub(in crate::generate) fn materialize_integrate_and_reduce_candidate_tree(
    plan: &GeneratePlan,
    profile: &ProfileConfig,
    planned_patch_infos: Option<&[patches::PatchInfo]>,
    keep_temp: bool,
    mut record_event: impl FnMut(CandidateMaterializationEvent) -> Result<()>,
) -> Result<CandidateMaterialization> {
    record_event(CandidateMaterializationEvent::Stage(
        GenerateStage::Materialize,
    ))?;
    log_candidate_generate_stage(GenerateStage::Materialize, "materialize");
    let materialized = materialize_resolved_candidate_tree(plan, keep_temp)?;
    let MaterializedTree {
        temp_dir,
        path,
        mutation_target,
    } = materialized;
    record_event(CandidateMaterializationEvent::Materialized(PathBuf::from(
        &path,
    )))?;

    record_event(CandidateMaterializationEvent::Stage(GenerateStage::Integrate))?;
    let applied_patch_infos = apply_patch_sources(profile, &mutation_target)?;
    ensure_patch_application_matches_plan(planned_patch_infos, applied_patch_infos.as_deref())?;

    record_event(CandidateMaterializationEvent::Stage(GenerateStage::Integrate))?;
    apply_integrations(profile, &mutation_target)?;

    record_event(CandidateMaterializationEvent::Stage(GenerateStage::Reduce))?;
    let reducer_stats = reduce_tree(&mutation_target, profile)?;
    if reducer_stats.ran {
        log_candidate_generate_stage(GenerateStage::Reduce, "reducer_summary");
        log::info!(
            "reducer: removed {} files, {} directories, cleaned {} empty parent directories, skipped {} missing paths, removed {} config blocks, rewrote {} config defaults, rewrote {} Kconfig refs, rewrote {} Makefile refs, recorded {} edits",
            reducer_stats.files_removed,
            reducer_stats.dirs_removed,
            reducer_stats.removal.empty_parents_cleaned.len(),
            reducer_stats.removal.missing_paths.len(),
            reducer_stats.configs_disabled,
            reducer_stats.defaults_overridden,
            reducer_stats.kconfig_refs_removed,
            reducer_stats.makefile_refs_removed,
            reducer_stats.edits.len()
        );
    }

    Ok(CandidateMaterialization {
        temp_dir,
        path,
        patch_infos: planned_patch_infos.map(<[patches::PatchInfo]>::to_vec),
        mode: plan.resolved.output_plan.mode.clone(),
        reducer_stats,
    })
}

pub(in crate::generate) fn ensure_patch_application_matches_plan(
    planned: Option<&[patches::PatchInfo]>,
    applied: Option<&[patches::PatchInfo]>,
) -> Result<()> {
    if planned.unwrap_or_default() != applied.unwrap_or_default() {
        anyhow::bail!(
            "applied patch sources no longer match the resolved generate plan; re-run generate to resolve a fresh plan"
        );
    }
    Ok(())
}

pub(in crate::generate) fn apply_patch_sources(
    profile: &ProfileConfig,
    target: &CandidateMutationTarget,
) -> Result<Option<Vec<patches::PatchInfo>>> {
    let Some(patches_cfg) = &profile.patches else {
        return Ok(None);
    };

    log_candidate_generate_stage(GenerateStage::Integrate, "apply_patch_sources");
    let infos = patches::apply_all(target.path_str()?, patches_cfg)
        .context("failed to apply patch worktree source during generate")?;
    log::info!(
        "patches: applied {} commit(s) from {} source(s)",
        patches::total_patch_count(&infos),
        infos.len()
    );
    for info in &infos {
        log::info!(
            "patches: source '{}' branch '{}' at {} -> {} commit(s)",
            info.source,
            info.branch,
            info.worktree_path,
            info.patch_count
        );
    }
    Ok(Some(infos))
}

#[allow(dead_code)]
pub(super) fn apply_patch_plan(
    plan: &GeneratePlan,
    target: &CandidateMutationTarget,
) -> Result<Option<Vec<patches::PatchInfo>>> {
    if plan.resolved.patch_plan.sources.is_empty() {
        return Ok(None);
    }
    let tree_path = target.path_str()?;
    let infos = plan
        .resolved
        .patch_plan
        .sources
        .iter()
        .map(patch_info_from_plan)
        .collect::<Vec<_>>();

    log_candidate_generate_stage(GenerateStage::Integrate, "apply_patch_plan");
    let applied = patches::apply_resolved_all(tree_path, &infos)
        .context("failed to apply resolved patch stack during candidate build")?;
    log::info!(
        "patches: applied {} commit(s) from {} resolved source(s)",
        patches::total_patch_count(&applied),
        applied.len()
    );
    for info in &applied {
        log::info!(
            "patches: source '{}' branch '{}' at {} -> {} commit(s)",
            info.source,
            info.branch,
            info.worktree_path,
            info.patch_count
        );
    }
    Ok(Some(applied))
}

fn patch_info_from_plan(source: &PatchSourcePlan) -> patches::PatchInfo {
    patches::PatchInfo {
        source: source.source.clone(),
        worktree_path: source.worktree_path.clone(),
        branch: source.branch.clone(),
        head_commit: source.head_commit.clone(),
        merge_base: source.merge_base.clone(),
        base_remote: source.base_remote.clone(),
        base_ref: source.base_ref.clone(),
        patch_count: source.patch_count,
    }
}

#[allow(dead_code)]
pub(super) fn apply_integration_plan(
    plan: &GeneratePlan,
    target: &CandidateMutationTarget,
) -> Result<bool> {
    if plan.resolved.integration_plan.entries.is_empty() {
        return Ok(false);
    }

    log_candidate_generate_stage(GenerateStage::Integrate, "apply_integration_plan");
    let project_root = project_root_for_requested_config(plan.requested.config_path.as_path());
    for entry in &plan.resolved.integration_plan.entries {
        match entry.kind.as_str() {
            "rtlmq" => {
                let rtlmq = plan
                    .resolved
                    .integration_plan
                    .rtlmq
                    .as_ref()
                    .ok_or_else(|| {
                        anyhow::anyhow!("resolved rtlmq integration entry is missing rtlmq plan")
                    })?;
                if entry.stable_id != rtlmq.stable_id {
                    anyhow::bail!("resolved rtlmq integration entry id does not match rtlmq plan");
                }
                let integrations = IntegrationsConfig {
                    rtlmq: Some(RtlmqIntegrationConfig {
                        source: rtlmq.source.clone(),
                        tests_source: rtlmq.tests_source.clone(),
                    }),
                };
                integrations::apply(&project_root, target.as_path(), &integrations).with_context(
                    || format!("failed to apply resolved integration {}", entry.stable_id),
                )?;
            }
            other => {
                anyhow::bail!("unsupported resolved integration kind: {}", other);
            }
        }
    }
    Ok(true)
}

fn prepare_candidate_prune(
    plan: &GeneratePlan,
    target: &CandidateMutationTarget,
) -> Result<CandidatePrunePreparation> {
    let tree_path = target.as_path();
    let manifest = removal_manifest_from_prune_plan_for_tree(tree_path, &plan.resolved.prune_plan)?;
    if plan.resolved.prune_plan.remove_paths.is_empty() {
        return Ok(CandidatePrunePreparation {
            manifest,
            declared_prune: None,
        });
    }

    log_candidate_generate_stage(GenerateStage::Prune, "prepare_candidate_prune");
    let reducer_config = reducer_config_from_plan(&plan.resolved.reducer_plan);
    let policy = prune::RemovalFailurePolicy::from_reducer_config(&reducer_config);
    let result =
        prune::prune_declared_paths_from_manifest_with_policy(tree_path, &manifest, policy)
            .context("failed to prune resolved candidate paths")?;
    validate_candidate_edit_records(&result.edits, &reducer_config)
        .context("invalid canonical proof source in candidate prune output")?;
    let empty_parent_cleanups = result.removal.empty_parents_cleaned.len();
    let direct_dirs_removed = result.dirs_removed.saturating_sub(empty_parent_cleanups);
    crate::edit_reason::ensure_edit_records_for_mutation(
        "prune.remove_path",
        result.files_removed + direct_dirs_removed,
        &result.edits,
    )
    .context("candidate path prune removed paths without edit records")?;
    crate::edit_reason::ensure_edit_records_for_mutation(
        "prune.cleanup_empty_parents",
        empty_parent_cleanups,
        &result.edits,
    )
    .context("candidate path prune cleaned empty parents without edit records")?;
    log::info!(
        "prune: removed {} file(s), {} dir(s), cleaned {} empty parent dir(s)",
        result.files_removed,
        result.dirs_removed,
        result.removal.empty_parents_cleaned.len()
    );
    Ok(CandidatePrunePreparation {
        manifest,
        declared_prune: Some(result),
    })
}

fn removal_manifest_from_prune_plan_for_tree(
    root: &Path,
    plan: &PrunePlan,
) -> Result<RemovalManifest> {
    let preservation = preservation_input_from_prune_plan(plan);
    let mut manifest = RemovalManifest::from_slim_config_for_tree_with_abi_policy_and_preservation(
        root,
        &slim_config_from_prune_plan(plan),
        preservation.as_ref(),
        &plan.abi_policy,
    )?;
    manifest.arch_policy = plan.arch_policy.clone();
    Ok(manifest)
}

fn slim_config_from_prune_plan(plan: &PrunePlan) -> SlimConfig {
    SlimConfig {
        remove_paths: plan
            .remove_paths
            .iter()
            .map(|path| path.to_config_string())
            .collect(),
        remove_configs: plan
            .remove_configs
            .iter()
            .map(|symbol| symbol.as_str().to_string())
            .collect(),
        set_defaults: plan
            .set_defaults
            .iter()
            .map(|(symbol, value)| (symbol.as_str().to_string(), value.clone()))
            .collect(),
        unsafe_allow_root_path_removal: plan.unsafe_allow_root_path_removal,
    }
}

fn preservation_input_from_prune_plan(plan: &PrunePlan) -> Option<FeaturePreservationInput> {
    let input = FeaturePreservationInput {
        preserve_paths: plan
            .preserve_paths
            .iter()
            .map(|path| path.to_config_string())
            .collect(),
        preserve_configs: plan
            .preserve_configs
            .iter()
            .map(|symbol| symbol.as_str().to_string())
            .collect(),
    };
    (!input.is_noop()).then_some(input)
}

fn reducer_config_from_plan(plan: &ReducerPlan) -> ReducerConfig {
    ReducerConfig {
        max_fixup_passes: plan.max_fixup_passes,
        report_unsupported_expressions: plan.report_unsupported_expressions,
        fail_on_unknown_diagnostics: plan.fail_on_unknown_diagnostics,
        reject_unproven_fixups: plan.reject_unproven_fixups,
        reject_unreasoned_edits: plan.reject_unreasoned_edits,
        reject_speculative_fallout_edits: plan.reject_speculative_fallout_edits,
        fail_on_missing_prune_paths: plan.fail_on_missing_prune_paths,
        ignore_unsupported_special_removals: plan.ignore_unsupported_special_removals,
    }
}

fn validate_candidate_edit_records(
    edits: &[crate::edit_reason::EditRecord],
    reducer_config: &ReducerConfig,
) -> Result<()> {
    crate::edit_reason::validate_edit_records_with_policy(
        edits,
        crate::edit_reason::EditValidationPolicy {
            reject_unreasoned_edits: reducer_config.reject_unreasoned_edits,
            reject_speculative_fallout_edits: reducer_config.reject_speculative_fallout_edits,
        },
    )
}

fn run_candidate_reducer(
    plan: &GeneratePlan,
    target: &CandidateMutationTarget,
    manifest: &RemovalManifest,
    declared_prune: Option<prune::DeclaredPathPruneResult>,
) -> Result<Option<reducer::ReducerStats>> {
    if manifest.is_noop() {
        return Ok(None);
    }

    let kernel_root = target.kernel_source_root()?;
    log_candidate_generate_stage(GenerateStage::Reduce, "run_candidate_reducer");
    let reducer_config = reducer_config_from_plan(&plan.resolved.reducer_plan);
    let result = match declared_prune {
        Some(declared_prune) => reducer::run_reducer_after_declared_prune(
            &kernel_root,
            manifest.clone(),
            declared_prune,
            &reducer_config,
        ),
        None => reducer::run_reducer_from_manifest(
            &kernel_root,
            manifest.clone(),
            &reducer_config,
        ),
    }
    .context("failed to run resolved candidate reducer")
    .with_context(|| CandidateBuildStageFailure::new(GenerateStage::Reduce))?;
    if let Err(err) = reducer::ensure_supported_fallout(&result.stats, &reducer_config) {
        let partial_reports = record_partial_candidate_reducer_reports(
            plan,
            &result.stats,
            &reducer_config,
            &manifest,
        )
        .with_context(|| CandidateBuildStageFailure::new(GenerateStage::Reduce))?;
        return Err(
            err.context(CandidateBuildStageFailure::with_partial_reports(
                GenerateStage::Reduce,
                partial_reports,
            )),
        );
    }
    Ok(Some(result.stats))
}

pub(in crate::generate) fn apply_integrations(
    profile: &ProfileConfig,
    target: &CandidateMutationTarget,
) -> Result<()> {
    if profile.integrations.rtlmq.is_none() {
        return Ok(());
    }

    log_candidate_generate_stage(GenerateStage::Integrate, "apply_integrations");
    let project_root = crate::fsutil::find_kslim_root()?;
    integrations::apply(
        project_root.as_std_path(),
        target.as_path(),
        &profile.integrations,
    )?;
    Ok(())
}

pub(in crate::generate) fn reduce_tree(
    target: &CandidateMutationTarget,
    profile: &ProfileConfig,
) -> Result<reducer::ReducerStats> {
    reject_profile_feature_conflicts_in_strict_mode(profile)?;
    let Some(removal_input) = profile.effective_removal_input() else {
        return Ok(reducer::ReducerStats::default());
    };
    let preservation_input = profile.effective_preservation_input();
    let abi_policy = profile.effective_abi_policy();

    let kernel_root = target.kernel_source_root()?;
    let result = reducer::run_reducer_with_policies_and_preservation(
        &kernel_root,
        &removal_input,
        preservation_input.as_ref(),
        &profile.reducer,
        &abi_policy,
        &profile.arch,
    )?;
    Ok(result.stats)
}

fn reject_profile_feature_conflicts_in_strict_mode(profile: &ProfileConfig) -> Result<()> {
    FeatureConflictReport::from_profile(profile)?
        .reject_blocking_conflicts_in_strict_mode(profile.reducer.strict_mode())
}

fn ensure_candidate_workspace_is_empty(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        anyhow::bail!("candidate workspace path is empty");
    }
    if !path.exists() {
        return Ok(());
    }
    if !path.is_dir() {
        anyhow::bail!(
            "candidate workspace path is not a directory: {}",
            path.display()
        );
    }

    let mut entries = std::fs::read_dir(path)
        .with_context(|| format!("failed to inspect candidate workspace {}", path.display()))?;
    if let Some(entry) = entries.next() {
        entry.with_context(|| {
            format!(
                "failed to inspect candidate workspace entry under {}",
                path.display()
            )
        })?;
        anyhow::bail!("candidate workspace is not empty: {}", path.display());
    }
    Ok(())
}
