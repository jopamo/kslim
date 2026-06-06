use super::common::*;

#[test]
fn kconfig_solver_analysis_lives_in_solver_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_source(&root.join("src/kconfig/mod.rs"));
    let solver = production_source(&root.join("src/kconfig/solver.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod solver;",
        "detect_kconfig_empty_menus",
        "detect_kconfig_orphaned_symbol_definitions",
        "parse_selected_profile_tristate_values",
    ] {
        assert!(
            kconfig.contains(required),
            "src/kconfig/mod.rs should delegate solver analysis through {required}"
        );
    }

    for required in [
        "pub(super) struct KconfigDefaultReenabledSymbol",
        "pub(super) struct KconfigSelectForcedRemovedSymbol",
        "pub(super) struct KconfigOrphanedSymbolDefinition",
        "pub(super) fn detect_kconfig_symbols_reenabled_by_defaults(",
        "pub(super) fn detect_kconfig_removed_symbols_forced_by_select(",
        "pub(super) fn detect_kconfig_empty_menus(",
        "pub(super) fn detect_kconfig_orphaned_symbol_definitions(",
        "pub(super) fn evaluate_kconfig_visibility(",
    ] {
        assert!(
            solver.contains(required),
            "src/kconfig/solver.rs should own solver analysis through {required}"
        );
    }

    for forbidden in [
        "\nstruct KconfigDefaultReenabledSymbol",
        "\nfn detect_kconfig_symbols_reenabled_by_defaults(",
        "\nfn detect_kconfig_empty_menus(",
        "\nfn evaluate_kconfig_visibility(",
        "\nfn kconfig_node_symbol_defaults(",
    ] {
        assert!(
            !kconfig.contains(forbidden),
            "src/kconfig/mod.rs should not retain extracted solver implementation {forbidden}"
        );
    }

    for required in [
        "`src/kconfig/solver.rs`",
        "visibility, defaults, reverse dependencies, impossible choices, empty menus, and orphaned definitions",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document extracted Kconfig solver ownership through {required}"
        );
    }
}
