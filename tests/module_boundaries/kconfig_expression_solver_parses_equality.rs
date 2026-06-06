use super::common::*;

#[test]
fn kconfig_expression_solver_parses_equality() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "Eq(Box<KconfigExpr>, Box<KconfigExpr>)",
        "ExprToken::Eq",
        "tokens.push(ExprToken::Eq)",
        "fn parse_kconfig_cmp_expr(",
        "Some(ExprToken::Eq)",
        "expr = KconfigExpr::Eq(Box::new(expr), Box::new(rhs));",
        "\"{} = {}\"",
        "KconfigExpr::Eq(_, _) | KconfigExpr::Ne(_, _) => 3",
        "string_literals_are_comparison_operands",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should parse equality through {required}"
        );
    }

    for required in [
        "parse_kconfig_expr_parses_equality_comparison",
        "FOO = y",
        "FOO = \\\"bar\\\"",
        "FOO = y && BAR",
        "KconfigExpr::Eq",
        "KconfigExpr::StringLiteral",
        "render_kconfig_expr(&string_literal)",
        "parse_kconfig_expr(\"FOO =\")",
        "FOO == y",
        "FOO = \\\"unterminated",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig equality parser tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("parse `=`")
            && architecture.contains("`KconfigExpr::Eq`")
            && kernel_build_guide.contains("parse `=`")
            && kernel_build_guide.contains("`KconfigExpr::Eq`"),
        "docs should describe Kconfig equality expression parsing"
    );
}
