use super::common::*;

#[test]
fn kconfig_rewrite_report_is_emitted_as_reducer_artifact() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_model = production_source(&root.join("src/reducer/report/model.rs"));
    let report_render = production_source(&root.join("src/reducer/report/render.rs"));
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
    let output_report = production_source(&root.join("src/output_repo/report.rs"));
    let output_repo = production_source(&root.join("src/output_repo.rs"));
    let output_report_writer = production_source(&root.join("src/output_repo/report_writer.rs"));
    let candidate_metadata = production_source(&root.join("src/generate/candidate/metadata.rs"));
    let verify_report = production_source(&root.join("src/generate/verify/report.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let iteration = kernel_build_iteration_docs(&root);

    for required in [
        "kconfig_rewrite_report_json",
        "render_kconfig_rewrite_report_json",
        "config_block_removal_count",
        "default_override_count",
        "relation_rewrite_count",
        "source_removal_count",
        "dead_symbol_definition_removal_count",
        "empty_menu_removal_count",
        "skipped_expression_count",
        "unsupported_kconfig_expressions",
        "edit_record_details",
    ] {
        let rendered = report_json.contains(required)
            || report_model.contains(required)
            || report_render.contains(required);
        assert!(
            rendered,
            "reducer report modules should render Kconfig rewrite report item {required}"
        );
    }

    for required in [
        "REDUCER_KCONFIG_REWRITE_REPORT_JSON",
        "kconfig-rewrite-report.json",
        "CANDIDATE_REPORT_FILES",
        "COMMITTED_REPORT_FILES",
    ] {
        assert!(
            output_report.contains(required),
            "output_repo/report.rs should own rewrite report artifact item {required}"
        );
    }
    assert!(
        output_repo.contains("REDUCER_KCONFIG_REWRITE_REPORT_JSON")
            && output_report_writer.contains("REDUCER_KCONFIG_REWRITE_REPORT_JSON"),
        "output_repo.rs should re-export and output_repo/report_writer.rs should write the rewrite report artifact"
    );
    assert!(
        candidate_metadata.contains("REDUCER_KCONFIG_REWRITE_REPORT_JSON"),
        "candidate attempt metadata should collect the rewrite report artifact"
    );
    assert!(
        verify_report.contains("kconfig_rewrite_report_json")
            && verify_report.contains("read_kconfig_rewrite_report"),
        "candidate verification should check the rewrite report artifact"
    );

    let docs_phrase = "Kconfig rewrite reports are emitted as `kconfig-rewrite-report.json`";
    assert!(architecture.contains(docs_phrase));
    assert!(iteration.contains(docs_phrase));
}
