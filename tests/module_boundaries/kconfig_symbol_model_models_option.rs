use super::common::*;

#[test]
fn kconfig_symbol_model_models_option() {
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
        kconfig.contains("KconfigOptionDefinition"),
        "src/kconfig/mod.rs should expose option modeling"
    );

    for required in [
        "KconfigOptionDefinition",
        "name: String",
        "value: Option<String>",
        "pub(crate) fn name(&self) -> &str",
        "pub(crate) fn value(&self) -> Option<&str>",
        "option_definitions: Vec<KconfigOptionDefinition>",
        "pub(crate) fn option_definitions(&self) -> impl Iterator<Item = &KconfigOptionDefinition>",
        "pub(crate) fn option_definitions(&self) -> &[KconfigOptionDefinition]",
        "fn parse_option_definitions(",
        "fn parse_option_definition(line: &KconfigRawLine)",
        "trimmed.strip_prefix(\"option\")",
        "payload.split_whitespace()",
        "token.split_once('=')",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig option model should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_models_option",
        "document.option_definitions()",
        "config.option_definitions()",
        "menuconfig.option_definitions()",
        "choice.option_definitions()",
        "\\toption env=\\\"CONFIG_OPTION_ENV\\\" # keep option note",
        "\\toption allnoconfig_y",
        "\\t  option ignored=HELP_TEXT",
        "Some(\"\\\"CONFIG_OPTION_ENV\\\"\")",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig option tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("option properties")
            && architecture.contains("`KconfigOptionDefinition`")
            && kernel_build_guide.contains("option properties")
            && kernel_build_guide.contains("`KconfigOptionDefinition`"),
        "docs should describe option modeling"
    );
}
