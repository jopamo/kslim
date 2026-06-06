use super::common::*;

#[test]
fn kconfig_rewrite_preserves_unknown_expressions() {
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
        "KCONFIG_UNSUPPORTED_REMOVED_SYMBOL_EXPRESSION_REASON",
        "KCONFIG_UNKNOWN_REMOVED_TARGET_CONDITION_REASON",
        "unsupported_kconfig_expression",
        "if let Some(condition) = condition",
        "parse_kconfig_expr(condition).is_none()",
        "return unsupported_kconfig_expression(",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig rewrite should preserve unknown expressions before mutation; missing {required}"
        );
    }

    for required in [
        "test_rewrite_kconfig_relations_preserves_unknown_removed_target_conditions",
        "LIVE + OTHER",
        "OTHER ? LIVE",
        "KCONFIG_UNKNOWN_REMOVED_TARGET_CONDITION_REASON",
    ] {
        assert!(
            kconfig_with_tests.contains(required),
            "unit tests should pin unknown-expression preservation; missing {required}"
        );
    }

    for docs in [architecture, kernel_build] {
        assert!(
            docs.contains("Unknown Kconfig expressions are fail-closed preservation sites"),
            "docs should describe fail-closed unknown-expression preservation"
        );
    }
}
