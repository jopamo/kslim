use super::common::*;

#[test]
fn kconfig_symbol_model_models_multiple_symbol_definitions() {
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

    for required in [
        "KconfigSymbolDefinition",
        "KconfigSymbolDefinitionGroup",
        "KconfigSymbolDefinitionKind",
    ] {
        assert!(
            kconfig.contains(required),
            "src/kconfig/mod.rs should expose multiple symbol definition modeling through {required}"
        );
    }

    for required in [
        "KconfigSymbolDefinitionEntry",
        "Config(&'a KconfigConfigEntry)",
        "Menuconfig(&'a KconfigMenuconfigEntry)",
        "Choice(&'a KconfigChoiceEntry)",
        "pub(crate) fn kind(&self) -> KconfigSymbolDefinitionKind",
        "pub(crate) fn symbol(&self) -> &'a KconfigSymbol",
        "pub(crate) fn definitions(&self) -> &[KconfigSymbolDefinition<'a>]",
        "pub(crate) fn is_multiple(&self) -> bool",
        "pub(crate) fn symbol_definitions(",
        "pub(crate) fn symbol_definition_groups(&self) -> Vec<KconfigSymbolDefinitionGroup<'_>>",
        "pub(crate) fn multiple_symbol_definition_groups(",
        "BTreeMap",
        ".entry(definition.symbol())",
        "choice.symbol().is_some()",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig multiple-symbol model should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_models_multiple_symbol_definitions",
        "document.symbol_definitions()",
        "document.symbol_definition_groups()",
        "document.multiple_symbol_definition_groups()",
        "DUP_SYMBOL",
        "UNIQUE_MENU",
        "CHOICE_SYMBOL",
        "KconfigSymbolDefinitionKind::Config",
        "KconfigSymbolDefinitionKind::Menuconfig",
        "KconfigSymbolDefinitionKind::Choice",
        "duplicate_group.definitions().len()",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig multiple-symbol tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("multiple symbol definitions")
            && architecture.contains("`KconfigSymbolDefinitionGroup`")
            && kernel_build_guide.contains("multiple symbol definitions")
            && kernel_build_guide.contains("`KconfigSymbolDefinitionGroup`"),
        "docs should describe multiple symbol definition modeling"
    );
}
