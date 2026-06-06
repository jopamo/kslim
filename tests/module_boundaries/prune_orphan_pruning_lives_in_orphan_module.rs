use super::common::*;

#[test]
fn prune_orphan_pruning_lives_in_orphan_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let prune = production_source(&root.join("src/prune.rs"));
    let path = production_source(&root.join("src/prune/path.rs"));
    let orphan = production_source(&root.join("src/prune/orphan.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod orphan;",
        "pub(in crate::prune) use orphan::cleanup_empty_parent_chain",
    ] {
        assert!(
            prune.contains(required),
            "src/prune.rs should expose orphan pruning through {required}"
        );
    }

    for required in [
        "pub(in crate::prune) struct EmptyParentCleanup",
        "pub(in crate::prune) fn cleanup_empty_parent_chain(",
        "fn path_intersects_preserved_roots(",
        "abi_sensitive_path_requires_exact_manifest_truth",
        "std::fs::remove_dir(&current)",
        "EditProofSource::removal_manifest_path",
        "ensure_edit_records_for_mutation(\"prune.cleanup_empty_parents\"",
        "normalized_relative_path_covers(preserved, path)",
        "normalized_relative_path_covers(path, preserved)",
    ] {
        assert!(
            orphan.contains(required),
            "src/prune/orphan.rs should own orphan pruning item {required}"
        );
    }

    assert!(
        path.contains("cleanup_empty_parent_chain(parent, root, manifest_path, remove_paths, preserved_paths)?"),
        "declared path pruning should delegate orphan cleanup to prune/orphan.rs"
    );
    for forbidden in [
        "\npub(in crate::prune) struct EmptyParentCleanup",
        "\npub(in crate::prune) fn cleanup_empty_parent_chain(",
        "\nfn path_intersects_preserved_roots(",
        "std::fs::remove_dir(&current)",
    ] {
        assert!(
            !path.contains(forbidden),
            "src/prune/path.rs should not retain extracted orphan pruning implementation {forbidden}"
        );
    }

    for required in ["`src/prune/orphan.rs`", "Prune orphan pruning"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document orphan module ownership through {required}"
        );
    }
}
