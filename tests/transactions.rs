mod common;
use common::*;

#[test]
fn test_generation_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok1, stdout1, _) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok1);
    assert!(
        stdout1.contains("Generated commit"),
        "first gen should commit, got: {}",
        stdout1
    );

    let (ok2, stdout2, _) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok2);
    assert!(
        stdout2.contains("No changes") || stdout2.contains("idempotent"),
        "second gen should be idempotent, got: {}",
        stdout2
    );
    assert!(
        stdout2.contains("stage: publish"),
        "idempotent generate stdout should expose final stage: {}",
        stdout2
    );

    let count = git_in(
        output_dir.to_str().unwrap(),
        &["rev-list", "--count", "HEAD"],
    );
    let n: usize = count.trim().parse().unwrap();
    // We expect: init commit + generated commit = 2. Second run is idempotent.
    assert_eq!(
        n,
        2,
        "should have exactly two commits (init + generated), got: {}",
        count.trim()
    );
}

#[test]
fn test_generate_rolls_back_output_repo_when_pre_commit_publish_step_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "rollback-output", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok1, _, stderr1) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok1, "initial generate failed: {}", stderr1);

    let head_before = git_in(output_dir.to_str().unwrap(), &["rev-parse", "HEAD"]);
    let branch_before = git_in(output_dir.to_str().unwrap(), &["branch", "--show-current"]);
    let status_before = git_in(output_dir.to_str().unwrap(), &["status", "--porcelain"]);
    let config_before = std::fs::read_to_string(output_dir.join(".git/config")).unwrap();
    let published_before =
        std::fs::read_to_string(output_meta_path(&output_dir, "published.toml")).unwrap();
    let lockfile_before = std::fs::read_to_string(kslim_dir.join("kslim.lock")).unwrap();

    std::fs::write(
        kslim_dir.join("kslim.toml"),
        format!(
            r#"[project]
name = "test-linux"

[upstream]
name = "linux"
url = "{upstream}"

[output]
path = "{output}"
branch_prefix = "kslim"
branch = "bad name"
"#,
            upstream = upstream,
            output = output_dir.display(),
        ),
    )
    .unwrap();

    let (ok2, _, stderr2) = kslim_in(&kslim_dir, &["generate"]);
    assert!(!ok2, "generate unexpectedly succeeded");
    assert!(
        stderr2.contains("not a valid branch name")
            || stderr2.contains("invalid branch name")
            || stderr2.contains("output publish failed"),
        "unexpected failure output: {}",
        stderr2
    );

    assert_eq!(
        git_in(output_dir.to_str().unwrap(), &["rev-parse", "HEAD"]),
        head_before,
        "output repo HEAD should be rolled back after pre-commit publish failure"
    );
    assert_eq!(
        git_in(output_dir.to_str().unwrap(), &["branch", "--show-current"]),
        branch_before,
        "output repo branch should be restored after pre-commit publish failure"
    );
    assert_eq!(
        git_in(output_dir.to_str().unwrap(), &["status", "--porcelain"]),
        status_before,
        "output repo should remain clean after rollback"
    );
    assert_eq!(
        std::fs::read_to_string(output_dir.join(".git/config")).unwrap(),
        config_before,
        "output repo git config should be restored after rollback"
    );
    assert_eq!(
        std::fs::read_to_string(output_meta_path(&output_dir, "published.toml")).unwrap(),
        published_before,
        "committed output metadata should remain unchanged after rollback"
    );
    assert_eq!(
        std::fs::read_to_string(kslim_dir.join("kslim.lock")).unwrap(),
        lockfile_before,
        "authoritative lockfile should remain unchanged after pre-commit publish failure"
    );
    assert!(project_failure_report_path(&kslim_dir).exists());
    assert!(project_failure_meta_path(&kslim_dir, "last-attempt.json").exists());
}

#[test]
fn test_generate_removes_fresh_output_repo_when_lockfile_write_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "fresh-lockfile-fail", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );
    std::fs::create_dir(kslim_dir.join("kslim.lock")).unwrap();

    let (ok, _, _) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        !ok,
        "generate should fail when kslim.lock is not writable as a file"
    );
    assert!(
        !output_dir.exists(),
        "failed generate must remove the fresh output repo created before lockfile failure"
    );
    assert!(
        kslim_dir.join("kslim.lock").is_dir(),
        "failed generate must not replace the preexisting kslim.lock path"
    );
    let last_attempt =
        std::fs::read_to_string(project_failure_meta_path(&kslim_dir, "last-attempt.json"))
            .unwrap();
    assert!(last_attempt.contains("\"stage\": \"publish\""));
    assert!(last_attempt.contains("\"updated\": false"));
}

#[test]
fn test_generate_rolls_back_existing_output_repo_when_lockfile_write_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream_v1 = create_fake_upstream(tmp.path(), "lock-rollback-v1", "1.0");
    let upstream_v2 = create_fake_upstream(tmp.path(), "lock-rollback-v2", "2.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream_v1,
    );

    let (ok1, _, stderr1) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok1, "initial generate failed: {}", stderr1);

    let head_before = git_in(output_dir.to_str().unwrap(), &["rev-parse", "HEAD"]);
    let branch_before = git_in(output_dir.to_str().unwrap(), &["branch", "--show-current"]);
    let status_before = git_in(output_dir.to_str().unwrap(), &["status", "--porcelain"]);
    let published_before =
        std::fs::read_to_string(output_meta_path(&output_dir, "published.toml")).unwrap();
    let lockfile_path = kslim_dir.join("kslim.lock");
    let lockfile_before = std::fs::read_to_string(&lockfile_path).unwrap();
    let mut lockfile_perms = std::fs::metadata(&lockfile_path).unwrap().permissions();
    lockfile_perms.set_readonly(true);
    std::fs::set_permissions(&lockfile_path, lockfile_perms).unwrap();

    std::fs::write(
        kslim_dir.join("kslim.toml"),
        format!(
            r#"[project]
name = "test-linux"

[upstream]
name = "linux"
url = "{upstream}"

[output]
path = "{output}"
branch_prefix = "kslim"
"#,
            upstream = upstream_v2,
            output = output_dir.display(),
        ),
    )
    .unwrap();
    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        "[profile]\nname = \"default\"\n\n[base]\nref = \"v2.0\"\n",
    )
    .unwrap();

    let (ok2, _, _) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        !ok2,
        "generate should fail when kslim.lock cannot be updated"
    );

    assert_eq!(
        git_in(output_dir.to_str().unwrap(), &["rev-parse", "HEAD"]),
        head_before,
        "failed lockfile update must roll back output repo HEAD"
    );
    assert_eq!(
        git_in(output_dir.to_str().unwrap(), &["branch", "--show-current"]),
        branch_before,
        "failed lockfile update must restore the published branch"
    );
    assert_eq!(
        git_in(output_dir.to_str().unwrap(), &["status", "--porcelain"]),
        status_before,
        "failed lockfile update must leave output repo clean"
    );
    assert_eq!(
        std::fs::read_to_string(output_meta_path(&output_dir, "published.toml")).unwrap(),
        published_before,
        "failed lockfile update must restore committed published metadata"
    );
    assert_eq!(
        std::fs::read_to_string(&lockfile_path).unwrap(),
        lockfile_before,
        "failed lockfile update must not alter the authoritative lockfile"
    );
    assert!(
        git_in(
            output_dir.to_str().unwrap(),
            &["branch", "--list", "kslim/v2.0/default"],
        )
        .trim()
        .is_empty(),
        "failed lockfile update must not leave the candidate branch pointer published"
    );
    let last_attempt =
        std::fs::read_to_string(project_failure_meta_path(&kslim_dir, "last-attempt.json"))
            .unwrap();
    assert!(last_attempt.contains("\"stage\": \"publish\""));
    assert!(last_attempt.contains("\"updated\": false"));
}

#[test]
fn test_different_base_creates_new_commit() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream_v1 = create_fake_upstream(tmp.path(), "testv1", "1.0");
    let upstream_v2 = create_fake_upstream(tmp.path(), "testv2", "2.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream_v1,
    );

    // Generate v1.0
    let (ok1, stdout1, _) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok1, "first generate failed: {}", stdout1);

    // Switch config to v2.0
    let config2 = format!(
        r#"[project]
name = "test-linux"

[upstream]
name = "linux"
url = "{u}"

[output]
path = "{o}"
branch_prefix = "kslim"
"#,
        u = upstream_v2,
        o = output_dir.to_str().unwrap(),
    );
    std::fs::write(kslim_dir.join("kslim.toml"), config2).unwrap();
    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        "[profile]\nname = \"default\"\n\n[base]\nref = \"v2.0\"\n",
    )
    .unwrap();

    let (ok2, stdout2, _) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok2, "second generate failed: {}", stdout2);

    let branches = git_in(output_dir.to_str().unwrap(), &["branch"]);
    assert!(
        branches.contains("v2.0"),
        "should have v2.0 branch in: {}",
        branches
    );
}

#[test]
fn test_refuses_dirty_output_repo() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok1, _, _) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok1);

    std::fs::write(output_dir.join("Makefile"), "# modified\n").unwrap();

    let (ok2, _, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(!ok2, "should refuse dirty repo");
    assert!(
        stderr.contains("uncommitted changes") || stderr.contains("dirty"),
        "expected dirty error, got: {}",
        stderr
    );
}

#[test]
fn test_force_allows_dirty_output_repo() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok1, _, _) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok1);

    std::fs::write(output_dir.join("Makefile"), "# modified\n").unwrap();

    let (ok2, stdout, _) = kslim_in(&kslim_dir, &["generate", "--force"]);
    assert!(ok2, "force should allow dirty repo, stdout: {}", stdout);
}

#[test]
fn test_dry_run_does_not_mutate_output() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok, stdout, _) = kslim_in(&kslim_dir, &["generate", "--dry-run"]);
    assert!(ok, "dry-run failed: {}", stdout);
    assert!(
        stdout.contains("would"),
        "dry-run output should say 'would': {}",
        stdout
    );
    assert!(!output_dir.exists(), "dry-run should not create output dir");
    assert!(
        !kslim_dir.join("kslim.lock").exists(),
        "dry-run should not update the authoritative lockfile"
    );
    assert!(
        !project_failure_report_path(&kslim_dir).exists(),
        "dry-run should not write non-authoritative attempt metadata"
    );
    assert!(
        !project_failure_meta_path(&kslim_dir, "last-attempt.json").exists(),
        "dry-run should not write last-attempt metadata"
    );
}

#[test]
fn test_deep_dry_run_verifies_candidate_without_mutating_output() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test-deep-dry-run", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate", "--deep-dry-run"]);
    assert!(
        ok,
        "deep dry-run failed: stdout={} stderr={}",
        stdout, stderr
    );
    assert!(
        stdout.contains("deep dry-run") && stdout.contains("would publish path"),
        "deep dry-run output should describe verified candidate without publication: {}",
        stdout
    );
    assert!(
        !output_dir.exists(),
        "deep dry-run should not create output dir"
    );
    assert!(
        !kslim_dir.join("kslim.lock").exists(),
        "deep dry-run should not update the authoritative lockfile"
    );
    assert!(
        !project_failure_report_path(&kslim_dir).exists(),
        "deep dry-run should not write non-authoritative attempt metadata"
    );
    assert!(
        !project_failure_meta_path(&kslim_dir, "last-attempt.json").exists(),
        "deep dry-run should not write last-attempt metadata"
    );
}

#[test]
fn test_keep_temp_preserves_deep_dry_run_candidate_without_publishing() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test-keep-temp", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate", "--deep-dry-run", "--keep-temp"]);
    assert!(
        ok,
        "keep-temp generate failed: stdout={stdout} stderr={stderr}"
    );
    let kept = stdout
        .lines()
        .find_map(|line| {
            line.split_once("kept temp:")
                .map(|(_, path)| std::path::PathBuf::from(path.trim()))
        })
        .unwrap_or_else(|| panic!("keep-temp output should print retained temp path: {stdout}"));

    assert!(kept.exists(), "kept temp candidate should remain on disk");
    assert!(
        kept.join("Makefile").exists(),
        "kept temp candidate should contain materialized kernel tree"
    );
    assert!(
        !output_dir.exists(),
        "keep-temp deep dry-run should not publish output"
    );
    assert!(
        !kslim_dir.join("kslim.lock").exists(),
        "keep-temp deep dry-run should not update the authoritative lockfile"
    );
    std::fs::remove_dir_all(&kept).unwrap();
}
