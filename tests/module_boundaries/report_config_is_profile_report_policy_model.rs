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
fn report_config_is_profile_report_policy_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_model = production_source(&root.join("src/config/model.rs"));
    let config_validate = production_source(&root.join("src/config/validate.rs"));
    let config_templates = production_source(&root.join("src/config/templates.rs"));
    let generate_state = state_source(root);
    let output_report = production_source(&root.join("src/output_repo/report.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);
    let architecture_flat = architecture.replace("\n  ", " ");

    assert!(
        config_model.contains("pub struct ReportConfig {"),
        "config/model.rs should define the [reports] report-policy model"
    );
    let profile_config = section_between(
        &config_model,
        "pub struct ProfileConfig",
        "impl ProfileConfig",
    );
    assert!(
        profile_config.contains("pub reports: ReportConfig,"),
        "ProfileConfig should carry ReportConfig selected by profile input"
    );

    let report_config = section_between(
        &config_model,
        "pub struct ReportConfig",
        "impl Default for ReportConfig",
    );
    for required in [
        "pub formats: Vec<String>,",
        "pub include_edit_records: bool,",
        "pub include_diagnostics: bool,",
        "pub include_source_map: bool,",
        "pub redact_host_paths: bool,",
        "pub include_raw_logs: bool,",
        "pub fail_on_error: bool,",
    ] {
        assert!(
            report_config.contains(required),
            "ReportConfig should own raw report policy field {required}"
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
        "SecurityConfig",
        "PerformanceConfig",
        "ReducerConfig",
        "SelfTestConfig",
        "FeatureResolutionState",
        "ReducerStats",
        "ReducerReportArtifacts",
        "GenerateReport",
        "PublishedSnapshotState",
        "CandidateMetadataDir",
        "PublishedMetadataDir",
        "LockfilePath",
    ] {
        assert!(
            !report_config.contains(forbidden),
            "ReportConfig must stay raw profile report policy, not concrete report/state {forbidden}"
        );
    }

    assert!(
        config_model.contains("fn default_report_formats() -> Vec<String>")
            && config_model.contains("[\"text\", \"markdown\", \"json\"]")
            && config_model.contains("include_edit_records: true")
            && config_model.contains("include_diagnostics: true")
            && config_model.contains("include_source_map: false")
            && config_model.contains("redact_host_paths: true")
            && config_model.contains("include_raw_logs: false")
            && config_model.contains("fail_on_error: true")
            && config_model.contains("pub fn is_default(&self) -> bool"),
        "ReportConfig defaults should match current fixed committed-report behavior and be detectable"
    );
    assert!(
        config_validate.contains("const REPORT_FORMATS")
            && config_validate.contains("fn validate_report_config(config: &ReportConfig) -> Result<()>")
            && config_validate.contains("validate_nonempty_unique_string_list(\"reports.formats\"")
            && config_validate.contains("reports.formats contains unsupported format")
            && config_validate.contains("reports.include_raw_logs is not supported")
            && config_validate.contains("report config is parsed but not yet supported")
            && config_validate.contains("validate_report_config(&profile.reports)?"),
        "profile validation should validate report fields and fail closed for nondefault ReportConfig"
    );
    assert!(
        config_templates.contains("reports: ReportConfig::default()")
            && config_templates.contains("[reports]")
            && config_templates.contains("formats = [\"text\", \"markdown\", \"json\"]")
            && config_templates
                .contains("Committed report artifacts and redaction policy are fixed"),
        "profile templates should show ReportConfig as future fail-closed policy"
    );
    assert!(
        !generate_state.contains("profile.reports")
            && !output_report.contains("ReportConfig"),
        "generate/output report code must not silently consume ReportConfig before report planning lands"
    );
    assert!(
        architecture_flat.contains("`ReportConfig` is the `[reports]` profile report-policy model")
            && architecture_flat.contains("formats, edit-record inclusion, diagnostics inclusion")
            && architecture_flat.contains("fails closed until report planning lands"),
        "architecture docs should describe ReportConfig ownership and fail-closed behavior"
    );
    assert!(
        kernel_build_guide.contains("[reports]")
            && kernel_build_guide.contains("report planning lands")
            && kernel_build_guide
                .contains("Committed report artifacts and redaction policy are fixed"),
        "kernel build iteration docs should explain ReportConfig is not active yet"
    );
}
