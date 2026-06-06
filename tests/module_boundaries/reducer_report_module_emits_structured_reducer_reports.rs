use super::common::*;

#[test]
fn reducer_report_module_emits_structured_reducer_reports() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let reducer = production_source(&root.join("src/reducer/mod.rs"));
    let report = production_source(&root.join("src/reducer/report.rs"));
    let report_model = production_source(&root.join("src/reducer/report/model.rs"));
    let report_render = production_source(&root.join("src/reducer/report/render.rs"));
    let report_json_facade = std::fs::read_to_string(root.join("src/reducer/report/json.rs"))
        .expect("failed to read src/reducer/report/json.rs");
    let report_json = production_sources(
        &root,
        &[
            "src/reducer/report/json.rs",
            "src/reducer/report/json/schema.rs",
            "src/reducer/report/json/serializer.rs",
            "src/reducer/report/json/escaping.rs",
            "src/reducer/report/json/canonical.rs",
        ],
    );
    let report_text = production_source(&root.join("src/reducer/report/text.rs"));
    let report_summary = production_source(&root.join("src/reducer/report/summary.rs"));
    let output_repo = production_source(&root.join("src/output_repo.rs"));

    assert!(
        reducer.contains("mod report;"),
        "reducer/mod.rs should register the reducer report module"
    );

    for required in [
        "mod model;",
        "mod render;",
        "mod json;",
        "mod text;",
        "mod summary;",
    ] {
        assert!(
            report.contains(required),
            "reducer/report.rs should register reducer report split module {required}"
        );
    }

    for required in [
        "REDUCER_REPORT_SCHEMA_VERSION",
        "pub(crate) struct ReducerReportArtifactNames",
        "pub(crate) struct RenderedReducerReportArtifacts",
        "pub(crate) struct ReducerFailureReport",
        "pub(super) struct KbuildEditReport",
        "pub(crate) struct UnknownDiagnosticPolicy",
        "pub(crate) struct UnsupportedSyntaxPolicy",
    ] {
        assert!(
            report_model.contains(required),
            "reducer/report/model.rs should define reducer report model item {required}"
        );
    }

    for required in [
        "render_reducer_stats_report_artifacts",
        "render_reducer_result_report_artifacts",
        "validate_report_edit_records",
        "ensure_supported_fallout",
        "text::render_reducer_report_md",
        "json::render_reducer_report_json",
        "json::render_reducer_diagnostics_json",
        "json::render_reducer_edit_summary_json",
        "json::render_kconfig_solver_report_json",
        "json::render_kconfig_rewrite_report_json",
        "json::render_reducer_skipped_sites_json",
    ] {
        assert!(
            report_render.contains(required),
            "reducer/report/render.rs should define report orchestration item {required}"
        );
    }

    for required in [
        "render_reducer_report_md",
        "## Reducer config",
        "## Normalized removal manifest",
        "## Kconfig reducer report",
        "## Kbuild reducer report",
        "## Preprocessor reducer report",
        "## Include reducer report",
        "## Deterministic fixups",
    ] {
        assert!(
            report_text.contains(required),
            "reducer/report/text.rs should define reducer markdown report item {required}"
        );
    }

    for required in [
        "render_reducer_report_json",
        "mod schema;",
        "mod serializer;",
        "mod escaping;",
        "mod canonical;",
        "REDUCER_REPORT_SCHEMA_VERSION",
        "json_escape",
        "json_compact",
        "render_edit_proof_source_count_entries",
        "schema_version",
        "reducer_config",
        "normalized_removal_manifest",
        "render_per_pass_edit_counts_json",
        "render_per_file_edit_records_json",
        "render_reducer_diagnostics_json",
        "render_diagnostic_log_summaries_by_command_json",
        "diagnostic_log_summaries_by_command",
        "classified_diagnostics",
        "unknown_diagnostics",
        "consumed_diagnostics",
        "skipped_diagnostics",
        "render_reducer_edit_summary_json",
        "render_kconfig_solver_report_json",
        "kconfig_solver_report_json",
        "render_kconfig_rewrite_report_json",
        "kconfig_rewrite_report_json",
        "render_reducer_skipped_sites_json",
        "render_manual_include_sites_json",
        "ambiguous_include_sites",
        "unsupported_kconfig_expressions",
        "unsupported_cpp_forms",
        "ambiguous_makefile_lines",
        "live_missing_includes",
        "render_fixup_summary_json",
        "skipped_sites",
        "matrix_status",
        "convergence_status",
        "final_status",
    ] {
        assert!(
            report_json.contains(required),
            "reducer/report/json.rs should define structured reducer JSON item {required}"
        );
    }
    assert!(
        report_json_facade.contains("#[cfg(test)]\nmod tests;"),
        "reducer/report/json.rs should register JSON unit tests as an external module"
    );

    for required in [
        "has_reducer_diagnostics",
        "render_kbuild_edit_report",
        "count_edit_proof_sources",
        "classified_diagnostics_from_stats",
        "render_classified_diagnostic_md",
        "render_fixup_proof_md",
        "committed_path_string",
        "render_unsupported_expression_report",
    ] {
        assert!(
            report_summary.contains(required),
            "reducer/report/summary.rs should define shared reducer report summary item {required}"
        );
    }

    for moved_rendering in [
        "fn render_reducer_report_md",
        "fn render_reducer_report_json",
        "fn render_reducer_diagnostics_json",
        "fn render_reducer_edit_summary_json",
        "fn render_edit_records_json",
    ] {
        assert!(
            !output_repo.contains(moved_rendering),
            "output_repo.rs must not own reducer report rendering after reducer/report.rs owns it; found {moved_rendering}"
        );
    }
}
