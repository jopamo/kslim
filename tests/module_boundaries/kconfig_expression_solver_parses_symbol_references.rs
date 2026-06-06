use super::common::*;

#[test]
fn kconfig_expression_solver_parses_symbol_references() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "Symbol(String)",
        "ExprToken::Symbol",
        "tokens.push(ExprToken::Symbol",
        "ch if is_kconfig_symbol_char(ch)",
        "fn parse_kconfig_primary_expr(",
        "_ => KconfigExpr::Symbol(symbol.clone()),",
        "KconfigExpr::Symbol(symbol) => symbol.clone(),",
        "fn is_kconfig_symbol_char(ch: char) -> bool",
        "ch.is_ascii_alphanumeric() || ch == '_'",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should parse symbol references through {required}"
        );
    }

    for required in [
        "parse_kconfig_expr_parses_symbol_references",
        "DRM_AMDGPU",
        "CONFIG_FOO",
        "64BIT",
        "DRM_AMDGPU && CONFIG_FOO",
        "KconfigExpr::Symbol",
        "render_kconfig_expr(&numeric_prefix)",
        "DRM-AMDGPU",
        "drivers/foo",
        "FOO BAR",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig symbol-reference parser tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("parse symbol references")
            && architecture.contains("`KconfigExpr::Symbol`")
            && kernel_build_guide.contains("parse symbol references")
            && kernel_build_guide.contains("`KconfigExpr::Symbol`"),
        "docs should describe Kconfig symbol-reference expression parsing"
    );
}
