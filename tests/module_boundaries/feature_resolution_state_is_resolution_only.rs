use super::common::*;

#[test]
fn feature_resolution_state_is_resolution_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let state = state_source(root);
    let plan = plan_source(root);

    let resolution_start = state
        .find("pub(crate) enum FeatureResolutionSource")
        .expect("generate/state.rs should define FeatureResolutionSource");
    let prune_start = state[resolution_start..]
        .find("pub(crate) struct PrunePlan")
        .map(|offset| resolution_start + offset)
        .expect("generate/state.rs should define FeatureResolutionState before PrunePlan");
    let resolution_section = &state[resolution_start..prune_start];

    for required in [
        "pub(crate) enum FeatureResolutionSource",
        "NoRemoval",
        "DirectSlim",
        "NamedFeatureRemove",
        "CombinedSlimAndNamedFeature",
        "source: FeatureResolutionSource",
        "remove_paths: Vec<RelativeKernelPath>",
        "remove_configs: Vec<KconfigSymbol>",
        "preserve_paths: Vec<RelativeKernelPath>",
        "preserve_configs: Vec<KconfigSymbol>",
        "set_defaults: BTreeMap<KconfigSymbol, String>",
        "abi_policy: AbiPolicyConfig",
        "feature_safety_levels: BTreeMap<String, FeatureSafetyLevel>",
        "feature_arch_scopes: BTreeMap<String, Vec<ArchName>>",
        "feature_test_matrices: BTreeMap<String, FeatureTestMatrixConfig>",
        "feature_report_modes: BTreeMap<String, FeatureReportModeConfig>",
        "unsafe_allow_root_path_removal: bool",
        "pub(crate) fn new(",
        "feature resolution without removal input cannot contain removal facts",
        "fn from_profile(profile: &ProfileConfig)",
        "profile.removal_input()",
        "profile.effective_removal_input()",
        "profile.effective_preservation_input()",
        "profile.effective_abi_policy()",
        "profile.effective_feature_safety_levels()",
        ".effective_feature_arch_scopes()",
        "profile.effective_feature_test_matrices()",
        "profile.effective_feature_report_modes()",
        ".features\n            .remove\n            .values()",
        "intent.declares_removal_input()",
        "ArchName::new",
        "RemovalManifest::from_slim_config_with_abi_policy_and_preservation",
        "RelativeKernelPath::new_for_explicit_unsafe_root_removal(path.clone())",
        "RelativeKernelPath::new(path.clone())",
        "KconfigSymbol::new",
        "pub(crate) fn source(&self)",
        "pub(crate) fn remove_paths(&self)",
        "pub(crate) fn remove_configs(&self)",
        "pub(crate) fn preserve_paths(&self)",
        "pub(crate) fn preserve_configs(&self)",
        "pub(crate) fn set_defaults(&self)",
        "pub(crate) fn abi_policy(&self)",
        "pub(crate) fn feature_safety_levels(&self)",
        "pub(crate) fn feature_arch_scopes(&self)",
        "pub(crate) fn feature_test_matrices(&self)",
        "pub(crate) fn feature_report_modes(&self)",
        "pub(crate) fn unsafe_allow_root_path_removal(&self)",
        "pub(crate) fn is_noop(&self)",
    ] {
        assert!(
            resolution_section.contains(required),
            "FeatureResolutionState should capture only resolved feature/removal intent; missing {required}"
        );
    }

    for forbidden in [
        "pub(crate) source",
        "pub(crate) remove_paths",
        "pub(crate) remove_configs",
        "pub(crate) preserve_paths",
        "pub(crate) preserve_configs",
        "pub(crate) set_defaults",
        "pub(crate) abi_policy",
        "pub(crate) feature_safety_levels",
        "pub(crate) feature_arch_scopes",
        "pub(crate) feature_test_matrices",
        "pub(crate) feature_report_modes",
        "pub(crate) unsafe_allow_root_path_removal",
        "RequestedGenerateState",
        "ResolvedCandidateState",
        "CandidateTreeState",
        "CandidateTreePath",
        "CandidateMetadataDir",
        "OutputRepoPath",
        "PublishedMetadataDir",
        "PublishedSnapshotState",
        "GenerateAttemptFailure",
        "AttemptMetadataDir",
        "CommittedOutputSnapshot",
        "SuccessfulCommitResult",
        "LockfilePath",
        "commit_if_changed",
        "write_authoritative_lockfile",
        "write_verified_published_snapshot_metadata",
        "std::fs::write",
    ] {
        assert!(
            !resolution_section.contains(forbidden),
            "FeatureResolutionState must not expose mutable fields or contain requested, candidate, published, failure, commit, lockfile, or mutation state; found {forbidden}"
        );
    }

    assert!(
        state.contains("feature_resolution: FeatureResolutionState")
            && state.contains(
                "let feature_resolution = FeatureResolutionState::from_profile(profile)?"
            )
            && state.contains("PrunePlan::from_feature_resolution(&feature_resolution)"),
        "ResolvedCandidateState should carry feature resolution and derive PrunePlan from it"
    );
    assert!(
        plan.contains("resolved.feature_resolution.source")
            && plan.contains("resolved.feature_resolution.remove_paths")
            && plan.contains("resolved.feature_resolution.preserve_paths")
            && plan.contains("resolved.feature_resolution.feature_safety_levels")
            && plan.contains("resolved.feature_resolution.feature_arch_scopes")
            && plan.contains("resolved.feature_resolution.feature_test_matrices")
            && plan.contains("resolved.feature_resolution.feature_report_modes")
            && plan.contains("resolved.feature_resolution.unsafe_allow_root_path_removal"),
        "generate plan fingerprint should include resolved feature-resolution state"
    );
}
