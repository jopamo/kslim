use super::common::*;

#[test]
fn kconfig_parser_parses_rsource_entries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = production_source(&root.join("src/kconfig/ast.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        kconfig.contains("KconfigRsourceEntry") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose rsource AST entry parsing"
    );

    for required in [
        "pub(crate) struct KconfigRsourceEntry",
        "Rsource(KconfigRsourceEntry)",
        "pub(crate) fn rsources(&self) -> impl Iterator<Item = &KconfigRsourceEntry>",
        "path: String",
        "KconfigEntryHeaderKind::Rsource",
        "parse_quoted_path_entry_header(trimmed, \"rsource\", line_number)?",
        "line: usize",
        "end_line: usize",
        "directive: KconfigRawLine",
        "rsource is handled as a marker",
        "missing a path",
        "missing a quoted path",
        "unterminated quoted path",
        "unexpected trailing tokens",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig rsource-entry parser should own {required}"
        );
    }

    assert!(
        architecture.contains("`rsource` entries")
            && architecture.contains("`KconfigRsourceEntry`")
            && kernel_build_guide.contains("`rsource` entries")
            && kernel_build_guide.contains("`KconfigRsourceEntry`"),
        "docs should describe the rsource-entry parser slice"
    );
}
