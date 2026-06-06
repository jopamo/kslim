use super::common::*;

#[test]
fn kconfig_symbol_model_models_tristate() {
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
        "src/kconfig/mod.rs should expose tristate type modeling"
    );

    for required in [
        "Tristate",
        "KconfigSymbolType::Tristate",
        "fn is_kconfig_tristate_type_line(trimmed: &str) -> bool",
        "trimmed.strip_prefix(\"tristate\")",
        "parse_type_definition(line: &KconfigRawLine)",
        "kind: KconfigSymbolType",
        "type_definitions: Vec<KconfigTypeDefinition>",
        "pub(crate) fn type_definitions(&self) -> impl Iterator<Item = &KconfigTypeDefinition>",
        "pub(crate) fn type_definitions(&self) -> &[KconfigTypeDefinition]",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig tristate symbol model should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_models_tristate_type_definitions",
        "KconfigSymbolType::Tristate",
        "document.type_definitions()",
        "config.type_definitions()",
        "menuconfig.type_definitions()",
        "choice.type_definitions()",
        "\\ttristate \\\"Prompt\\\" if MODULES # keep type note",
        "\\t  tristate \\\"not a type\\\"",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig tristate model tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("`tristate` type lines")
            && architecture.contains("`KconfigSymbolType::Tristate`")
            && kernel_build_guide.contains("`tristate` type lines")
            && kernel_build_guide.contains("`KconfigSymbolType::Tristate`"),
        "docs should describe tristate type modeling"
    );
}
