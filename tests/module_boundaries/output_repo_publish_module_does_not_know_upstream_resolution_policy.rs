use super::common::*;

#[test]
fn output_repo_publish_module_does_not_know_upstream_resolution_policy() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output_repo = production_source(&root.join("src/output_repo.rs"));
    let publish = production_source(&root.join("src/output_repo/publish.rs"));

    for required in [
        "pub(crate) fn publish_output_candidate",
        "pub fn validate_output_candidate",
        "fn validate_candidate_tree_shape",
        "metadata::validate_candidate_metadata",
        "report::validate_candidate_committed_reports_temporary_paths",
        "sync::sync_working_tree",
    ] {
        assert!(
            publish.contains(required),
            "output_repo/publish.rs should own candidate publication item {required}"
        );
    }

    for moved_item in [
        "pub(crate) fn publish_output_candidate",
        "pub fn validate_output_candidate",
        "crate::upstream::validate_tree",
    ] {
        assert!(
            !output_repo.contains(moved_item),
            "output_repo.rs should not retain candidate publication implementation {moved_item}"
        );
    }

    let forbidden_upstream_resolution_policy = [
        "crate::upstream",
        "upstream::",
        "ResolvedBase",
        "resolve_ref",
        "resolve_candidate_plan",
        "load_lockfile",
        "load_authoritative_published_state",
        "KslimConfig",
        "ProfileConfig",
        "crate::generate",
        "branch_name(",
        "tag_name(",
    ];

    for forbidden in forbidden_upstream_resolution_policy {
        assert!(
            !publish.contains(forbidden),
            "output_repo/publish.rs must not know upstream resolution policy; found forbidden token {forbidden}"
        );
    }

    for forbidden_candidate_publication_write in [
        "write_verified_published_snapshot_metadata",
        "write_verified_committed_published_snapshot_metadata",
    ] {
        assert!(
            !publish.contains(forbidden_candidate_publication_write),
            "output_repo/publish.rs must not write published metadata through candidate publication APIs; found {forbidden_candidate_publication_write}"
        );
    }
}
