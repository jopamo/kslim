use super::common::*;

#[test]
fn tree_index_is_read_only_and_policy_free() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let facade = production_source(&root.join("src/tree_index.rs"));
    let tree_index = index_source(root);
    let production = production_sources(
        &root,
        &[
            "src/index/mod.rs",
            "src/index/abi_index.rs",
            "src/index/file_index.rs",
            "src/index/kbuild_index.rs",
            "src/index/kconfig_index.rs",
            "src/index/query.rs",
            "src/index/source_index.rs",
        ],
    );

    assert!(
        main.contains("mod index;") && main.contains("mod tree_index;"),
        "main.rs should register index ownership and the tree_index compatibility facade"
    );
    assert!(
        facade.contains("pub(crate) use crate::index::*;")
            && facade.contains("Compatibility facade for read-only kernel tree indexes."),
        "src/tree_index.rs should be only the compatibility facade over crate::index"
    );

    assert!(
        production.contains("std::fs::read_to_string"),
        "src/index/* should build facts by reading source files"
    );
    assert!(
        production.contains("walkdir::WalkDir"),
        "src/index/* should discover files through read-only walking"
    );
    for required_controlled_api in [
        "pub(crate) enum TreeIndexRebuildDomain",
        "pub(crate) enum TreeIndexMutatingPass",
        "pub(crate) fn rebuild_after_mutating_pass",
        "fn rebuild_all",
        "fn rebuild_kconfig",
        "fn rebuild_kbuild",
        "fn rebuild_c_family",
    ] {
        assert!(
            tree_index.contains(required_controlled_api),
            "src/index/mod.rs should expose controlled in-memory rebuild API item {required_controlled_api}"
        );
    }
    for forbidden_public_rebuild in [
        "pub fn rebuild_all",
        "pub fn rebuild_kconfig",
        "pub fn rebuild_kbuild",
        "pub fn rebuild_c_family",
        "pub(crate) fn rebuild_all",
        "pub(crate) fn rebuild_kconfig",
        "pub(crate) fn rebuild_kbuild",
        "pub(crate) fn rebuild_c_family",
    ] {
        assert!(
            !tree_index.contains(forbidden_public_rebuild),
            "src/index/mod.rs direct rebuild helpers must stay private behind rebuild_after_mutating_pass; found {forbidden_public_rebuild}"
        );
    }

    let forbidden_mutations_or_policy = [
        "std::fs::write",
        "std::fs::remove",
        "std::fs::rename",
        "std::fs::copy",
        "std::fs::create_dir",
        "File::create",
        "OpenOptions",
        "serde::",
        "Serialize",
        "Deserialize",
        "EditRecord",
        "EditReason",
        "write_verified_rewrite",
        "ensure_edit_records_for_mutation",
        "rewrite_",
        "render_",
        "write_report",
        "ReportArtifact",
        "crate::config",
        "crate::generate",
        "crate::output_repo",
        "crate::patches",
        "crate::prune",
        "crate::publish",
        "crate::reducer",
        "crate::removal_manifest",
        "crate::selftest",
        "crate::upstream",
    ];

    for forbidden in forbidden_mutations_or_policy {
        assert!(
            !production.contains(forbidden),
            "src/index/* must remain read-only indexing; found forbidden token {forbidden}"
        );
    }
}
