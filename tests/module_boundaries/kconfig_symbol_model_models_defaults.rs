use super::common::*;

#[test]
fn kconfig_symbol_model_models_defaults() {
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
        kconfig.contains("KconfigDefaultDefinition"),
        "src/kconfig/mod.rs should expose default modeling"
    );

    for required in [
        "KconfigDefaultDefinition",
        "value: String",
        "condition: Option<String>",
        "pub(crate) fn value(&self) -> &str",
        "pub(crate) fn condition(&self) -> Option<&str>",
        "default_definitions: Vec<KconfigDefaultDefinition>",
        "pub(crate) fn default_definitions(&self) -> impl Iterator<Item = &KconfigDefaultDefinition>",
        "pub(crate) fn default_definitions(&self) -> &[KconfigDefaultDefinition]",
        "fn parse_default_definitions(",
        "fn parse_default_definition(line: &KconfigRawLine)",
        "trimmed.strip_prefix(\"default\")",
        "split_kconfig_if_clause(rest.trim_start())",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig default model should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_models_defaults",
        "document.default_definitions()",
        "config.default_definitions()",
        "menuconfig.default_definitions()",
        "choice.default_definitions()",
        "\\tdefault y if EXPERT # keep default note",
        "\\tdefault \\\"hello world\\\"",
        "\\t  default n if BROKEN",
        "Some(\"CHOICE_VISIBLE\")",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig default tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("default values")
            && architecture.contains("`KconfigDefaultDefinition`")
            && kernel_build_guide.contains("default values")
            && kernel_build_guide.contains("`KconfigDefaultDefinition`"),
        "docs should describe default modeling"
    );
}
