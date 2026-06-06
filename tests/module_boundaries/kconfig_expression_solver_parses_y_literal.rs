use super::common::*;

#[test]
fn kconfig_expression_solver_parses_y_literal() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "enum TristateLiteral",
        "Y",
        "\"y\" => KconfigExpr::Literal(TristateLiteral::Y)",
        "KconfigExpr::Literal(TristateLiteral::Y) => String::from(\"y\")",
        "KconfigExpr::Literal(_) | KconfigExpr::StringLiteral(_) => 5",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should parse literal y through {required}"
        );
    }

    for required in [
        "parse_kconfig_expr_parses_y_literal",
        "parse_kconfig_expr(\"y\")",
        "y && FOO",
        "parse_kconfig_expr(\"Y\")",
        "KconfigExpr::Literal(TristateLiteral::Y)",
        "KconfigExpr::Symbol(String::from(\"Y\"))",
        "render_kconfig_expr(&expr)",
        "render_kconfig_expr(&uppercase)",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig y-literal parser tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("parse literal `y`")
            && architecture.contains("`TristateLiteral::Y`")
            && kernel_build_guide.contains("parse literal `y`")
            && kernel_build_guide.contains("`TristateLiteral::Y`"),
        "docs should describe Kconfig y literal expression parsing"
    );
}
