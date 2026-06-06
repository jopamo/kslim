use super::common::*;

#[test]
fn generate_attempt_failure_is_attempt_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let state = state_source(root);
    let failure = production_source(&root.join("src/generate/failure.rs"));

    let failure_section = state
        .split("pub(crate) struct GenerateAttemptFailure")
        .nth(1)
        .expect("generate/state.rs should define GenerateAttemptFailure as generate-private state");

    for required in [
        "stage: GenerateStage",
        "error_kind: GenerateErrorKind",
        "message: String",
        "attempt_metadata_dir: AttemptMetadataDir",
        "partial_reports: Vec<ReportPath>",
        "pub(crate) fn from_stage(",
        "GenerateErrorKind::from_stage(stage)",
        "pub(crate) fn new(",
        "generate attempt failure message is empty",
        "report.as_path().starts_with(attempt_metadata_dir.as_path())",
        "generate attempt failure report outside attempt metadata",
        "partial_reports.sort()",
        "partial_reports.dedup()",
        "pub(crate) fn stage(&self)",
        "pub(crate) fn error_kind(&self)",
        "pub(crate) fn message(&self)",
        "pub(crate) fn attempt_metadata_dir(&self)",
        "pub(crate) fn partial_reports(&self)",
    ] {
        assert!(
            failure_section.contains(required),
            "GenerateAttemptFailure should capture only non-authoritative attempt failure facts; missing {required}"
        );
    }

    for forbidden in [
        "pub(crate) struct GenerateAttemptFailure",
        "pub(crate) stage",
        "pub(crate) error_kind",
        "pub(crate) message",
        "pub(crate) attempt_metadata_dir",
        "pub(crate) partial_reports",
        "RequestedGenerateState",
        "ResolvedCandidateState",
        "CandidateTreeState",
        "CandidateVerification",
        "OutputRepoPath",
        "PublishedMetadataDir",
        "PublishedSnapshotState",
        "CommittedOutputSnapshot",
        "SuccessfulCommitResult",
        "LockfilePath",
        "commit_if_changed",
        "write_authoritative_lockfile",
        "write_verified_published_snapshot_metadata",
        "std::fs::write",
    ] {
        assert!(
            !failure_section.contains(forbidden),
            "GenerateAttemptFailure must not expose mutable crate-wide fields or contain requested, resolved, candidate, published, commit, lockfile, or mutation state; found {forbidden}"
        );
    }

    assert!(
        failure.contains("GenerateAttemptFailure::from_stage(")
            && failure.contains(".map(|attempt| attempt.stage())")
            && failure
                .contains(".map(|attempt| render_json_string(attempt.error_kind().as_str()))")
            && failure.contains(".map(|attempt| attempt.message())"),
        "generate/failure.rs should record and render attempt failure through typed accessors"
    );
    assert!(
        failure.contains(
            "const NON_AUTHORITATIVE_ATTEMPT_SCOPE: &str = \"non-authoritative-attempt\""
        ) && failure.contains("metadata_scope: String::from(NON_AUTHORITATIVE_ATTEMPT_SCOPE)")
            && failure.contains("authoritative: false")
            && failure.contains("GenerateErrorKind::from_stage(stage)"),
        "generate/failure.rs should write only non-authoritative attempt metadata"
    );
}
