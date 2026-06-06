use super::common::*;

#[test]
fn kconfig_parser_parses_comment_entries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = production_source(&root.join("src/kconfig/ast.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        kconfig.contains("KconfigCommentEntry") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose comment AST entry parsing"
    );

    for required in [
        "pub(crate) struct KconfigCommentEntry",
        "Comment(KconfigCommentEntry)",
        "pub(crate) fn comments(&self) -> impl Iterator<Item = &KconfigCommentEntry>",
        "prompt: String",
        "body: Vec<KconfigRawLine>",
        "KconfigEntryHeaderKind::Comment",
        "parse_quoted_string_entry_header(trimmed, \"comment\", line_number)?",
        "parse_quoted_string_literal",
        "line: usize",
        "end_line: usize",
        "directive: KconfigRawLine",
        "comment parser should require a prompt",
        "missing a prompt",
        "missing a quoted prompt",
        "unterminated quoted prompt",
        "unexpected trailing tokens",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig comment-entry parser should own {required}"
        );
    }

    assert!(
        architecture.contains("`comment` entries")
            && architecture.contains("`KconfigCommentEntry`")
            && kernel_build_guide.contains("`comment` entries")
            && kernel_build_guide.contains("`KconfigCommentEntry`"),
        "docs should describe the comment-entry parser slice"
    );
}
