use super::common::*;

#[test]
fn kconfig_symbol_model_validates_type_consistency_across_definitions() {
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
        "KconfigTypeConsistencyDefinition",
        "KconfigTypeConsistencyViolation",
    ] {
        assert!(
            kconfig.contains(required),
            "src/kconfig/mod.rs should expose type consistency validation through {required}"
        );
    }

    for required in [
        "KconfigTypeConsistencyDefinition",
        "kind: KconfigSymbolType",
        "symbol_definition_kind: KconfigSymbolDefinitionKind",
        "definition_line: usize",
        "type_line: usize",
        "KconfigTypeConsistencyViolation",
        "pub(crate) fn type_consistency_violation(",
        "pub(crate) fn type_consistency_violations(",
        "definition.type_definitions().iter()",
        "type_definition.kind()",
        "definition.kind()",
        "definition.line()",
        "type_definition.line()",
        "self.symbol.clone()",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig type-consistency model should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_validates_type_consistency_across_definitions",
        "document.type_consistency_violations()",
        "TYPE_CONFLICT",
        "SAME_TYPE",
        "CHOICE_CONFLICT",
        "KconfigSymbolType::Bool",
        "KconfigSymbolType::Tristate",
        "KconfigSymbolDefinitionKind::Config",
        "KconfigSymbolDefinitionKind::Menuconfig",
        "definition_line()",
        "type_line()",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig type-consistency tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("type consistency across definitions")
            && architecture.contains("`KconfigTypeConsistencyViolation`")
            && kernel_build_guide.contains("type consistency across definitions")
            && kernel_build_guide.contains("`KconfigTypeConsistencyViolation`"),
        "docs should describe Kconfig type consistency validation"
    );
}
