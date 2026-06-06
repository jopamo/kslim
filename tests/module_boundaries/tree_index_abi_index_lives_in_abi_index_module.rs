use super::common::*;

#[test]
fn tree_index_abi_index_lives_in_abi_index_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tree_index = index_source(root);
    let abi_index = production_source(&root.join("src/index/abi_index.rs"));
    let abi_surface = production_source(&root.join("src/abi/surface.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod abi_index;",
        "pub use abi_index::{",
        "AbiPathFact",
        "AbiPathIndex",
        "AbiSourceReference",
        "AbiSourceReferenceIndex",
        "AbiSurfaceKind",
        "use abi_index::{abi_path_fact, abi_source_reference_from_include_site};",
        "abi_path_fact(relative)",
        "abi_source_reference_from_include_site(&site)",
        "pub abi_paths: AbiPathIndex",
        "pub abi_source_refs: AbiSourceReferenceIndex",
        "abi_paths_indexed",
        "abi_source_refs_indexed",
    ] {
        assert!(
            tree_index.contains(required),
            "src/index/mod.rs should route ABI indexing through abi_index item {required}"
        );
    }

    for required in [
        "pub type AbiPathIndex = BTreeSet<AbiPathFact>",
        "pub type AbiSourceReferenceIndex = BTreeSet<AbiSourceReference>",
        "pub use crate::abi::AbiSurfaceKind",
        "use crate::abi::classify_abi_header_path",
        "pub struct AbiPathFact",
        "pub struct AbiSourceReference",
        "pub(in crate::index) fn abi_path_fact(path: &Path) -> Option<AbiPathFact>",
        "pub(in crate::index) fn abi_source_reference_from_include_site(",
        "fn include_target_to_abi_path(target: &str) -> Option<PathBuf>",
        "is_host_absolute_path_like(target)",
        "Path::new(\"include\").join(path)",
    ] {
        assert!(
            abi_index.contains(required),
            "src/index/abi_index.rs should own ABI index item {required}"
        );
    }

    for required in [
        "pub enum AbiSurfaceKind",
        "PublicHeader",
        "UapiHeader",
        "pub(crate) fn classify_abi_header_path(path: &Path) -> Option<AbiSurfaceKind>",
        "pub(crate) fn has_header_extension(path: &Path) -> bool",
        "UapiPath::matches_path(path)",
        "path.starts_with(\"include/linux\") || path.starts_with(\"include/net\")",
    ] {
        assert!(
            abi_surface.contains(required),
            "src/abi/surface.rs should own ABI surface classification item {required}"
        );
    }

    for moved in [
        "\npub type AbiPathIndex",
        "\npub type AbiSourceReferenceIndex",
        "\npub struct AbiPathFact",
        "\npub struct AbiSourceReference",
        "\nfn abi_path_fact",
        "\nfn abi_source_reference_from_include_site",
        "\nfn include_target_to_abi_path",
        "UapiPath::matches_path(path)",
        "path.starts_with(\"include/linux\") || path.starts_with(\"include/net\")",
    ] {
        assert!(
            !tree_index.contains(moved),
            "src/index/mod.rs should not retain moved ABI index implementation {moved}"
        );
    }

    for required in [
        "`src/index/abi_index.rs`",
        "Tree index ABI-sensitive facts",
        "ABI surface classification and removal policy remain outside the tree index",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document tree index ABI ownership through {required}"
        );
    }
}
