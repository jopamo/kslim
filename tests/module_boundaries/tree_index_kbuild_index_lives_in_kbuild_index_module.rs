use super::common::*;

#[test]
fn tree_index_kbuild_index_lives_in_kbuild_index_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tree_index = index_source(root);
    let kbuild_index = production_source(&root.join("src/index/kbuild_index.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod kbuild_index;",
        "pub use kbuild_index::{",
        "KbuildDirectoryReference",
        "KbuildDirectoryReferenceIndex",
        "KbuildFileIndex",
        "KbuildObjectProviderIndex",
        "KbuildObjectReference",
        "KbuildObjectReferenceIndex",
        "use kbuild_index::build_kbuild_domain;",
        "let facts = build_kbuild_domain(root)?;",
        "self.kbuild_files = facts.files;",
        "self.kbuild_object_providers = facts.object_providers;",
        "self.kbuild_object_refs = facts.object_refs;",
        "self.kbuild_dir_refs = facts.directory_refs;",
    ] {
        assert!(
            tree_index.contains(required),
            "src/index/mod.rs should route Kbuild indexing through kbuild_index item {required}"
        );
    }

    for required in [
        "pub type KbuildFileIndex = BTreeSet<PathBuf>",
        "pub type KbuildObjectProviderIndex = BTreeSet<PathBuf>",
        "pub type KbuildObjectReferenceIndex = BTreeSet<KbuildObjectReference>",
        "pub type KbuildDirectoryReferenceIndex = BTreeSet<KbuildDirectoryReference>",
        "pub struct KbuildDirectoryReference",
        "pub struct KbuildObjectReference",
        "pub(in crate::index) struct KbuildDomainFacts",
        "pub(in crate::index) fn build_kbuild_domain(root: &Path) -> Result<KbuildDomainFacts>",
        "crate::kbuild::makefiles(root)",
        "relative_path_under_root(root, &path)?",
        "crate::kbuild::build_kbuild_index(root)?",
        "object_providers",
        "object_references",
        "directory_references",
        "is_relative_index_path(path)",
        "is_host_absolute_path_like(&reference.object)",
        "normalize_relative_to_root(root, &current_dir.join(&reference.object))",
        "crate::kbuild::make_dir_candidates(root, &current_dir, &reference.directory)",
    ] {
        assert!(
            kbuild_index.contains(required),
            "src/index/kbuild_index.rs should own Kbuild index item {required}"
        );
    }

    for moved in [
        "\npub type KbuildFileIndex",
        "\npub type KbuildObjectProviderIndex",
        "\npub type KbuildObjectReferenceIndex",
        "\npub type KbuildDirectoryReferenceIndex",
        "\npub struct KbuildDirectoryReference",
        "\npub struct KbuildObjectReference",
        "crate::kbuild::makefiles(root)",
        "crate::kbuild::build_kbuild_index(root)?",
        "crate::kbuild::make_dir_candidates(root, &current_dir, &reference.directory)",
        "normalize_relative_to_root(root, &current_dir.join(&reference.object))",
    ] {
        assert!(
            !tree_index.contains(moved),
            "src/index/mod.rs should not retain moved Kbuild index implementation {moved}"
        );
    }

    for required in [
        "`src/index/kbuild_index.rs`",
        "Tree index Kbuild indexing",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document tree index Kbuild ownership through {required}"
        );
    }
}
