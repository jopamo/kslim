use super::common::*;

#[test]
fn kconfig_rewrite_module_uses_ast_and_tristate_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &[
            "src/kconfig/mod.rs",
            "src/kconfig/expression.rs",
            "src/kconfig/parser.rs",
            "src/kconfig/rewrite.rs",
        ],
    );

    for required in [
        "enum KconfigDirective",
        "enum KconfigExpr",
        "enum TristateLiteral",
        "fn parse_kconfig_directive",
        "fn parse_kconfig_expr",
        "fn simplify_kconfig_expr",
        "fn tristate_and",
        "fn tristate_or",
        "fn kconfig_help_text_mask",
        "fn is_kconfig_help_directive",
        "RelationLineAnalysis::Unsupported",
    ] {
        assert!(
            kconfig.contains(required),
            "src/kconfig/mod.rs should keep rewrites AST-aware and tristate-aware through {required}"
        );
    }

    for forbidden in [
        "line.contains(\"depends on",
        "line.replace(\"depends on",
        "line.contains(\"select",
        "line.replace(\"select",
        "line.contains(\"source",
        "line.replace(\"source",
    ] {
        assert!(
            !kconfig.contains(forbidden),
            "Kconfig rewrites must not use raw line substring mutation for syntax; found {forbidden}"
        );
    }
}
