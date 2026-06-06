use super::common::*;

#[test]
fn kconfig_expression_solver_parses_inequality() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "Ne(Box<KconfigExpr>, Box<KconfigExpr>)",
        "ExprToken::Ne",
        "tokens.push(ExprToken::Ne)",
        "Some(ExprToken::Ne)",
        "expr = KconfigExpr::Ne(Box::new(expr), Box::new(rhs));",
        "\"{} != {}\"",
        "KconfigExpr::Eq(_, _) | KconfigExpr::Ne(_, _) => 3",
        "string_literals_are_comparison_operands",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should parse inequality through {required}"
        );
    }

    for required in [
        "parse_kconfig_expr_parses_inequality_comparison",
        "FOO != n",
        "FOO != \\\"bar\\\"",
        "FOO != n || BAR",
        "KconfigExpr::Ne",
        "KconfigExpr::StringLiteral",
        "render_kconfig_expr(&string_literal)",
        "parse_kconfig_expr(\"FOO !=\")",
        "FOO !== n",
        "FOO ! = n",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig inequality parser tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("parse `!=`")
            && architecture.contains("`KconfigExpr::Ne`")
            && kernel_build_guide.contains("parse `!=`")
            && kernel_build_guide.contains("`KconfigExpr::Ne`"),
        "docs should describe Kconfig inequality expression parsing"
    );
}
