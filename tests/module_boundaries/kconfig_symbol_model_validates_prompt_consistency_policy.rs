use super::common::*;

#[test]
fn kconfig_symbol_model_validates_prompt_consistency_policy() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = format!(
        "{}\n{}",
        production_source(&root.join("src/kconfig/ast.rs")),
        production_source(&root.join("src/kconfig/ast/document_model.rs"))
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
        "KconfigPromptConsistencyDefinition",
        "KconfigPromptConsistencyViolation",
    ] {
        assert!(
            kconfig.contains(required),
            "src/kconfig/mod.rs should expose prompt consistency policy through {required}"
        );
    }

    for required in [
        "KconfigPromptConsistencyDefinition",
        "prompt: String",
        "condition: Option<String>",
        "symbol_definition_kind: KconfigSymbolDefinitionKind",
        "definition_line: usize",
        "prompt_line: usize",
        "KconfigPromptConsistencyViolation",
        "pub(crate) fn prompt_consistency_violation(",
        "pub(crate) fn prompt_consistency_violations(",
        "prompt_definitions.len() <= 1",
        "prompt_definition.prompt()",
        "prompt_definition.condition()",
        "prompt_definition.line()",
        "prompt_definition.directive().clone()",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig prompt-consistency policy model should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_validates_prompt_consistency_policy",
        "document.prompt_consistency_violations()",
        "CONFIG_PROMPT_CONFLICT",
        "MENU_PROMPT_CONFLICT",
        "CHOICE_PROMPT_CONFLICT",
        "SINGLE_PROMPT",
        "KconfigPromptConsistencyDefinition::prompt",
        "KconfigSymbolDefinitionKind::Config",
        "KconfigSymbolDefinitionKind::Menuconfig",
        "KconfigSymbolDefinitionKind::Choice",
        "definition_line()",
        "prompt_line()",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig prompt-consistency tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("prompt consistency policy")
            && architecture.contains("`KconfigPromptConsistencyViolation`")
            && kernel_build_guide.contains("prompt consistency policy")
            && kernel_build_guide.contains("`KconfigPromptConsistencyViolation`"),
        "docs should describe Kconfig prompt consistency validation"
    );
}
