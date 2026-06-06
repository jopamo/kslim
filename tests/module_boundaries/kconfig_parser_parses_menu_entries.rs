use super::common::*;

#[test]
fn kconfig_parser_parses_menu_entries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = production_source(&root.join("src/kconfig/ast.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        kconfig.contains("KconfigMenuEntry") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose menu AST entry parsing"
    );

    for required in [
        "pub(crate) struct KconfigMenuEntry",
        "Menu(KconfigMenuEntry)",
        "pub(crate) fn menus(&self) -> impl Iterator<Item = &KconfigMenuEntry>",
        "prompt: String",
        "KconfigEntryHeaderKind::Menu",
        "parse_quoted_string_entry_header(trimmed, \"menu\", line_number)?",
        "parse_quoted_string_literal",
        "line: usize",
        "end_line: usize",
        "directive: KconfigRawLine",
        "body: Vec<KconfigRawLine>",
        "Kconfig {keyword} directive",
        "missing a quoted prompt",
        "unterminated quoted prompt",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig menu-entry parser should own {required}"
        );
    }

    assert!(
        architecture.contains("`menu` entries")
            && architecture.contains("`KconfigMenuEntry`")
            && kernel_build_guide.contains("`menu` entries")
            && kernel_build_guide.contains("`KconfigMenuEntry`"),
        "docs should describe the menu-entry parser slice"
    );
}
