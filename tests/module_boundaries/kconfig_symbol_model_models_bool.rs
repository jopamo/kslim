use super::common::*;

#[test]
fn kconfig_symbol_model_models_bool() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = format!(
        "{}\n{}\n{}",
        production_source(&root.join("src/kconfig/ast.rs")),
        production_source(&root.join("src/kconfig/ast/document_model.rs")),
        production_source(&root.join("src/kconfig/ast/symbol_model.rs"))
    );
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
        kconfig.contains("KconfigSymbolType")
            && kconfig.contains("KconfigTypeDefinition")
            && kconfig.contains("parse_kconfig_document"),
        "src/kconfig/mod.rs should expose bool type modeling"
    );

    for required in [
        "pub(crate) enum KconfigSymbolType",
        "Bool",
        "pub(crate) struct KconfigTypeDefinition",
        "kind: KconfigSymbolType",
        "type_definitions: Vec<KconfigTypeDefinition>",
        "pub(crate) fn type_definitions(&self) -> impl Iterator<Item = &KconfigTypeDefinition>",
        "pub(crate) fn type_definitions(&self) -> &[KconfigTypeDefinition]",
        "parse_type_definitions(&body)",
        "fn parse_type_definitions(",
        "fn parse_type_definition(line: &KconfigRawLine) -> Option<KconfigTypeDefinition>",
        "fn is_kconfig_bool_type_line(trimmed: &str) -> bool",
        "KconfigSymbolType::Bool",
        "split_kconfig_trailing_comment(line.text())",
        "directive: KconfigRawLine",
        "line: usize",
        "end_line: usize",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig bool symbol model should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_models_bool_type_definitions",
        "document.type_definitions()",
        "config.type_definitions()",
        "menuconfig.type_definitions()",
        "choice.type_definitions()",
        "KconfigSymbolType::Bool",
        "\\tbool \\\"Prompt\\\" if EXPERT # keep type note",
        "\\t  bool \\\"not a type\\\"",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig bool model tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("`bool` type lines")
            && architecture.contains("`KconfigSymbolType::Bool`")
            && kernel_build_guide.contains("`bool` type lines")
            && kernel_build_guide.contains("`KconfigSymbolType::Bool`"),
        "docs should describe bool type modeling"
    );
}
