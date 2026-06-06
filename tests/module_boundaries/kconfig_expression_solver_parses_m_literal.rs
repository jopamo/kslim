use super::common::*;

#[test]
fn kconfig_expression_solver_parses_m_literal() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "M,",
        "\"m\" => KconfigExpr::Literal(TristateLiteral::M)",
        "KconfigExpr::Literal(TristateLiteral::M) => String::from(\"m\")",
        "TristateLiteral::M => 1",
        "KconfigExpr::Literal(_) | KconfigExpr::StringLiteral(_) => 5",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should parse literal m through {required}"
        );
    }

    for required in [
        "parse_kconfig_expr_parses_m_literal",
        "parse_kconfig_expr(\"m\")",
        "m || FOO",
        "parse_kconfig_expr(\"M\")",
        "KconfigExpr::Literal(TristateLiteral::M)",
        "KconfigExpr::Symbol(String::from(\"M\"))",
        "render_kconfig_expr(&expr)",
        "render_kconfig_expr(&uppercase)",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig m-literal parser tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("parse literal `m`")
            && architecture.contains("`TristateLiteral::M`")
            && kernel_build_guide.contains("parse literal `m`")
            && kernel_build_guide.contains("`TristateLiteral::M`"),
        "docs should describe Kconfig m literal expression parsing"
    );
}
