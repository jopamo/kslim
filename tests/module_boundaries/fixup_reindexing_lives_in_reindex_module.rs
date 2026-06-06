use super::common::*;

#[test]
fn fixup_reindexing_lives_in_reindex_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixups = production_source(&root.join("src/fixups.rs"));
    let application = production_source(&root.join("src/fixups/application.rs"));
    let reindex = production_source(&root.join("src/fixups/reindex.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod reindex;",
        "pub(in crate::fixups) use reindex::{",
        "build_fixup_index",
        "is_tree_index_truth",
        "proof_matches_tree_index",
        "let index = build_fixup_index(root)?;",
    ] {
        assert!(
            fixups.contains(required),
            "src/fixups.rs should route tree-index rebuilding through {required}"
        );
    }

    for required in [
        "pub(in crate::fixups) fn build_fixup_index(root: &Path) -> Result<TreeIndex>",
        "TreeIndex::build(root, &())",
        "pub(in crate::fixups) fn is_tree_index_truth(proof: &FixupProof) -> bool",
        "FixupProof::TreeIndexIncludeSite",
        "FixupProof::TreeIndexKbuildDirectoryRef",
        "FixupProof::TreeIndexKbuildObjectRef",
        "FixupProof::TreeIndexKconfigSourceRef",
        "pub(in crate::fixups) fn proof_matches_tree_index(proof: &FixupProof, index: &TreeIndex)",
        "index.has_include_site(file, *line, target)",
        "index.has_kbuild_directory_ref(file, *line, assignment_lhs, directory, resolved_path)",
        "index.has_kbuild_object_ref(file, *line, assignment_lhs, object, resolved_path)",
        "index.has_kconfig_source_ref(file, *line, source, *optional, *relative)",
    ] {
        assert!(
            reindex.contains(required),
            "src/fixups/reindex.rs should own reindexing proof detail {required}"
        );
    }

    for required in [
        "is_tree_index_truth(proof) && proof_matches_tree_index(proof, index)",
        "without tree index truth proof",
    ] {
        assert!(
            application.contains(required),
            "src/fixups/application.rs should consume reindex proof helpers through {required}"
        );
    }

    for forbidden in [
        "TreeIndex::build(root, &())",
        "\n    fn is_tree_index_truth(&self)",
        "\n    fn matches_tree_index(&self",
    ] {
        assert!(
            !fixups.contains(forbidden),
            "src/fixups.rs should not keep reindexing helper body {forbidden}"
        );
    }

    for required in ["`src/fixups/reindex.rs`", "Fixup reindexing"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document the fixup reindexing split {required}"
        );
    }
}
