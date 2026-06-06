use super::common::*;

#[test]
fn kconfig_rewrite_cleans_empty_menus_only_with_solver_proof() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(&root, &["src/kconfig/mod.rs", "src/kconfig/rewrite.rs"]);
    let kconfig_with_tests = production_sources(
        &root,
        &[
            "src/kconfig/tests.rs",
            "src/kconfig/tests_report.rs",
            "src/kconfig/tests_rewrite.rs",
            "src/kconfig/tests_root_facade.rs",
            "src/kconfig/tests_solver.rs",
        ],
    );
    let prune_path = root.join("src/prune.rs");
    let prune = production_sources(&root, &["src/prune.rs", "src/prune/semantic.rs"]);
    let prune_with_tests = std::fs::read_to_string(&prune_path).expect("failed to read prune.rs");
    let edit_reason = production_sources(
        &root,
        &[
            "src/edit_reason.rs",
            "src/edit_reason/reason.rs",
            "src/edit_reason/proof_source.rs",
            "src/edit_reason/render.rs",
        ],
    );
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
    let architecture = production_source(&root.join("docs/architecture.md"));
    let iteration = kernel_build_iteration_docs(&root);

    for required in [
        "pub(crate) struct KconfigEmptyMenuRemovalProof",
        "pub(crate) fn prove_empty_kconfig_menus",
        "detect_kconfig_empty_menus(",
        "pub(crate) fn rewrite_empty_kconfig_menus",
        "kconfig_empty_menu_removal_proof_matches_document",
        "kconfig_empty_menu_block_is_cleanup_only",
        "kconfig_wrapper_body_is_cleanup_safe",
        "# kslim: removed empty",
        "kconfig.rewrite_empty_menus",
    ] {
        assert!(
            kconfig.contains(required),
            "src/kconfig/mod.rs should clean empty menus only with solver cleanup proof; missing {required}"
        );
    }

    for required in [
        "RemovedEmptyKconfigMenu",
        "KconfigSolverKey::EmptyMenu",
        "kconfig_solver_empty_menu",
        "removed_empty_kconfig_menu",
        "solver=empty_menu",
        "kconfig.rewrite_empty_menus",
    ] {
        assert!(
            edit_reason.contains(required),
            "edit provenance should model solver-backed empty-menu cleanup; missing {required}"
        );
    }

    for required in [
        "test_rewrite_empty_kconfig_menus_requires_solver_cleanup_proof",
        "KconfigEmptyMenuRemovalProof",
        "rewrite_empty_kconfig_menus(root, &[])",
        "menu \\\"Not pruned yet\\\"",
        "config REMOVED_DRIVER",
        "# kslim: removed empty menu \\\"Dead drivers\\\"",
    ] {
        assert!(
            kconfig_with_tests.contains(required),
            "unit tests should pin proof-gated empty-menu cleanup; missing {required}"
        );
    }

    for required in [
        "rewrite_empty_kconfig_menus(root, &selected_profile_values, &removed_config_symbols)",
        "removed_empty_menus",
    ] {
        assert!(
            prune.contains(required),
            "prune stage should run and report empty-menu cleanup; missing {required}"
        );
    }
    assert!(prune_with_tests.contains(
        "test_rewrite_kconfig_stage_cleans_empty_menus_after_config_removal"
    ));

    assert!(report_json.contains("empty_menu_removal_count"));
    assert!(report_json.contains("removed_empty_menus"));
    assert!(report_json.contains("kconfig.rewrite_empty_menus"));
    assert!(report_text.contains("Removed empty menus"));

    for docs in [architecture, iteration] {
        assert!(
            docs.contains("Empty Kconfig menu cleanup is solver-proof-gated"),
            "docs should describe solver-proof-gated empty-menu cleanup"
        );
        assert!(
            docs.contains("cleanup-only menu") && docs.contains("shells"),
            "docs should say empty-menu cleanup removes only cleanup-only shells"
        );
    }
}
