use super::common::*;

#[test]
fn kbuild_object_graph_lives_in_object_graph_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kbuild = production_source(&root.join("src/kbuild/mod.rs"));
    let object_graph = production_source(&root.join("src/kbuild/object_graph.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod object_graph;",
        "pub(crate) use object_graph::{",
        "build_kbuild_index",
        "has_object_provider",
        "is_build_graph_assignment",
        "make_dir_candidates",
        "makefiles",
        "normalize_relative",
        "KbuildIndex",
    ] {
        assert!(
            kbuild.contains(required),
            "src/kbuild/mod.rs should expose Kbuild object graph through {required}"
        );
    }

    for required in [
        "pub(crate) struct KbuildIndex",
        "pub(crate) struct KbuildObjectReference",
        "pub(crate) struct KbuildCompositeObjectMember",
        "pub(crate) struct KbuildDirectoryReference",
        "pub(crate) struct KbuildConfigGatedReference",
        "pub(crate) struct KbuildIncludePathFlag",
        "pub(crate) fn build_kbuild_index(",
        "fn index_source_object_providers(",
        "pub(in crate::kbuild) fn object_provider_path(",
        "fn index_assignment(",
        "fn index_tokens(",
        "pub(crate) fn is_build_graph_assignment(",
        "pub(crate) fn has_object_provider(",
        "pub(in crate::kbuild) fn has_direct_object_provider(",
        "pub(crate) fn make_dir_candidates(",
        "pub(in crate::kbuild) fn include_path_candidates(",
        "pub(crate) fn makefiles(",
        "pub(crate) fn normalize_relative(",
        "fn walk_named(",
        "walkdir::WalkDir::new(root)",
        "parse_kbuild_assignment(&line.joined)",
    ] {
        assert!(
            object_graph.contains(required),
            "src/kbuild/object_graph.rs should own Kbuild object graph item {required}"
        );
    }

    for forbidden in [
        "\npub(crate) struct KbuildIndex",
        "\npub(crate) struct KbuildObjectReference",
        "\npub(crate) fn build_kbuild_index(",
        "\nfn index_source_object_providers(",
        "\nfn index_assignment(",
        "\nfn index_tokens(",
        "\npub(crate) fn has_object_provider(",
        "\npub(crate) fn make_dir_candidates(",
        "\npub(crate) fn makefiles(",
        "\npub(crate) fn normalize_relative(",
        "walkdir::WalkDir::new(root)",
    ] {
        assert!(
            !kbuild.contains(forbidden),
            "src/kbuild/mod.rs should not retain extracted Kbuild object graph implementation {forbidden}"
        );
    }

    for required in ["`src/kbuild/object_graph.rs`", "Kbuild object graph"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document Kbuild object graph ownership through {required}"
        );
    }
}
