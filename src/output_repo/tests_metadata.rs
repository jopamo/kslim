use super::*;

#[test]
fn test_committed_metadata_sanitizes_host_specific_absolute_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");
    let upstream_path = tmp.path().join("upstreams/linux.git");
    let patch_path = tmp.path().join("patches/topic");
    let upstream_path = upstream_path.to_string_lossy().to_string();
    let patch_path = patch_path.to_string_lossy().to_string();

    let mut config = crate::config::default_kslim_config("demo", output.to_str().unwrap());
    config.upstream.name = String::from("linux-local");
    config.upstream.url = upstream_path.clone();
    let profile = crate::config::default_profile_config("v1.0");
    let resolved = crate::lockfile::ResolvedBase {
        upstream: config.upstream.name.clone(),
        url: config.upstream.url.clone(),
        r#ref: String::from("v1.0"),
        commit: String::from("0123456789012345678901234567890123456789"),
        resolved_at: String::from("2026-01-01T00:00:00Z"),
    };
    let patches = vec![PatchInfo {
        source: String::from("worktree"),
        worktree_path: patch_path.clone(),
        branch: String::from("topic/path"),
        head_commit: String::from("abcdef0123456789abcdef0123456789abcdef01"),
        merge_base: String::from("1111111111111111111111111111111111111111"),
        base_remote: String::from("origin"),
        base_ref: String::from("master"),
        patch_count: 1,
    }];

    write_base_metadata(
        output.to_str().unwrap(),
        &config,
        &profile,
        &resolved,
        "slimmed",
    )
    .unwrap();
    write_patch_metadata(output.to_str().unwrap(), Some(&patches)).unwrap();
    write_report(
        output.to_str().unwrap(),
        &config,
        &profile,
        &resolved,
        1,
        2,
        "slimmed",
        GenerateStage::Metadata,
        Some(&patches),
        None,
    )
    .unwrap();
    let commit_details = CommitMessageDetails::new(
        "fingerprint-plan",
        "ran=true files_removed=0 dirs_removed=0 edits=0",
        "enabled=false built_in_checks=0 kernel_builds=0 commands=0",
    );
    let commit_message =
        commit_message(&config, &profile, &resolved, "slimmed", &commit_details);

    let metadata_dir = output.join(".kslim");
    let committed_metadata_and_message = [
        std::fs::read_to_string(metadata_dir.join(BASE_METADATA_FILE)).unwrap(),
        std::fs::read_to_string(metadata_dir.join(PATCH_METADATA_FILE)).unwrap(),
        std::fs::read_to_string(metadata_dir.join(REPORT_FILE)).unwrap(),
        commit_message,
    ]
    .join("\n");

    assert!(!committed_metadata_and_message.contains(&upstream_path));
    assert!(!committed_metadata_and_message.contains(&patch_path));
    assert!(committed_metadata_and_message.contains("local-upstream:linux-local"));
    assert!(committed_metadata_and_message.contains("local-worktree:topic/path"));
    assert!(committed_metadata_and_message.contains("Stage: metadata"));
}

#[test]
fn test_host_specific_path_detection_covers_local_url_and_windows_forms() {
    assert!(is_host_specific_absolute_path(
        "/tmp/linux.git"
    ));
    assert!(is_host_specific_absolute_path(
        "file:///tmp/linux.git"
    ));
    assert!(is_host_specific_absolute_path(
        "C:\\tmp\\linux.git"
    ));
    assert!(is_host_specific_absolute_path(
        "\\\\server\\share\\linux.git"
    ));
    assert!(!is_host_specific_absolute_path(
        "https://example.com/linux.git"
    ));
    assert!(!is_host_specific_absolute_path(
        "git@example.com:linux/kernel.git"
    ));
    assert!(!is_host_specific_absolute_path("\\\\n"));
    assert!(!is_host_specific_absolute_path(
        "\\\\\\\\n"
    ));
}

#[test]
fn test_generated_metadata_requires_reproducible_timestamp() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");

    let err = write_generated_metadata(output.to_str().unwrap(), "")
        .unwrap_err()
        .to_string();
    assert!(err.contains("reproducible RFC3339 timestamp"));

    let err = write_generated_metadata(output.to_str().unwrap(), "2026-01-01T00:00:00")
        .unwrap_err()
        .to_string();
    assert!(err.contains("reproducible RFC3339 timestamp"));

    write_generated_metadata(output.to_str().unwrap(), "2026-01-01T00:00:00+00:00")
        .unwrap();
    let generated =
        std::fs::read_to_string(output.join(".kslim").join(GENERATED_METADATA_FILE)).unwrap();
    assert!(generated.contains("generated_at = \"2026-01-01T00:00:00+00:00\""));
}

#[test]
fn test_reproducible_timestamp_policy_accepts_z_and_offset_forms() {
    assert!(is_reproducible_timestamp(
        "2026-01-01T00:00:00Z"
    ));
    assert!(is_reproducible_timestamp(
        "2026-01-01T00:00:00+00:00"
    ));
    assert!(is_reproducible_timestamp(
        "2026-01-01T00:00:00-06:00"
    ));
    assert!(!is_reproducible_timestamp(""));
    assert!(!is_reproducible_timestamp(
        "2026-01-01T00:00:00"
    ));
    assert!(!is_reproducible_timestamp(
        "2026-01-01T00:00:00.123Z"
    ));
    assert!(!is_reproducible_timestamp("generated now"));
}
