use super::common::*;

#[test]
fn kconfig_rewrite_simplifies_expressions_only_when_tristate_equivalent() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/rewrite.rs"],
    );
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
        "const KCONFIG_EXPR_EQUIVALENCE_SYMBOL_LIMIT",
        "equivalent_kconfig_expr_simplification",
        "kconfig_expr_rewrite_is_tristate_equivalent",
        "evaluate_kconfig_expr_under_removed_tristate_semantics",
        "evaluate_kconfig_const_under_removed_tristate_semantics",
        "return RelationLineAnalysis::Noop;",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression rewrites should require tristate equivalence; missing {required}"
        );
    }

    for required in [
        "test_kconfig_expression_simplification_requires_tristate_equivalence",
        "REMOVED || LIVE",
        "KconfigExpr::Literal(TristateLiteral::N)",
        "A && B && C && D && E && F && G && H && I",
        "equivalent_kconfig_expr_simplification(&too_many_symbols, &removed)",
    ] {
        assert!(
            kconfig_with_tests.contains(required),
            "unit tests should pin tristate-equivalence-gated simplification; missing {required}"
        );
    }

    for docs in [architecture, kernel_build] {
        assert!(
            docs.contains("Kconfig expression simplification is gated by tristate-equivalence proof"),
            "docs should describe tristate-equivalence-gated Kconfig simplification"
        );
    }
}
