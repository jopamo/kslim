use super::common::*;

#[test]
fn tree_index_query_api_lives_in_query_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tree_index = index_source(root);
    let query = production_source(&root.join("src/index/query.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in ["mod query;", "pub struct TreeIndex", "impl TreeIndex"] {
        assert!(
            tree_index.contains(required),
            "src/index/mod.rs should keep tree index state while routing query API through {required}"
        );
    }

    for required in [
        "impl TreeIndex",
        "pub fn contains_file(&self, path: &Path) -> bool",
        "pub fn find_include_site(&self, file: &Path, target: &str) -> Option<&IncludeSite>",
        "pub fn has_include_site(&self, file: &Path, line: usize, target: &str) -> bool",
        "pub fn find_kconfig_source_ref(",
        "pub fn has_kconfig_source_ref(",
        "pub fn find_kbuild_directory_refs(&self, path: &str) -> Vec<&KbuildDirectoryReference>",
        "pub fn has_kbuild_directory_ref(",
        "pub fn find_kbuild_object_refs(&self, path: &str) -> Vec<&KbuildObjectReference>",
        "pub fn has_kbuild_object_ref(",
        "fn normalize_directory_subject(path: &str) -> Option<PathBuf>",
        "fn normalize_object_subject(path: &str) -> Option<PathBuf>",
        "crate::kbuild::normalize_relative(Path::new(trimmed))",
    ] {
        assert!(
            query.contains(required),
            "src/index/query.rs should own query API item {required}"
        );
    }

    for moved in [
        "\npub fn contains_file",
        "\npub fn find_include_site",
        "\npub fn has_include_site",
        "\npub fn find_kconfig_source_ref",
        "\npub fn has_kconfig_source_ref",
        "\npub fn find_kbuild_directory_refs",
        "\npub fn has_kbuild_directory_ref",
        "\npub fn find_kbuild_object_refs",
        "\npub fn has_kbuild_object_ref",
        "\nfn normalize_directory_subject",
        "\nfn normalize_object_subject",
    ] {
        assert!(
            !tree_index.contains(moved),
            "src/index/mod.rs should not retain moved query API implementation {moved}"
        );
    }

    for required in ["`src/index/query.rs`", "Tree index query API"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document tree index query ownership through {required}"
        );
    }
}
