use super::common::*;

#[test]
fn kconfig_expression_solver_detects_symbols_reenabled_by_defaults() {
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
        "struct KconfigDefaultReenabledSymbol",
        "fn detect_kconfig_symbols_reenabled_by_defaults(",
        "document.nodes()",
        "kconfig_node_symbol_defaults(node)",
        "default_definitions_by_symbol",
        "evaluate_first_active_kconfig_default(",
        "value != TristateLiteral::N",
        "symbol: symbol.to_string()",
        "fn kconfig_node_symbol_defaults(",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig expression solver should detect default re-enablements through {required}"
        );
    }

    for required in [
        "detect_kconfig_symbols_reenabled_by_defaults_reports_removed_non_n_defaults",
        "REMOVED_Y",
        "REMOVED_M",
        "REMOVED_CONDITION_OFF",
        "REMOVED_VALUE_FORCED_N",
        "REMOVED_DUP",
        "REMOVED_MENU",
        "REMOVED_CHOICE",
        "default REMOVED_GATE if LIVE",
        "menuconfig REMOVED_MENU",
        "choice REMOVED_CHOICE",
        "detect_kconfig_symbols_reenabled_by_defaults",
        "symbol.symbol().to_string()",
        "symbol.value()",
        "TristateLiteral::M",
        "TristateLiteral::Y",
    ] {
        assert!(
            expression_tests.contains(required),
            "Kconfig default re-enable detection tests should cover {required}"
        );
    }

    for required in [
        "test_rewrite_kconfig_stage_reports_removed_symbol_reenabled_by_default",
        "kconfig_solver_report.default_reenabled_symbols",
        "REMOVED_CONDITION_OFF",
        "REMOVED_DEFAULT",
    ] {
        assert!(
            prune.contains(required),
            "Kconfig prune-stage tests should preserve default re-enable reports through {required}"
        );
    }

    assert!(
        architecture.contains("detect symbols re-enabled by defaults")
            && architecture.contains("first active post-removal")
            && architecture.contains("fallback activation auditable")
            && kernel_build_guide.contains("detect symbols re-enabled by defaults")
            && kernel_build_guide.contains("first active post-removal default")
            && kernel_build_guide.contains("fallback activation auditable"),
        "docs should describe Kconfig default re-enable detection"
    );
}
