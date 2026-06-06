use super::common::*;

#[test]
fn tree_index_unit_tests_live_beside_owned_modules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tree_index = std::fs::read_to_string(root.join("src/index/mod.rs"))
        .expect("failed to read src/index/mod.rs");
    let tests = production_source(&root.join("src/index/tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        tree_index.contains("\n#[cfg(test)]\nmod tests;\n"),
        "src/index/mod.rs should register a sibling tree index tests module"
    );
    assert!(
        !tree_index.contains("\nmod tests {"),
        "src/index/mod.rs should not retain inline tree index unit tests"
    );

    for required in [
        "use super::*;",
        "use super::file_index::{index_path_is_under, is_relative_index_path};",
        "use super::source_index::parse_include_target;",
        "test_tree_index_build_indexes_files_include_sites_kconfig_refs_and_kbuild_refs",
        "test_tree_index_full_build_is_deterministic",
        "test_tree_index_rebuild_apis_refresh_domain_indexes",
        "test_tree_index_incremental_rebuild_matches_full_build_for_touched_changes",
        "test_tree_index_incremental_rebuild_removes_missing_file_and_directory_entries",
        "test_tree_index_build_does_not_mutate_tree",
        "test_tree_index_skips_absolute_reference_literals",
        "test_tree_index_parse_include_target_accepts_simple_supported_forms_only",
    ] {
        assert!(
            tests.contains(required),
            "src/index/tests.rs should carry tree index unit test coverage through {required}"
        );
    }

    for required in ["`src/index/tests.rs`", "Tree index unit tests"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document tree index test ownership through {required}"
        );
    }
}
