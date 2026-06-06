use super::common::*;

#[test]
fn kconfig_expression_solver_parses_or() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "Or(Box<KconfigExpr>, Box<KconfigExpr>)",
        "ExprToken::Or",
        "fn parse_kconfig_or_expr(",
        "parse_kconfig_and_expr(tokens, idx)?",
        "expr = KconfigExpr::Or(Box::new(expr), Box::new(rhs));",
        "\"{} || {}\"",
        "KconfigExpr::Or(_, _) => 1",
        "KconfigExpr::And(_, _) => 2",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should parse || through {required}"
        );
    }

    for required in [
        "parse_kconfig_expr_parses_or_expression",
        "FOO || BAR || BAZ",
        "FOO || BAR && BAZ",
        "KconfigExpr::Or",
        "KconfigExpr::And",
        "render_kconfig_expr(&mixed)",
        "FOO | BAR",
        "FOO ||",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig || parser tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("parse `||`")
            && architecture.contains("`KconfigExpr::Or`")
            && kernel_build_guide.contains("parse `||`")
            && kernel_build_guide.contains("`KconfigExpr::Or`"),
        "docs should describe Kconfig || expression parsing"
    );
}
