use super::common::*;

#[test]
fn kconfig_expression_solver_evaluates_removed_symbol_effect() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "fn evaluate_kconfig_expr_after_removed_symbols(",
        "removed_symbols: &HashSet<&str>",
        "let simplified = simplify_kconfig_expr(expr, removed_symbols);",
        "evaluate_kconfig_expr(&simplified, symbol_values)",
        "fn evaluate_kconfig_visibility_after_removed_symbols(",
        "evaluate_kconfig_visibility_with(",
        "fn evaluate_kconfig_reachability_after_removed_symbols(",
        ".map(|visibility| visibility != TristateLiteral::N)",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should evaluate removed-symbol effect through {required}"
        );
    }

    for required in [
        "evaluate_kconfig_removed_symbol_effect_forces_removed_symbols_to_n",
        "removed_symbols",
        "REMOVED && LIVE",
        "REMOVED = n",
        "\\\"Still\\\" if LIVE",
        "depends on REMOVED || DEP",
        "depends on REMOVED && DEP",
        "evaluate_kconfig_expr_after_removed_symbols",
        "evaluate_kconfig_visibility_after_removed_symbols",
        "evaluate_kconfig_reachability_after_removed_symbols",
        "Some(TristateLiteral::M)",
        "Some(false)",
        "Some(true)",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig removed-symbol effect tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("evaluate effect of removed symbols")
            && architecture.contains("`n` before visibility")
            && kernel_build_guide.contains("evaluate effect of removed symbols")
            && kernel_build_guide.contains("`n` before visibility"),
        "docs should describe Kconfig removed-symbol effect evaluation"
    );
}
