use super::common::*;

#[test]
fn kconfig_parser_parses_endmenu_markers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = production_source(&root.join("src/kconfig/ast.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        kconfig.contains("KconfigEndmenuEntry") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose endmenu AST marker parsing"
    );

    for required in [
        "pub(crate) struct KconfigEndmenuEntry",
        "Endmenu(KconfigEndmenuEntry)",
        "pub(crate) fn endmenus(&self) -> impl Iterator<Item = &KconfigEndmenuEntry>",
        "KconfigEntryHeaderKind::Endmenu",
        "parse_keyword_only_entry_header(trimmed, \"endmenu\", line_number)?",
        "line: usize",
        "end_line: usize",
        "directive: KconfigRawLine",
        "endmenu is handled as a marker",
        "Kconfig {keyword} directive",
        "unexpected trailing tokens",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig endmenu parser should own {required}"
        );
    }

    assert!(
        architecture.contains("`endmenu` markers")
            && architecture.contains("`KconfigEndmenuEntry`")
            && kernel_build_guide.contains("`endmenu` markers")
            && kernel_build_guide.contains("`KconfigEndmenuEntry`"),
        "docs should describe the endmenu marker parser slice"
    );
}
