use super::common::*;

#[test]
fn kconfig_parser_parses_menuconfig_entries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = production_source(&root.join("src/kconfig/ast.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        kconfig.contains("KconfigMenuconfigEntry") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose menuconfig AST entry parsing"
    );

    for required in [
        "pub(crate) struct KconfigMenuconfigEntry",
        "Menuconfig(KconfigMenuconfigEntry)",
        "pub(crate) fn menuconfigs(&self) -> impl Iterator<Item = &KconfigMenuconfigEntry>",
        "KconfigEntryHeaderKind::Menuconfig",
        "parse_symbol_entry_header(trimmed, \"menuconfig\", line_number)?",
        "symbol: KconfigSymbol",
        "line: usize",
        "end_line: usize",
        "directive: KconfigRawLine",
        "body: Vec<KconfigRawLine>",
        "Kconfig {keyword} directive",
        "invalid Kconfig {keyword} symbol",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig menuconfig-entry parser should own {required}"
        );
    }

    assert!(
        architecture.contains("`menuconfig` entries")
            && architecture.contains("`KconfigMenuconfigEntry`")
            && kernel_build_guide.contains("`menuconfig` entries")
            && kernel_build_guide.contains("`KconfigMenuconfigEntry`"),
        "docs should describe the menuconfig-entry parser slice"
    );
}
