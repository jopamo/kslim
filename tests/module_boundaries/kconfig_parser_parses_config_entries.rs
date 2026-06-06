use super::common::*;

#[test]
fn kconfig_parser_parses_config_entries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = production_source(&root.join("src/kconfig/ast.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        kconfig.contains("mod ast;")
            && kconfig.contains("parse_kconfig_document")
            && kconfig.contains("KconfigConfigEntry")
            && kconfig.contains("KconfigDocument")
            && kconfig.contains("KconfigNode"),
        "src/kconfig/mod.rs should expose the Kconfig AST parser surface"
    );

    for required in [
        "pub(crate) struct KconfigDocument",
        "nodes: Vec<KconfigNode>",
        "pub(crate) enum KconfigNode",
        "Config(KconfigConfigEntry)",
        "pub(crate) struct KconfigConfigEntry",
        "symbol: KconfigSymbol",
        "line: usize",
        "end_line: usize",
        "directive: KconfigRawLine",
        "body: Vec<KconfigRawLine>",
        "pub(crate) struct KconfigRawLine",
        "pub(crate) fn parse_kconfig_document(source: &str) -> Result<KconfigDocument>",
        "kconfig_help_text_mask(&lines)",
        "split_kconfig_trailing_comment(line)",
        "is_kconfig_boundary(trimmed)",
        "KconfigSymbol::new(symbol)",
        "missing a symbol",
        "unexpected trailing tokens",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig config-entry AST parser should own {required}"
        );
    }

    assert!(
        architecture.contains("The Kconfig AST parser now parses `config` entries")
            && architecture.contains("`KconfigDocument`")
            && architecture.contains("`KconfigConfigEntry`")
            && kernel_build_guide.contains("The Kconfig AST parser now parses `config` entries")
            && kernel_build_guide.contains("line spans, raw directive lines, and raw body lines"),
        "docs should describe the config-entry parser slice"
    );
}
