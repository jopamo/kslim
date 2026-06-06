use super::common::*;

#[test]
fn kconfig_expression_logic_lives_in_expression_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(&root, &["src/kconfig/mod.rs", "src/kconfig/rewrite.rs"]);
    let expression = production_source(&root.join("src/kconfig/expression.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod expression;",
        "use expression::{",
        "parse_kconfig_expr",
        "render_kconfig_expr",
        "KconfigExpr",
        "TristateLiteral",
    ] {
        assert!(
            kconfig.contains(required),
            "src/kconfig/mod.rs should delegate expression ownership through {required}"
        );
    }

    for required in [
        "pub(super) enum KconfigExpr",
        "pub(super) enum TristateLiteral",
        "fn tokenize_kconfig_expr(",
        "fn parse_kconfig_or_expr(",
        "pub(super) fn simplify_kconfig_expr(",
        "pub(super) fn evaluate_kconfig_expr(",
        "pub(super) fn render_kconfig_expr(",
    ] {
        assert!(
            expression.contains(required),
            "src/kconfig/expression.rs should own expression logic through {required}"
        );
    }

    for forbidden in [
        "\nenum KconfigExpr",
        "\nfn tokenize_kconfig_expr(",
        "\nfn parse_kconfig_or_expr(",
        "\nfn render_kconfig_expr(",
    ] {
        assert!(
            !kconfig.contains(forbidden),
            "src/kconfig/mod.rs should not retain extracted expression implementation {forbidden}"
        );
    }

    for required in [
        "`src/kconfig/expression.rs`",
        "parsing, simplification, evaluation, and rendering",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document extracted Kconfig expression ownership through {required}"
        );
    }
}
