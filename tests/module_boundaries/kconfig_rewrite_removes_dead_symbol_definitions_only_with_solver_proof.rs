use super::common::*;

#[test]
fn kconfig_rewrite_removes_dead_symbol_definitions_only_with_solver_proof() {
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
    let edit_reason = production_sources(
        &root,
        &[
            "src/edit_reason.rs",
            "src/edit_reason/reason.rs",
            "src/edit_reason/proof_source.rs",
            "src/edit_reason/render.rs",
        ],
    );
    let architecture =
        std::fs::read_to_string(root.join("docs/architecture.md")).expect("failed to read docs");
    let kernel_build = kernel_build_iteration_docs(&root);

    for required in [
        "pub(crate) struct KconfigDeadSymbolDefinitionProof",
        "pub(crate) fn prove_dead_kconfig_symbol_definitions",
        "detect_kconfig_orphaned_symbol_definitions(",
        "kconfig_dead_symbol_definition_kind_is_rewrite_supported",
        "pub(crate) fn rewrite_dead_kconfig_symbol_definitions",
        "kconfig_dead_symbol_definition_proof_matches_line",
        "EditReason::RemovedDeadKconfigSymbolDefinition",
        "EditProofSource::kconfig_solver_unreachable_symbol_definition",
        "kconfig.rewrite_dead_symbol_definitions",
    ] {
        assert!(
            kconfig.contains(required),
            "src/kconfig/mod.rs should remove dead symbol definitions only with solver proof; missing {required}"
        );
    }

    for required in [
        "RemovedDeadKconfigSymbolDefinition",
        "KconfigSolverProof",
        "KconfigSolverKey::UnreachableSymbolDefinition",
        "kconfig_solver_unreachable_symbol_definition",
        "removed_dead_kconfig_symbol_definition",
    ] {
        assert!(
            edit_reason.contains(required),
            "edit provenance should model solver-backed dead-symbol rewrites; missing {required}"
        );
    }

    for required in [
        "test_rewrite_dead_kconfig_symbol_definitions_requires_solver_proof",
        "rewrite_dead_kconfig_symbol_definitions(root, &[])",
        "prove_dead_kconfig_symbol_definitions(",
        "KconfigDeadSymbolDefinitionProof",
        "definition_kind: KconfigSymbolDefinitionKind::Config",
        "definition_kind: KconfigSymbolDefinitionKind::Menuconfig",
    ] {
        assert!(
            kconfig_with_tests.contains(required),
            "unit tests should pin solver-proof-gated dead-symbol rewrites; missing {required}"
        );
    }

    for docs in [architecture, kernel_build] {
        assert!(
            docs.contains("Dead Kconfig config/menuconfig definition removal is solver-proof-gated"),
            "docs should describe solver-proof-gated dead symbol-definition removal"
        );
        assert!(
            docs.contains("definitions without an unreachable-symbol solver proof are preserved"),
            "docs should say definitions without solver proof are preserved"
        );
    }
}
