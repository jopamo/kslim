use super::common::*;

#[test]
fn kconfig_expression_solver_parses_not() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "Not(Box<KconfigExpr>)",
        "ExprToken::Not",
        "tokens.push(ExprToken::Not)",
        "fn parse_kconfig_unary_expr(",
        "KconfigExpr::Not(Box::new(parse_kconfig_unary_expr(",
        "\"!{}\"",
        "KconfigExpr::Not(_) => 4",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should parse ! through {required}"
        );
    }

    for required in [
        "parse_kconfig_expr_parses_not_expression",
        "!FOO",
        "!!FOO",
        "!(FOO || BAR)",
        "!FOO && BAR",
        "KconfigExpr::Not",
        "render_kconfig_expr(&grouped)",
        "parse_kconfig_expr(\"!\")",
        "FOO ! BAR",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig ! parser tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("parse `!`")
            && architecture.contains("`KconfigExpr::Not`")
            && kernel_build_guide.contains("parse `!`")
            && kernel_build_guide.contains("`KconfigExpr::Not`"),
        "docs should describe Kconfig ! expression parsing"
    );
}
