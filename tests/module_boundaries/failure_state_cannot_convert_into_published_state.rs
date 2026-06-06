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
fn failure_state_cannot_convert_into_published_state() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let state = state_source(root);
    let generate = production_source(&root.join("src/generate.rs"));
    let failure = production_source(&root.join("src/generate/failure.rs"));
    let publish = production_source(&root.join("src/generate/publish.rs"));
    let lockfile = production_source(&root.join("src/lockfile.rs"));
    let docs = std::fs::read_to_string(root.join("docs/architecture.md"))
        .expect("architecture doc should be readable");

    let published_snapshot_section = section_between(
        &state,
        "pub(crate) struct PublishedSnapshotState",
        "pub(crate) enum GenerateErrorKind",
    );
    let attempt_failure_section = state
        .split("pub(crate) struct GenerateAttemptFailure")
        .nth(1)
        .expect("generate/state.rs should define GenerateAttemptFailure");
    let published_generate_section = section_between(
        &generate,
        "struct PublishedGenerateState",
        "struct FailureGenerateState",
    );
    let publication_guard_section = section_between(
        &failure,
        "fn ensure_no_attempt_failure_before_publication(",
        "fn project_failure_report_path(",
    );
    let publish_command_section = section_between(
        &publish,
        "log_publish_stage(PublishStage::BuildPublishedSnapshot);",
        "write_authoritative_lockfile_from_committed_publish(",
    );

    for required in [
        "pub(crate) struct CommittedOutputSnapshot",
        "pub(crate) fn from_successful_commit(",
        "commit: &SuccessfulCommitResult",
        "pub(crate) fn from_committed_output(snapshot: CommittedOutputSnapshot)",
    ] {
        assert!(
            published_snapshot_section.contains(required),
            "published snapshot state should require committed output proof before publication; missing {required}"
        );
    }

    for required in [
        "stage: GenerateStage",
        "error_kind: GenerateErrorKind",
        "message: String",
        "attempt_metadata_dir: AttemptMetadataDir",
        "partial_reports: Vec<ReportPath>",
    ] {
        assert!(
            attempt_failure_section.contains(required),
            "attempt failure state should contain only non-authoritative failure facts; missing {required}"
        );
    }

    for forbidden in [
        "impl From<GenerateAttemptFailure> for PublishedSnapshotState",
        "impl TryFrom<GenerateAttemptFailure> for PublishedSnapshotState",
        "impl From<&GenerateAttemptFailure> for PublishedSnapshotState",
        "impl TryFrom<&GenerateAttemptFailure> for PublishedSnapshotState",
        "impl From<GenerateAttemptFailure> for CommittedOutputSnapshot",
        "impl TryFrom<GenerateAttemptFailure> for CommittedOutputSnapshot",
        "impl From<&GenerateAttemptFailure> for CommittedOutputSnapshot",
        "impl TryFrom<&GenerateAttemptFailure> for CommittedOutputSnapshot",
        "impl From<GenerateAttemptFailure> for PublishedGenerateState",
        "impl TryFrom<GenerateAttemptFailure> for PublishedGenerateState",
        "impl From<&GenerateAttemptFailure> for PublishedGenerateState",
        "impl TryFrom<&GenerateAttemptFailure> for PublishedGenerateState",
        "impl From<GenerateAttemptFailure> for PublishedLockfileUpdate",
        "impl TryFrom<GenerateAttemptFailure> for PublishedLockfileUpdate",
        "impl From<&GenerateAttemptFailure> for PublishedLockfileUpdate",
        "impl TryFrom<&GenerateAttemptFailure> for PublishedLockfileUpdate",
        "fn from_failure",
    ] {
        for (label, source) in [
            ("generate/state.rs", state.as_str()),
            ("generate.rs", generate.as_str()),
            ("generate/failure.rs", failure.as_str()),
            ("generate/publish.rs", publish.as_str()),
            ("lockfile.rs", lockfile.as_str()),
        ] {
            assert!(
                !source.contains(forbidden),
                "{label} must not define a failure-to-published conversion; found {forbidden}"
            );
        }
    }

    for forbidden in [
        "GenerateAttemptFailure",
        "FailureGenerateState",
        "FailureReportContext",
        "attempt_failure",
        "from_failure",
    ] {
        assert!(
            !published_snapshot_section.contains(forbidden),
            "PublishedSnapshotState construction must not mention failure state; found {forbidden}"
        );
        assert!(
            !published_generate_section.contains(forbidden),
            "PublishedGenerateState construction must not mention failure state; found {forbidden}"
        );
        assert!(
            !publish_command_section.contains(forbidden),
            "publish command snapshot construction must not mention failure state; found {forbidden}"
        );
    }

    for required in [
        "failure.attempt_failure.as_ref()",
        "attempt.stage().as_str()",
        "cannot be converted into published state",
    ] {
        assert!(
            publication_guard_section.contains(required),
            "generate publication should fail closed when an attempt failure exists; missing {required}"
        );
    }
    let guard_offset = generate
        .find("ensure_no_attempt_failure_before_publication(failure)?;")
        .expect("generate should guard publication against prior attempt failure");
    let record_published_offset = generate[guard_offset..]
        .find(".record_published(PublishedGenerateState::from_successful_commit(")
        .expect("publication guard should run before recording published state");
    assert!(
        record_published_offset > 0,
        "publication guard should precede published state recording"
    );

    for required in [
        "GenerateAttemptFailure` records only non-authoritative attempt metadata",
        "It must never be",
        "converted into `PublishedSnapshotState`",
        "`PublishedGenerateState`",
        "publication still flows only through",
        "`CommittedOutputSnapshot` built from a successful output commit",
    ] {
        assert!(
            docs.contains(required),
            "architecture docs should document the failure-to-published boundary; missing {required}"
        );
    }
}
