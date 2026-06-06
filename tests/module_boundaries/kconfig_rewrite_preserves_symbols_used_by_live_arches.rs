use super::common::*;

#[test]
fn kconfig_rewrite_preserves_symbols_used_by_live_arches() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(&root, &["src/kconfig/mod.rs", "src/kconfig/rewrite.rs"]);
    let kconfig_with_tests = production_sources(
        &root,
        &[
            "src/kconfig/tests.rs",
            "src/kconfig/tests_report.rs",
            "src/kconfig/tests_rewrite.rs",
            "src/kconfig/tests_root_facade.rs",
            "src/kconfig/tests_solver.rs",
        ],
    );
    let architecture =
        std::fs::read_to_string(root.join("docs/architecture.md")).expect("failed to read docs");
    let kernel_build = kernel_build_iteration_docs(&root);

    for required in [
        "struct KconfigLiveArchSymbolUsage",
        "fn kconfig_live_arch_symbol_usage",
        "fn kconfig_path_is_live_arch_kconfig",
        "fn kconfig_collect_live_arch_symbol_usage",
        "fn kconfig_collect_live_arch_expr_usage",
        "live_arch_symbol_usage.preserves_symbol(definition.symbol())",
    ] {
        assert!(
            kconfig.contains(required),
            "dead Kconfig symbol proofs should preserve live-arch symbol usage; missing {required}"
        );
    }

    for required in [
        "test_rewrite_dead_kconfig_symbol_definitions_requires_solver_proof",
        "arch/x86/Kconfig",
        "select ARCH_USED",
        "proofs.iter().all(|proof| proof.symbol != \"ARCH_USED\")",
    ] {
        assert!(
            kconfig_with_tests.contains(required),
            "unit tests should pin live-arch symbol preservation; missing {required}"
        );
    }

    for docs in [architecture, kernel_build] {
        assert!(
            docs.contains("Dead Kconfig symbol definition proofs preserve symbols used by live arch Kconfig files"),
            "docs should describe live-arch symbol preservation"
        );
    }
}
