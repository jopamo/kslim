use super::common::*;

#[test]
fn include_unit_tests_live_beside_owned_modules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let includes = std::fs::read_to_string(root.join("src/source_scan/includes/mod.rs"))
        .expect("failed to read src/source_scan/includes/mod.rs");
    let include_tests = production_source(&root.join("src/source_scan/includes/tests.rs"));
    let index_tests = production_source(&root.join("src/source_scan/includes/tests_index.rs"));
    let cleanup_tests = production_source(&root.join("src/source_scan/includes/tests_cleanup.rs"));
    let private_header_tests =
        production_source(&root.join("src/source_scan/includes/tests_private_header.rs"));
    let public_header_tests =
        production_source(&root.join("src/source_scan/includes/tests_public_header_policy.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        includes.contains("#[cfg(test)]\nmod tests;"),
        "src/source_scan/includes/mod.rs should register a sibling include tests module"
    );
    assert!(
        !includes.contains("mod tests {"),
        "src/source_scan/includes/mod.rs should not keep inline include unit tests"
    );

    for required in [
        "#[path = \"tests_index.rs\"]\nmod index;",
        "#[path = \"tests_cleanup.rs\"]\nmod cleanup;",
        "#[path = \"tests_private_header.rs\"]\nmod private_header;",
        "#[path = \"tests_public_header_policy.rs\"]\nmod public_header_policy;",
    ] {
        assert!(
            include_tests.contains(required),
            "src/source_scan/includes/tests.rs should register behavior-focused include test module {required}"
        );
    }

    assert!(
        !include_tests.contains("#[test]"),
        "src/source_scan/includes/tests.rs should keep shared helpers and module declarations only"
    );
    assert!(
        index_tests.contains("test_parse_include_site_supports_quoted_and_angle_forms")
            && index_tests.contains("test_index_include_sites_collects_quoted_and_angle_sites")
            && index_tests.contains("test_resolve_include_targets_uses_local_directory_for_angle_include"),
        "src/source_scan/includes/tests_index.rs should own include parsing, indexing, and target resolution tests"
    );
    assert!(
        cleanup_tests.contains("test_report_live_include_site_needing_manual_handling_reports_live_missing_include")
            && cleanup_tests.contains("test_rewrite_removed_header_includes_removes_dead_branch_backed_include")
            && cleanup_tests.contains("test_rewrite_removed_header_includes_reports_ambiguous_include"),
        "src/source_scan/includes/tests_cleanup.rs should own include cleanup report and rewrite tests"
    );
    assert!(
        private_header_tests.contains("test_target_is_gone_from_reduced_tree_accepts_removed_and_absent_targets")
            && private_header_tests.contains("test_rewrite_removed_header_includes_removes_manifest_removed_private_header")
            && private_header_tests.contains("test_rewrite_removed_header_includes_removes_file_relative_quoted_private_header"),
        "src/source_scan/includes/tests_private_header.rs should own private-header orphaning tests"
    );
    assert!(
        public_header_tests.contains("test_classify_include_targets_marks_public_preserved_headers")
            && public_header_tests.contains("test_rewrite_removed_header_includes_removes_explicitly_removed_public_header_with_abi_policy")
            && public_header_tests.contains("test_rewrite_removed_header_includes_removes_explicit_uapi_header_with_uapi_policy"),
        "src/source_scan/includes/tests_public_header_policy.rs should own public-header and UAPI policy tests"
    );
    assert!(
        architecture.contains("Include unit tests are split by behavior"),
        "docs/architecture.md should document include test module ownership"
    );
}
