use super::common::*;

#[test]
fn kconfig_symbol_model_models_dependencies() {
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
        kconfig.contains("KconfigDependencyDefinition"),
        "src/kconfig/mod.rs should expose dependency modeling"
    );

    for required in [
        "KconfigDependencyDefinition",
        "expression: String",
        "pub(crate) fn expression(&self) -> &str",
        "dependency_definitions: Vec<KconfigDependencyDefinition>",
        "pub(crate) fn dependency_definitions(",
        "impl Iterator<Item = &KconfigDependencyDefinition>",
        "pub(crate) fn dependency_definitions(&self) -> &[KconfigDependencyDefinition]",
        "fn parse_dependency_definitions(",
        "fn parse_dependency_definition(",
        "trimmed.strip_prefix(\"depends\")",
        "rest.strip_prefix(\"on\")",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig dependency model should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_models_dependencies",
        "document.dependency_definitions()",
        "config.dependency_definitions()",
        "menuconfig.dependency_definitions()",
        "choice.dependency_definitions()",
        "\\tdepends on NET && (PCI || USB) # keep dependency note",
        "\\tdepends on MODULES",
        "\\t  depends on BROKEN",
        "CHOICE_VISIBLE || EXPERT",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig dependency tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("dependency expressions")
            && architecture.contains("`KconfigDependencyDefinition`")
            && kernel_build_guide.contains("dependency expressions")
            && kernel_build_guide.contains("`KconfigDependencyDefinition`"),
        "docs should describe dependency modeling"
    );
}
