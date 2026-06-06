use super::common::*;

#[test]
fn kconfig_parser_parses_endchoice_markers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = production_source(&root.join("src/kconfig/ast.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        kconfig.contains("KconfigEndchoiceEntry") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose endchoice AST marker parsing"
    );

    for required in [
        "pub(crate) struct KconfigEndchoiceEntry",
        "Endchoice(KconfigEndchoiceEntry)",
        "pub(crate) fn endchoices(&self) -> impl Iterator<Item = &KconfigEndchoiceEntry>",
        "KconfigEntryHeaderKind::Endchoice",
        "parse_keyword_only_entry_header(trimmed, \"endchoice\", line_number)?",
        "line: usize",
        "end_line: usize",
        "directive: KconfigRawLine",
        "endchoice is handled as a marker",
        "Kconfig {keyword} directive",
        "unexpected trailing tokens",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig endchoice parser should own {required}"
        );
    }

    assert!(
        architecture.contains("`endchoice` markers")
            && architecture.contains("`KconfigEndchoiceEntry`")
            && kernel_build_guide.contains("`endchoice` markers")
            && kernel_build_guide.contains("`KconfigEndchoiceEntry`"),
        "docs should describe the endchoice marker parser slice"
    );
}
