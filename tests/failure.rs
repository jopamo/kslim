mod common;
use common::*;

#[test]
fn test_generate_rejects_dirty_patch_worktree_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "patched-dirty", "1.0");
    let patch_worktree = create_patch_worktree(
        tmp.path(),
        &upstream,
        "topic/dirty",
        "Makefile",
        "1.0 + committed change",
    );
    std::fs::write(patch_worktree.join("Kconfig"), "# dirty\n").unwrap();

    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        format!(
            r#"[profile]
name = "default"
description = "Reject dirty patch worktree"

[base]
ref = "v1.0"

[patches]
source = "worktree"
path = "{}"
base_remote = "origin"
base_ref = "master"
require_clean = true
"#,
            patch_worktree.display()
        ),
    )
    .unwrap();

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(!ok, "generate should fail for dirty patch worktree");
    assert!(
        stderr.contains("patch worktree") && stderr.contains("uncommitted changes"),
        "expected dirty patch worktree failure, got: {}",
        stderr
    );
}

#[test]
fn test_generate_fails_when_selftest_command_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "selftest-fail", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Broken selftest"

[base]
ref = "v1.0"

[selftests]
commands = ["test -f DOES_NOT_EXIST"]
"#,
    )
    .unwrap();

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(!ok, "generate should fail when selftests fail");
    assert!(
        stderr.contains("selftest command failed"),
        "expected selftest failure, got: {}",
        stderr
    );

    let report_path = project_failure_report_path(&kslim_dir);
    assert!(
        report_path.exists(),
        "failure report should be written at {}",
        report_path.display()
    );
    let report = std::fs::read_to_string(report_path).unwrap();
    assert!(report.contains("Status: failure"));
    assert!(report.contains("Stage: selftest"));
    assert!(report.contains("selftest command failed"));

    let (ok_status, stdout_status, stderr_status) = kslim_in(&kslim_dir, &["status"]);
    assert!(
        ok_status,
        "status should inspect failed attempt: {}",
        stderr_status
    );
    assert!(stdout_status.contains("last attempt:"));
    assert!(stdout_status.contains("metadata scope: non-authoritative-attempt"));
    assert!(stdout_status.contains("stage: selftest"));
    assert!(stdout_status.contains("error kind: selftest"));
    assert!(stdout_status.contains("failure report: .kslim/attempt/report.txt"));
}

#[test]
fn test_repair_clears_only_non_authoritative_attempt_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "repair-attempt", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok_generate, _, stderr_generate) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok_generate, "generate failed: {}", stderr_generate);
    let lockfile_before = std::fs::read_to_string(kslim_dir.join("kslim.lock")).unwrap();
    let output_head_before = git_in(output_dir.to_str().unwrap(), &["rev-parse", "HEAD"]);
    assert!(output_meta_path(&output_dir, "report.txt").exists());
    std::fs::remove_dir_all(&upstream).unwrap();

    let attempt_dir = kslim_dir.join(".kslim/attempt");
    std::fs::create_dir_all(&attempt_dir).unwrap();
    std::fs::write(attempt_dir.join("report.txt"), "stale attempt report\n").unwrap();
    std::fs::write(
        attempt_dir.join("last-attempt.json"),
        r#"{"authoritative":true,"metadata_scope":"published","output_commit":"do-not-trust-attempt"}"#,
    )
    .unwrap();

    let (ok_repair, stdout_repair, stderr_repair) = kslim_in(&kslim_dir, &["repair"]);
    assert!(
        ok_repair,
        "repair failed: stdout={:?} stderr={:?}",
        stdout_repair, stderr_repair
    );
    assert!(stdout_repair.contains("repair: cleared non-authoritative attempt metadata"));
    assert!(stdout_repair.contains("removed: .kslim/attempt"));
    assert!(stdout_repair.contains("authoritative state: unchanged"));
    assert!(!attempt_dir.exists());
    assert_eq!(
        std::fs::read_to_string(kslim_dir.join("kslim.lock")).unwrap(),
        lockfile_before,
        "repair must not change authoritative lockfile"
    );
    assert_eq!(
        git_in(output_dir.to_str().unwrap(), &["rev-parse", "HEAD"]),
        output_head_before,
        "repair must not change output commit"
    );
    assert!(output_meta_path(&output_dir, "report.txt").exists());

    let (ok_again, stdout_again, stderr_again) = kslim_in(&kslim_dir, &["repair"]);
    assert!(
        ok_again,
        "idempotent repair failed: stdout={:?} stderr={:?}",
        stdout_again, stderr_again
    );
    assert!(stdout_again.contains("repair: nothing to repair"));
}

#[test]
fn test_failed_generate_does_not_poison_fresh_output_path_and_success_clears_failure_report() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "retry-after-failure", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Broken selftest"

[base]
ref = "v1.0"

[selftests]
commands = ["false"]
"#,
    )
    .unwrap();

    let (ok1, _, stderr1) = kslim_in(&kslim_dir, &["generate"]);
    assert!(!ok1, "first generate should fail");
    assert!(
        stderr1.contains("selftest command failed"),
        "unexpected first failure: {}",
        stderr1
    );
    assert!(
        !output_dir.exists(),
        "failed generate must not publish or poison output path"
    );

    let failure_report = project_failure_report_path(&kslim_dir);
    assert!(failure_report.exists());
    let report = std::fs::read_to_string(&failure_report).unwrap();
    assert!(report.contains("Status: failure"));
    assert!(report.contains("Stage: selftest"));
    let last_attempt = project_failure_meta_path(&kslim_dir, "last-attempt.json");
    assert!(
        last_attempt.exists(),
        "failure should record last-attempt metadata"
    );
    let last_attempt_json = std::fs::read_to_string(&last_attempt).unwrap();
    assert!(last_attempt_json.contains("\"authoritative\": false"));
    assert!(last_attempt_json.contains("\"metadata_scope\": \"non-authoritative-attempt\""));
    assert!(last_attempt_json.contains("\"metadata_dir\": \".kslim/attempt\""));
    assert!(last_attempt_json.contains("\"stage\": \"selftest\""));
    assert!(last_attempt_json.contains("\"updated\": false"));
    let generate_failure = project_failure_meta_path(&kslim_dir, "generate-failure.toml");
    assert!(
        generate_failure.exists(),
        "failed generate should record typed failure metadata under attempt metadata"
    );
    let generate_failure_toml = std::fs::read_to_string(&generate_failure).unwrap();
    assert!(generate_failure_toml.contains("metadata_scope = \"non-authoritative-attempt\""));
    assert!(generate_failure_toml.contains("authoritative = false"));
    assert!(generate_failure_toml.contains("stage = \"selftest\""));
    let project_meta_entries = std::fs::read_dir(kslim_dir.join(".kslim"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        project_meta_entries,
        vec![String::from("attempt")],
        "failed generate may write only the non-authoritative attempt namespace under project .kslim"
    );
    assert!(
        !kslim_dir.join(".kslim/report.txt").exists(),
        "failed generate must not write failure report outside the attempt namespace"
    );
    assert!(
        !kslim_dir.join(".kslim/last-attempt.json").exists(),
        "failed generate must not write last-attempt outside the attempt namespace"
    );
    assert!(
        !kslim_dir.join(".kslim/generate-failure.toml").exists(),
        "failed generate must not write typed failure metadata outside the attempt namespace"
    );

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Fixed selftest"

[base]
ref = "v1.0"
"#,
    )
    .unwrap();

    let (ok2, stdout2, stderr2) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok2,
        "second generate should succeed after fixing profile: stdout={:?} stderr={:?}",
        stdout2, stderr2
    );
    assert!(output_dir.join(".git").exists());
    assert!(
        !failure_report.exists(),
        "success should clear stale failure report"
    );
    assert!(
        !project_failure_meta_path(&kslim_dir, "reducer-report.json").exists(),
        "success should clear stale structured failure report"
    );
    assert!(
        !project_failure_meta_path(&kslim_dir, "reducer-failure.json").exists(),
        "success should clear stale reducer failure report"
    );
    assert!(
        !project_failure_meta_path(&kslim_dir, "diagnostics.json").exists(),
        "success should clear stale reducer diagnostics"
    );
    assert!(
        !project_failure_meta_path(&kslim_dir, "last-attempt.json").exists(),
        "success should clear stale last-attempt metadata"
    );
}

#[test]
fn test_failed_generate_keeps_existing_authoritative_lockfile_and_records_candidate_attempt() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream_v1 = create_fake_upstream(tmp.path(), "lock-v1", "1.0");
    let upstream_v2 = create_fake_upstream(tmp.path(), "lock-v2", "2.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream_v1,
    );

    let (ok1, _, stderr1) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok1, "initial generate failed: {}", stderr1);

    let lockfile_path = kslim_dir.join("kslim.lock");
    let authoritative_before = std::fs::read_to_string(&lockfile_path).unwrap();
    let v2_commit = git_in(&upstream_v2, &["rev-parse", "HEAD"]);

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
        r#"[profile]
name = "default"
description = "Fail after resolve"

[base]
ref = "HEAD"

[selftests]
commands = ["false"]
"#,
    )
    .unwrap();

    let (ok2, _, stderr2) = kslim_in(&kslim_dir, &["generate"]);
    assert!(!ok2, "failing generate unexpectedly succeeded");
    assert!(
        stderr2.contains("selftest command failed"),
        "unexpected failing generate stderr: {}",
        stderr2
    );

    let authoritative_after = std::fs::read_to_string(&lockfile_path).unwrap();
    assert_eq!(
        authoritative_after, authoritative_before,
        "failed generate must not update authoritative lockfile"
    );
    assert!(
        !authoritative_after.contains(&v2_commit),
        "authoritative lockfile must not record failed candidate commit"
    );

    let last_attempt =
        std::fs::read_to_string(project_failure_meta_path(&kslim_dir, "last-attempt.json"))
            .unwrap();
    assert!(last_attempt.contains("\"authoritative\": false"));
    assert!(last_attempt.contains("\"stage\": \"selftest\""));
    assert!(last_attempt.contains(&upstream_v2));
    assert!(last_attempt.contains("\"ref\": \"HEAD\""));
    assert!(last_attempt.contains("\"updated\": false"));
}

#[test]
fn test_non_local_upstream_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok1, _, _) = kslim_in(&kslim_dir, &["upstream", "sync"]);
    assert!(ok1);

    let config = format!(
        r#"[project]
name = "test-linux"

[upstream]
name = "linux"
url = "https://different.example.com/linux.git"

[output]
path = "{o}"
branch_prefix = "kslim"
"#,
        o = output_dir.to_str().unwrap(),
    );
    std::fs::write(kslim_dir.join("kslim.toml"), config).unwrap();

    let (ok2, _, stderr) = kslim_in(&kslim_dir, &["upstream", "sync"]);
    assert!(!ok2, "should fail on non-local upstream");
    assert!(
        stderr.contains("existing local git tree")
            || stderr.contains("not a readable local git repository"),
        "expected direct upstream validation error, got: {}",
        stderr
    );
}

#[test]
fn test_cache_config_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = tmp.path().join("kslim");
    std::fs::create_dir_all(&kslim_dir).unwrap();
    std::fs::create_dir_all(kslim_dir.join("profiles")).unwrap();
    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        "[profile]\nname = \"default\"\n\n[base]\nref = \"v1.0\"\n",
    )
    .unwrap();
    std::fs::write(
        kslim_dir.join("kslim.toml"),
        format!(
            r#"[project]
name = "test-linux"

[upstream]
name = "linux"
url = "{upstream}"
cache = "/tmp/forbidden.git"

[output]
path = "{output}"
branch_prefix = "kslim"
"#,
            upstream = upstream,
            output = output_dir.to_string_lossy(),
        ),
    )
    .unwrap();

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["status"]);
    assert!(!ok, "cache config should be rejected");
    assert!(
        stderr.contains("upstream.cache is no longer supported"),
        "unexpected error: {}",
        stderr
    );
}

#[test]
fn test_missing_upstream_ref_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test", "1.0");
    let kslim_dir = tmp.path().join("kslim");
    let output_dir = tmp.path().join("output");

    std::fs::create_dir_all(&kslim_dir).unwrap();
    let config = format!(
        r#"[project]
name = "test-linux"

[upstream]
name = "linux"
url = "{u}"

[output]
path = "{o}"
branch_prefix = "kslim"
"#,
        u = upstream,
        o = output_dir.to_str().unwrap(),
    );
    std::fs::write(kslim_dir.join("kslim.toml"), config).unwrap();
    std::fs::create_dir_all(kslim_dir.join("profiles")).unwrap();
    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        "[profile]\nname = \"default\"\n\n[base]\nref = \"v99.0\"\n",
    )
    .unwrap();

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(!ok, "should fail for missing ref");
    assert!(
        stderr.contains("failed to resolve") || stderr.contains("does the ref exist"),
        "expected missing ref error, got: {}",
        stderr
    );
}
