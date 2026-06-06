use super::common::*;

#[test]
fn kconfig_expression_solver_detects_removed_symbols_forced_by_select() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(
        &root,
        &["src/kconfig/mod.rs", "src/kconfig/expression.rs", "src/kconfig/solver.rs"],
    );
    let expression_tests = production_source(&root.join("src/kconfig/expression_tests.rs"));
    let prune = std::fs::read_to_string(root.join("src/prune.rs")).expect("failed to read prune");
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "struct KconfigSelectForcedRemovedSymbol",
        "fn detect_kconfig_removed_symbols_forced_by_select(",
        "kconfig_node_symbol_selects(node)",
        "evaluate_kconfig_symbol_after_removed_symbols(",
        "select.target().as_str()",
        "!removed_symbols.contains(target_symbol)",
        "select.condition()",
        "let value = tristate_and(source_value, condition);",
        "value != TristateLiteral::N",
        "fn kconfig_node_symbol_selects(",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should detect removed symbols forced by select through {required}"
        );
    }

    for required in [
        "detect_kconfig_removed_symbols_forced_by_select_reports_live_select_edges",
        "detect_kconfig_removed_symbols_forced_by_select_bypasses_target_dependencies",
        "REMOVED_BY_Y",
        "REMOVED_BY_M",
        "REMOVED_BY_N",
        "REMOVED_DEP_BLOCKED",
        "REMOVED_COND_DEP_BLOCKED",
        "REMOVED_CONDITION_OFF",
        "REMOVED_FROM_REMOVED_SOURCE",
        "REMOVED_BY_MENU",
        "REMOVED_BY_CHOICE",
        "depends on BLOCKED_DEP",
        "evaluate_kconfig_visibility_after_removed_symbols(",
        "select REMOVED_BY_M if LIVE_COND",
        "select REMOVED_COND_DEP_BLOCKED if LIVE_COND",
        "select REMOVED_CONDITION_OFF if REMOVED_GATE",
        "menuconfig LIVE_MENU",
        "choice LIVE_CHOICE",
        "detect_kconfig_removed_symbols_forced_by_select",
        "selection.source_symbol().to_string()",
        "selection.target_symbol().to_string()",
        "selection.value()",
        "TristateLiteral::M",
        "TristateLiteral::Y",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig select-force detection tests should cover {required}"
        );
    }

    for required in [
        "test_rewrite_kconfig_stage_reports_removed_symbol_selected_by_live_feature",
        "kconfig_solver_report.forced_selects",
        "CONFIG_LIVE_FEATURE=y",
        "select REMOVED_SELECTED if LIVE_GATE",
        "depends on BLOCKED_DEP",
        "dropped_selects",
    ] {
        assert!(
            prune.contains(required),
            "Kconfig prune-stage tests should preserve live select reports through {required}"
        );
    }

    assert!(
        architecture.contains("detect removed symbols forced by `select`")
            && architecture.contains("live source symbol value")
            && architecture.contains("bypasses the target symbol's own dependencies")
            && architecture.contains("Live-feature selections of removed symbols")
            && kernel_build_guide.contains("detect removed symbols forced by `select`")
            && kernel_build_guide.contains("live source symbol value")
            && kernel_build_guide.contains("bypasses the target symbol's own dependencies")
            && kernel_build_guide.contains("Live-feature selections of removed symbols"),
        "docs should describe Kconfig select-force detection"
    );
}
