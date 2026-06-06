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
fn slim_config_is_direct_removal_intent_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_model = production_source(&root.join("src/config/model.rs"));
    let config_validate = production_source(&root.join("src/config/validate.rs"));
    let removal_parse = production_source(&root.join("src/removal_manifest/parse.rs"));
    let removal_validate = production_source(&root.join("src/removal_manifest/validate.rs"));
    let generate_state = state_source(root);
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);
    let architecture_flat = architecture.replace("\n  ", " ");

    assert!(
        config_model.contains("pub struct SlimConfig {"),
        "config/model.rs should define the direct [slim] removal-intent model"
    );
    let slim_config = section_between(&config_model, "pub struct SlimConfig", "impl SlimConfig");
    for required in [
        "pub remove_paths: Vec<String>,",
        "pub remove_configs: Vec<String>,",
        "pub set_defaults: BTreeMap<String, String>,",
        "pub unsafe_allow_root_path_removal: bool,",
    ] {
        assert!(
            slim_config.contains(required),
            "SlimConfig should own direct user-removal field {required}"
        );
    }

    for forbidden in [
        "KslimConfig",
        "ProfileConfig",
        "ReducerConfig",
        "SelfTestConfig",
        "PatchConfig",
        "IntegrationsConfig",
        "FeatureResolutionState",
        "PrunePlan",
        "RemovalManifest",
        "CandidateTreeState",
        "PublishedSnapshotState",
        "LockfilePath",
    ] {
        assert!(
            !slim_config.contains(forbidden),
            "SlimConfig must stay raw direct removal intent, not derived policy/state {forbidden}"
        );
    }

    assert!(
        config_model.contains("pub slim: Option<SlimConfig>,")
            && config_model.contains("pub fn removal_input(&self) -> Option<&SlimConfig>")
            && config_model.contains("self.slim.as_ref()"),
        "ProfileConfig should carry SlimConfig as direct user-facing removal input"
    );
    assert!(
        config_model.contains("pub fn is_noop(&self) -> bool")
            && config_model.contains("self.remove_paths.is_empty()")
            && config_model.contains("self.remove_configs.is_empty()")
            && config_model.contains("self.set_defaults.is_empty()"),
        "SlimConfig should expose effective no-op detection for removal intent"
    );
    assert!(
        config_validate.contains("RemovalManifest::from_slim_config_with_abi_policy")
            && config_validate.contains("profile.effective_removal_input()")
            && config_validate.contains(
                "reducer settings may only be customized when [slim] or [features.remove] declares removal input"
            ),
        "config validation should validate [slim] through the normalized removal manifest"
    );
    assert!(
        removal_parse.contains("SlimConfig")
            && removal_parse.contains("from_slim_config_with_abi_policy")
            && removal_parse.contains("slim.remove_paths")
            && removal_parse.contains("slim.remove_configs")
            && removal_parse.contains("slim.set_defaults")
            && removal_parse.contains("slim.unsafe_allow_root_path_removal")
            && removal_parse.contains("RemovalReason::SlimRemovePath")
            && removal_parse.contains("RemovalReason::SlimRemoveConfig")
            && removal_parse.contains("RemovalReason::SlimDefaultOverride"),
        "removal_manifest should derive normalized reducer truth from SlimConfig"
    );
    assert!(
        removal_validate.contains("slim.remove_paths must not contain empty values")
            && removal_validate.contains("declared removal paths must be relative")
            && removal_validate.contains("declared removal paths must not contain '..'")
            && removal_validate.contains("slim.unsafe_allow_root_path_removal = true"),
        "removal_manifest validation should fail closed for unsafe [slim] paths"
    );
    assert!(
        removal_parse.contains("slim.remove_configs must not contain empty values")
            && removal_parse.contains("invalid Kconfig symbol")
            && removal_parse.contains("slim.set_defaults must not contain empty symbols or values")
            && removal_parse.contains("both target"),
        "removal_manifest should validate [slim] Kconfig symbols and default overrides"
    );
    assert!(
        generate_state.contains("FeatureResolutionState::from_profile(profile)")
            && generate_state.contains("let direct_slim_input = profile.removal_input()")
            && generate_state.contains("let removal_input = profile.effective_removal_input()")
            && generate_state.contains("FeatureResolutionSource::DirectSlim")
            && generate_state.contains("from_slim_config_with_abi_policy_and_preservation"),
        "generate state should resolve SlimConfig into FeatureResolutionState before planning"
    );
    assert!(
        architecture_flat.contains("`SlimConfig` is the direct `[slim]` profile removal-intent model")
            && architecture_flat.contains(
                "removal paths, Kconfig symbols, default overrides, and the explicit unsafe tree-root removal opt-in",
            )
            && architecture_flat.contains(
                "`removal_manifest/*` turns it into normalized reducer truth",
            ),
        "architecture docs should describe SlimConfig ownership"
    );
    assert!(
        kernel_build_guide.contains("[slim]")
            && kernel_build_guide.contains("remove_paths")
            && kernel_build_guide.contains("remove_configs")
            && kernel_build_guide.contains("set_defaults")
            && kernel_build_guide.contains("unsafe_allow_root_path_removal"),
        "kernel build iteration docs should document the [slim] profile shape"
    );
}
