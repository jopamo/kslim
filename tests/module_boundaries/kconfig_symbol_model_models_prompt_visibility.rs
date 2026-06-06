use super::common::*;

#[test]
fn kconfig_symbol_model_models_prompt_visibility() {
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
        kconfig.contains("KconfigPromptDefinition"),
        "src/kconfig/mod.rs should expose prompt visibility modeling"
    );

    for required in [
        "KconfigPromptDefinition",
        "condition: Option<String>",
        "pub(crate) fn condition(&self) -> Option<&str>",
        "prompt_definitions: Vec<KconfigPromptDefinition>",
        "pub(crate) fn prompt_definitions(&self) -> impl Iterator<Item = &KconfigPromptDefinition>",
        "pub(crate) fn prompt_definitions(&self) -> &[KconfigPromptDefinition]",
        "fn parse_prompt_definitions(",
        "fn parse_prompt_definition(line: &KconfigRawLine)",
        "fn parse_prompt_payload(",
        "trimmed.strip_prefix(\"prompt\")",
        "split_kconfig_if_clause(trailing)",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig prompt visibility model should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_models_prompt_visibility",
        "document.prompt_definitions()",
        "config.prompt_definitions()",
        "menuconfig.prompt_definitions()",
        "choice.prompt_definitions()",
        "\\tbool \\\"Typed Prompt\\\" if EXPERT # keep prompt note",
        "\\tprompt \\\"Explicit Prompt\\\" if MODULES",
        "\\t  prompt \\\"not a prompt\\\" if BROKEN",
        "Some(\"CHOICE_VISIBLE\")",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig prompt visibility tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("prompt visibility")
            && architecture.contains("`KconfigPromptDefinition`")
            && kernel_build_guide.contains("prompt visibility")
            && kernel_build_guide.contains("`KconfigPromptDefinition`"),
        "docs should describe prompt visibility modeling"
    );
}
