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
fn profile_config_is_profile_file_user_intent_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_model = production_source(&root.join("src/config/model.rs"));
    let config_mod = production_source(&root.join("src/config/mod.rs"));
    let config_load = production_source(&root.join("src/config/load.rs"));
    let config_templates = production_source(&root.join("src/config/templates.rs"));
    let config_validate = production_source(&root.join("src/config/validate.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);
    let architecture_flat = architecture.replace("\n  ", " ");

    assert!(
        config_model.contains("pub struct ProfileConfig {"),
        "config/model.rs should define the profile-file ProfileConfig model"
    );
    let profile_config = section_between(
        &config_model,
        "pub struct ProfileConfig",
        "impl ProfileConfig",
    );
    for required in [
        "pub profile: ProfileSection,",
        "pub base: BaseSection,",
        "pub slim: Option<SlimConfig>,",
        "pub abi: AbiPolicyConfig,",
        "pub arch: ArchPolicyConfig,",
        "pub build_matrix: BuildMatrixConfig,",
        "pub runtime_matrix: RuntimeMatrixConfig,",
        "pub reports: ReportConfig,",
        "pub security: SecurityConfig,",
        "pub performance: PerformanceConfig,",
        "pub patches: Option<PatchConfig>,",
        "pub integrations: IntegrationsConfig,",
        "pub reducer: ReducerConfig,",
        "pub selftests: SelfTestConfig,",
    ] {
        assert!(
            profile_config.contains(required),
            "ProfileConfig should own profile user-intent field {required}"
        );
    }

    for required in [
        "pub struct ProfileSection",
        "pub struct BaseSection",
        "pub struct ArchPolicyConfig",
        "pub struct BuildMatrixConfig",
        "pub struct RuntimeMatrixConfig",
        "pub struct ReportConfig",
        "pub struct SecurityConfig",
        "pub struct PerformanceConfig",
        "pub enum PatchConfig",
        "pub struct PatchSourceConfig",
        "pub struct PatchSourcesConfig",
        "pub struct IntegrationsConfig",
        "pub struct RtlmqIntegrationConfig",
        "pub struct SelfTestConfig",
        "pub struct KernelBuildConfig",
    ] {
        assert!(
            config_model.contains(required),
            "config/model.rs should define ProfileConfig child model {required}"
        );
    }

    for forbidden in [
        "KslimConfig",
        "ProjectConfig",
        "UpstreamConfig",
        "OutputConfig",
        "GitConfig",
        "PublishConfig",
        "CandidateTreeState",
        "CandidateVerification",
        "PublishedSnapshotState",
        "GenerateAttemptFailure",
        "LockfilePath",
    ] {
        assert!(
            !profile_config.contains(forbidden),
            "ProfileConfig must not absorb project-root config or lifecycle state {forbidden}"
        );
    }

    assert!(
        config_model.contains("pub fn removal_input(&self) -> Option<&SlimConfig>")
            && config_model.contains("self.slim.as_ref()")
            && config_model.contains("pub fn effective_removal_input(&self) -> Option<SlimConfig>")
            && config_model.contains("pub fn effective_preservation_input(&self)")
            && config_model.contains("pub fn effective_abi_policy(&self) -> AbiPolicyConfig")
            && config_model.contains("pub fn effective_feature_safety_levels(&self)")
            && config_model.contains("pub fn effective_feature_arch_scopes(&self)")
            && config_model.contains("pub fn effective_feature_test_matrices(&self)")
            && config_model.contains("pub fn effective_feature_report_modes(&self)")
            && config_model.contains("self.features.remove.values()")
            && config_model.contains("self.features.preserve.values()"),
        "ProfileConfig should expose direct and effective removal/preservation/ABI/safety/arch/test-matrix/report policy intent"
    );
    assert!(
        config_templates.contains("pub fn default_profile_config(ref_name: &str) -> ProfileConfig")
            && config_templates.contains("profile: ProfileSection")
            && config_templates.contains("base: BaseSection")
            && config_templates.contains("slim: None")
            && config_templates.contains("abi: AbiPolicyConfig::default()")
            && config_templates.contains("arch: ArchPolicyConfig::default()")
            && config_templates.contains("build_matrix: BuildMatrixConfig::default()")
            && config_templates.contains("runtime_matrix: RuntimeMatrixConfig::default()")
            && config_templates.contains("reports: ReportConfig::default()")
            && config_templates.contains("security: SecurityConfig::default()")
            && config_templates.contains("performance: PerformanceConfig::default()")
            && config_templates.contains("patches: None")
            && config_templates.contains("integrations: IntegrationsConfig::default()")
            && config_templates.contains("reducer: ReducerConfig::default()")
            && config_templates.contains("selftests: SelfTestConfig::default()"),
        "config/templates.rs should construct a complete default ProfileConfig"
    );
    assert!(
        config_load.contains(
            "pub fn load_profile(root: &Path, profile_name: &str) -> Result<ProfileConfig>"
        ) && config_load
            .contains("root.join(\"profiles\").join(format!(\"{}.toml\", profile_name))")
            && config_load.contains("toml::from_str(&contents)")
            && config_load.contains("validate_profile(&profile)?")
            && config_load.contains("profile name mismatch"),
        "config/load.rs should load and validate named profile files from profiles/<name>.toml"
    );
    assert!(
        config_validate.contains("pub fn validate_profile(profile: &ProfileConfig) -> Result<()>")
            && config_validate.contains("profile.name must not be empty")
            && config_validate.contains("base.ref must not be empty")
            && config_validate.contains("patches must contain at least one source")
            && config_validate.contains("integrations.rtlmq.source must not be empty")
            && config_validate
                .contains("RemovalManifest::from_slim_config_with_abi_policy_and_preservation")
            && config_validate.contains(
                "reducer settings may only be customized when [slim] or [features.remove] declares removal input"
            )
            && config_validate.contains("selftests.commands must not contain empty commands")
            && config_validate.contains("ARCH env is invalid"),
        "config/validate.rs should own ProfileConfig validation"
    );
    assert!(
        config_mod.contains("mod model;")
            && config_mod.contains("pub use model::*;")
            && config_mod.contains("load_profile")
            && config_mod.contains("default_profile_config")
            && config_mod.contains("validate_profile"),
        "config/mod.rs should re-export ProfileConfig model, defaults, loading, and validation"
    );
    assert!(
        architecture_flat.contains(
            "`ProfileConfig` is the per-profile `profiles/<name>.toml` user-intent model"
        ) && architecture_flat.contains(
            "base ref, removal intent, ABI policy, profile-inheritance intent, patches, integrations, reducer knobs, and selftests"
        ) && architecture_flat.contains("Architecture policy also belongs here")
            && architecture_flat.contains("Build matrix policy also belongs here")
            && architecture_flat.contains("Runtime matrix policy also belongs here")
            && architecture_flat.contains("Report policy also belongs here")
            && architecture_flat.contains("Security policy also belongs here")
            && architecture_flat.contains("Performance policy also belongs here")
            && architecture_flat
            .contains("does not own project-root upstream, output, git, or publish settings"),
        "architecture docs should describe ProfileConfig ownership"
    );
    assert!(
        kernel_build_guide.contains("## Profile shape")
            && kernel_build_guide.contains("[profile]")
            && kernel_build_guide.contains("[base]")
            && kernel_build_guide.contains("[slim]")
            && kernel_build_guide.contains("[patches]")
            && kernel_build_guide.contains("[selftests]"),
        "kernel build iteration docs should show the ProfileConfig file shape"
    );
}
