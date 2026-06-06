use super::common::*;

#[test]
fn kconfig_expression_solver_evaluates_tristate_min_max() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "fn tristate_and(lhs: TristateLiteral, rhs: TristateLiteral) -> TristateLiteral",
        "std::cmp::min_by_key(lhs, rhs, |value| tristate_rank(*value))",
        "fn tristate_or(lhs: TristateLiteral, rhs: TristateLiteral) -> TristateLiteral",
        "std::cmp::max_by_key(lhs, rhs, |value| tristate_rank(*value))",
        "TristateLiteral::N => 0",
        "TristateLiteral::M => 1",
        "TristateLiteral::Y => 2",
        "KconfigExpr::Literal(tristate_and(*lhs, *rhs))",
        "KconfigExpr::Literal(tristate_or(*lhs, *rhs))",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should evaluate tristate min/max through {required}"
        );
    }

    for required in [
        "simplify_kconfig_expr_evaluates_tristate_min_max",
        "y && m",
        "m && n",
        "n || m",
        "m || y",
        "(m && y) || n",
        "(m || n) && y",
        "render_kconfig_expr(&simplified)",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig tristate min/max tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("evaluate tristate min/max")
            && architecture.contains("`&&` as tristate minimum")
            && architecture.contains("`||` as tristate maximum")
            && kernel_build_guide.contains("evaluate tristate min/max")
            && kernel_build_guide.contains("`&&` as tristate minimum")
            && kernel_build_guide.contains("`||` as tristate maximum"),
        "docs should describe Kconfig tristate min/max expression evaluation"
    );
}
