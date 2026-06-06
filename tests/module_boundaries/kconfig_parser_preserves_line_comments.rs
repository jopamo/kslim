use super::common::*;

#[test]
fn kconfig_parser_preserves_line_comments() {
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
        kconfig.contains("KconfigLineComment") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose line-comment preservation"
    );

    for required in [
        "pub(crate) struct KconfigLineComment",
        "LineComment(KconfigLineComment)",
        "pub(crate) fn line_comments(&self) -> impl Iterator<Item = &KconfigLineComment>",
        "KconfigNode::LineComment(KconfigLineComment {",
        "if is_kconfig_line_comment(lines[idx])",
        "fn is_kconfig_line_comment(line: &str) -> bool",
        "line.trim_start().starts_with('#')",
        "raw: KconfigRawLine",
        "pub(crate) fn raw(&self) -> &KconfigRawLine",
        "line: usize",
        "end_line: usize",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig line-comment preservation should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_preserves_line_comments",
        "document.line_comments()",
        "line_comment_texts",
        "# SPDX-License-Identifier: GPL-2.0",
        "\\t# body note",
        "config FOO # inline directive note",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig line-comment tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("`#` line comments")
            && architecture.contains("`KconfigLineComment`")
            && kernel_build_guide.contains("`#` line comments")
            && kernel_build_guide.contains("`KconfigLineComment`"),
        "docs should describe line-comment preservation"
    );
}
