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
fn output_config_is_project_output_repo_intent_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_model = production_source(&root.join("src/config/model.rs"));
    let config_validate = production_source(&root.join("src/config/validate.rs"));
    let config_templates = production_source(&root.join("src/config/templates.rs"));
    let generate_state = state_source(root);
    let output_naming = production_source(&root.join("src/output_repo/naming.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let architecture_flat = architecture.replace("\n  ", " ");

    assert!(
        config_model.contains("pub struct OutputConfig {"),
        "config/model.rs should define the [output] project-root model"
    );

    let kslim_config = section_between(
        &config_model,
        "pub struct KslimConfig",
        "pub struct ProjectConfig",
    );
    assert!(
        kslim_config.contains("pub output: OutputConfig,"),
        "KslimConfig should carry OutputConfig selected by project-root input"
    );

    let output_config = section_between(
        &config_model,
        "pub struct OutputConfig",
        "impl OutputConfig",
    );
    for required in [
        "pub path: String,",
        "pub branch_prefix: String,",
        "pub branch: Option<String>,",
    ] {
        assert!(
            output_config.contains(required),
            "OutputConfig should own raw output policy field {required}"
        );
    }

    for forbidden in [
        "ProfileConfig",
        "SlimConfig",
        "FeatureConfig",
        "AbiPolicyConfig",
        "ReportConfig",
        "SecurityConfig",
        "PerformanceConfig",
        "ReducerConfig",
        "SelfTestConfig",
        "OutputPlan",
        "OutputRepoPath",
        "PublishedSnapshotState",
        "CandidateTreeState",
        "LockfilePath",
        "PublishedMetadataDir",
    ] {
        assert!(
            !output_config.contains(forbidden),
            "OutputConfig must stay requested project-root output intent, not profile/plan/state {forbidden}"
        );
    }

    assert!(
        config_model.contains("pub fn new(path: impl Into<String>) -> Self")
            && config_model.contains("branch_prefix: default_branch_prefix()")
            && config_model.contains("pub fn has_explicit_branch(&self) -> bool"),
        "OutputConfig should provide default branch-prefix construction and explicit-branch detection"
    );
    assert!(
        config_templates.contains("output: OutputConfig::new(output_path)"),
        "default project config should construct OutputConfig through its defaulted model constructor"
    );
    assert!(
        config_validate.contains("fn validate_output_config(config: &OutputConfig) -> Result<()>")
            && config_validate.contains("OutputRepoPath::new(config.path.as_str())")
            && config_validate.contains("output.path must not be empty")
            && config_validate.contains("output.path is invalid")
            && config_validate.contains("output.branch_prefix must not be empty")
            && config_validate.contains("output.branch must not be empty when specified")
            && config_validate.contains("must not contain empty branch path components")
            && config_validate.contains("fn validate_output_branch_name")
            && config_validate.contains("validate_output_config(&config.output)?"),
        "config validation should validate OutputConfig and convert output.path through the typed output boundary"
    );
    assert!(
        output_naming.contains("config.output.branch")
            && output_naming.contains("config.output.branch_prefix")
            && output_naming.contains("pub(crate) fn initial_branch(config: &KslimConfig)")
            && generate_state.contains("output_path: OutputRepoPath")
            && generate_state.contains("branch_prefix: config.output.branch_prefix.clone()")
            && generate_state.contains("explicit_branch: config.output.branch.clone()"),
        "resolved output planning/naming should consume OutputConfig without storing raw project config as state"
    );
    assert!(
        architecture_flat
            .contains("`OutputConfig` is the project-root `[output]` repository-intent model")
            && architecture_flat.contains(
                "managed output path, generated branch prefix, and optional exact branch"
            )
            && architecture_flat.contains("not resolved output state"),
        "architecture docs should describe OutputConfig ownership"
    );
}
