use anyhow::Result;

use crate::config::ProfileConfig;
use crate::cpp::CppReportCounts;
use crate::edit_reason::sort_edit_records;
use crate::includes::IncludeReportCounts;
use crate::paths::KernelSourceRoot;
use crate::prune::DeclaredPathPruneResult;
use crate::removal_manifest::RemovalManifest;
use crate::tree_index::{TreeIndex, TreeIndexMutatingPass, TreeIndexRebuildDomain};

use super::actions::{audit_mutating_pass_edits, validate_reducer_edit_provenance};
use super::{ReducerResult, ReducerStage, ReducerStats};

#[allow(dead_code)]
pub(crate) const FIXED_REDUCER_PIPELINE: &[&str] = &[
    ReducerStage::BuildManifest.description(),
    ReducerStage::BuildInitialIndex.description(),
    ReducerStage::PruneDeclaredPaths.description(),
    ReducerStage::RebuildFullIndex.description(),
    ReducerStage::RewriteKconfig.description(),
    ReducerStage::RebuildKconfigIndex.description(),
    ReducerStage::RewriteKbuild.description(),
    ReducerStage::RebuildKbuildIndex.description(),
    ReducerStage::FoldPreprocessor.description(),
    ReducerStage::RebuildCHeaderIndex.description(),
    ReducerStage::RewriteIncludes.description(),
    ReducerStage::RunSelftests.description(),
    ReducerStage::ClassifyDiagnostics.description(),
    ReducerStage::ApplyFixups.description(),
    ReducerStage::ReindexAndRepeat.description(),
];

fn log_reducer_stage(stage: ReducerStage) {
    log::info!("reducer: stage={}", stage.as_str());
}

#[allow(dead_code)]
pub fn run(tree_path: &str, profile: &ProfileConfig) -> Result<ReducerStats> {
    let root = KernelSourceRoot::new(tree_path)?;
    Ok(run_reducer_for_profile(&root, profile)?.stats)
}

pub fn run_reducer_for_profile(
    root: &KernelSourceRoot,
    profile: &ProfileConfig,
) -> Result<ReducerResult> {
    let Some(removal_input) = profile.effective_removal_input() else {
        return Ok(ReducerResult::default());
    };
    let preservation_input = profile.effective_preservation_input();
    let abi_policy = profile.effective_abi_policy();

    let result = run_reducer_with_policies_and_preservation(
        root,
        &removal_input,
        preservation_input.as_ref(),
        &profile.reducer,
        &abi_policy,
        &profile.arch,
    )?;
    super::ensure_supported_fallout(&result.stats, &profile.reducer)?;
    Ok(result)
}

#[allow(dead_code)]
pub fn run_reducer(
    root: &KernelSourceRoot,
    slim_config: &crate::config::SlimConfig,
    reducer_config: &crate::config::ReducerConfig,
) -> Result<ReducerResult> {
    run_reducer_with_abi_policy(
        root,
        slim_config,
        reducer_config,
        &crate::config::AbiPolicyConfig::default(),
    )
}

pub fn run_reducer_with_abi_policy(
    root: &KernelSourceRoot,
    slim_config: &crate::config::SlimConfig,
    reducer_config: &crate::config::ReducerConfig,
    abi_policy: &crate::config::AbiPolicyConfig,
) -> Result<ReducerResult> {
    run_reducer_with_abi_policy_and_preservation(
        root,
        slim_config,
        None,
        reducer_config,
        abi_policy,
    )
}

pub fn run_reducer_with_abi_policy_and_preservation(
    root: &KernelSourceRoot,
    slim_config: &crate::config::SlimConfig,
    preservation: Option<&crate::config::FeaturePreservationInput>,
    reducer_config: &crate::config::ReducerConfig,
    abi_policy: &crate::config::AbiPolicyConfig,
) -> Result<ReducerResult> {
    run_reducer_with_policies_and_preservation(
        root,
        slim_config,
        preservation,
        reducer_config,
        abi_policy,
        &crate::config::ArchPolicyConfig::default(),
    )
}

pub fn run_reducer_with_policies_and_preservation(
    root: &KernelSourceRoot,
    slim_config: &crate::config::SlimConfig,
    preservation: Option<&crate::config::FeaturePreservationInput>,
    reducer_config: &crate::config::ReducerConfig,
    abi_policy: &crate::config::AbiPolicyConfig,
    arch_policy: &crate::config::ArchPolicyConfig,
) -> Result<ReducerResult> {
    log_reducer_stage(ReducerStage::BuildManifest);
    let manifest =
        build_removal_manifest(root, slim_config, preservation, abi_policy, arch_policy)?;
    if manifest.is_noop() {
        return Ok(ReducerResult::default());
    }
    log_reducer_stage(ReducerStage::BuildInitialIndex);
    let initial_index = build_initial_tree_index(root, &manifest)?;
    let removal_policy = crate::prune::RemovalFailurePolicy::from_reducer_config(reducer_config);
    log_reducer_stage(ReducerStage::PruneDeclaredPaths);
    let declared_prune = crate::prune::prune_declared_paths_from_manifest_with_policy(
        root.as_path(),
        &manifest,
        removal_policy,
    )?;
    finish_reducer_after_declared_prune(
        root,
        manifest,
        Some(initial_index),
        declared_prune,
        reducer_config,
    )
}

pub(crate) fn run_reducer_after_declared_prune(
    root: &KernelSourceRoot,
    manifest: RemovalManifest,
    declared_prune: DeclaredPathPruneResult,
    reducer_config: &crate::config::ReducerConfig,
) -> Result<ReducerResult> {
    if manifest.is_noop() {
        return Ok(ReducerResult::default());
    }
    finish_reducer_after_declared_prune(root, manifest, None, declared_prune, reducer_config)
}

fn finish_reducer_after_declared_prune(
    root: &KernelSourceRoot,
    manifest: RemovalManifest,
    initial_index: Option<TreeIndex>,
    declared_prune: DeclaredPathPruneResult,
    reducer_config: &crate::config::ReducerConfig,
) -> Result<ReducerResult> {
    audit_declared_prune_edits(&declared_prune, reducer_config)?;
    log_reducer_stage(ReducerStage::RebuildFullIndex);
    let post_prune_index = rebuild_tree_index_after_prune(root, &manifest, &declared_prune)?;
    log_reducer_stage(ReducerStage::RewriteKconfig);
    let kconfig_stage = crate::prune::rewrite_kconfig_stage(root.as_path(), &manifest)?;
    audit_kconfig_stage_edits(&kconfig_stage, reducer_config)?;
    log_reducer_stage(ReducerStage::RebuildKconfigIndex);
    let post_kconfig_index = rebuild_kconfig_index_after_rewrite(root, &manifest, &kconfig_stage)?;
    log_reducer_stage(ReducerStage::RewriteKbuild);
    let stats = crate::prune::continue_prune_after_kconfig(
        root.as_path(),
        &manifest,
        &declared_prune,
        kconfig_stage,
    )?;
    audit_mutating_pass_edits(
        "prune.rewrite_makefiles",
        stats.makefile_refs_removed,
        &stats.edits,
        reducer_config,
    )?;
    log_reducer_stage(ReducerStage::RebuildKbuildIndex);
    let post_kbuild_index = rebuild_kbuild_index_after_rewrite(root, &manifest, &stats)?;
    log_reducer_stage(ReducerStage::FoldPreprocessor);
    let cpp_report = crate::cpp::fold_removed_config_branches_report(
        root.as_path(),
        &stats.removal.removed_config_symbols,
    )?;
    if reducer_config.report_unsupported_expressions
        && (!stats.unsupported_kconfig_expressions.is_empty()
            || !cpp_report.unsupported_expressions.is_empty())
    {
        let mut edits = stats.edits;
        sort_edit_records(&mut edits);
        let stats = ReducerStats {
            ran: true,
            files_removed: stats.files_removed,
            dirs_removed: stats.dirs_removed,
            configs_disabled: stats.configs_disabled,
            defaults_overridden: stats.defaults_overridden,
            kconfig_refs_removed: stats.kconfig_refs_removed,
            makefile_refs_removed: stats.makefile_refs_removed,
            kconfig_report: stats.kconfig_report,
            kconfig_solver_report: stats.kconfig_solver_report,
            cpp_report: CppReportCounts::default(),
            include_report: IncludeReportCounts::default(),
            unsupported_kconfig_expressions: stats.unsupported_kconfig_expressions,
            unsupported_cpp_expressions: cpp_report.unsupported_expressions,
            skipped_cpp_nested_edge_cases: cpp_report.skipped_nested_edge_cases,
            skipped_makefile_lines: stats.skipped_makefile_lines,
            removal: stats.removal,
            edits,
            applied_fixups: Vec::new(),
            skipped_fixups: Vec::new(),
            classified_diagnostics: Vec::new(),
            raw_diagnostic_excerpts: Vec::new(),
            manual_include_sites: Vec::new(),
        };
        validate_reducer_edit_provenance(&stats, reducer_config)?;
        let mut result = ReducerResult::from_pipeline_artifacts(
            Some(manifest),
            initial_index,
            Some(declared_prune),
            Some(post_prune_index),
            Some(post_kconfig_index),
            Some(post_kbuild_index),
            None,
            None,
            stats,
        );
        result.apply_unsupported_syntax_policy(reducer_config.report_unsupported_expressions);
        return Ok(result);
    }

    audit_mutating_pass_edits(
        "cpp.fold_removed_config_branches",
        cpp_report
            .counts
            .branches_folded
            .max(cpp_report.counts.files_touched),
        &cpp_report.edits,
        reducer_config,
    )?;
    crate::cpp::apply_fold_report(root.as_path(), &cpp_report)?;
    log_reducer_stage(ReducerStage::RebuildCHeaderIndex);
    let post_cpp_index = rebuild_c_header_index_after_cpp(root, &manifest, &cpp_report)?;
    let manifest_removed_paths = manifest.removed_paths_vec();
    let manifest_removed_headers = manifest.removed_header_paths_vec();
    let header_removal_proofs =
        crate::includes::HeaderRemovalProofs::from_manifest_paths_with_abi_policy(
            &manifest_removed_paths,
            &manifest_removed_headers,
            &manifest.abi_policy,
        );
    log_reducer_stage(ReducerStage::RewriteIncludes);
    let include_report =
        crate::includes::rewrite_removed_header_includes_report_with_removed_configs(
            root.as_path(),
            &header_removal_proofs,
            &stats.removal.removed_config_symbols,
        )?;
    audit_mutating_pass_edits(
        "includes.rewrite_removed_headers",
        include_report.counts.removed_include_lines,
        &include_report.edits,
        reducer_config,
    )?;
    crate::includes::apply_include_rewrite_report(root.as_path(), &include_report)?;
    let post_include_index =
        rebuild_c_header_index_after_include(root, &manifest, &include_report)?;
    let mut edits = stats.edits;
    edits.extend(cpp_report.edits);
    edits.extend(include_report.edits);
    sort_edit_records(&mut edits);
    let stats = ReducerStats {
        ran: true,
        files_removed: stats.files_removed,
        dirs_removed: stats.dirs_removed,
        configs_disabled: stats.configs_disabled,
        defaults_overridden: stats.defaults_overridden,
        kconfig_refs_removed: stats.kconfig_refs_removed,
        makefile_refs_removed: stats.makefile_refs_removed,
        kconfig_report: stats.kconfig_report,
        kconfig_solver_report: stats.kconfig_solver_report,
        cpp_report: cpp_report.counts,
        include_report: include_report.counts,
        unsupported_kconfig_expressions: stats.unsupported_kconfig_expressions,
        unsupported_cpp_expressions: cpp_report.unsupported_expressions,
        skipped_cpp_nested_edge_cases: cpp_report.skipped_nested_edge_cases,
        skipped_makefile_lines: stats.skipped_makefile_lines,
        removal: stats.removal,
        edits,
        applied_fixups: Vec::new(),
        skipped_fixups: Vec::new(),
        classified_diagnostics: Vec::new(),
        raw_diagnostic_excerpts: Vec::new(),
        manual_include_sites: include_report.manual_sites,
    };
    validate_reducer_edit_provenance(&stats, reducer_config)?;
    let mut result = ReducerResult::from_pipeline_artifacts(
        Some(manifest),
        initial_index,
        Some(declared_prune),
        Some(post_prune_index),
        Some(post_kconfig_index),
        Some(post_kbuild_index),
        Some(post_cpp_index),
        Some(post_include_index),
        stats,
    );
    result.apply_unsupported_syntax_policy(reducer_config.report_unsupported_expressions);
    Ok(result)
}

fn audit_declared_prune_edits(
    declared: &DeclaredPathPruneResult,
    reducer_config: &crate::config::ReducerConfig,
) -> Result<()> {
    let empty_parent_cleanups = declared.removal.empty_parents_cleaned.len();
    let direct_dirs_removed = declared.dirs_removed.saturating_sub(empty_parent_cleanups);
    audit_mutating_pass_edits(
        "prune.remove_path",
        declared.files_removed + direct_dirs_removed,
        &declared.edits,
        reducer_config,
    )?;
    audit_mutating_pass_edits(
        "prune.cleanup_empty_parents",
        empty_parent_cleanups,
        &declared.edits,
        reducer_config,
    )
}

fn audit_kconfig_stage_edits(
    stage: &crate::prune::KconfigPruneStageResult,
    reducer_config: &crate::config::ReducerConfig,
) -> Result<()> {
    audit_mutating_pass_edits(
        "prune.prune_configs",
        stage.configs_disabled,
        &stage.edits,
        reducer_config,
    )?;
    audit_mutating_pass_edits(
        "prune.rewrite_kconfig_defaults",
        stage.defaults_overridden,
        &stage.edits,
        reducer_config,
    )?;

    let relation_rewrites = stage.kconfig_report.dropped_selects
        + stage.kconfig_report.dropped_implies
        + stage.kconfig_report.simplified_depends
        + stage.kconfig_report.simplified_visible_if
        + stage.kconfig_report.simplified_defaults;
    audit_mutating_pass_edits(
        "kconfig.rewrite_relations",
        relation_rewrites,
        &stage.edits,
        reducer_config,
    )?;
    audit_mutating_pass_edits(
        "prune.rewrite_kconfig_sources",
        stage.kconfig_report.removed_sources,
        &stage.edits,
        reducer_config,
    )?;
    audit_mutating_pass_edits(
        "kconfig.rewrite_empty_menus",
        stage.kconfig_report.removed_empty_menus,
        &stage.edits,
        reducer_config,
    )
}

fn build_removal_manifest(
    root: &KernelSourceRoot,
    slim_config: &crate::config::SlimConfig,
    preservation: Option<&crate::config::FeaturePreservationInput>,
    abi_policy: &crate::config::AbiPolicyConfig,
    arch_policy: &crate::config::ArchPolicyConfig,
) -> Result<RemovalManifest> {
    let mut manifest = RemovalManifest::from_slim_config_for_tree_with_abi_policy_and_preservation(
        root.as_path(),
        slim_config,
        preservation,
        abi_policy,
    )?;
    manifest.arch_policy = arch_policy.clone();
    Ok(manifest)
}

fn build_initial_tree_index(
    root: &KernelSourceRoot,
    manifest: &RemovalManifest,
) -> Result<TreeIndex> {
    TreeIndex::build(root.as_path(), manifest)
}

fn rebuild_tree_index_after_prune(
    root: &KernelSourceRoot,
    manifest: &RemovalManifest,
    _declared_prune: &DeclaredPathPruneResult,
) -> Result<TreeIndex> {
    let mut index = TreeIndex::default();
    index.rebuild_after_mutating_pass(
        root.as_path(),
        manifest,
        TreeIndexRebuildDomain::All,
        &[],
        TreeIndexMutatingPass::DeclaredPrune,
    )?;
    Ok(index)
}

fn rebuild_kconfig_index_after_rewrite(
    root: &KernelSourceRoot,
    _manifest: &RemovalManifest,
    _kconfig_stage: &crate::prune::KconfigPruneStageResult,
) -> Result<TreeIndex> {
    let mut index = TreeIndex::default();
    index.rebuild_after_mutating_pass(
        root.as_path(),
        _manifest,
        TreeIndexRebuildDomain::Kconfig,
        &[],
        TreeIndexMutatingPass::KconfigRewrite,
    )?;
    Ok(index)
}

fn rebuild_kbuild_index_after_rewrite(
    root: &KernelSourceRoot,
    _manifest: &RemovalManifest,
    _stats: &crate::prune::PruneStats,
) -> Result<TreeIndex> {
    let mut index = TreeIndex::default();
    index.rebuild_after_mutating_pass(
        root.as_path(),
        _manifest,
        TreeIndexRebuildDomain::Kbuild,
        &[],
        TreeIndexMutatingPass::KbuildRewrite,
    )?;
    Ok(index)
}

fn rebuild_c_header_index_after_cpp(
    root: &KernelSourceRoot,
    _manifest: &RemovalManifest,
    _cpp_report: &crate::cpp::CppFoldReport,
) -> Result<TreeIndex> {
    let mut index = TreeIndex::default();
    index.rebuild_after_mutating_pass(
        root.as_path(),
        _manifest,
        TreeIndexRebuildDomain::CFamily,
        &[],
        TreeIndexMutatingPass::CppFold,
    )?;
    Ok(index)
}

fn rebuild_c_header_index_after_include(
    root: &KernelSourceRoot,
    _manifest: &RemovalManifest,
    _include_report: &crate::includes::IncludeRewriteReport,
) -> Result<TreeIndex> {
    let mut index = TreeIndex::default();
    index.rebuild_after_mutating_pass(
        root.as_path(),
        _manifest,
        TreeIndexRebuildDomain::CFamily,
        &[],
        TreeIndexMutatingPass::IncludeRewrite,
    )?;
    Ok(index)
}
