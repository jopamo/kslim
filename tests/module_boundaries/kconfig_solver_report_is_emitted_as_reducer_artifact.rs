use super::common::*;

#[test]
fn kconfig_solver_report_is_emitted_as_reducer_artifact() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let solver_report = production_source(&root.join("src/kconfig/report.rs"));
    let prune = production_sources(
        &root,
        &[
            "src/prune.rs",
            "src/prune/semantic.rs",
            "src/prune/report.rs",
        ],
    );
    let result = production_source(&root.join("src/reducer/result.rs"));
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

    assert!(
        kconfig.contains("mod report;"),
        "src/kconfig/mod.rs should register the solver report slice"
    );
    for required in [
        "pub(crate) struct KconfigSolverReport",
        "pub(crate) fn kconfig_solver_report",
        "read_kconfig_selected_profile_values",
        "detect_kconfig_symbols_reenabled_by_defaults",
        "detect_kconfig_removed_symbols_forced_by_select",
        "detect_kconfig_removed_symbols_weakly_enabled_by_imply",
        "detect_kconfig_impossible_choices",
        "detect_kconfig_empty_menus",
        "detect_kconfig_orphaned_symbol_definitions",
        "prove_dead_kconfig_symbol_definitions",
    ] {
        assert!(
            solver_report.contains(required),
            "report.rs should collect Kconfig solver report item {required}"
        );
    }

    for required in [
        "kconfig_solver_report: KconfigSolverReport",
        "read_kconfig_selected_profile_values(root)",
        "kconfig_solver_report(",
    ] {
        assert!(
            prune.contains(required),
            "prune.rs should populate solver report field {required}"
        );
    }
    assert!(
        result.contains("pub kconfig_solver_report: KconfigSolverReport"),
        "ReducerStats should carry the Kconfig solver report"
    );

    for required in [
        "kconfig_solver_report_json",
        "render_kconfig_solver_report_json",
        "default_reenabled_symbols",
        "forced_selects",
        "weak_implies",
        "impossible_choices",
        "empty_menus",
        "orphaned_symbol_definitions",
        "dead_symbol_definition_proofs",
        "skipped_files",
    ] {
        let rendered = report_json.contains(required)
            || report_model.contains(required)
            || report_render.contains(required);
        assert!(
            rendered,
            "reducer report modules should render solver report item {required}"
        );
    }

    for required in [
        "REDUCER_KCONFIG_SOLVER_REPORT_JSON",
        "kconfig-solver-report.json",
        "CANDIDATE_REPORT_FILES",
        "COMMITTED_REPORT_FILES",
    ] {
        assert!(
            output_report.contains(required),
            "output_repo/report.rs should own solver report artifact item {required}"
        );
    }
    assert!(
        output_repo.contains("REDUCER_KCONFIG_SOLVER_REPORT_JSON")
            && output_report_writer.contains("REDUCER_KCONFIG_SOLVER_REPORT_JSON"),
        "output_repo.rs should re-export and output_repo/report_writer.rs should write the solver report artifact"
    );
    assert!(
        candidate_metadata.contains("REDUCER_KCONFIG_SOLVER_REPORT_JSON"),
        "candidate attempt metadata should collect the solver report artifact"
    );
    assert!(
        verify_report.contains("kconfig_solver_report_json")
            && verify_report.contains("read_kconfig_solver_report"),
        "candidate verification should check the solver report artifact"
    );

    let docs_phrase = "Kconfig solver reports are emitted as `kconfig-solver-report.json`";
    assert!(architecture.contains(docs_phrase));
    assert!(iteration.contains(docs_phrase));
}
