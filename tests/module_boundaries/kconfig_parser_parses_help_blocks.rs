use super::common::*;

#[test]
fn kconfig_parser_parses_help_blocks() {
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
        kconfig.contains("KconfigHelpBlock") && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose help-block AST parsing"
    );

    for required in [
        "pub(crate) struct KconfigHelpBlock",
        "help_blocks: Vec<KconfigHelpBlock>",
        "pub(crate) fn help_blocks(&self) -> impl Iterator<Item = &KconfigHelpBlock>",
        "pub(crate) fn help_blocks(&self) -> &[KconfigHelpBlock]",
        "parse_help_blocks(&body)",
        "fn parse_help_blocks(body: &[KconfigRawLine]) -> Vec<KconfigHelpBlock>",
        "fn is_kconfig_help_block_directive(trimmed: &str) -> bool",
        "trimmed == \"help\"",
        "trimmed.starts_with(\"help \")",
        "trimmed == \"---help---\"",
        "text: Vec<KconfigRawLine>",
        "directive: KconfigRawLine",
        "line: usize",
        "end_line: usize",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig help-block parser should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_parses_help_blocks",
        "document.help_blocks()",
        "config.help_blocks()",
        "menuconfig.help_blocks()",
        "choice.help_blocks()",
        "\\t---help---",
        "\\t  config NOT_A_SYMBOL",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig help-block tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("`help` blocks")
            && architecture.contains("`KconfigHelpBlock`")
            && kernel_build_guide.contains("`help` blocks")
            && kernel_build_guide.contains("`KconfigHelpBlock`"),
        "docs should describe the help-block parser slice"
    );
}
