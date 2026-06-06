use super::common::*;

#[test]
fn kconfig_parser_parses_source_entries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = production_source(&root.join("src/kconfig/ast.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        kconfig.contains("KconfigSourceEntry") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose source AST entry parsing"
    );

    for required in [
        "pub(crate) struct KconfigSourceEntry",
        "Source(KconfigSourceEntry)",
        "pub(crate) fn sources(&self) -> impl Iterator<Item = &KconfigSourceEntry>",
        "path: String",
        "KconfigEntryHeaderKind::Source",
        "parse_quoted_path_entry_header(trimmed, \"source\", line_number)?",
        "line: usize",
        "end_line: usize",
        "directive: KconfigRawLine",
        "source is handled as a marker",
        "missing a path",
        "missing a quoted path",
        "unterminated quoted path",
        "unexpected trailing tokens",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig source-entry parser should own {required}"
        );
    }

    assert!(
        architecture.contains("`source` entries")
            && architecture.contains("`KconfigSourceEntry`")
            && kernel_build_guide.contains("`source` entries")
            && kernel_build_guide.contains("`KconfigSourceEntry`"),
        "docs should describe the source-entry parser slice"
    );
}
