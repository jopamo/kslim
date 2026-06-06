use super::common::*;

#[test]
fn kconfig_expression_solver_evaluates_visibility() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "fn evaluate_kconfig_visibility(",
        "prompt_definitions: &[KconfigPromptDefinition]",
        "dependency_definitions: &[KconfigDependencyDefinition]",
        "symbol_values: &BTreeMap<String, TristateLiteral>",
        "fn evaluate_kconfig_visibility_with(",
        "mut evaluate_expr: impl FnMut(&KconfigExpr) -> Option<TristateLiteral>",
        "prompt.condition()",
        "dependency.expression()",
        "evaluate_kconfig_expr(expr, symbol_values)",
        "evaluate_expr(&parse_kconfig_expr(condition)?)",
        "evaluate_expr(&expr)?",
        "Some(tristate_and(prompt_visibility, dependency_visibility))",
        "fn evaluate_kconfig_expr(",
        "KconfigExpr::Symbol(symbol) => symbol_values.get(symbol).copied()",
        "fn evaluate_kconfig_const(",
        "fn tristate_not(value: TristateLiteral) -> TristateLiteral",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should evaluate visibility through {required}"
        );
    }

    for required in [
        "evaluate_kconfig_visibility_lowers_dependencies_by_tristate_minimum",
        "evaluate_kconfig_visibility_combines_prompt_and_dependency_expressions",
        "\\\"Lowered to m\\\"",
        "\\\"Lowered to n\\\"",
        "depends on DEP_M",
        "depends on DEP_N",
        "parse_kconfig_document",
        "\\\"Visible\\\" if PROMPT",
        "depends on DEP_A",
        "depends on DEP_B || m",
        "\\\"Unknown\\\" if MISSING",
        "MODE = \\\"y\\\"",
        "depends on !BLOCKED",
        "Some(TristateLiteral::M)",
        "Some(TristateLiteral::N)",
        "Some(TristateLiteral::Y)",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig visibility evaluator tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("evaluate local visibility")
            && architecture.contains("prompt conditions")
            && architecture.contains("dependency expressions")
            && kernel_build_guide.contains("evaluate local visibility")
            && kernel_build_guide.contains("prompt conditions")
            && kernel_build_guide.contains("dependency expressions"),
        "docs should describe Kconfig visibility expression evaluation"
    );
}
