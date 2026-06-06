use super::common::*;

#[test]
fn tree_index_file_index_lives_in_file_index_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tree_index = index_source(root);
    let file_index = production_source(&root.join("src/index/file_index.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod file_index;",
        "pub use file_index::{FileIndex, HeaderIndex};",
        "indexed_tree_files(root)?",
        "existing_touched_files(root, &touched)?",
    ] {
        assert!(
            tree_index.contains(required),
            "src/index/mod.rs should route file indexing through file_index module item {required}"
        );
    }

    for required in [
        "pub type FileIndex = BTreeSet<PathBuf>",
        "pub type HeaderIndex = BTreeSet<PathBuf>",
        "pub(in crate::index) fn indexed_tree_files(root: &Path) -> Result<Vec<(PathBuf, PathBuf)>>",
        "pub(in crate::index) fn is_header_path(path: &Path) -> bool",
        "pub(in crate::index) fn normalize_touched_paths(",
        "pub(in crate::index) fn existing_touched_files(",
        "pub(in crate::index) fn relative_path_under_root(",
        "pub(in crate::index) fn normalize_relative_to_root(",
        "pub(in crate::index) fn ensure_relative_index_path(",
        "fn ensure_relative_input_path(path: &Path) -> Result<()>",
        "pub(in crate::index) fn is_relative_index_path(",
        "pub(in crate::index) fn index_path_is_under(",
        "pub(in crate::index) fn ensure_index_text_not_host_absolute_path(",
        "pub(in crate::index) fn is_host_absolute_path_like(",
        "fn is_windows_absolute_path_like(",
        "walkdir::WalkDir::new(root)",
        "walkdir::WalkDir::new(&path)",
    ] {
        assert!(
            file_index.contains(required),
            "src/index/file_index.rs should own file/path index item {required}"
        );
    }

    for moved in [
        "\npub type FileIndex",
        "\npub type HeaderIndex",
        "\nfn is_header_path",
        "\nfn normalize_touched_paths",
        "\nfn existing_touched_files",
        "\nfn relative_path_under_root",
        "\nfn normalize_relative_to_root",
        "\nfn ensure_relative_index_path",
        "\nfn ensure_relative_input_path",
        "\nfn is_relative_index_path",
        "\nfn index_path_is_under",
        "\nfn ensure_index_text_not_host_absolute_path",
        "\nfn is_host_absolute_path_like",
        "\nfn is_windows_absolute_path_like",
    ] {
        assert!(
            !tree_index.contains(moved),
            "src/index/mod.rs should not retain moved file index implementation {moved}"
        );
    }

    for required in [
        "`src/index/file_index.rs`",
        "Tree index file/path indexing",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document tree index file-index ownership through {required}"
        );
    }
}
