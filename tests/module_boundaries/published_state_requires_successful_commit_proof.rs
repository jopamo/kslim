use super::common::*;

fn section_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let start_offset = source
        .find(start)
        .unwrap_or_else(|| panic!("missing source section start: {start}"));
    let rest = &source[start_offset..];
    let end_offset = rest
        .find(end)
        .unwrap_or_else(|| panic!("missing source section end after {start}: {end}"));
    &rest[..end_offset]
}

#[test]
fn published_state_requires_successful_commit_proof() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let generate = production_source(&root.join("src/generate.rs"));
    let state = state_source(root);
    let publish = production_source(&root.join("src/generate/publish.rs"));
    let docs = std::fs::read_to_string(root.join("docs/architecture.md"))
        .expect("architecture doc should be readable");

    let published_section = section_between(
        &state,
        "pub(crate) struct PublishedSnapshotState",
        "pub(crate) enum GenerateErrorKind",
    );
    let generate_commit_section = section_between(&generate, "// ── commit", "drop(temp_dir);");
    let commit_proof_section = section_between(
        &publish,
        "fn commit_output_repo_state(",
        "fn write_output_metadata_report_and_manifest(",
    );
    let publish_snapshot_section = section_between(
        &publish,
        "let publish_result = (|| -> Result<PublishedSnapshotState> {",
        "write_authoritative_lockfile_from_committed_publish(",
    );

    for required in [
        "struct OutputTargetReservation",
        "snapshot: PublishedSnapshotState",
        "fn from_successful_commit(",
        ".record_output_target(OutputTargetReservation::from_output_target(",
        ".record_published(PublishedGenerateState::from_successful_commit(",
    ] {
        assert!(
            generate.contains(required),
            "generate.rs should separate output target reservation from successful published state; missing {required}"
        );
    }
    assert!(
        !generate.contains("snapshot: Option<PublishedSnapshotState>"),
        "PublishedGenerateState must contain a concrete snapshot, not a pre-commit optional snapshot"
    );
    assert!(
        state.contains("commit: &SuccessfulCommitResult"),
        "published snapshot conversion should require a successful commit proof"
    );
    assert!(
        !state.contains("commit: &CommitResult"),
        "published snapshot conversion must not accept a generic commit result"
    );

    for required in [
        "pub(crate) struct CommittedOutputSnapshot",
        "pub(crate) fn from_successful_commit(",
        "commit: &SuccessfulCommitResult",
        "pub(crate) fn from_committed_output(snapshot: CommittedOutputSnapshot)",
    ] {
        assert!(
            published_section.contains(required),
            "PublishedSnapshotState should require committed output proof; missing {required}"
        );
    }
    for forbidden in [
        "pub(crate) fn new(",
        "pub(crate) fn new(",
        "impl From<SuccessfulCommitResult> for PublishedSnapshotState",
        "impl TryFrom<SuccessfulCommitResult> for PublishedSnapshotState",
        "impl From<&SuccessfulCommitResult> for PublishedSnapshotState",
        "impl TryFrom<&SuccessfulCommitResult> for PublishedSnapshotState",
        "commit_if_changed",
        "head_commit",
    ] {
        assert!(
            !published_section.contains(forbidden),
            "PublishedSnapshotState must be reached through CommittedOutputSnapshot proof, not direct commit or constructor state; found {forbidden}"
        );
    }

    let output_target_offset = generate_commit_section
        .find(".record_output_target(OutputTargetReservation::from_output_target(")
        .expect("generate should reserve output target before commit");
    let commit_result_offset = generate_commit_section
        .find("let commit_result = commit_output_repo_state(")
        .expect("generate should obtain commit proof before published state");
    let published_offset = generate_commit_section
        .find(".record_published(PublishedGenerateState::from_successful_commit(")
        .expect("generate should record published state from successful commit proof");
    assert!(
        output_target_offset < commit_result_offset && commit_result_offset < published_offset,
        "generate must reserve output, complete output commit, then record published state"
    );
    for forbidden_before_commit in [
        ".record_published(",
        "PublishedGenerateState::from_successful_commit(",
        "PublishedSnapshotState::from_committed_output(",
        "CommittedOutputSnapshot::from_successful_commit(",
    ] {
        assert!(
            !generate_commit_section[..commit_result_offset].contains(forbidden_before_commit),
            "generate must not create published state before commit_output_repo_state returns; found {forbidden_before_commit}"
        );
    }

    let commit_if_changed_offset = commit_proof_section
        .find("crate::git::commit_if_changed(output_path, &message)?")
        .expect("commit proof should run output commit/no-op");
    let head_commit_offset = commit_proof_section
        .find("let output_commit = crate::git::head_commit(output_path)?")
        .expect("commit proof should read output HEAD after commit/no-op");
    let proof_offset = commit_proof_section
        .find("Ok(SuccessfulCommitResult {")
        .expect("commit proof should return SuccessfulCommitResult");
    assert!(
        commit_if_changed_offset < head_commit_offset && head_commit_offset < proof_offset,
        "SuccessfulCommitResult may be returned only after commit/no-op and HEAD lookup succeed"
    );
    for forbidden in [
        "PublishedGenerateState",
        "PublishedSnapshotState::from_committed_output",
        "CommittedOutputSnapshot::from_successful_commit",
        ".record_published(",
    ] {
        assert!(
            !commit_proof_section.contains(forbidden),
            "commit_output_repo_state should produce commit proof only, not published state; found {forbidden}"
        );
    }

    let publish_commit_offset = publish_snapshot_section
        .find("crate::git::commit_if_changed(output_path, &commit_message(plan, &verification))?")
        .expect("publish should run output commit/no-op before snapshot");
    let publish_head_offset = publish_snapshot_section
        .find("let output_commit = crate::git::head_commit(output_path)?")
        .expect("publish should read output HEAD before snapshot");
    let publish_proof_offset = publish_snapshot_section
        .find("let commit = SuccessfulCommitResult {")
        .expect("publish should build successful commit proof before snapshot");
    let publish_committed_snapshot_offset = publish_snapshot_section
        .find("CommittedOutputSnapshot::from_successful_commit(")
        .expect("publish should convert successful commit proof into committed snapshot");
    let publish_snapshot_offset = publish_snapshot_section
        .find("PublishedSnapshotState::from_committed_output(committed)?")
        .expect("publish should create published snapshot from committed proof");
    assert!(
        publish_commit_offset < publish_head_offset
            && publish_head_offset < publish_proof_offset
            && publish_proof_offset < publish_committed_snapshot_offset
            && publish_committed_snapshot_offset < publish_snapshot_offset,
        "publish must commit output, read HEAD, build proof, then create published snapshot"
    );
    for forbidden_before_publish_commit in [
        "SuccessfulCommitResult {",
        "CommittedOutputSnapshot::from_successful_commit(",
        "PublishedSnapshotState::from_committed_output(",
    ] {
        assert!(
            !publish_snapshot_section[..publish_commit_offset]
                .contains(forbidden_before_publish_commit),
            "publish must not create published proof/state before output commit succeeds; found {forbidden_before_publish_commit}"
        );
    }

    for required in [
        "`PublishedSnapshotState` records only committed output repo",
        "`CommittedOutputSnapshot`, not from candidate, failure, or generic commit",
        "built only from",
        "`SuccessfulCommitResult` after the output commit/no-op succeeds",
        "HEAD has been read",
    ] {
        assert!(
            docs.contains(required),
            "architecture docs should document published-state commit proof boundary; missing {required}"
        );
    }
}
