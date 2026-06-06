use super::common::*;

#[test]
fn published_snapshot_state_is_commit_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let state = state_source(root);
    let generate = production_source(&root.join("src/generate.rs"));
    let publish = production_source(&root.join("src/generate/publish.rs"));

    let published_section = state
        .split("pub(crate) struct PublishedSnapshotState")
        .nth(1)
        .and_then(|rest| rest.split("pub(crate) enum GenerateErrorKind").next())
        .expect("generate/state.rs should define published snapshot state before generate errors");

    for required in [
        "output_repo: OutputRepoPath",
        "metadata_dir: PublishedMetadataDir",
        "branch: OutputBranchName",
        "commit: GitCommitId",
        "lockfile: LockfilePath",
        "pub(crate) struct CommittedOutputSnapshot",
        "pub(crate) fn from_successful_commit(",
        "commit: &SuccessfulCommitResult",
        "OutputRepoPath::new(output_repo)?",
        "OutputBranchName::new(commit.branch.clone())?",
        "GitCommitId::new(commit.output_commit.clone())?",
        "pub(crate) fn from_committed_output(snapshot: CommittedOutputSnapshot)",
        "crate::output_repo::published_metadata_dir(&snapshot.output_repo)?",
        "pub(crate) fn output_repo(&self)",
        "pub(crate) fn metadata_dir(&self)",
        "pub(crate) fn branch(&self)",
        "pub(crate) fn commit(&self)",
    ] {
        assert!(
            published_section.contains(required),
            "PublishedSnapshotState should be constructed only from committed output proof; missing {required}"
        );
    }

    for forbidden in [
        "RequestedGenerateState",
        "ResolvedCandidateState",
        "CandidateTreeState",
        "CandidateVerification",
        "CandidateMetadataDir",
        "GenerateAttemptFailure",
        "AttemptMetadataDir",
        "commit: &CommitResult",
        "commit_if_changed",
        "write_authoritative_lockfile",
        "std::fs::write",
    ] {
        assert!(
            !published_section.contains(forbidden),
            "PublishedSnapshotState must not contain requested, resolved, candidate, failure, mutation, or generic commit state; found {forbidden}"
        );
    }

    assert!(
        generate.contains("snapshot: PublishedSnapshotState")
            && !generate.contains("snapshot: Option<PublishedSnapshotState>")
            && publish.contains("CommittedOutputSnapshot::from_successful_commit(")
            && publish.contains("PublishedSnapshotState::from_committed_output(committed)?"),
        "published state should be concrete and reached only after successful output commit proof"
    );
}
