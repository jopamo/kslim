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
fn security_config_is_profile_security_policy_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_model = production_source(&root.join("src/config/model.rs"));
    let config_validate = production_source(&root.join("src/config/validate.rs"));
    let config_templates = production_source(&root.join("src/config/templates.rs"));
    let main = production_source(&root.join("src/main.rs"));
    let security_mod = production_source(&root.join("src/security/mod.rs"));
    let security_policy = production_source(&root.join("src/security/policy.rs"));
    let generate_state = state_source(root);
    let output_metadata = production_source(&root.join("src/output_repo/metadata.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);
    let architecture_flat = architecture.replace("\n  ", " ");

    assert!(
        config_model.contains("pub struct SecurityConfig {"),
        "config/model.rs should define the [security] trust-boundary policy model"
    );
    let profile_config = section_between(
        &config_model,
        "pub struct ProfileConfig",
        "impl ProfileConfig",
    );
    assert!(
        profile_config.contains("pub security: SecurityConfig,"),
        "ProfileConfig should carry SecurityConfig selected by profile input"
    );

    let security_config = section_between(
        &config_model,
        "pub struct SecurityConfig",
        "impl Default for SecurityConfig",
    );
    for required in [
        "pub allow_network: bool,",
        "pub require_local_upstream: bool,",
        "pub reject_host_paths_in_committed_metadata: bool,",
        "pub reject_temp_paths_in_committed_metadata: bool,",
        "pub reject_raw_logs_in_committed_metadata: bool,",
        "pub require_reproducible_timestamps: bool,",
        "pub require_phase_typed_metadata: bool,",
        "pub compatibility_mode: Option<String>,",
        "pub fail_on_policy_violation: bool,",
    ] {
        assert!(
            security_config.contains(required),
            "SecurityConfig should own raw security policy field {required}"
        );
    }

    for forbidden in [
        "KslimConfig",
        "OutputConfig",
        "FeatureConfig",
        "AbiPolicyConfig",
        "ArchPolicyConfig",
        "BuildMatrixConfig",
        "RuntimeMatrixConfig",
        "ReportConfig",
        "PerformanceConfig",
        "ReducerConfig",
        "SelfTestConfig",
        "FeatureResolutionState",
        "CandidateTreeState",
        "PublishedSnapshotState",
        "OutputRepoPath",
        "PublishedMetadataDir",
        "LockfilePath",
    ] {
        assert!(
            !security_config.contains(forbidden),
            "SecurityConfig must stay raw profile security policy, not plan/state/authority {forbidden}"
        );
    }

    assert!(
        config_model.contains("allow_network: false")
            && config_model.contains("require_local_upstream: true")
            && config_model.contains("reject_host_paths_in_committed_metadata: true")
            && config_model.contains("reject_temp_paths_in_committed_metadata: true")
            && config_model.contains("reject_raw_logs_in_committed_metadata: true")
            && config_model.contains("require_reproducible_timestamps: true")
            && config_model.contains("require_phase_typed_metadata: true")
            && config_model.contains("compatibility_mode: None")
            && config_model.contains("fail_on_policy_violation: true")
            && config_model.contains("pub fn is_default(&self) -> bool"),
        "SecurityConfig defaults should be fixed, fail-closed, and detectable"
    );
    assert!(
        main.contains("mod security;")
            && security_mod.contains("mod policy;")
            && security_mod.contains("pub(crate) use policy::validate_security_config;")
            && security_policy
                .contains("pub(crate) fn validate_security_config(config: &SecurityConfig) -> Result<()>")
            && security_policy.contains("security.compatibility_mode must not be empty")
            && security_policy.contains("security.allow_network is not supported")
            && security_policy.contains("reject_host_paths_in_committed_metadata cannot be disabled")
            && security_policy.contains("reject_temp_paths_in_committed_metadata cannot be disabled")
            && security_policy.contains("reject_raw_logs_in_committed_metadata cannot be disabled")
            && security_policy.contains("security.fail_on_policy_violation cannot be disabled")
            && security_policy.contains("security config is parsed but not yet supported")
            && config_validate.contains("crate::security::validate_security_config(&profile.security)?"),
        "security/policy.rs should validate security fields and fail closed for nondefault SecurityConfig"
    );
    for forbidden in [
        "fn validate_security_config(config: &SecurityConfig) -> Result<()>",
        "security.compatibility_mode must not be empty",
        "security.allow_network is not supported",
        "reject_host_paths_in_committed_metadata cannot be disabled",
        "reject_temp_paths_in_committed_metadata cannot be disabled",
        "reject_raw_logs_in_committed_metadata cannot be disabled",
        "security.fail_on_policy_violation cannot be disabled",
    ] {
        assert!(
            !config_validate.contains(forbidden),
            "config/validate.rs should call centralized security policy instead of embedding security validation detail {forbidden}"
        );
    }
    assert!(
        config_templates.contains("security: SecurityConfig::default()")
            && config_templates.contains("[security]")
            && config_templates.contains("allow_network = false")
            && config_templates
                .contains("Security trust-boundary checks are fixed and fail-closed"),
        "profile templates should show SecurityConfig as future fail-closed policy"
    );
    assert!(
        !generate_state.contains("profile.security")
            && !output_metadata.contains("SecurityConfig"),
        "generate/output metadata code must not silently consume SecurityConfig before security planning lands"
    );
    assert!(
        architecture_flat
            .contains("`SecurityConfig` is the `[security]` profile trust-boundary policy model")
            && architecture_flat
                .contains("network input, local-upstream requirement, committed metadata path")
            && architecture_flat.contains("fails closed until security planning lands"),
        "architecture docs should describe SecurityConfig ownership and fail-closed behavior"
    );
    assert!(
        kernel_build_guide.contains("[security]")
            && kernel_build_guide.contains("security planning lands")
            && kernel_build_guide
                .contains("Security trust-boundary checks are fixed and fail-closed"),
        "kernel build iteration docs should explain SecurityConfig is not active yet"
    );
}
