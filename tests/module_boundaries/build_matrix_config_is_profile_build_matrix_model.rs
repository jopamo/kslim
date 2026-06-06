use super::common::*;

fn section_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let (_, rest) = source
        .split_once(start)
        .unwrap_or_else(|| panic!("missing section start marker {start:?}"));
    let (section, _) = rest
        .split_once(end)
        .unwrap_or_else(|| panic!("missing section end marker {end:?}"));
    section
}

#[test]
fn build_matrix_config_is_profile_build_matrix_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_model = production_source(&root.join("src/config/model.rs"));
    let config_validate = production_source(&root.join("src/config/validate.rs"));
    let config_templates = production_source(&root.join("src/config/templates.rs"));
    let generate_state = state_source(root);
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);
    let architecture_flat = architecture.replace("\n  ", " ");

    assert!(
        config_model.contains("pub struct BuildMatrixConfig {"),
        "config/model.rs should define the [build_matrix] verification-matrix model"
    );
    let profile_config = section_between(
        &config_model,
        "pub struct ProfileConfig",
        "impl ProfileConfig",
    );
    assert!(
        profile_config.contains("pub build_matrix: BuildMatrixConfig,"),
        "ProfileConfig should carry BuildMatrixConfig selected by profile input"
    );

    let build_matrix_config = section_between(
        &config_model,
        "pub struct BuildMatrixConfig",
        "impl Default for BuildMatrixConfig",
    );
    for required in [
        "pub enabled: bool,",
        "pub presets: Vec<String>,",
        "pub arches: Vec<String>,",
        "pub config_targets: Vec<String>,",
        "pub targets: Vec<String>,",
        "pub randconfig_seed: Option<String>,",
        "pub jobs: Option<usize>,",
        "pub fail_on_error: bool,",
    ] {
        assert!(
            build_matrix_config.contains(required),
            "BuildMatrixConfig should own raw matrix policy field {required}"
        );
    }

    for forbidden in [
        "KslimConfig",
        "OutputConfig",
        "FeatureConfig",
        "AbiPolicyConfig",
        "ArchPolicyConfig",
        "ReducerConfig",
        "SelfTestConfig",
        "KernelBuildConfig",
        "FeatureResolutionState",
        "SelftestPlan",
        "KernelBuildPlan",
        "BuildMatrixStatus",
        "PrunePlan",
        "RemovalManifest",
        "CandidateTreeState",
        "PublishedSnapshotState",
        "LockfilePath",
    ] {
        assert!(
            !build_matrix_config.contains(forbidden),
            "BuildMatrixConfig must stay raw profile policy, not active selftest/plan/state {forbidden}"
        );
    }

    assert!(
        config_model.contains("enabled: false")
            && config_model.contains("presets: Vec::new()")
            && config_model.contains("arches: Vec::new()")
            && config_model.contains("config_targets: Vec::new()")
            && config_model.contains("targets: Vec::new()")
            && config_model.contains("randconfig_seed: None")
            && config_model.contains("jobs: None")
            && config_model.contains("fail_on_error: true")
            && config_model.contains("pub fn is_default(&self) -> bool"),
        "BuildMatrixConfig defaults should be inactive, fail-on-error, and detectable"
    );
    assert!(
        config_validate.contains("const BUILD_MATRIX_PRESETS")
            && config_validate
                .contains("fn validate_build_matrix_config(config: &BuildMatrixConfig) -> Result<()>")
            && config_validate.contains("validate_arch_name_list(\"build_matrix.arches\"")
            && config_validate.contains("build_matrix.presets contains unsupported preset")
            && config_validate.contains("build_matrix.randconfig_seed must not be empty")
            && config_validate.contains("build_matrix.jobs must be greater than zero")
            && config_validate.contains("build matrix config is parsed but not yet supported")
            && config_validate.contains("validate_build_matrix_config(&profile.build_matrix)?"),
        "profile validation should validate matrix fields and fail closed for nondefault BuildMatrixConfig"
    );
    assert!(
        config_templates.contains("build_matrix: BuildMatrixConfig::default()")
            && config_templates.contains("[build_matrix]")
            && config_templates.contains("presets = [\"default\"]")
            && config_templates.contains("targets = [\"vmlinux\", \"modules\"]")
            && config_templates.contains("Use `[selftests]` / `[[selftests.kernel_builds]]`"),
        "profile templates should show BuildMatrixConfig as future fail-closed policy"
    );
    assert!(
        generate_state.contains("pub(crate) struct BuildMatrixPlan")
            && generate_state.contains("BuildMatrixPlan::from_config(&profile.build_matrix)?")
            && generate_state.contains("build_matrix_plan: BuildMatrixPlan"),
        "generate state should capture BuildMatrixConfig only as inert resolved plan/fingerprint truth before matrix planning lands"
    );
    assert!(
        !generate_state.contains("BuildMatrixStatus"),
        "generate state must not turn BuildMatrixConfig into active build status before matrix planning lands"
    );
    assert!(
        architecture_flat.contains(
            "`BuildMatrixConfig` is the `[build_matrix]` profile verification-matrix model"
        ) && architecture_flat.contains("preset names, architecture names, config targets")
            && architecture_flat.contains("fails closed until matrix planning lands")
            && architecture_flat.contains("default/inert resolved policy is captured"),
        "architecture docs should describe BuildMatrixConfig ownership and fail-closed behavior"
    );
    assert!(
        kernel_build_guide.contains("[build_matrix]")
            && kernel_build_guide.contains("build matrix planning lands")
            && kernel_build_guide.contains("Use `[selftests]` / `[[selftests.kernel_builds]]`"),
        "kernel build iteration docs should explain BuildMatrixConfig is not active yet"
    );
}
