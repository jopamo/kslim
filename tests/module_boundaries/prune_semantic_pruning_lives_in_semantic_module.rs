use super::common::*;

#[test]
fn prune_semantic_pruning_lives_in_semantic_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let prune = production_source(&root.join("src/prune.rs"));
    let semantic = production_source(&root.join("src/prune/semantic.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod semantic;",
        "pub(in crate::prune) use semantic::effective_removed_config_symbols_for_abi_policy",
        "pub(crate) use semantic::{rewrite_kconfig_stage, KconfigPruneStageResult}",
    ] {
        assert!(
            prune.contains(required),
            "src/prune.rs should expose semantic pruning through {required}"
        );
    }

    for required in [
        "pub(crate) struct KconfigPruneStageResult",
        "pub(crate) fn rewrite_kconfig_stage(",
        "read_kconfig_selected_profile_values(root)",
        "kconfig_solver_report(",
        "fn prune_configs(",
        "fn rewrite_kconfig_defaults(",
        "fn rewrite_kconfig_relations(",
        "fn rewrite_empty_kconfig_menus(",
        "struct AbiGuardSymbolUse",
        "pub(in crate::prune) fn effective_removed_config_symbols_for_abi_policy(",
        "fn abi_guard_symbol_uses(",
        "fn config_symbols_in_text(",
        "allow_public_header_removal",
        "allow_uapi_header_removal",
    ] {
        assert!(
            semantic.contains(required),
            "src/prune/semantic.rs should own semantic pruning item {required}"
        );
    }

    for forbidden in [
        "\npub(crate) struct KconfigPruneStageResult",
        "\npub(crate) fn rewrite_kconfig_stage(",
        "\nstruct AbiGuardSymbolUse",
        "\nfn abi_guard_symbol_uses(",
        "\nfn config_symbols_in_text(",
        "\nfn prune_configs(",
        "\nfn rewrite_kconfig_defaults(",
        "\nfn rewrite_kconfig_relations(",
        "\nfn rewrite_empty_kconfig_menus(",
    ] {
        assert!(
            !prune.contains(forbidden),
            "src/prune.rs should not retain extracted semantic pruning implementation {forbidden}"
        );
    }

    for required in ["`src/prune/semantic.rs`", "Prune semantic pruning"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document semantic pruning module ownership through {required}"
        );
    }
}
