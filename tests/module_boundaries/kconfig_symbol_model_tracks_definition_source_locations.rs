use super::common::*;

#[test]
fn kconfig_symbol_model_tracks_definition_source_locations() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let ast = format!(
        "{}\n{}",
        production_source(&root.join("src/kconfig/ast.rs")),
        production_source(&root.join("src/kconfig/ast/document_model.rs"))
    );
    let source_location_tests =
        production_source(&root.join("src/kconfig/ast/source_location_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        kconfig.contains("KconfigDefinitionSourceLocation"),
        "src/kconfig/mod.rs should expose definition source locations"
    );

    for required in [
        "KconfigDefinitionSourceLocation",
        "symbol: &'a KconfigSymbol",
        "kind: KconfigSymbolDefinitionKind",
        "line: usize",
        "end_line: usize",
        "directive: &'a KconfigRawLine",
        "pub(crate) fn source_location(&self) -> KconfigDefinitionSourceLocation<'a>",
        "pub(crate) fn source_locations(&self) -> Vec<KconfigDefinitionSourceLocation<'a>>",
        "pub(crate) fn symbol_definition_source_locations(",
        "symbol: self.symbol()",
        "kind: self.kind()",
        "line: self.line()",
        "end_line: self.end_line()",
        "directive: self.directive()",
    ] {
        assert!(
            ast.contains(required),
            "Kconfig definition source-location model should own {required}"
        );
    }

    for required in [
        "parse_kconfig_document_tracks_definition_source_locations",
        "symbol_definition_source_locations()",
        "choice_group.source_locations()",
        ".source_location()",
        "LOCATION_CONFIG",
        "LOCATION_MENU",
        "LOCATION_CHOICE",
        "KconfigDefinitionSourceLocation::line",
        "KconfigDefinitionSourceLocation::end_line",
        "KconfigSymbolDefinitionKind::Config",
        "KconfigSymbolDefinitionKind::Menuconfig",
        "KconfigSymbolDefinitionKind::Choice",
    ] {
        assert!(
            source_location_tests.contains(required),
            "Kconfig definition source-location tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("definition source locations")
            && architecture.contains("`KconfigDefinitionSourceLocation`")
            && kernel_build_guide.contains("definition source locations")
            && kernel_build_guide.contains("`KconfigDefinitionSourceLocation`"),
        "docs should describe Kconfig definition source locations"
    );
}
