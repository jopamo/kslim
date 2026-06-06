use super::common::*;

#[test]
fn kconfig_symbol_model_models_ranges() {
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
        kconfig.contains("KconfigRangeDefinition"),
        "src/kconfig/mod.rs should expose range modeling"
    );

    for required in [
        "KconfigRangeDefinition",
        "minimum: String",
        "maximum: String",
        "condition: Option<String>",
        "pub(crate) fn minimum(&self) -> &str",
        "pub(crate) fn maximum(&self) -> &str",
        "pub(crate) fn condition(&self) -> Option<&str>",
        "range_definitions: Vec<KconfigRangeDefinition>",
        "pub(crate) fn range_definitions(&self) -> impl Iterator<Item = &KconfigRangeDefinition>",
        "pub(crate) fn range_definitions(&self) -> &[KconfigRangeDefinition]",
        "fn parse_range_definitions(",
        "fn parse_range_definition(line: &KconfigRawLine)",
        "trimmed.strip_prefix(\"range\")",
        "split_kconfig_if_clause(rest.trim_start())",
        "bounds.split_whitespace()",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig range model should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_models_ranges",
        "document.range_definitions()",
        "config.range_definitions()",
        "menuconfig.range_definitions()",
        "choice.range_definitions()",
        "\\trange 0 255 if EXPERT # keep range note",
        "\\trange 0x10 0xff",
        "\\t  range 1 2 if BROKEN",
        "Some(\"CHOICE_VISIBLE\")",
    ] {
        assert!(
            ast_tests.contains(required),
            "Kconfig range tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("range constraints")
            && architecture.contains("`KconfigRangeDefinition`")
            && kernel_build_guide.contains("range constraints")
            && kernel_build_guide.contains("`KconfigRangeDefinition`"),
        "docs should describe range modeling"
    );
}
