use super::common::*;

#[test]
fn kconfig_expression_solver_detects_orphaned_symbol_definitions() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "struct KconfigOrphanedSymbolDefinition",
        "fn detect_kconfig_orphaned_symbol_definitions(",
        "kconfig_symbols_with_live_reverse_dependencies(",
        "kconfig_node_symbol_definition_solver_inputs(node)",
        "selected_kconfig_symbol_value_after_removed_symbols(",
        "live_reverse_dependencies.contains(definition.symbol)",
        "evaluate_kconfig_visibility_after_removed_symbols(",
        "evaluate_kconfig_defaults_after_removed_symbols(",
        "KconfigSymbolDefinitionKind::Config",
        "KconfigSymbolDefinitionKind::Menuconfig",
        "KconfigSymbolDefinitionKind::Choice",
        "fn kconfig_symbols_with_live_reverse_dependencies(",
        "tristate_and(source_value, condition) != TristateLiteral::N",
        ".unwrap_or(TristateLiteral::N)",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should detect orphaned symbol definitions through {required}"
        );
    }

    for required in [
        "detect_kconfig_orphaned_symbol_definitions_reports_unactivated_unreachable_definitions",
        "ORPHANED_DEP",
        "ORPHANED_HIDDEN",
        "LIVE_PROMPT",
        "SELECT_SOURCE",
        "SELECTED_TARGET",
        "IMPLY_SOURCE",
        "IMPLIED_TARGET",
        "DEFAULTED_TARGET",
        "PROFILE_SELECTED",
        "REMOVED_SYMBOL",
        "menuconfig ORPHANED_MENU",
        "choice ORPHANED_CHOICE",
        "detect_kconfig_orphaned_symbol_definitions",
        "definition.symbol().to_string()",
        "definition.definition_kind()",
        "definition.line()",
        "definition.visibility()",
        "KconfigSymbolDefinitionKind::Menuconfig",
        "KconfigSymbolDefinitionKind::Choice",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig orphaned-symbol detection tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("detect orphaned symbol definitions")
            && architecture.contains("no selected value")
            && architecture.contains("live reverse dependency")
            && architecture.contains("active")
            && architecture.contains("non-`n` default")
            && kernel_build_guide.contains("detect orphaned symbol definitions")
            && kernel_build_guide.contains("no selected value, live reverse dependency, reachable visibility, or active non-`n` default"),
        "docs should describe Kconfig orphaned-symbol detection"
    );
}
