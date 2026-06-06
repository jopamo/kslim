use super::common::*;

#[test]
fn kconfig_ast_tests_are_behavior_focused() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ast_tests = production_source(&root.join("src/kconfig/ast/tests.rs"));
    let directives = production_source(&root.join("src/kconfig/ast/tests_directives.rs"));
    let malformed = production_source(&root.join("src/kconfig/ast/tests_malformed.rs"));
    let preservation = production_source(&root.join("src/kconfig/ast/tests_preservation.rs"));
    let symbol_model = production_source(&root.join("src/kconfig/ast/tests_symbol_model.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "#[path = \"tests_directives.rs\"]\nmod directives;",
        "#[path = \"tests_malformed.rs\"]\nmod malformed;",
        "#[path = \"tests_preservation.rs\"]\nmod preservation;",
        "#[path = \"tests_symbol_model.rs\"]\nmod symbol_model;",
    ] {
        assert!(
            ast_tests.contains(required),
            "src/kconfig/ast/tests.rs should register behavior-focused test module {required}"
        );
    }

    for forbidden in ["#[test]", "parse_kconfig_document_models_bool_type_definitions"] {
        assert!(
            !ast_tests.contains(forbidden),
            "src/kconfig/ast/tests.rs should keep shared helpers only; found {forbidden}"
        );
    }

    assert!(
        directives.contains("parse_kconfig_document_parses_config_entries")
            && directives.contains("parse_kconfig_document_parses_source_entries")
            && directives.contains("parse_kconfig_document_parses_help_blocks"),
        "src/kconfig/ast/tests_directives.rs should own successful directive parsing tests"
    );
    assert!(
        malformed.contains("parse_kconfig_document_rejects_malformed_config_headers")
            && malformed.contains("parse_kconfig_document_rejects_malformed_orsource_headers"),
        "src/kconfig/ast/tests_malformed.rs should own malformed directive rejection tests"
    );
    assert!(
        preservation.contains("parse_kconfig_document_preserves_unknown_syntax_as_skipped_sites")
            && preservation.contains("parse_kconfig_document_ignores_config_text_inside_help"),
        "src/kconfig/ast/tests_preservation.rs should own formatting/preservation tests"
    );
    assert!(
        symbol_model.contains("parse_kconfig_document_models_bool_type_definitions")
            && symbol_model.contains("parse_kconfig_document_models_multiple_symbol_definitions")
            && symbol_model
                .contains("parse_kconfig_document_validates_prompt_consistency_policy"),
        "src/kconfig/ast/tests_symbol_model.rs should own symbol model and policy tests"
    );

    assert!(
        architecture.contains("Kconfig AST/parser unit tests are split by behavior"),
        "docs/architecture.md should document Kconfig AST test ownership"
    );
}
