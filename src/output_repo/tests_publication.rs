use super::*;

#[test]
fn test_publish_output_candidate_rejects_verified_tree_path_in_committed_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");
    let candidate = tmp.path().join("candidate");
    let verified_tree = tmp.path().join("verified-tree");

    std::fs::create_dir_all(output.join(".git/kslim")).unwrap();
    std::fs::write(output.join("Makefile"), "old\n").unwrap();
    create_valid_output_candidate(&candidate);
    std::fs::write(
        candidate.join(format!(".kslim/{}", REPORT_FILE)),
        format!("verified tree path: {}\n", verified_tree.display()),
    )
    .unwrap();

    let err = publish_output_candidate(
        &output_repo_path(&output),
        &candidate_tree_path(&candidate),
        &candidate_tree_path(&verified_tree),
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("temporary path"));
    assert!(err.contains("non-authoritative attempt metadata"));
    assert_eq!(
        std::fs::read_to_string(output.join("Makefile")).unwrap(),
        "old\n"
    );
    assert!(!output.join(".kslim").exists());
}

#[test]
fn test_publish_output_candidate_syncs_payload_and_non_published_metadata_together() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");
    let candidate = tmp.path().join("candidate");

    std::fs::create_dir_all(output.join(".git/kslim")).unwrap();
    std::fs::write(output.join("Makefile"), "old\n").unwrap();
    std::fs::write(output.join(".git/kslim/stale.txt"), "stale\n").unwrap();

    for dir in &[
        "arch", "drivers", "fs", "include", "kernel", "mm", "net", "scripts", ".kslim",
    ] {
        std::fs::create_dir_all(candidate.join(dir)).unwrap();
    }
    std::fs::write(candidate.join("Makefile"), "new\n").unwrap();
    std::fs::write(candidate.join("Kconfig"), "# test\n").unwrap();
    std::fs::write(
        candidate.join(".kslim/managed.toml"),
        "managed_by = \"kslim\"\n",
    )
    .unwrap();
    std::fs::write(
        candidate.join(format!(".kslim/{}", BASE_METADATA_FILE)),
        "base_ref = \"v1.0\"\n",
    )
    .unwrap();
    std::fs::write(
        candidate.join(format!(".kslim/{}", GENERATED_METADATA_FILE)),
        "generated_at = \"2026-01-01T00:00:00Z\"\n",
    )
    .unwrap();
    std::fs::write(candidate.join(".kslim/manifest.txt"), "hash  1  Makefile\n").unwrap();
    std::fs::write(
        candidate.join(format!(".kslim/{}", REPORT_FILE)),
        "report\n",
    )
    .unwrap();

    publish_output_candidate(
        &output_repo_path(&output),
        &candidate_tree_path(&candidate),
        &candidate_tree_path(&candidate),
    )
    .unwrap();

    assert_eq!(
        std::fs::read_to_string(output.join("Makefile")).unwrap(),
        "new\n"
    );
    assert!(output.join(".git/kslim/managed.toml").exists());
    assert!(output.join(".kslim/managed.toml").exists());
    assert!(!output
        .join(format!(".git/kslim/{}", PUBLISHED_SNAPSHOT_FILE))
        .exists());
    assert!(!output
        .join(format!(".kslim/{}", PUBLISHED_SNAPSHOT_FILE))
        .exists());
    assert!(!output.join(".git/kslim/stale.txt").exists());
}

#[test]
fn test_publish_output_candidate_fails_closed_before_output_sync_when_candidate_invalid() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");
    let candidate = tmp.path().join("candidate");

    std::fs::create_dir_all(output.join(".git/kslim")).unwrap();
    std::fs::write(output.join("Makefile"), "old\n").unwrap();
    std::fs::write(output.join(".git/kslim/managed.toml"), "old-managed\n").unwrap();

    for dir in &[
        "arch", "drivers", "fs", "include", "kernel", "mm", "net", "scripts", ".kslim",
    ] {
        std::fs::create_dir_all(candidate.join(dir)).unwrap();
    }
    std::fs::write(candidate.join("Makefile"), "new\n").unwrap();
    std::fs::write(candidate.join("Kconfig"), "# test\n").unwrap();
    std::fs::write(
        candidate.join(".kslim/managed.toml"),
        "managed_by = \"kslim\"\n",
    )
    .unwrap();

    let err = publish_output_candidate(
        &output_repo_path(&output),
        &candidate_tree_path(&candidate),
        &candidate_tree_path(&candidate),
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("required candidate metadata missing"));
    assert_eq!(
        std::fs::read_to_string(output.join("Makefile")).unwrap(),
        "old\n"
    );
    assert_eq!(
        std::fs::read_to_string(output.join(".git/kslim/managed.toml")).unwrap(),
        "old-managed\n"
    );
}

#[test]
fn test_authoritative_published_state_rejects_candidate_metadata_only() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    let output = tmp.path().join("output");
    let output_str = output.to_str().unwrap();
    let branch = "kslim/v1.0/default";
    let tag = "kslim-v1.0-default-r1";
    let generated_at = "2026-01-01T00:00:00Z";

    std::fs::create_dir_all(&project).unwrap();
    crate::git::init_repo(output_str).unwrap();
    crate::process::run_in_dir(
        output_str,
        "git",
        &["config", "user.email", "test@kslim.local"],
    )
    .unwrap();
    crate::process::run_in_dir(output_str, "git", &["config", "user.name", "kslim test"])
        .unwrap();
    std::fs::write(output.join("Makefile"), "# test\n").unwrap();
    crate::git::add_all(output_str).unwrap();
    crate::git::commit(output_str, "initial").unwrap();
    crate::git::create_branch(output_str, branch).unwrap();
    let output_commit = crate::git::head_commit(output_str).unwrap();

    let candidate_metadata = output.join(".git/kslim");
    std::fs::create_dir_all(&candidate_metadata).unwrap();
    std::fs::write(
        candidate_metadata.join(BASE_METADATA_FILE),
        concat!(
            "upstream_name = \"linux\"\n",
            "upstream_url = \"/tmp/linux.git\"\n",
            "base_ref = \"v1.0\"\n",
            "base_commit = \"deadbeef\"\n",
            "profile = \"default\"\n",
            "mode = \"unmodified-upstream\"\n",
            "kslim_version = \"test\"\n",
        ),
    )
    .unwrap();
    std::fs::write(
        candidate_metadata.join(GENERATED_METADATA_FILE),
        format!(
            "generated_by = \"kslim\"\ngenerated_at = \"{}\"\nkslim_version = \"test\"\n",
            generated_at
        ),
    )
    .unwrap();
    std::fs::write(
        candidate_metadata.join("manifest.txt"),
        "hash  1  Makefile\n",
    )
    .unwrap();
    std::fs::write(candidate_metadata.join(REPORT_FILE), "report\n").unwrap();
    std::fs::write(
        candidate_metadata.join(PUBLISHED_SNAPSHOT_FILE),
        format!(
            concat!(
                "branch = \"{}\"\n",
                "tag = \"{}\"\n",
                "base_ref = \"v1.0\"\n",
                "base_commit = \"deadbeef\"\n",
                "profile = \"default\"\n",
                "mode = \"unmodified-upstream\"\n",
                "generated_at = \"{}\"\n",
                "base_metadata_file = \"{}\"\n",
                "generated_metadata_file = \"{}\"\n",
                "manifest_file = \"manifest.txt\"\n",
                "report_file = \"{}\"\n",
                "kslim_version = \"test\"\n",
            ),
            branch, tag, generated_at, BASE_METADATA_FILE, GENERATED_METADATA_FILE, REPORT_FILE
        ),
    )
    .unwrap();

    let lockfile_update = crate::lockfile::PublishedLockfileUpdate::new(
        crate::lockfile::ResolvedBase {
            upstream: "linux".to_string(),
            url: "/tmp/linux.git".to_string(),
            r#ref: "v1.0".to_string(),
            commit: "deadbeef".to_string(),
            resolved_at: generated_at.to_string(),
        },
        crate::lockfile::PublishedLockfile {
            output_branch: branch.to_string(),
            output_commit,
            tag: tag.to_string(),
            base_ref: "v1.0".to_string(),
            base_commit: "deadbeef".to_string(),
            profile: "default".to_string(),
            mode: "unmodified-upstream".to_string(),
            generated_at: generated_at.to_string(),
        },
    )
    .unwrap();
    let lockfile_path = LockfilePath::new_in_project_root(&project).unwrap();
    crate::lockfile::write_published_lockfile(&lockfile_path, &lockfile_update).unwrap();

    let output_repo = crate::paths::OutputRepoPath::new(output_str).unwrap();
    let err = load_authoritative_published_state(&lockfile_path, &output_repo)
        .unwrap_err()
        .to_string();

    assert!(err.contains("required committed published metadata missing"));
    assert!(err.contains(".kslim/base.toml"));
}

fn commit_minimal_published_metadata(output: &std::path::Path) -> String {
    let output_str = output.to_str().unwrap();
    crate::git::init_repo(output_str).unwrap();
    crate::process::run_in_dir(
        output_str,
        "git",
        &["config", "user.email", "test@kslim.local"],
    )
    .unwrap();
    crate::process::run_in_dir(output_str, "git", &["config", "user.name", "kslim test"])
        .unwrap();
    std::fs::write(output.join("Makefile"), "# test\n").unwrap();
    std::fs::create_dir_all(output.join(COMMITTED_METADATA_DIR)).unwrap();
    std::fs::write(
        output
            .join(COMMITTED_METADATA_DIR)
            .join(PUBLISHED_SNAPSHOT_FILE),
        "branch = \"kslim/v1.0/default\"\ntag = \"kslim-v1.0-default-r1\"\n",
    )
    .unwrap();
    crate::git::add_all(output_str).unwrap();
    crate::git::commit(output_str, "commit published metadata without lockfile").unwrap();
    crate::git::head_commit(output_str).unwrap()
}

#[test]
fn test_authoritative_published_state_rejects_committed_metadata_without_lockfile() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    let output = tmp.path().join("output");
    std::fs::create_dir_all(&project).unwrap();
    let commit = commit_minimal_published_metadata(&output);

    let output_repo = crate::paths::OutputRepoPath::new(&output).unwrap();
    let lockfile_path = LockfilePath::new_in_project_root(&project).unwrap();
    let err = load_authoritative_published_state(&lockfile_path, &output_repo)
        .unwrap_err()
        .to_string();

    assert!(err.contains("committed published metadata exists"));
    assert!(err.contains(&commit));
    assert!(err.contains("kslim.lock is missing"));
    assert!(err.contains("refusing to recover implicitly"));
}

#[test]
fn test_authoritative_published_state_rejects_committed_metadata_without_published_lock() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    let output = tmp.path().join("output");
    std::fs::create_dir_all(&project).unwrap();
    let commit = commit_minimal_published_metadata(&output);
    let resolved = crate::lockfile::ResolvedBase {
        upstream: "linux".to_string(),
        url: "/tmp/linux.git".to_string(),
        r#ref: "v1.0".to_string(),
        commit: "deadbeef".to_string(),
        resolved_at: "2026-01-01T00:00:00Z".to_string(),
    };
    let lockfile_update = crate::lockfile::ResolvedBaseLockfileUpdate::new(resolved);
    let lockfile_path = LockfilePath::new_in_project_root(&project).unwrap();
    crate::lockfile::write_resolved_base_lockfile(&lockfile_path, &lockfile_update).unwrap();

    let output_repo = crate::paths::OutputRepoPath::new(&output).unwrap();
    let err = load_authoritative_published_state(&lockfile_path, &output_repo)
        .unwrap_err()
        .to_string();

    assert!(err.contains("committed published metadata exists"));
    assert!(err.contains(&commit));
    assert!(err.contains("kslim.lock has no published snapshot"));
    assert!(err.contains("refusing to recover implicitly"));
}
