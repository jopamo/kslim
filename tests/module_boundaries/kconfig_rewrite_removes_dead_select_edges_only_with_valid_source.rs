use super::common::*;

#[test]
fn kconfig_rewrite_removes_dead_select_edges_only_with_valid_source() {
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
    let architecture =
        std::fs::read_to_string(root.join("docs/architecture.md")).expect("failed to read docs");
    let kernel_build = kernel_build_iteration_docs(&root);

    for required in [
        "struct KconfigRelationSourceContext",
        "clear_kconfig_relation_source_context_for_line",
        "kconfig_relation_source_context_from_line",
        "kconfig_relation_source_remains_valid",
        "matches!(keyword, \"select\" | \"imply\")",
        "!kconfig_relation_source_remains_valid(source_symbol, removed)",
    ] {
        assert!(
            kconfig.contains(required),
            "src/kconfig/mod.rs should remove dead select edges only from valid source symbols; missing {required}"
        );
    }

    for required in [
        "test_rewrite_kconfig_relations_drops_removed_selects_and_implies_only_from_valid_sources",
        "\"\\tselect REMOVED\\n\"",
        "String::from(\"REMOVED_SOURCE\")",
        "config REMOVED_SOURCE",
    ] {
        assert!(
            kconfig_with_tests.contains(required),
            "unit tests should pin valid-source gating for dead select-edge removal; missing {required}"
        );
    }

    for docs in [architecture, kernel_build] {
        assert!(
            docs.contains("Dead Kconfig `select` edge removal requires both a removed target")
                && docs.contains("still-valid source symbol"),
            "docs should describe valid-source gating for removed select edges"
        );
    }
}
