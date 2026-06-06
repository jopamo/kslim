use super::common::*;

#[test]
fn kconfig_symbol_model_models_modules() {
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
        kconfig.contains("KconfigModulesDefinition"),
        "src/kconfig/mod.rs should expose modules modeling"
    );

    for required in [
        "KconfigModulesDefinition",
        "pub(crate) fn line(&self) -> usize",
        "pub(crate) fn end_line(&self) -> usize",
        "pub(crate) fn directive(&self) -> &KconfigRawLine",
        "modules_definitions: Vec<KconfigModulesDefinition>",
        "pub(crate) fn modules_definitions(&self) -> impl Iterator<Item = &KconfigModulesDefinition>",
        "pub(crate) fn modules_definitions(&self) -> &[KconfigModulesDefinition]",
        "fn parse_modules_definitions(",
        "fn parse_modules_definition(line: &KconfigRawLine)",
        "trimmed != \"modules\"",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig modules model should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_models_modules",
        "document.modules_definitions()",
        "config.modules_definitions()",
        "menuconfig.modules_definitions()",
        "choice.modules_definitions()",
        "\\tmodules # keep modules note",
        "\\t  modules",
        "modules_lines(&document)",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig modules tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("modules markers")
            && architecture.contains("`KconfigModulesDefinition`")
            && kernel_build_guide.contains("modules markers")
            && kernel_build_guide.contains("`KconfigModulesDefinition`"),
        "docs should describe modules modeling"
    );
}
