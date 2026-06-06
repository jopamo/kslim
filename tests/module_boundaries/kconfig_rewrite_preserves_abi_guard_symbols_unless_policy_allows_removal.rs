use super::common::*;

#[test]
fn kconfig_rewrite_preserves_abi_guard_symbols_unless_policy_allows_removal() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let prune_path = root.join("src/prune.rs");
    let prune = production_sources(
        &root,
        &[
            "src/prune.rs",
            "src/prune/semantic.rs",
            "src/prune/report.rs",
        ],
    );
    let prune_with_tests =
        std::fs::read_to_string(&prune_path).expect("failed to read src/prune.rs");
    let architecture =
        std::fs::read_to_string(root.join("docs/architecture.md")).expect("failed to read docs");
    let kernel_build = kernel_build_iteration_docs(&root);

    for required in [
        "struct AbiGuardSymbolUse",
        "fn effective_removed_config_symbols_for_abi_policy",
        "fn abi_guard_symbol_uses",
        "fn config_symbols_in_text",
        "allow_uapi_header_removal",
        "allow_public_header_removal",
        "kconfig_stage.removed_config_symbols",
    ] {
        assert!(
            prune.contains(required),
            "prune should filter ABI guard symbols through ABI policy before Kconfig/Kbuild rewrites; missing {required}"
        );
    }

    for required in [
        "test_prune_preserves_abi_guard_config_symbols_without_abi_policy",
        "test_prune_removes_abi_guard_config_symbols_with_matching_abi_policy",
        "CONFIG_ABI_GUARD",
    ] {
        assert!(
            prune_with_tests.contains(required),
            "unit tests should pin ABI guard symbol preservation policy; missing {required}"
        );
    }

    for docs in [architecture, kernel_build] {
        assert!(
            docs.contains(
                "ABI guard Kconfig symbols are preserved unless matching ABI policy allows removal"
            ),
            "docs should describe ABI guard symbol preservation"
        );
    }
}
