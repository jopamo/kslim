use super::common::*;

#[test]
fn kconfig_symbol_model_models_weak_reverse_dependencies_through_imply() {
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
        kconfig.contains("KconfigImplyDefinition"),
        "src/kconfig/mod.rs should expose imply weak reverse-dependency modeling"
    );

    for required in [
        "KconfigImplyDefinition",
        "target: KconfigSymbol",
        "condition: Option<String>",
        "pub(crate) fn target(&self) -> &KconfigSymbol",
        "pub(crate) fn condition(&self) -> Option<&str>",
        "imply_definitions: Vec<KconfigImplyDefinition>",
        "pub(crate) fn imply_definitions(&self) -> impl Iterator<Item = &KconfigImplyDefinition>",
        "pub(crate) fn imply_definitions(&self) -> &[KconfigImplyDefinition]",
        "fn parse_imply_definitions(",
        "fn parse_imply_definition(line: &KconfigRawLine)",
        "trimmed.strip_prefix(\"imply\")",
        "split_kconfig_if_clause(rest.trim_start())",
        "KconfigSymbol::new(target)",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig imply model should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_models_weak_reverse_dependencies_through_imply",
        "document.imply_definitions()",
        "config.imply_definitions()",
        "menuconfig.imply_definitions()",
        "choice.imply_definitions()",
        "\\timply NET_CORE if NET # keep imply note",
        "\\timply RFKILL",
        "\\t  imply BROKEN if HELP_TEXT",
        "Some(\"CHOICE_VISIBLE\")",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig imply tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("weak reverse dependencies through `imply`")
            && architecture.contains("`KconfigImplyDefinition`")
            && kernel_build_guide.contains("weak reverse dependencies through `imply`")
            && kernel_build_guide.contains("`KconfigImplyDefinition`"),
        "docs should describe imply weak reverse-dependency modeling"
    );
}
