use super::common::*;

#[test]
fn kconfig_parser_parses_orsource_entries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = production_source(&root.join("src/kconfig/ast.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        kconfig.contains("KconfigOrsourceEntry") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose orsource AST entry parsing"
    );

    for required in [
        "pub(crate) struct KconfigOrsourceEntry",
        "Orsource(KconfigOrsourceEntry)",
        "pub(crate) fn orsources(&self) -> impl Iterator<Item = &KconfigOrsourceEntry>",
        "path: String",
        "KconfigEntryHeaderKind::Orsource",
        "parse_quoted_path_entry_header(trimmed, \"orsource\", line_number)?",
        "line: usize",
        "end_line: usize",
        "directive: KconfigRawLine",
        "orsource is handled as a marker",
        "missing a path",
        "missing a quoted path",
        "unterminated quoted path",
        "unexpected trailing tokens",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig orsource-entry parser should own {required}"
        );
    }

    assert!(
        architecture.contains("`orsource` entries")
            && architecture.contains("`KconfigOrsourceEntry`")
            && kernel_build_guide.contains("`orsource` entries")
            && kernel_build_guide.contains("`KconfigOrsourceEntry`"),
        "docs should describe the orsource-entry parser slice"
    );
}
