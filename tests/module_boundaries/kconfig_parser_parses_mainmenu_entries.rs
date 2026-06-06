use super::common::*;

#[test]
fn kconfig_parser_parses_mainmenu_entries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = production_source(&root.join("src/kconfig/ast.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        kconfig.contains("KconfigMainmenuEntry") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose mainmenu AST entry parsing"
    );

    for required in [
        "pub(crate) struct KconfigMainmenuEntry",
        "Mainmenu(KconfigMainmenuEntry)",
        "pub(crate) fn mainmenus(&self) -> impl Iterator<Item = &KconfigMainmenuEntry>",
        "prompt: String",
        "KconfigEntryHeaderKind::Mainmenu",
        "parse_quoted_string_entry_header(trimmed, \"mainmenu\", line_number)?",
        "parse_quoted_string_literal",
        "line: usize",
        "end_line: usize",
        "directive: KconfigRawLine",
        "mainmenu is handled as a marker",
        "missing a prompt",
        "missing a quoted prompt",
        "unterminated quoted prompt",
        "unexpected trailing tokens",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig mainmenu-entry parser should own {required}"
        );
    }

    assert!(
        architecture.contains("`mainmenu` entries")
            && architecture.contains("`KconfigMainmenuEntry`")
            && kernel_build_guide.contains("`mainmenu` entries")
            && kernel_build_guide.contains("`KconfigMainmenuEntry`"),
        "docs should describe the mainmenu-entry parser slice"
    );
}
