use super::common::*;

#[test]
fn output_repo_report_module_owns_report_names_paths_authority_and_copy() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output_repo = production_source(&root.join("src/output_repo.rs"));
    let report = production_source(&root.join("src/output_repo/report.rs"));
    let generate = production_source(&root.join("src/generate.rs"));
    let generate_failure = production_source(&root.join("src/generate/failure.rs"));
    let generate_publish = production_source(&root.join("src/generate/publish.rs"));
    let cli = cli_sources(root);
    let commands = commands_source(root);

    assert!(
        output_repo.contains("mod report;"),
        "output_repo.rs should register the report naming/path module"
    );

    for required in [
        "pub const REDUCER_REMOVAL_MANIFEST: &str = \"removal-manifest.toml\"",
        "pub const REDUCER_REPORT_JSON: &str = \"reducer-report.json\"",
        "pub const REDUCER_DIAGNOSTICS_JSON: &str = \"diagnostics.json\"",
        "pub const REDUCER_KCONFIG_SOLVER_REPORT_JSON: &str = \"kconfig-solver-report.json\"",
        "pub const REDUCER_KCONFIG_REWRITE_REPORT_JSON: &str = \"kconfig-rewrite-report.json\"",
        "pub const REDUCER_SKIPPED_SITES_JSON: &str = \"skipped-sites.json\"",
        "pub const MATRIX_REPORT_JSON: &str = \"matrix-report.json\"",
        "pub const GENERATE_REPORT_JSON: &str = \"generate-report.json\"",
        "pub const LAST_ATTEMPT_JSON: &str = \"last-attempt.json\"",
        "pub const NON_AUTHORITATIVE_ATTEMPT_SCOPE: &str = \"non-authoritative-attempt\"",
        "pub(crate) fn output_report_path",
        "output_repo: &OutputRepoPath",
        "pub(crate) fn metadata_report_path",
        "pub(crate) fn validate_report_file_name",
        "fn validate_published_report_file_name",
        "artifact_name == LAST_ATTEMPT_JSON",
        "pub(crate) fn attempt_last_attempt_report_path",
        "metadata_dir: &AttemptMetadataDir",
        "pub(crate) fn validate_last_attempt_json",
        "authoritative_lockfile",
        "const CANDIDATE_REPORT_FILES",
        "const COMMITTED_REPORT_FILES",
        "pub(crate) struct CandidateReportCopySummary",
        "pub(crate) fn copy_candidate_reports_to_output_candidate_metadata",
        "std::fs::copy",
        "pub(crate) fn validate_candidate_committed_reports_temporary_paths",
        "metadata::validate_committed_metadata_named_files_have_no_temporary_paths",
        "\"committed report\"",
    ] {
        assert!(
            report.contains(required),
            "output_repo/report.rs should own report naming/path/authority item {required}"
        );
    }

    for moved_item in [
        "pub const REDUCER_REMOVAL_MANIFEST",
        "pub const REDUCER_REPORT_JSON",
        "pub const REDUCER_DIAGNOSTICS_JSON",
        "pub const REDUCER_KCONFIG_SOLVER_REPORT_JSON",
        "pub const REDUCER_KCONFIG_REWRITE_REPORT_JSON",
        "pub const REDUCER_SKIPPED_SITES_JSON",
        "pub const REDUCER_FAILURE_JSON",
        "fn validate_reducer_artifact_name",
    ] {
        assert!(
            !output_repo.contains(moved_item),
            "output_repo.rs should not retain report naming/path implementation {moved_item}"
        );
    }

    assert!(
        !generate.contains("const LAST_ATTEMPT_FILE"),
        "generate.rs should use output_repo/report.rs for last-attempt naming"
    );
    assert!(
        !cli.contains("const LAST_ATTEMPT_FILE"),
        "src/cli/* should use output_repo/report.rs for last-attempt naming"
    );
    assert!(
        commands.contains("output_repo::LAST_ATTEMPT_JSON"),
        "src/commands/* should use output_repo/report.rs for last-attempt naming"
    );
    assert!(
        generate_failure.contains("output_repo::attempt_last_attempt_report_path"),
        "generate/failure.rs should construct last-attempt writes through output_repo/report.rs"
    );
    assert!(
        generate_failure.contains("output_repo::validate_last_attempt_json"),
        "generate/failure.rs should validate last-attempt metadata before writing it"
    );
    assert!(
        generate_publish
            .contains("output_repo::copy_candidate_reports_to_output_candidate_metadata"),
        "generate/publish.rs should copy candidate report artifacts through output_repo/report.rs"
    );

    let forbidden_rendering_or_publication = [
        "ReducerStats",
        "SelfTestResult",
        "ClassifiedDiagnostic",
        "EditRecord",
        "KslimConfig",
        "ProfileConfig",
        "std::fs::write",
        "std::fs::remove",
        "render_reducer",
        "write_reducer_metadata",
        "publish_output_candidate",
        "load_authoritative_published_state",
        "crate::generate",
    ];

    for forbidden in forbidden_rendering_or_publication {
        assert!(
            !report.contains(forbidden),
            "output_repo/report.rs must stay limited to report names, paths, authority checks, and report copying; found forbidden token {forbidden}"
        );
    }
}
