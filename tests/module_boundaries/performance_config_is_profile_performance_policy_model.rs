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
fn performance_config_is_profile_performance_policy_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_model = production_source(&root.join("src/config/model.rs"));
    let config_validate = production_source(&root.join("src/config/validate.rs"));
    let config_templates = production_source(&root.join("src/config/templates.rs"));
    let generate_state = state_source(root);
    let reducer_engine = production_source(&root.join("src/reducer/engine.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);
    let architecture_flat = architecture.replace("\n  ", " ");

    assert!(
        config_model.contains("pub struct PerformanceConfig {"),
        "config/model.rs should define the [performance] hot-path work-shape policy model"
    );
    let profile_config = section_between(
        &config_model,
        "pub struct ProfileConfig",
        "impl ProfileConfig",
    );
    assert!(
        profile_config.contains("pub performance: PerformanceConfig,"),
        "ProfileConfig should carry PerformanceConfig selected by profile input"
    );

    let performance_config = section_between(
        &config_model,
        "pub struct PerformanceConfig",
        "impl Default for PerformanceConfig",
    );
    for required in [
        "pub enabled: bool,",
        "pub max_worker_threads: Option<usize>,",
        "pub max_io_threads: Option<usize>,",
        "pub cache_tree_index: bool,",
        "pub incremental_reindex: bool,",
        "pub collect_timing_metrics: bool,",
        "pub profile_hot_paths: bool,",
        "pub fail_on_regression: bool,",
    ] {
        assert!(
            performance_config.contains(required),
            "PerformanceConfig should own raw performance policy field {required}"
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
        "SecurityConfig",
        "ReducerConfig",
        "SelfTestConfig",
        "FeatureResolutionState",
        "ReducerStats",
        "TreeIndex",
        "ReducerContext",
        "CandidateTreeState",
        "PublishedSnapshotState",
        "LockfilePath",
    ] {
        assert!(
            !performance_config.contains(forbidden),
            "PerformanceConfig must stay raw profile policy, not hot-path plan/state {forbidden}"
        );
    }

    assert!(
        config_model.contains("enabled: false")
            && config_model.contains("max_worker_threads: None")
            && config_model.contains("max_io_threads: None")
            && config_model.contains("cache_tree_index: false")
            && config_model.contains("incremental_reindex: false")
            && config_model.contains("collect_timing_metrics: false")
            && config_model.contains("profile_hot_paths: false")
            && config_model.contains("fail_on_regression: true")
            && config_model.contains("pub fn is_default(&self) -> bool"),
        "PerformanceConfig defaults should be inactive, fail-on-regression, and detectable"
    );
    assert!(
        config_validate
            .contains("fn validate_performance_config(config: &PerformanceConfig) -> Result<()>")
            && config_validate
                .contains("performance.max_worker_threads must be greater than zero")
            && config_validate.contains("performance.max_io_threads must be greater than zero")
            && config_validate.contains("performance.fail_on_regression cannot be disabled")
            && config_validate.contains("performance config is parsed but not yet supported")
            && config_validate.contains("validate_performance_config(&profile.performance)?"),
        "profile validation should validate performance fields and fail closed for nondefault PerformanceConfig"
    );
    assert!(
        config_templates.contains("performance: PerformanceConfig::default()")
            && config_templates.contains("[performance]")
            && config_templates.contains("max_worker_threads = 16")
            && config_templates.contains("Hot-path work shape is fixed"),
        "profile templates should show PerformanceConfig as future fail-closed policy"
    );
    assert!(
        !generate_state.contains("profile.performance")
            && !reducer_engine.contains("profile.performance"),
        "generate/reducer hot paths must not silently consume PerformanceConfig before performance planning lands"
    );
    assert!(
        architecture_flat.contains(
            "`PerformanceConfig` is the `[performance]` profile hot-path work-shape policy model"
        ) && architecture_flat.contains("worker and I/O thread caps, tree-index cache")
            && architecture_flat.contains("fails closed until performance planning lands"),
        "architecture docs should describe PerformanceConfig ownership and fail-closed behavior"
    );
    assert!(
        kernel_build_guide.contains("[performance]")
            && kernel_build_guide.contains("performance planning lands")
            && kernel_build_guide.contains("Hot-path work shape is fixed"),
        "kernel build iteration docs should explain PerformanceConfig is not active yet"
    );
}
