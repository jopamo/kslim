use super::common::*;

#[test]
fn kconfig_parser_parses_endif_markers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = production_source(&root.join("src/kconfig/ast.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        kconfig.contains("KconfigEndifEntry") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose endif AST marker parsing"
    );

    for required in [
        "pub(crate) struct KconfigEndifEntry",
        "Endif(KconfigEndifEntry)",
        "pub(crate) fn endifs(&self) -> impl Iterator<Item = &KconfigEndifEntry>",
        "KconfigEntryHeaderKind::Endif",
        "parse_keyword_only_entry_header(trimmed, \"endif\", line_number)?",
        "line: usize",
        "end_line: usize",
        "directive: KconfigRawLine",
        "endif is handled as a marker",
        "Kconfig {keyword} directive",
        "unexpected trailing tokens",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig endif parser should own {required}"
        );
    }

    assert!(
        architecture.contains("`endif` markers")
            && architecture.contains("`KconfigEndifEntry`")
            && kernel_build_guide.contains("`endif` markers")
            && kernel_build_guide.contains("`KconfigEndifEntry`"),
        "docs should describe the endif marker parser slice"
    );
}
