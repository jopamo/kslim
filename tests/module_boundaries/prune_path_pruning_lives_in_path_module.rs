use super::common::*;

#[test]
fn prune_path_pruning_lives_in_path_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let prune = production_source(&root.join("src/prune.rs"));
    let path = production_source(&root.join("src/prune/path.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod path;",
        "pub use path::{",
        "DeclaredPathPruneResult",
        "RemovalAccounting",
        "RemovalFailurePolicy",
        "pub(crate) use path::prune_declared_paths_from_manifest_with_policy",
        "pub(crate) use path::prune_declared_paths_from_manifest",
    ] {
        assert!(
            prune.contains(required),
            "src/prune.rs should expose declared path pruning through {required}"
        );
    }

    for required in [
        "pub struct RemovalAccounting",
        "pub struct DeclaredPathPruneResult",
        "pub struct PruneResult",
        "pub struct PrunedPath",
        "pub struct FailedRemoval",
        "pub enum FailedRemovalKind",
        "pub struct RemovalFailurePolicy",
        "pub(crate) fn prune_declared_paths_from_manifest(",
        "pub(crate) fn prune_declared_paths_from_manifest_with_policy(",
        "pub(in crate::prune) fn prune_declared_paths(",
        "fn prune_declared_paths_with_preservation(",
        "fn remove_path(",
        "fn abi_sensitive_path_requires_exact_manifest_truth(",
        "crate::abi::validate_declared_removal",
        "EditProofSource::removal_manifest_path",
        "std::fs::remove_file",
        "std::fs::remove_dir",
    ] {
        assert!(
            path.contains(required),
            "src/prune/path.rs should own declared path pruning item {required}"
        );
    }

    for forbidden in [
        "\nfn prune_declared_paths_with_preservation(",
        "\nfn remove_path(",
        "\nfn abi_sensitive_path_requires_exact_manifest_truth(",
        "EditProofSource::removal_manifest_path",
        "std::fs::remove_file",
        "std::fs::remove_dir(",
    ] {
        assert!(
            !prune.contains(forbidden),
            "src/prune.rs should not retain extracted declared path pruning implementation {forbidden}"
        );
    }

    for required in ["`src/prune/path.rs`", "Prune path pruning"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document path-pruning module ownership through {required}"
        );
    }
}
