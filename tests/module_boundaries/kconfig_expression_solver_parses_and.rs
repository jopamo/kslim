use super::common::*;

#[test]
fn kconfig_expression_solver_parses_and() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "And(Box<KconfigExpr>, Box<KconfigExpr>)",
        "ExprToken::And",
        "fn parse_kconfig_and_expr(",
        "parse_kconfig_cmp_expr(tokens, idx)?",
        "expr = KconfigExpr::And(Box::new(expr), Box::new(rhs));",
        "\"{} && {}\"",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should parse && through {required}"
        );
    }

    for required in [
        "parse_kconfig_expr_parses_and_expression",
        "FOO && BAR && BAZ",
        "KconfigExpr::And",
        "render_kconfig_expr(&expr)",
        "FOO & BAR",
        "FOO &&",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig && parser tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("Expression solver slices parse `&&`")
            && architecture.contains("`KconfigExpr::And`")
            && kernel_build_guide.contains("Expression solver slices parse `&&`")
            && kernel_build_guide.contains("`KconfigExpr::And`"),
        "docs should describe Kconfig && expression parsing"
    );
}
