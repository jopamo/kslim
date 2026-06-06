use super::common::*;

#[test]
fn kconfig_symbol_model_models_reverse_dependencies_through_select() {
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
        kconfig.contains("KconfigSelectDefinition"),
        "src/kconfig/mod.rs should expose select reverse-dependency modeling"
    );

    for required in [
        "KconfigSelectDefinition",
        "target: KconfigSymbol",
        "condition: Option<String>",
        "pub(crate) fn target(&self) -> &KconfigSymbol",
        "pub(crate) fn condition(&self) -> Option<&str>",
        "select_definitions: Vec<KconfigSelectDefinition>",
        "pub(crate) fn select_definitions(&self) -> impl Iterator<Item = &KconfigSelectDefinition>",
        "pub(crate) fn select_definitions(&self) -> &[KconfigSelectDefinition]",
        "fn parse_select_definitions(",
        "fn parse_select_definition(line: &KconfigRawLine)",
        "trimmed.strip_prefix(\"select\")",
        "split_kconfig_if_clause(rest.trim_start())",
        "KconfigSymbol::new(target)",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig select model should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_models_reverse_dependencies_through_select",
        "document.select_definitions()",
        "config.select_definitions()",
        "menuconfig.select_definitions()",
        "choice.select_definitions()",
        "\\tselect NET_CORE if NET # keep select note",
        "\\tselect RFKILL",
        "\\t  select BROKEN if HELP_TEXT",
        "Some(\"CHOICE_VISIBLE\")",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig select tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("reverse dependencies through `select`")
            && architecture.contains("`KconfigSelectDefinition`")
            && kernel_build_guide.contains("reverse dependencies through `select`")
            && kernel_build_guide.contains("`KconfigSelectDefinition`"),
        "docs should describe select reverse-dependency modeling"
    );
}
