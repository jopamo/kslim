use super::common::*;

#[test]
fn generate_state_tests_are_behavior_focused() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let state_tests = production_source(&root.join("src/state/tests.rs"));
    let requested = production_source(&root.join("src/state/tests_requested.rs"));
    let resolved = production_source(&root.join("src/state/tests_resolved.rs"));
    let candidate = production_source(&root.join("src/state/tests_candidate.rs"));
    let published = production_source(&root.join("src/state/tests_published.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "#[path = \"tests_requested.rs\"]\nmod requested;",
        "#[path = \"tests_resolved.rs\"]\nmod resolved;",
        "#[path = \"tests_candidate.rs\"]\nmod candidate;",
        "#[path = \"tests_published.rs\"]\nmod published;",
    ] {
        assert!(
            state_tests.contains(required),
            "src/state/tests.rs should register lifecycle-focused generate state test module {required}"
        );
    }

    for forbidden in [
        "#[test]",
        "test_requested_generate_state_captures_request_inputs",
        "test_resolved_candidate_state_captures_plans_without_candidate_or_published_state",
        "test_candidate_tree_state_tracks_candidate_lifecycle_flags",
        "test_published_snapshot_state_requires_committed_output_identity",
    ] {
        assert!(
            !state_tests.contains(forbidden),
            "src/state/tests.rs should keep shared helpers and module declarations only; found {forbidden}"
        );
    }

    assert!(
        requested.contains("test_requested_generate_state_captures_request_inputs")
            && requested.contains("test_requested_generate_state_rejects_conflicting_strictness_flags")
            && requested.contains("test_requested_generate_state_rejects_invalid_request_identity_parts"),
        "src/state/tests_requested.rs should own requested state and CLI override tests"
    );
    assert!(
        resolved.contains(
            "test_resolved_candidate_state_captures_plans_without_candidate_or_published_state"
        ) && resolved.contains("test_feature_resolution_state_resolves_named_feature_remove_input")
            && resolved.contains("test_abi_decision_state_records_approved_abi_sensitive_removals"),
        "src/state/tests_resolved.rs should own resolved plan, feature, and ABI tests"
    );
    assert!(
        candidate.contains("test_candidate_tree_state_tracks_candidate_lifecycle_flags")
            && candidate.contains("test_candidate_tree_state_rejects_unmaterialized_progress")
            && candidate.contains("test_candidate_tree_state_rejects_metadata_outside_tree"),
        "src/state/tests_candidate.rs should own private candidate lifecycle tests"
    );
    assert!(
        published.contains("test_published_snapshot_state_requires_committed_output_identity")
            && published.contains("test_published_snapshot_state_rejects_missing_commit_or_lockfile")
            && published.contains("test_generate_attempt_failure_is_not_a_published_snapshot_source"),
        "src/state/tests_published.rs should own publication and failed-attempt authority tests"
    );
    assert!(
        architecture.contains("Generate state unit tests are split by lifecycle phase"),
        "docs/architecture.md should document generate state test ownership"
    );
}
