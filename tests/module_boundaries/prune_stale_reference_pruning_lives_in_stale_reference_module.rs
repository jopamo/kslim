use super::common::*;

#[test]
fn prune_stale_reference_pruning_lives_in_stale_reference_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let prune = production_source(&root.join("src/prune.rs"));
    let stale = production_source(&root.join("src/prune/stale_reference.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod stale_reference;",
        "pub(in crate::prune) use stale_reference::{rewrite_build_graph, rewrite_kconfig_sources}",
    ] {
        assert!(
            prune.contains(required),
            "src/prune.rs should expose stale-reference pruning through {required}"
        );
    }

    for required in [
        "pub(in crate::prune) struct StaleReferenceRewriteStats",
        "pub(in crate::prune) fn rewrite_build_graph(",
        "pub(in crate::prune) fn rewrite_kconfig_sources(",
        "fn kconfig_source_removal_proofs(",
        "fn manifest_removed_kconfig_source_target(",
        "fn rewrite_makefiles(",
        "TreeIndex::build(root, manifest)?",
        "manifest.removed_kconfig_sources_vec()",
        "index.kconfig_sources",
        "if source_ref.optional || source_ref.source.contains('$')",
        "crate::kconfig::rewrite_kconfig_sources(root, &proofs)",
        "crate::kbuild::rewrite_makefiles_report",
    ] {
        assert!(
            stale.contains(required),
            "src/prune/stale_reference.rs should own stale-reference pruning item {required}"
        );
    }

    for forbidden in [
        "\nstruct RewriteStats",
        "\nfn rewrite_build_graph(",
        "\nfn rewrite_kconfig_sources(",
        "\nfn kconfig_source_removal_proofs(",
        "\nfn manifest_removed_kconfig_source_target(",
        "\nfn rewrite_makefiles(",
    ] {
        assert!(
            !prune.contains(forbidden),
            "src/prune.rs should not retain extracted stale-reference pruning implementation {forbidden}"
        );
    }

    for required in [
        "`src/prune/stale_reference.rs`",
        "Prune stale reference pruning",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document stale-reference module ownership through {required}"
        );
    }
}
