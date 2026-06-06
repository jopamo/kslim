mod common;
use common::*;

#[test]
fn test_offline_plan_uses_lockfile_without_upstream_access() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "offline-plan", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );
    let locked_commit = git_in(&upstream, &["rev-parse", "v1.0^{commit}"]);

    let (ok_resolve, stdout_resolve, stderr_resolve) = kslim_in(&kslim_dir, &["base", "resolve"]);
    assert!(
        ok_resolve,
        "base resolve failed: stdout={stdout_resolve:?} stderr={stderr_resolve:?}"
    );
    std::fs::remove_dir_all(&upstream).unwrap();

    let (ok_plan, stdout_plan, stderr_plan) = kslim_in(&kslim_dir, &["plan", "--offline"]);

    assert!(
        ok_plan,
        "offline plan should use kslim.lock: stdout={stdout_plan:?} stderr={stderr_plan:?}"
    );
    assert!(stdout_plan.contains(&format!("base: v1.0 -> {}", locked_commit.trim())));
    assert!(!output_dir.exists());
}

#[test]
fn test_offline_generate_uses_locked_base_when_ref_is_unavailable() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "offline-generate", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );
    let locked_commit = git_in(&upstream, &["rev-parse", "v1.0^{commit}"]);

    let (ok_resolve, stdout_resolve, stderr_resolve) = kslim_in(&kslim_dir, &["base", "resolve"]);
    assert!(
        ok_resolve,
        "base resolve failed: stdout={stdout_resolve:?} stderr={stderr_resolve:?}"
    );
    git_in(&upstream, &["tag", "-d", "v1.0"]);

    let (ok_generate, stdout_generate, stderr_generate) =
        kslim_in(&kslim_dir, &["generate", "--offline"]);

    assert!(
        ok_generate,
        "offline generate should use locked commit: stdout={stdout_generate:?} stderr={stderr_generate:?}"
    );
    let metadata =
        std::fs::read_to_string(committed_output_meta_path(&output_dir, "base.toml")).unwrap();
    assert!(metadata.contains(&format!("commit = \"{}\"", locked_commit.trim())));
}

#[test]
fn test_offline_plan_requires_matching_lockfile() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "offline-missing-lock", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok_plan, stdout_plan, stderr_plan) = kslim_in(&kslim_dir, &["plan", "--offline"]);

    assert!(!ok_plan, "offline plan should fail: stdout={stdout_plan:?}");
    assert!(stderr_plan.contains("--offline requires"));
    assert!(!kslim_dir.join("kslim.lock").exists());
}
