use super::common::*;

#[test]
fn kconfig_symbol_model_models_hex() {
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
        kconfig.contains("KconfigSymbolType") && kconfig.contains("KconfigTypeDefinition"),
        "src/kconfig/mod.rs should expose hex type modeling"
    );

    for required in [
        "Hex",
        "KconfigSymbolType::Hex",
        "fn is_kconfig_hex_type_line(trimmed: &str) -> bool",
        "trimmed.strip_prefix(\"hex\")",
        "parse_type_definition(line: &KconfigRawLine)",
        "kind: KconfigSymbolType",
        "type_definitions: Vec<KconfigTypeDefinition>",
        "pub(crate) fn type_definitions(&self) -> impl Iterator<Item = &KconfigTypeDefinition>",
        "pub(crate) fn type_definitions(&self) -> &[KconfigTypeDefinition]",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig hex symbol model should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_models_hex_type_definitions",
        "KconfigSymbolType::Hex",
        "document.type_definitions()",
        "config.type_definitions()",
        "menuconfig.type_definitions()",
        "choice.type_definitions()",
        "\\thex \\\"Prompt\\\" if EXPERT # keep type note",
        "\\t  hex \\\"not a type\\\"",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig hex model tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("`hex` type lines")
            && architecture.contains("`KconfigSymbolType::Hex`")
            && kernel_build_guide.contains("`hex` type lines")
            && kernel_build_guide.contains("`KconfigSymbolType::Hex`"),
        "docs should describe hex type modeling"
    );
}
