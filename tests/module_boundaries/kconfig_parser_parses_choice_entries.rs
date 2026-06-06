use super::common::*;

#[test]
fn kconfig_parser_parses_choice_entries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = production_source(&root.join("src/kconfig/ast.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        kconfig.contains("KconfigChoiceEntry") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose choice AST entry parsing"
    );

    for required in [
        "pub(crate) struct KconfigChoiceEntry",
        "Choice(KconfigChoiceEntry)",
        "pub(crate) fn choices(&self) -> impl Iterator<Item = &KconfigChoiceEntry>",
        "symbol: Option<KconfigSymbol>",
        "KconfigEntryHeaderKind::Choice",
        "parse_optional_symbol_entry_header(trimmed, \"choice\", line_number)?",
        "line: usize",
        "end_line: usize",
        "directive: KconfigRawLine",
        "body: Vec<KconfigRawLine>",
        "Kconfig {keyword} directive",
        "invalid Kconfig {keyword} symbol",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig choice parser should own {required}"
        );
    }

    assert!(
        architecture.contains("`choice` entries")
            && architecture.contains("`KconfigChoiceEntry`")
            && kernel_build_guide.contains("`choice` entries")
            && kernel_build_guide.contains("`KconfigChoiceEntry`"),
        "docs should describe the choice-entry parser slice"
    );
}
