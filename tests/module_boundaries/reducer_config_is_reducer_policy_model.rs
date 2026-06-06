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
fn reducer_config_is_reducer_policy_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_model = production_source(&root.join("src/config/model.rs"));
    let config_validate = production_source(&root.join("src/config/validate.rs"));
    let config_templates = production_source(&root.join("src/config/templates.rs"));
    let generate_state = state_source(root);
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);
    let architecture_flat = architecture.replace("\n  ", " ");

    assert!(
        config_model.contains("pub struct ReducerConfig {"),
        "config/model.rs should define the [reducer] policy model"
    );
    let reducer_config = section_between(
        &config_model,
        "pub struct ReducerConfig",
        "impl Default for ReducerConfig",
    );
    for required in [
        "pub max_fixup_passes: usize,",
        "pub report_unsupported_expressions: bool,",
        "pub fail_on_unknown_diagnostics: bool,",
        "pub reject_unproven_fixups: bool,",
        "pub reject_unreasoned_edits: bool,",
        "pub reject_speculative_fallout_edits: bool,",
        "pub fail_on_missing_prune_paths: bool,",
        "pub ignore_unsupported_special_removals: bool,",
    ] {
        assert!(
            reducer_config.contains(required),
            "ReducerConfig should own reducer policy field {required}"
        );
    }

    for forbidden in [
        "KslimConfig",
        "ProfileConfig",
        "SlimConfig",
        "AbiPolicyConfig",
        "SelfTestConfig",
        "PatchConfig",
        "IntegrationsConfig",
        "FeatureResolutionState",
        "CandidateTreeState",
        "PublishedSnapshotState",
        "LockfilePath",
    ] {
        assert!(
            !reducer_config.contains(forbidden),
            "ReducerConfig must stay reducer policy, not config/lifecycle state {forbidden}"
        );
    }

    let reducer_impl = section_between(
        &config_model,
        "impl Default for ReducerConfig",
        "#[derive(Debug, Clone, Serialize, Deserialize)]\n#[serde(untagged)]",
    );
    assert!(
        reducer_impl.contains("max_fixup_passes: default_max_fixup_passes()")
            && reducer_impl.contains("report_unsupported_expressions: true")
            && reducer_impl.contains("fail_on_unknown_diagnostics: true")
            && reducer_impl.contains("reject_unproven_fixups: true")
            && reducer_impl.contains("reject_unreasoned_edits: true")
            && reducer_impl.contains("reject_speculative_fallout_edits: true")
            && reducer_impl.contains("fail_on_missing_prune_paths: false")
            && reducer_impl.contains("ignore_unsupported_special_removals: false"),
        "ReducerConfig defaults should be strict and deterministic"
    );

    let strict_mode = section_between(
        &config_model,
        "pub fn strict_mode(&self) -> bool",
        "#[derive(Debug, Clone, Serialize, Deserialize)]\n#[serde(untagged)]",
    );
    for required in [
        "self.report_unsupported_expressions",
        "self.fail_on_unknown_diagnostics",
        "self.reject_unproven_fixups",
        "self.reject_unreasoned_edits",
        "self.reject_speculative_fallout_edits",
    ] {
        assert!(
            strict_mode.contains(required),
            "strict_mode should require publish-safety gate {required}"
        );
    }
    for forbidden in [
        "max_fixup_passes",
        "fail_on_missing_prune_paths",
        "ignore_unsupported_special_removals",
    ] {
        assert!(
            !strict_mode.contains(forbidden),
            "strict_mode should not depend on non-strict reducer knob {forbidden}"
        );
    }

    assert!(
        config_model.contains("#[serde(default)]\n    pub reducer: ReducerConfig,")
            && config_templates.contains("reducer: ReducerConfig::default()"),
        "ProfileConfig/default_profile_config should carry ReducerConfig with defaults"
    );
    assert!(
        config_validate.contains("fn is_default_reducer_config(config: &ReducerConfig) -> bool")
            && config_validate.contains("config == &ReducerConfig::default()")
            && config_validate.contains("let reducer_has_effective_input")
            && config_validate.contains("profile.effective_removal_input()")
            && config_validate.contains(
                "reducer settings may only be customized when [slim] or [features.remove] declares removal input"
            ),
        "config validation should allow custom reducer policy only with effective removal input"
    );
    assert!(
        generate_state.contains("pub(crate) struct ReducerPlan")
            && generate_state.contains("fn from_config(config: &ReducerConfig) -> Self")
            && generate_state.contains("max_fixup_passes: config.max_fixup_passes")
            && generate_state
                .contains("report_unsupported_expressions: config.report_unsupported_expressions")
            && generate_state
                .contains("fail_on_unknown_diagnostics: config.fail_on_unknown_diagnostics")
            && generate_state.contains("reject_unproven_fixups: config.reject_unproven_fixups")
            && generate_state.contains("reject_unreasoned_edits: config.reject_unreasoned_edits")
            && generate_state.contains(
                "reject_speculative_fallout_edits: config.reject_speculative_fallout_edits"
            )
            && generate_state
                .contains("fail_on_missing_prune_paths: config.fail_on_missing_prune_paths")
            && generate_state.contains(
                "ignore_unsupported_special_removals: config.ignore_unsupported_special_removals"
            ),
        "generate state should copy ReducerConfig into a resolved ReducerPlan"
    );

    for required in [
        "# [reducer]",
        "# max_fixup_passes = 3",
        "# report_unsupported_expressions = true",
        "# fail_on_unknown_diagnostics = true",
        "# reject_unproven_fixups = true",
        "# reject_unreasoned_edits = true",
        "# reject_speculative_fallout_edits = true",
        "# fail_on_missing_prune_paths = false",
        "# ignore_unsupported_special_removals = false",
    ] {
        assert!(
            config_templates.contains(required),
            "profile template should document ReducerConfig field {required}"
        );
    }
    assert!(
        architecture_flat.contains("`ReducerConfig` is the `[reducer]` profile policy model")
            && architecture_flat.contains("fixed-point fixup limits")
            && architecture_flat.contains("strict-mode gates")
            && architecture_flat.contains(
                "custom reducer settings require effective `[slim]` or `[features.remove]` input"
            ),
        "architecture docs should describe ReducerConfig ownership"
    );
    assert!(
        kernel_build_guide.contains("[reducer]")
            && kernel_build_guide.contains("max_fixup_passes = 3")
            && kernel_build_guide.contains("reject_unproven_fixups = true")
            && kernel_build_guide.contains("reject_speculative_fallout_edits = true"),
        "kernel build iteration docs should show ReducerConfig profile shape"
    );
}
