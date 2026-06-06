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
fn runtime_matrix_config_is_profile_runtime_matrix_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_model = production_source(&root.join("src/config/model.rs"));
    let config_validate = production_source(&root.join("src/config/validate.rs"));
    let config_templates = production_source(&root.join("src/config/templates.rs"));
    let generate_state = state_source(root);
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);
    let architecture_flat = architecture.replace("\n  ", " ");

    assert!(
        config_model.contains("pub struct RuntimeMatrixConfig {"),
        "config/model.rs should define the [runtime_matrix] runtime-validation model"
    );
    let profile_config = section_between(
        &config_model,
        "pub struct ProfileConfig",
        "impl ProfileConfig",
    );
    assert!(
        profile_config.contains("pub runtime_matrix: RuntimeMatrixConfig,"),
        "ProfileConfig should carry RuntimeMatrixConfig selected by profile input"
    );

    let runtime_matrix_config = section_between(
        &config_model,
        "pub struct RuntimeMatrixConfig",
        "impl Default for RuntimeMatrixConfig",
    );
    for required in [
        "pub enabled: bool,",
        "pub boot_arches: Vec<String>,",
        "pub qemu_machines: Vec<String>,",
        "pub kunit_suites: Vec<String>,",
        "pub kselftest_targets: Vec<String>,",
        "pub module_smoke: bool,",
        "pub require_clean_dmesg: bool,",
        "pub boot_timeout_seconds: Option<u64>,",
        "pub fail_on_error: bool,",
    ] {
        assert!(
            runtime_matrix_config.contains(required),
            "RuntimeMatrixConfig should own raw runtime policy field {required}"
        );
    }

    for forbidden in [
        "KslimConfig",
        "OutputConfig",
        "FeatureConfig",
        "AbiPolicyConfig",
        "ArchPolicyConfig",
        "BuildMatrixConfig",
        "ReducerConfig",
        "SelfTestConfig",
        "KernelBuildConfig",
        "FeatureResolutionState",
        "SelftestPlan",
        "KernelBuildPlan",
        "BuildMatrixStatus",
        "RuntimeRegistrationRemovalProof",
        "PrunePlan",
        "RemovalManifest",
        "CandidateTreeState",
        "PublishedSnapshotState",
        "LockfilePath",
    ] {
        assert!(
            !runtime_matrix_config.contains(forbidden),
            "RuntimeMatrixConfig must stay raw profile policy, not active runtime/plan/state {forbidden}"
        );
    }

    assert!(
        config_model.contains("enabled: false")
            && config_model.contains("boot_arches: Vec::new()")
            && config_model.contains("qemu_machines: Vec::new()")
            && config_model.contains("kunit_suites: Vec::new()")
            && config_model.contains("kselftest_targets: Vec::new()")
            && config_model.contains("module_smoke: false")
            && config_model.contains("require_clean_dmesg: true")
            && config_model.contains("boot_timeout_seconds: None")
            && config_model.contains("fail_on_error: true")
            && config_model.contains("pub fn is_default(&self) -> bool"),
        "RuntimeMatrixConfig defaults should be inactive, fail-on-error, clean-dmesg, and detectable"
    );
    assert!(
        config_validate
            .contains("fn validate_runtime_matrix_config(config: &RuntimeMatrixConfig) -> Result<()>")
            && config_validate.contains("validate_arch_name_list(\"runtime_matrix.boot_arches\"")
            && config_validate.contains("runtime_matrix.qemu_machines")
            && config_validate.contains("runtime_matrix.kunit_suites")
            && config_validate.contains("runtime_matrix.kselftest_targets")
            && config_validate
                .contains("runtime_matrix.boot_timeout_seconds must be greater than zero")
            && config_validate.contains("runtime matrix config is parsed but not yet supported")
            && config_validate.contains("validate_runtime_matrix_config(&profile.runtime_matrix)?"),
        "profile validation should validate runtime fields and fail closed for nondefault RuntimeMatrixConfig"
    );
    assert!(
        config_templates.contains("runtime_matrix: RuntimeMatrixConfig::default()")
            && config_templates.contains("[runtime_matrix]")
            && config_templates.contains("boot_arches = [\"x86\"]")
            && config_templates.contains("qemu_machines = [\"q35\"]")
            && config_templates.contains("Use `[selftests].commands`"),
        "profile templates should show RuntimeMatrixConfig as future fail-closed policy"
    );
    assert!(
        !generate_state.contains("profile.runtime_matrix"),
        "generate state must not silently consume RuntimeMatrixConfig before runtime matrix planning lands"
    );
    assert!(
        architecture_flat.contains(
            "`RuntimeMatrixConfig` is the `[runtime_matrix]` profile runtime-validation model"
        ) && architecture_flat.contains("boot architectures, QEMU machine labels, KUnit")
            && architecture_flat.contains("fails closed until runtime matrix planning lands"),
        "architecture docs should describe RuntimeMatrixConfig ownership and fail-closed behavior"
    );
    assert!(
        kernel_build_guide.contains("[runtime_matrix]")
            && kernel_build_guide.contains("runtime matrix planning lands")
            && kernel_build_guide.contains("Use `[selftests].commands`"),
        "kernel build iteration docs should explain RuntimeMatrixConfig is not active yet"
    );
}
