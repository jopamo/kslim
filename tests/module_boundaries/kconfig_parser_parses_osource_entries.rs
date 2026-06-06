use super::common::*;

#[test]
fn kconfig_parser_parses_osource_entries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = production_source(&root.join("src/kconfig/ast.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        kconfig.contains("KconfigOsourceEntry") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose osource AST entry parsing"
    );

    for required in [
        "pub(crate) struct KconfigOsourceEntry",
        "Osource(KconfigOsourceEntry)",
        "pub(crate) fn osources(&self) -> impl Iterator<Item = &KconfigOsourceEntry>",
        "path: String",
        "KconfigEntryHeaderKind::Osource",
        "parse_quoted_path_entry_header(trimmed, \"osource\", line_number)?",
        "line: usize",
        "end_line: usize",
        "directive: KconfigRawLine",
        "osource is handled as a marker",
        "missing a path",
        "missing a quoted path",
        "unterminated quoted path",
        "unexpected trailing tokens",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig osource-entry parser should own {required}"
        );
    }

    assert!(
        architecture.contains("`osource` entries")
            && architecture.contains("`KconfigOsourceEntry`")
            && kernel_build_guide.contains("`osource` entries")
            && kernel_build_guide.contains("`KconfigOsourceEntry`"),
        "docs should describe the osource-entry parser slice"
    );
}
