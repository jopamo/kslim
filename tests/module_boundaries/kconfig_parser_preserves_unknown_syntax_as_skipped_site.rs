use super::common::*;

#[test]
fn kconfig_parser_preserves_unknown_syntax_as_skipped_site() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = production_source(&root.join("src/kconfig/ast.rs"));
    let ast_tests = production_sources(
        &root,
        &[
            "src/kconfig/ast/tests.rs",
            "src/kconfig/ast/tests_directives.rs",
            "src/kconfig/ast/tests_malformed.rs",
            "src/kconfig/ast/tests_preservation.rs",
            "src/kconfig/ast/tests_symbol_model.rs",
        ],
    );
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        kconfig.contains("KconfigSkippedSite") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose skipped-site preservation"
    );

    for required in [
        "pub(crate) struct KconfigSkippedSite",
        "SkippedSite(KconfigSkippedSite)",
        "pub(crate) fn skipped_sites(&self) -> impl Iterator<Item = &KconfigSkippedSite>",
        "KconfigNode::SkippedSite(KconfigSkippedSite {",
        "let Some(header) = parse_entry_header(lines[idx], line)? else",
        "raw: KconfigRawLine",
        "pub(crate) fn raw(&self) -> &KconfigRawLine",
        "line: usize",
        "end_line: usize",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig skipped-site preservation should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_preserves_unknown_syntax_as_skipped_sites",
        "document.skipped_sites()",
        "skipped_site_texts",
        "modules\\n",
        "optional FOO # keep raw unknown\\n",
        "\\tweird body syntax\\n",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig skipped-site tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("unknown top-level syntax")
            && architecture.contains("`KconfigSkippedSite`")
            && kernel_build_guide.contains("unknown top-level syntax")
            && kernel_build_guide.contains("`KconfigSkippedSite`"),
        "docs should describe skipped-site preservation"
    );
}
