use super::common::*;

#[test]
fn resolved_candidate_state_is_plan_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let state = state_source(root);
    let fingerprint = production_source(&root.join("src/state/fingerprint.rs"));
    let plan = plan_source(root);
    let build_matrix_fingerprint =
        production_source(&root.join("src/plan/build_matrix_fingerprint.rs"));

    let resolved_section = state
        .split("pub(crate) struct ResolvedCandidateState")
        .nth(1)
        .and_then(|rest| rest.split("pub(crate) struct CandidateTreeState").next())
        .expect(
            "generate/state.rs should define resolved candidate state before candidate tree state",
        );

    for required in [
        "base: ResolvedBase",
        "patch_plan: PatchPlan",
        "integration_plan: IntegrationPlan",
        "feature_intent_plan: FeatureIntentPlan",
        "feature_graph_fingerprint: FeatureGraphFingerprint",
        "removal_manifest_fingerprint: RemovalManifestFingerprint",
        "abi_policy_fingerprint: AbiPolicyFingerprint",
        "arch_policy_fingerprint: ArchPolicyFingerprint",
        "feature_resolution: FeatureResolutionState",
        "abi_decision: AbiDecisionState",
        "prune_plan: PrunePlan",
        "reducer_plan: ReducerPlan",
        "build_matrix_plan: BuildMatrixPlan",
        "selftest_plan: SelftestPlan",
        "output_plan: OutputPlan",
        "pub(crate) fn from_resolved_inputs(",
        "let feature_resolution = FeatureResolutionState::from_profile(profile)?",
        "let feature_graph_fingerprint = FeatureGraphFingerprint::from_resolved_feature_graph(",
        "&feature_intent_plan,",
        "&feature_resolution,",
        "let removal_manifest_fingerprint = RemovalManifestFingerprint::from_profile(profile)?",
        "let abi_policy_fingerprint =",
        "AbiPolicyFingerprint::from_policy(feature_resolution.abi_policy())?",
        "let arch_policy_fingerprint = ArchPolicyFingerprint::from_profile(profile)?",
        "let abi_decision = AbiDecisionState::from_feature_resolution(&feature_resolution)?",
        "let prune_plan = PrunePlan::from_feature_resolution(&feature_resolution)",
        "PatchPlan::from_patch_infos(patch_infos)",
        "IntegrationPlan::from_profile(profile)",
        "FeatureIntentPlan::from_profile(profile)?",
        "ReducerPlan::from_config(&profile.reducer)",
        "BuildMatrixPlan::from_config(&profile.build_matrix)?",
        "SelftestPlan::from_config(&profile.selftests)?",
        "OutputPlan::new(config, profile, &base, target_branch, mode)?",
        "remove_paths: Vec<RelativeKernelPath>",
        "remove_configs: Vec<KconfigSymbol>",
        "set_defaults: BTreeMap<KconfigSymbol, String>",
        "abi_policy: AbiPolicyConfig",
        "output_path: OutputRepoPath",
        "branch: String",
        "mode: String",
        "lockfile_path: Option<LockfilePath>",
        "naming: OutputNamingPlan",
        "pub(crate) struct OutputNamingPlan",
        "project_name: String",
        "profile_name: String",
        "branch_prefix: String",
        "explicit_branch: Option<String>",
        "base_ref: String",
        "base_commit: String",
        "project_name: config.project.name.clone()",
        "profile_name: profile.profile.name.clone()",
        "branch_prefix: config.output.branch_prefix.clone()",
        "explicit_branch: config.output.branch.clone()",
        "base_ref: base.r#ref.clone()",
        "base_commit: base.commit.clone()",
        "arch: Option<ArchName>",
        "output_dir: Option<KernelBuildDir>",
        "pub(crate) struct ReducerPlan",
        "max_fixup_passes: usize",
        "report_unsupported_expressions: bool",
        "fail_on_unknown_diagnostics: bool",
        "reject_unproven_fixups: bool",
        "reject_unreasoned_edits: bool",
        "reject_speculative_fallout_edits: bool",
        "fail_on_missing_prune_paths: bool",
        "ignore_unsupported_special_removals: bool",
        "max_fixup_passes: config.max_fixup_passes",
        "report_unsupported_expressions: config.report_unsupported_expressions",
        "fail_on_unknown_diagnostics: config.fail_on_unknown_diagnostics",
        "reject_unproven_fixups: config.reject_unproven_fixups",
        "reject_unreasoned_edits: config.reject_unreasoned_edits",
        "reject_speculative_fallout_edits: config.reject_speculative_fallout_edits",
        "fail_on_missing_prune_paths: config.fail_on_missing_prune_paths",
        "ignore_unsupported_special_removals: config.ignore_unsupported_special_removals",
        "pub(crate) struct BuildMatrixPlan",
        "enabled: bool",
        "presets: Vec<String>",
        "arches: Vec<ArchName>",
        "config_targets: Vec<String>",
        "randconfig_seed: Option<String>",
        "fail_on_error: bool",
        "enabled: config.enabled",
        "presets: sorted_nonempty_strings(&config.presets)?",
        "arches: sorted_arch_names(&config.arches)?",
        "fail_on_error: config.fail_on_error",
        "pub(crate) struct SelftestPlan",
        "enabled: bool",
        "check_kconfig_sources: bool",
        "check_makefiles: bool",
        "kernel_builds: Vec<KernelBuildPlan>",
        "commands: Vec<String>",
        "enabled: config.enabled",
        "check_kconfig_sources: config.check_kconfig_sources",
        "check_makefiles: config.check_makefiles",
        "pub(crate) struct KernelBuildPlan",
        "config_target: Option<String>",
        "targets: Vec<String>",
        "jobs: Option<usize>",
        "clean: bool",
        "make_program: Option<String>",
        "make_args: Vec<String>",
        "env: BTreeMap<String, String>",
        "KernelBuildPlan::from_config",
        "pub(crate) struct FeatureIntentPlan",
        "pub(crate) struct FeatureIntentEntryPlan",
        "stable_id: String",
        "action: String",
        "name: String",
        "roots: Vec<RelativeKernelPath>",
        "exported_symbols: Vec<ExportedSymbol>",
        "remove_exported_symbols: Vec<ExportedSymbol>",
        "module_names: Vec<ModuleName>",
        "remove_module_names: Vec<ModuleName>",
        "module_aliases: Vec<ModuleAlias>",
        "remove_module_aliases: Vec<ModuleAlias>",
        "device_compatibles: Vec<DeviceCompatible>",
        "remove_device_compatibles: Vec<DeviceCompatible>",
        "acpi_ids: Vec<AcpiId>",
        "remove_acpi_ids: Vec<AcpiId>",
        "pci_ids: Vec<PciId>",
        "remove_pci_ids: Vec<PciId>",
        "usb_ids: Vec<UsbId>",
        "remove_usb_ids: Vec<UsbId>",
        "firmware_paths: Vec<FirmwarePath>",
        "remove_firmware_paths: Vec<FirmwarePath>",
        "initcalls: Vec<Initcall>",
        "remove_initcalls: Vec<Initcall>",
        "runtime_registrations: Vec<RuntimeRegistrationSurface>",
        "remove_runtime_registrations: Vec<RuntimeRegistrationSurface>",
        "docs: Vec<DocumentationPath>",
        "remove_docs: Vec<DocumentationPath>",
        "tools: Vec<ToolPath>",
        "remove_tools: Vec<ToolPath>",
        "samples: Vec<SamplePath>",
        "remove_samples: Vec<SamplePath>",
        "kunit_suites: Vec<KunitSuite>",
        "remove_kunit_suites: Vec<KunitSuite>",
        "kselftest_targets: Vec<KselftestTarget>",
        "remove_kselftest_targets: Vec<KselftestTarget>",
    ] {
        assert!(
            resolved_section.contains(required),
            "ResolvedCandidateState should capture typed resolved plan facts; missing {required}"
        );
    }

    for required in [
        "resolved.reducer_plan.max_fixup_passes",
        "resolved.reducer_plan.report_unsupported_expressions",
        "resolved.reducer_plan.fail_on_unknown_diagnostics",
        "resolved.reducer_plan.reject_unproven_fixups",
        "resolved.reducer_plan.reject_unreasoned_edits",
        "resolved.reducer_plan.reject_speculative_fallout_edits",
        "resolved.reducer_plan.fail_on_missing_prune_paths",
        "resolved.reducer_plan.ignore_unsupported_special_removals",
    ] {
        assert!(
            plan.contains(required),
            "generate plan fingerprint should serialize resolved reducer config; missing {required}"
        );
    }

    for required in [
        "resolved.build_matrix_plan.enabled",
        "resolved.build_matrix_plan.preset_count",
        "resolved.build_matrix_plan.arch_count",
        "resolved.build_matrix_plan.config_target_count",
        "resolved.build_matrix_plan.target_count",
        "resolved.build_matrix_plan.randconfig_seed",
        "resolved.build_matrix_plan.jobs",
        "resolved.build_matrix_plan.fail_on_error",
    ] {
        assert!(
            build_matrix_fingerprint.contains(required),
            "generate plan fingerprint should serialize resolved build matrix config; missing {required}"
        );
    }

    for required in [
        "resolved.selftest_plan.enabled",
        "resolved.selftest_plan.check_kconfig_sources",
        "resolved.selftest_plan.check_makefiles",
        "resolved.selftest_plan.kernel_builds",
        "resolved.selftest_plan.commands",
        "config_target",
        "make_program",
        "make_args",
    ] {
        assert!(
            plan.contains(required),
            "generate plan fingerprint should serialize resolved selftest matrix; missing {required}"
        );
    }

    for required in [
        "resolved.output_plan.branch",
        "resolved.output_plan.mode",
        "resolved.output_plan.naming.project_name",
        "resolved.output_plan.naming.profile_name",
        "resolved.output_plan.naming.branch_prefix",
        "resolved.output_plan.naming.explicit_branch",
        "resolved.output_plan.naming.base_ref",
        "resolved.output_plan.naming.base_commit",
    ] {
        assert!(
            plan.contains(required),
            "generate plan fingerprint should serialize output branch and commit naming inputs; missing {required}"
        );
    }

    for required in [
        "pub(crate) struct FeatureGraphFingerprint",
        "pub(crate) struct RemovalManifestFingerprint",
        "pub(crate) struct AbiPolicyFingerprint",
        "pub(crate) struct ArchPolicyFingerprint",
        "pub(super) fn from_resolved_feature_graph(",
        "pub(super) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "pub(super) fn from_policy(policy: &AbiPolicyConfig) -> Result<Self>",
        "fn from_policy(policy: &ArchPolicyConfig) -> Result<Self>",
    ] {
        assert!(
            fingerprint.contains(required),
            "resolved plan fingerprint module should own typed fingerprint facts; missing {required}"
        );
    }

    for forbidden in [
        "CandidateTreeState",
        "CandidateTreePath",
        "CandidateMetadataDir",
        "PublishedMetadataDir",
        "PublishedSnapshotState",
        "CommittedOutputSnapshot",
        "GenerateAttemptFailure",
        "AttemptMetadataDir",
        "SuccessfulCommitResult",
        "commit_if_changed",
        "sync_working_tree",
        "write_authoritative_lockfile",
        "write_verified_published_snapshot_metadata",
        "std::fs::write",
    ] {
        assert!(
            !resolved_section.contains(forbidden),
            "ResolvedCandidateState must not contain candidate tree, published, failure, commit, or mutation state; found {forbidden}"
        );
    }

    assert!(
        plan.contains("let resolved_state = ResolvedCandidateState::from_resolved_inputs(")
            && plan.contains("GeneratePlan::from_parts(\n        requested,\n        resolved_state,"),
        "plan resolution should construct ResolvedCandidateState before creating the immutable GeneratePlan"
    );
}
