use super::common::*;

#[test]
fn kconfig_parser_parses_if_entries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = production_source(&root.join("src/kconfig/ast.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        kconfig.contains("KconfigIfEntry") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose if AST entry parsing"
    );

    for required in [
        "pub(crate) struct KconfigIfEntry",
        "If(KconfigIfEntry)",
        "pub(crate) fn ifs(&self) -> impl Iterator<Item = &KconfigIfEntry>",
        "condition: String",
        "KconfigEntryHeaderKind::If",
        "parse_condition_entry_header(trimmed, \"if\", line_number)?",
        "line: usize",
        "end_line: usize",
        "directive: KconfigRawLine",
        "body: Vec<KconfigRawLine>",
        "Kconfig {keyword} directive",
        "missing a condition",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig if-entry parser should own {required}"
        );
    }

    assert!(
        architecture.contains("`if` entries")
            && architecture.contains("`KconfigIfEntry`")
            && kernel_build_guide.contains("`if` entries")
            && kernel_build_guide.contains("`KconfigIfEntry`"),
        "docs should describe the if-entry parser slice"
    );
}
