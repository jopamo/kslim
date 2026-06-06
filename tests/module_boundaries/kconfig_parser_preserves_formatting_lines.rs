use super::common::*;

#[test]
fn kconfig_parser_preserves_formatting_lines() {
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
        kconfig.contains("KconfigBlankLine") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose formatting-line preservation"
    );

    for required in [
        "pub(crate) struct KconfigBlankLine",
        "BlankLine(KconfigBlankLine)",
        "pub(crate) fn blank_lines(&self) -> impl Iterator<Item = &KconfigBlankLine>",
        "KconfigNode::BlankLine(KconfigBlankLine {",
        "if is_kconfig_blank_line(lines[idx])",
        "fn is_kconfig_blank_line(line: &str) -> bool",
        "line.trim().is_empty()",
        "raw: KconfigRawLine",
        "pub(crate) fn raw(&self) -> &KconfigRawLine",
        "line: usize",
        "end_line: usize",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig formatting preservation should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_preserves_formatting_lines",
        "document.blank_lines()",
        "blank_line_texts",
        "\\t \\n",
        "\\t  \\n",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig formatting tests should cover {required:?}"
        );
    }

    assert!(
        architecture.contains("formatting blank lines")
            && architecture.contains("`KconfigBlankLine`")
            && kernel_build_guide.contains("formatting blank lines")
            && kernel_build_guide.contains("`KconfigBlankLine`"),
        "docs should describe formatting preservation"
    );
}
