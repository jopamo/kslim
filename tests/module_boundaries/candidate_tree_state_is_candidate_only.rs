use super::common::*;

#[test]
fn candidate_tree_state_is_candidate_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let state = state_source(root);
    let candidate_write = production_source(&root.join("src/generate/candidate/write.rs"));
    let verify = production_source(&root.join("src/generate/verify.rs"));
    let publish = production_source(&root.join("src/generate/publish.rs"));

    let candidate_section = state
        .split("pub(crate) struct CandidateTreeState")
        .nth(1)
        .and_then(|rest| {
            rest.split("pub(crate) struct PublishedSnapshotState")
                .next()
        })
        .expect("generate/state.rs should define candidate tree state before published state");

    for required in [
        "tree: CandidateTreePath",
        "metadata_dir: CandidateMetadataDir",
        "materialized: bool",
        "integrated: bool",
        "pruned: bool",
        "reduced: bool",
        "selftested: bool",
        "pub(crate) fn new(",
        "CandidateMetadataDir::new_in_candidate_tree(&tree, metadata_dir.as_path())?",
        "candidate tree state cannot advance before materialization",
        "pub(crate) fn from_materialized_tree(",
        "crate::output_repo::candidate_metadata_dir(&tree)?",
        "pub(crate) fn mark_integrated(",
        "pub(crate) fn mark_pruned(",
        "pub(crate) fn mark_reduced(",
        "pub(crate) fn mark_selftested(",
        "fn ensure_materialized_for(",
    ] {
        assert!(
            candidate_section.contains(required),
            "CandidateTreeState should capture only candidate tree lifecycle facts; missing {required}"
        );
    }

    for forbidden in [
        "RequestedGenerateState",
        "ResolvedCandidateState",
        "ResolvedBase",
        "OutputRepoPath",
        "PublishedMetadataDir",
        "PublishedSnapshotState",
        "CommittedOutputSnapshot",
        "GenerateAttemptFailure",
        "AttemptMetadataDir",
        "SuccessfulCommitResult",
        "LockfilePath",
        "write_authoritative_lockfile",
        "commit_if_changed",
        "write_verified_published_snapshot_metadata",
    ] {
        assert!(
            !candidate_section.contains(forbidden),
            "CandidateTreeState must not contain requested, resolved, published, failure, commit, or lockfile state; found {forbidden}"
        );
    }

    assert!(
        candidate_write.contains("Result<CandidateTreeState>")
            && verify.contains("candidate: &CandidateTreeState")
            && publish.contains("candidate: CandidateTreeState"),
        "candidate build, verification, and publish boundaries should consume typed candidate tree state"
    );
}
