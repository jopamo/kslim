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
fn arch_policy_config_is_profile_arch_policy_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_model = production_source(&root.join("src/config/model.rs"));
    let config_validate = production_source(&root.join("src/config/validate.rs"));
    let config_templates = production_source(&root.join("src/config/templates.rs"));
    let generate_state = state_source(root);
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);
    let architecture_flat = architecture.replace("\n  ", " ");

    assert!(
        config_model.contains("pub struct ArchPolicyConfig {"),
        "config/model.rs should define the [arch] architecture-policy model"
    );
    let profile_config = section_between(
        &config_model,
        "pub struct ProfileConfig",
        "impl ProfileConfig",
    );
    assert!(
        profile_config.contains("pub arch: ArchPolicyConfig,"),
        "ProfileConfig should carry ArchPolicyConfig selected by profile input"
    );

    let arch_config = section_between(
        &config_model,
        "pub struct ArchPolicyConfig",
        "impl Default for ArchPolicyConfig",
    );
    for required in [
        "pub primary_arch: Option<String>,",
        "pub secondary_arches: Vec<String>,",
        "pub disabled_arches: Vec<String>,",
        "pub allow_arch_local_removal: bool,",
        "pub preserve_arch_shared: bool,",
    ] {
        assert!(
            arch_config.contains(required),
            "ArchPolicyConfig should own raw architecture policy field {required}"
        );
    }

    for forbidden in [
        "KslimConfig",
        "OutputConfig",
        "FeatureConfig",
        "AbiPolicyConfig",
        "ReducerConfig",
        "SelfTestConfig",
        "KernelBuildConfig",
        "FeatureResolutionState",
        "ArchPolicy",
        "SelftestPlan",
        "PrunePlan",
        "RemovalManifest",
        "CandidateTreeState",
        "PublishedSnapshotState",
        "LockfilePath",
    ] {
        assert!(
            !arch_config.contains(forbidden),
            "ArchPolicyConfig must stay raw profile policy, not resolved plan/state {forbidden}"
        );
    }

    assert!(
        config_model.contains("primary_arch: None")
            && config_model.contains("secondary_arches: Vec::new()")
            && config_model.contains("disabled_arches: Vec::new()")
            && config_model.contains("allow_arch_local_removal: false")
            && config_model.contains("preserve_arch_shared: true")
            && config_model.contains("pub fn is_default(&self) -> bool"),
        "ArchPolicyConfig defaults should be safe and detectable"
    );
    assert!(
        config_validate.contains("fn validate_arch_policy_config(config: &ArchPolicyConfig) -> Result<()>")
            && config_validate.contains("ArchName::new(arch)")
            && config_validate.contains("must not contain duplicate architecture")
            && config_validate.contains("cannot be declared in both arch.secondary_arches and arch.disabled_arches")
            && config_validate.contains("arch.allow_arch_local_removal requires arch.primary_arch")
            && config_validate.contains("arch.allow_arch_local_removal is not yet supported")
            && config_validate.contains("arch.preserve_arch_shared=false is not yet supported")
            && config_validate.contains("validate_arch_policy_config(&profile.arch)?"),
        "profile validation should validate arch names and fail closed for unsupported arch-policy switches"
    );
    assert!(
        config_templates.contains("arch: ArchPolicyConfig::default()")
            && config_templates.contains("[arch]")
            && config_templates.contains("primary_arch = \"x86\"")
            && config_templates.contains("secondary_arches = [\"arm64\", \"riscv\"]")
            && config_templates.contains("Kconfig trees are treated as")
            && config_templates.contains("dead-definition solver proofs"),
        "profile templates should describe the supported live-arch proof scope"
    );
    assert!(
        generate_state.contains("pub(crate) arch_policy: ArchPolicyConfig,")
            && generate_state.contains("arch_policy: profile.arch.clone()")
            && generate_state.contains("arch_policy_fingerprint"),
        "resolved state should carry ArchPolicyConfig into prune planning and fingerprint truth"
    );
    assert!(
        architecture_flat
            .contains("`ArchPolicyConfig` is the `[arch]` profile architecture-policy model")
            && architecture_flat.contains("primary, secondary, and disabled architecture names")
            && architecture_flat.contains("constrains which `arch/*` Kconfig trees are treated as live")
            && architecture_flat.contains("`allow_arch_local_removal` remains reserved"),
        "architecture docs should describe the supported live-arch proof scope and reserved switches"
    );
    assert!(
        kernel_build_guide.contains("[arch]")
            && kernel_build_guide.contains("dead-definition solver proofs")
            && kernel_build_guide.contains("slim.remove_paths")
            && kernel_build_guide.contains("[[selftests.kernel_builds]].env.ARCH"),
        "kernel build iteration docs should explain the active arch-policy slice"
    );
}
