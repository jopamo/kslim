use super::common::*;

#[test]
fn kconfig_expression_solver_parses_n_literal() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "N,",
        "\"n\" => KconfigExpr::Literal(TristateLiteral::N)",
        "KconfigExpr::Literal(TristateLiteral::N) => String::from(\"n\")",
        "TristateLiteral::N => 0",
        "KconfigExpr::Literal(_) | KconfigExpr::StringLiteral(_) => 5",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should parse literal n through {required}"
        );
    }

    for required in [
        "parse_kconfig_expr_parses_n_literal",
        "parse_kconfig_expr(\"n\")",
        "n && FOO",
        "parse_kconfig_expr(\"N\")",
        "KconfigExpr::Literal(TristateLiteral::N)",
        "KconfigExpr::Symbol(String::from(\"N\"))",
        "render_kconfig_expr(&expr)",
        "render_kconfig_expr(&uppercase)",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig n-literal parser tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("parse literal `n`")
            && architecture.contains("`TristateLiteral::N`")
            && kernel_build_guide.contains("parse literal `n`")
            && kernel_build_guide.contains("`TristateLiteral::N`"),
        "docs should describe Kconfig n literal expression parsing"
    );
}
