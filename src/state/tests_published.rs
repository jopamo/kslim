use super::*;

#[test]
fn test_published_snapshot_state_requires_committed_output_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");
    let lockfile = tmp.path().join("project").join("kslim.lock");
    std::fs::create_dir_all(output.join(".git")).unwrap();

    let committed = CommittedOutputSnapshot::from_successful_commit(
        &output,
        LockfilePath::new(&lockfile).unwrap(),
        &test_commit_result(),
    )
    .unwrap();
    let state = PublishedSnapshotState::from_committed_output(committed).unwrap();

    assert_eq!(state.output_repo.as_path(), output.as_path());
    assert_eq!(
        state.metadata_dir.as_path(),
        output.join(".git/kslim").as_path()
    );
    assert_eq!(state.branch.as_str(), "kslim/v1.0/default");
    assert_eq!(state.commit.as_str(), "deadbeef");
    assert_eq!(state.lockfile.as_path(), lockfile.as_path());
}

#[test]
fn test_published_snapshot_state_rejects_missing_commit_or_lockfile() {
    let mut commit = test_commit_result();
    commit.output_commit = String::from(" ");
    let err = CommittedOutputSnapshot::from_successful_commit(
        "/tmp/output",
        LockfilePath::new("/tmp/project/kslim.lock").unwrap(),
        &commit,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("git commit id must not be empty"));

    let err = LockfilePath::new("").unwrap_err().to_string();
    assert!(err.contains("authoritative lockfile path is empty"));
}

#[test]
fn test_generate_attempt_failure_is_not_a_published_snapshot_source() {
    let failure = GenerateAttemptFailure::from_stage(
        GenerateStage::Publish,
        "publish failed",
        AttemptMetadataDir::new("/tmp/project/.kslim/attempt").unwrap(),
        vec![ReportPath::new("/tmp/project/.kslim/attempt/report.txt").unwrap()],
    )
    .unwrap();

    assert_eq!(failure.error_kind, GenerateErrorKind::Publish);

    let committed = CommittedOutputSnapshot::from_successful_commit(
        "/tmp/output",
        LockfilePath::new("/tmp/project/kslim.lock").unwrap(),
        &test_commit_result(),
    )
    .unwrap();
    let published = PublishedSnapshotState::from_committed_output(committed).unwrap();

    assert_eq!(published.commit.as_str(), "deadbeef");
}

#[test]
fn test_generate_attempt_failure_captures_non_authoritative_failure_state() {
    let attempt_dir = AttemptMetadataDir::new("/tmp/project/.kslim/attempt").unwrap();
    let partial_reports = vec![
        ReportPath::new("/tmp/project/.kslim/attempt/report.txt").unwrap(),
        ReportPath::new("/tmp/project/.kslim/attempt/last-attempt.json").unwrap(),
        ReportPath::new("/tmp/project/.kslim/attempt/report.txt").unwrap(),
    ];
    let sorted_partial_reports = vec![
        ReportPath::new("/tmp/project/.kslim/attempt/last-attempt.json").unwrap(),
        ReportPath::new("/tmp/project/.kslim/attempt/report.txt").unwrap(),
    ];

    let failure = GenerateAttemptFailure::from_stage(
        GenerateStage::Selftest,
        "selftest failed",
        attempt_dir.clone(),
        partial_reports.clone(),
    )
    .unwrap();

    assert_eq!(failure.stage, GenerateStage::Selftest);
    assert_eq!(failure.error_kind, GenerateErrorKind::Selftest);
    assert_eq!(failure.message, "selftest failed");
    assert_eq!(failure.attempt_metadata_dir, attempt_dir);
    assert_eq!(failure.partial_reports, sorted_partial_reports);
}

#[test]
fn test_generate_attempt_failure_rejects_empty_message_or_reports_outside_attempt_dir() {
    let attempt_dir = AttemptMetadataDir::new("/tmp/project/.kslim/attempt").unwrap();

    let err = GenerateAttemptFailure::from_stage(
        GenerateStage::Reduce,
        " ",
        attempt_dir.clone(),
        Vec::new(),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("generate attempt failure message is empty"));

    let err = GenerateAttemptFailure::from_stage(
        GenerateStage::Reduce,
        "reducer failed",
        attempt_dir,
        vec![ReportPath::new("/tmp/project/.kslim/report.txt").unwrap()],
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("outside attempt metadata"));
}
