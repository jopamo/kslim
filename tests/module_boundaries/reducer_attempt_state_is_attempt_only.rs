use super::common::*;

#[test]
fn reducer_attempt_state_is_attempt_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let reducer = production_source(&root.join("src/reducer/mod.rs"));
    let state = production_source(&root.join("src/reducer/state.rs"));

    assert!(
        reducer.contains("mod state;")
            && reducer.contains("pub(crate) use state::ReducerAttemptState;"),
        "reducer/mod.rs should register and expose reducer attempt state inside the crate"
    );

    for required in [
        "pub(crate) struct ReducerAttemptState",
        "stage: ReducerStage",
        "status: Option<ReducerStatus>",
        "attempt_metadata_dir: AttemptMetadataDir",
        "partial_reports: Vec<ReportPath>",
        "pub(crate) fn new(",
        "report.as_path().starts_with(attempt_metadata_dir.as_path())",
        "reducer attempt report outside attempt metadata",
        "partial_reports.sort()",
        "partial_reports.dedup()",
        "pub(crate) fn in_progress(",
        "Self::new(stage, None, attempt_metadata_dir, partial_reports)",
        "pub(crate) fn completed(",
        "Self::new(stage, Some(status), attempt_metadata_dir, partial_reports)",
        "pub(crate) fn stage(&self)",
        "pub(crate) fn status(&self)",
        "pub(crate) fn attempt_metadata_dir(&self)",
        "pub(crate) fn partial_reports(&self)",
        "pub(crate) fn is_complete(&self)",
    ] {
        assert!(
            state.contains(required),
            "ReducerAttemptState should capture only reducer attempt facts; missing {required}"
        );
    }

    for forbidden in [
        "pub(crate) stage",
        "pub(crate) status",
        "pub(crate) attempt_metadata_dir",
        "pub(crate) partial_reports",
        "RequestedGenerateState",
        "ResolvedCandidateState",
        "CandidateTreeState",
        "CandidateVerification",
        "CandidateMetadataDir",
        "OutputRepoPath",
        "PublishedMetadataDir",
        "PublishedSnapshotState",
        "GenerateAttemptFailure",
        "CommittedOutputSnapshot",
        "SuccessfulCommitResult",
        "LockfilePath",
        "commit_if_changed",
        "write_authoritative_lockfile",
        "write_verified_published_snapshot_metadata",
        "std::fs::write",
    ] {
        assert!(
            !state.contains(forbidden),
            "ReducerAttemptState must not expose mutable fields or contain requested, resolved, candidate, published, commit, lockfile, or mutation state; found {forbidden}"
        );
    }
}
