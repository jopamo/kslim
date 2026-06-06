use super::*;

#[test]
fn test_validate_output_candidate_requires_candidate_metadata_before_publish() {
    let tmp = tempfile::tempdir().unwrap();
    let candidate = tmp.path().join("candidate");
    for dir in &[
        "arch", "drivers", "fs", "include", "kernel", "mm", "net", "scripts", ".kslim",
    ] {
        std::fs::create_dir_all(candidate.join(dir)).unwrap();
    }
    std::fs::write(candidate.join("Makefile"), "# test\n").unwrap();
    std::fs::write(candidate.join("Kconfig"), "# test\n").unwrap();
    std::fs::write(
        candidate.join(".kslim/managed.toml"),
        "managed_by = \"kslim\"\n",
    )
    .unwrap();
    std::fs::write(candidate.join(format!(".kslim/{}", BASE_METADATA_FILE)), "").unwrap();
    std::fs::write(
        candidate.join(format!(".kslim/{}", GENERATED_METADATA_FILE)),
        "",
    )
    .unwrap();
    std::fs::write(candidate.join(".kslim/manifest.txt"), "hash  1  Makefile\n").unwrap();

    let err = validate_output_candidate(&candidate)
        .unwrap_err()
        .to_string();
    assert!(err.contains(REPORT_FILE));
    assert!(err.contains("required candidate metadata missing"));
}

#[test]
fn test_validate_output_candidate_rejects_candidate_published_metadata_before_publish() {
    let tmp = tempfile::tempdir().unwrap();
    let candidate = tmp.path().join("candidate");
    create_valid_output_candidate(&candidate);
    std::fs::write(
        candidate.join(format!(".kslim/{}", PUBLISHED_SNAPSHOT_FILE)),
        "branch = \"candidate-should-not-publish\"\n",
    )
    .unwrap();

    let err = validate_output_candidate(&candidate)
        .unwrap_err()
        .to_string();

    assert!(err.contains("candidate metadata must not contain published snapshot metadata"));
    assert!(err.contains(PUBLISHED_SNAPSHOT_FILE));
}

#[test]
fn test_validate_output_candidate_rejects_temporary_candidate_path_in_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    let candidate = tmp.path().join("candidate");
    create_valid_output_candidate(&candidate);
    std::fs::write(
        candidate.join(format!(".kslim/{}", BASE_METADATA_FILE)),
        format!("base_ref = \"{}\"\n", candidate.display()),
    )
    .unwrap();

    let err = validate_output_candidate(&candidate)
        .unwrap_err()
        .to_string();

    assert!(err.contains("temporary path"));
    assert!(err.contains(BASE_METADATA_FILE));
    assert!(err.contains("non-authoritative attempt metadata"));
}

#[test]
fn test_validate_output_candidate_rejects_host_absolute_path_in_committed_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    let candidate = tmp.path().join("candidate");
    create_valid_output_candidate(&candidate);
    std::fs::write(
        candidate.join(format!(".kslim/{}", BASE_METADATA_FILE)),
        "upstream_url = \"file:///var/lib/host-only/linux.git\"\n",
    )
    .unwrap();

    let err = validate_output_candidate(&candidate)
        .unwrap_err()
        .to_string();

    assert!(err.contains("host-only absolute path"));
    assert!(err.contains(BASE_METADATA_FILE));
    assert!(err.contains("non-authoritative attempt metadata"));
}

#[test]
fn test_validate_output_candidate_rejects_temporary_candidate_path_in_committed_report() {
    let tmp = tempfile::tempdir().unwrap();
    let candidate = tmp.path().join("candidate");
    create_valid_output_candidate(&candidate);
    std::fs::write(
        candidate.join(format!(".kslim/{}", REDUCER_REPORT_JSON)),
        format!("{{\"candidate\":\"{}\"}}", candidate.display()),
    )
    .unwrap();

    let err = validate_output_candidate(&candidate)
        .unwrap_err()
        .to_string();

    assert!(err.contains("committed report"));
    assert!(err.contains("temporary path"));
    assert!(err.contains("non-authoritative attempt metadata"));
}

#[test]
fn test_validate_output_candidate_rejects_timestamp_outside_reproducible_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let candidate = tmp.path().join("candidate");
    create_valid_output_candidate(&candidate);
    std::fs::write(
        candidate.join(format!(".kslim/{}", REPORT_FILE)),
        "report generated at 2026-01-02T00:00:00Z\n",
    )
    .unwrap();

    let err = validate_output_candidate(&candidate)
        .unwrap_err()
        .to_string();

    assert!(err.contains("outside reproducible timestamp policy"));
    assert!(err.contains(REPORT_FILE));
}

#[test]
fn test_validate_output_candidate_rejects_fractional_real_timestamp_in_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    let candidate = tmp.path().join("candidate");
    create_valid_output_candidate(&candidate);
    std::fs::write(
        candidate.join(format!(".kslim/{}", REPORT_FILE)),
        "wall-clock: 2026-01-01T00:00:00.123456789Z\n",
    )
    .unwrap();

    let err = validate_output_candidate(&candidate)
        .unwrap_err()
        .to_string();

    assert!(err.contains("non-reproducible timestamp"));
    assert!(err.contains(REPORT_FILE));
}

#[test]
fn test_reducer_artifact_path_rejects_last_attempt_name() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");

    let err = reducer_artifact_path(&output, LAST_ATTEMPT_JSON)
        .unwrap_err()
        .to_string();

    assert!(err.contains("non-authoritative attempt metadata"));
}

#[test]
fn test_reducer_artifact_path_rejects_nested_names() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");

    let err = reducer_artifact_path(&output, "reports/reducer-report.json")
        .unwrap_err()
        .to_string();

    assert!(err.contains("single file name"));
}
