use super::common::*;

#[test]
fn kconfig_expression_solver_detects_removed_symbols_weakly_enabled_by_imply() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "struct KconfigImplyWeaklyEnabledRemovedSymbol",
        "fn detect_kconfig_removed_symbols_weakly_enabled_by_imply(",
        "kconfig_node_symbol_implies(node)",
        "evaluate_kconfig_symbol_after_removed_symbols(",
        "imply.target().as_str()",
        "!removed_symbols.contains(target_symbol)",
        "imply.condition()",
        "let implied_value = tristate_and(source_value, condition);",
        "evaluate_kconfig_symbol_dependency_upper_bound_after_removed_symbols(",
        "let value = tristate_and(implied_value, target_dependency_upper_bound);",
        "value != TristateLiteral::N",
        "fn evaluate_kconfig_symbol_dependency_upper_bound_after_removed_symbols(",
        "fn kconfig_node_symbol_dependencies(",
        "fn kconfig_node_symbol_implies(",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should detect removed symbols weakly enabled by imply through {required}"
        );
    }

    for required in [
        "detect_kconfig_removed_symbols_weakly_enabled_by_imply_reports_live_imply_edges",
        "detect_kconfig_removed_symbols_weakly_enabled_by_imply_respects_target_dependencies",
        "REMOVED_BY_Y",
        "REMOVED_BY_M",
        "REMOVED_BY_N",
        "REMOVED_DEP_BLOCKED",
        "REMOVED_DEP_LOWERED_TO_M",
        "REMOVED_CONDITION_OFF",
        "REMOVED_FROM_REMOVED_SOURCE",
        "REMOVED_BY_MENU",
        "REMOVED_BY_CHOICE",
        "depends on BLOCKED_DEP",
        "depends on MODULE_DEP",
        "evaluate_kconfig_symbol_dependency_upper_bound_after_removed_symbols",
        "imply REMOVED_BY_M if LIVE_COND",
        "imply REMOVED_DEP_LOWERED_TO_M",
        "imply REMOVED_CONDITION_OFF if REMOVED_GATE",
        "menuconfig LIVE_MENU",
        "choice LIVE_CHOICE",
        "detect_kconfig_removed_symbols_weakly_enabled_by_imply",
        "implication.source_symbol().to_string()",
        "implication.target_symbol().to_string()",
        "implication.value()",
        "TristateLiteral::M",
        "TristateLiteral::Y",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig imply weak-enable detection tests should cover {required}"
        );
    }

    assert!(
        architecture.contains("removed symbols weakly enabled by `imply`")
            && architecture.contains("live source symbol value")
            && architecture.contains("bounding the weak value by the target symbol's own dependencies")
            && kernel_build_guide.contains("detect removed symbols weakly enabled by `imply`")
            && kernel_build_guide.contains("live source symbol value")
            && kernel_build_guide.contains("bounding the weak value by the target symbol's own dependencies"),
        "docs should describe Kconfig imply weak-enable detection"
    );
}
