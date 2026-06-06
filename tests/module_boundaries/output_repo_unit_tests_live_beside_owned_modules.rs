use super::common::*;

#[test]
fn output_repo_unit_tests_live_beside_owned_modules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output_repo = std::fs::read_to_string(root.join("src/output_repo.rs"))
        .expect("failed to read src/output_repo.rs");
    let tests = production_source(&root.join("src/output_repo/tests.rs"));
    let metadata_tests = production_source(&root.join("src/output_repo/tests_metadata.rs"));
    let sync_tests = production_source(&root.join("src/output_repo/tests_sync.rs"));
    let report_tests = production_source(&root.join("src/output_repo/tests_report.rs"));
    let publication_tests = production_source(&root.join("src/output_repo/tests_publication.rs"));
    let safety_tests = production_source(&root.join("src/output_repo/tests_safety.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        output_repo.contains("\n#[cfg(test)]\nmod tests;\n"),
        "src/output_repo.rs should register a sibling output repo tests module"
    );
    assert!(
        !output_repo.contains("\nmod tests {"),
        "src/output_repo.rs should not retain inline output repo unit tests"
    );

    for required in [
        "#[path = \"tests_metadata.rs\"]\nmod metadata;",
        "#[path = \"tests_sync.rs\"]\nmod sync;",
        "#[path = \"tests_report.rs\"]\nmod report;",
        "#[path = \"tests_publication.rs\"]\nmod publication;",
        "#[path = \"tests_safety.rs\"]\nmod safety;",
    ] {
        assert!(
            tests.contains(required),
            "src/output_repo/tests.rs should register behavior-focused output repo test module {required}"
        );
    }

    assert!(
        !tests.contains("#[test]"),
        "src/output_repo/tests.rs should keep shared helpers and module declarations only"
    );
    assert!(
        metadata_tests.contains("test_committed_metadata_sanitizes_host_specific_absolute_paths")
            && metadata_tests.contains("test_host_specific_path_detection_covers_local_url_and_windows_forms")
            && metadata_tests.contains("test_generated_metadata_requires_reproducible_timestamp"),
        "src/output_repo/tests_metadata.rs should own committed metadata and timestamp policy tests"
    );
    assert!(
        sync_tests.contains("test_sync_working_tree_is_incremental_for_unchanged_files")
            && sync_tests.contains("test_sync_working_tree_preserves_top_level_git_and_kslim")
            && sync_tests.contains("test_candidate_metadata_sync_does_not_write_published_metadata"),
        "src/output_repo/tests_sync.rs should own output tree and candidate metadata sync tests"
    );
    assert!(
        report_tests.contains("test_reducer_artifact_path_uses_worktree_metadata_dir_without_git_repo")
            && report_tests.contains("test_write_failure_report_stores_stage_enum_stable_name")
            && report_tests.contains("test_render_reducer_diagnostics_json_sorts_diagnostics_by_stable_keys")
            && report_tests.contains("test_write_reducer_metadata_writes_report_and_summary_when_reducer_ran"),
        "src/output_repo/tests_report.rs should own report path, rendering, and writer tests"
    );
    assert!(
        publication_tests.contains("test_publish_output_candidate_syncs_payload_and_non_published_metadata_together")
            && publication_tests.contains("test_authoritative_published_state_rejects_candidate_metadata_only")
            && publication_tests.contains("test_authoritative_published_state_rejects_committed_metadata_without_lockfile"),
        "src/output_repo/tests_publication.rs should own publication and authoritative-state tests"
    );
    assert!(
        safety_tests.contains("test_validate_output_candidate_requires_candidate_metadata_before_publish")
            && safety_tests.contains("test_validate_output_candidate_rejects_temporary_candidate_path_in_metadata")
            && safety_tests.contains("test_reducer_artifact_path_rejects_last_attempt_name"),
        "src/output_repo/tests_safety.rs should own candidate and report publication safety tests"
    );
    assert!(
        architecture.contains("Output repo unit tests are split by behavior"),
        "docs/architecture.md should document output repo test module ownership"
    );
}
