use super::common::*;

#[test]
fn kconfig_expression_solver_evaluates_defaults_after_removal() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "fn evaluate_kconfig_defaults(",
        "default_definitions: &[KconfigDefaultDefinition]",
        "fn evaluate_kconfig_defaults_with(",
        "default.condition()",
        "condition == TristateLiteral::N",
        "default.value()",
        "Some(TristateLiteral::N)",
        "fn evaluate_kconfig_defaults_after_removed_symbols(",
        "evaluate_kconfig_expr_after_removed_symbols(",
        "selected_profile_values",
        "removed_symbols",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should evaluate defaults after removal through {required}"
        );
    }

    for required in [
        "evaluate_kconfig_defaults_after_removal_uses_first_active_default",
        "FIRST_AFTER_REMOVAL",
        "default y if REMOVED",
        "default m if LIVE",
        "VALUE_FORCED_TO_N",
        "default REMOVED if LIVE",
        "UNCONDITIONAL_SYMBOL",
        "default OTHER",
        "REMOVED = n",
        "NO_ACTIVE_DEFAULT",
        "evaluate_kconfig_defaults(",
        "evaluate_kconfig_defaults_after_removed_symbols",
        "Some(TristateLiteral::M)",
        "Some(TristateLiteral::N)",
        "Some(TristateLiteral::Y)",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig defaults-after-removal tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("evaluate defaults after removal")
            && architecture.contains("first active default")
            && kernel_build_guide.contains("evaluate defaults after removal")
            && kernel_build_guide.contains("first active default"),
        "docs should describe Kconfig defaults-after-removal evaluation"
    );
}
