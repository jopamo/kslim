use super::common::*;

#[test]
fn kconfig_unit_tests_live_beside_owned_modules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = std::fs::read_to_string(root.join("src/kconfig/mod.rs"))
        .expect("failed to read src/kconfig/mod.rs");
    let kconfig_tests = production_source(&root.join("src/kconfig/tests.rs"));
    let rewrite_tests = production_source(&root.join("src/kconfig/tests_rewrite.rs"));
    let solver_tests = production_source(&root.join("src/kconfig/tests_solver.rs"));
    let report_tests = production_source(&root.join("src/kconfig/tests_report.rs"));
    let root_facade_tests = production_source(&root.join("src/kconfig/tests_root_facade.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        kconfig.contains("\n#[cfg(test)]\nmod tests;\n"),
        "src/kconfig/mod.rs should delegate remaining unit tests to src/kconfig/tests.rs"
    );
    assert!(
        !kconfig.contains("\nmod tests {") && !kconfig.contains("\n#[test]"),
        "src/kconfig/mod.rs should not retain inline unit test bodies"
    );

    for required in [
        "#[path = \"tests_rewrite.rs\"]\nmod rewrite;",
        "#[path = \"tests_solver.rs\"]\nmod solver;",
        "#[path = \"tests_report.rs\"]\nmod report;",
        "#[path = \"tests_root_facade.rs\"]\nmod root_facade;",
    ] {
        assert!(
            kconfig_tests.contains(required),
            "src/kconfig/tests.rs should register behavior-focused Kconfig test module {required}"
        );
    }

    assert!(
        !kconfig_tests.contains("#[test]"),
        "src/kconfig/tests.rs should keep shared helpers and module declarations only"
    );
    assert!(
        rewrite_tests.contains("test_prune_configs_removes_multiple_symbol_definitions")
            && rewrite_tests.contains(
                "test_rewrite_kconfig_relations_preserves_prompt_text_unless_removing_full_symbol_block"
            )
            && rewrite_tests.contains("test_rewrite_kconfig_sources_requires_manifest_index_proof"),
        "src/kconfig/tests_rewrite.rs should own Kconfig rewrite behavior tests"
    );
    assert!(
        solver_tests.contains("test_kconfig_expression_simplification_requires_tristate_equivalence")
            && solver_tests
                .contains("test_rewrite_dead_kconfig_symbol_definitions_requires_solver_proof")
            && solver_tests.contains(
                "test_rewrite_empty_kconfig_menus_requires_solver_cleanup_proof"
            ),
        "src/kconfig/tests_solver.rs should own solver-proof and tristate-equivalence tests"
    );
    assert!(
        report_tests.contains(
            "test_rewrite_kconfig_relations_drops_removed_selects_and_implies_only_from_valid_sources"
        ) && report_tests
            .contains("test_rewrite_kconfig_relations_reports_unsupported_expression_syntax")
            && report_tests.contains(
                "test_rewrite_kconfig_relations_reports_unsupported_if_block_expression_syntax"
            ),
        "src/kconfig/tests_report.rs should own Kconfig rewrite report and skipped-site tests"
    );
    assert!(
        root_facade_tests.contains("test_parse_kconfig_directive_defines_minimal_ast_nodes")
            && root_facade_tests
                .contains("test_defined_symbols_in_file_returns_sorted_unique_symbols")
            && root_facade_tests.contains("test_render_kconfig_expr_preserves_parentheses_when_needed"),
        "src/kconfig/tests_root_facade.rs should own root facade parser/expression tests"
    );
    assert!(
        architecture.contains("Kconfig root-level unit tests are split by behavior"),
        "docs/architecture.md should document extracted Kconfig test ownership"
    );
}
