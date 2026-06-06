mod common;
use common::*;

#[test]
fn test_status_shows_authoritative_published_snapshot_state() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "status-published", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok_generate, _, stderr_generate) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok_generate, "generate failed: {}", stderr_generate);

    let (ok_status, stdout_status, _) = kslim_in(&kslim_dir, &["status"]);
    assert!(ok_status);
    assert!(stdout_status.contains("published snapshot:"));
    assert!(stdout_status.contains("branch: kslim/v1.0/default"));
    assert!(stdout_status.contains("tag: kslim-v1.0-default-r1"));
    assert!(stdout_status.contains("published output commit:"));
}

#[test]
fn test_publish_uses_committed_metadata_without_upstream_or_profile() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "publish-truth", "1.0");
    let remote = tmp.path().join("publish-remote.git");
    git_in(
        tmp.path().to_str().unwrap(),
        &["init", "--bare", remote.to_str().unwrap()],
    );
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

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

[publish]
remote = "{remote}"
"#,
            upstream = upstream,
            output = output_dir.display(),
            remote = remote.display(),
        ),
    )
    .unwrap();

    let (ok_generate, _, stderr_generate) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok_generate, "generate failed: {}", stderr_generate);
    std::fs::remove_dir_all(output_dir.join(".git/kslim")).unwrap();

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

[publish]
remote = "{remote}"
"#,
            upstream = tmp.path().join("missing-upstream.git").display(),
            output = output_dir.display(),
            remote = remote.display(),
        ),
    )
    .unwrap();
    std::fs::remove_file(kslim_dir.join("profiles/default.toml")).unwrap();

    let (ok_publish, stdout_publish, stderr_publish) = kslim_in(&kslim_dir, &["publish"]);
    assert!(
        ok_publish,
        "publish should use committed metadata only: stdout={:?} stderr={:?}",
        stdout_publish, stderr_publish
    );
    assert!(stdout_publish.contains("Published successfully"));
    assert_eq!(
        git_in(
            remote.to_str().unwrap(),
            &["rev-parse", "refs/heads/kslim/v1.0/default"],
        )
        .trim()
        .len(),
        40
    );
    assert_eq!(
        git_in(
            remote.to_str().unwrap(),
            &["rev-parse", "refs/tags/kslim-v1.0-default-r1^{tag}"],
        )
        .trim()
        .len(),
        40
    );
}

#[test]
fn test_publish_force_uses_committed_metadata_when_worktree_metadata_is_dirty() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "publish-dirty-worktree-metadata", "1.0");
    let remote = tmp
        .path()
        .join("publish-remote-dirty-worktree-metadata.git");
    git_in(
        tmp.path().to_str().unwrap(),
        &["init", "--bare", remote.to_str().unwrap()],
    );
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

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

[publish]
remote = "{remote}"
"#,
            upstream = upstream,
            output = output_dir.display(),
            remote = remote.display(),
        ),
    )
    .unwrap();

    let (ok_generate, _, stderr_generate) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok_generate, "generate failed: {}", stderr_generate);

    let committed_published = committed_output_meta_path(&output_dir, "published.toml");
    let committed_contents = std::fs::read_to_string(&committed_published).unwrap();
    std::fs::write(
        &committed_published,
        committed_contents.replace(
            "tag = \"kslim-v1.0-default-r1\"",
            "tag = \"kslim-v1.0-dirty-worktree-r1\"",
        ),
    )
    .unwrap();
    let private_published = output_meta_path(&output_dir, "published.toml");
    let private_contents = std::fs::read_to_string(&private_published).unwrap();
    std::fs::write(
        &private_published,
        private_contents.replace(
            "tag = \"kslim-v1.0-default-r1\"",
            "tag = \"kslim-v1.0-private-metadata-r1\"",
        ),
    )
    .unwrap();
    std::fs::write(
        output_meta_path(&output_dir, "candidate.toml"),
        "this is intentionally invalid candidate metadata = [\n",
    )
    .unwrap();

    assert!(
        git_in(output_dir.to_str().unwrap(), &["status", "--porcelain"])
            .contains(".kslim/published.toml"),
        "test setup should dirty committed metadata in the output worktree"
    );

    let (ok_publish, stdout_publish, stderr_publish) =
        kslim_in(&kslim_dir, &["publish", "--dry-run", "--force"]);
    assert!(
        ok_publish,
        "publish should consume committed output metadata only: stdout={:?} stderr={:?}",
        stdout_publish, stderr_publish
    );
    assert!(stdout_publish.contains("[dry-run] would push tag:     kslim-v1.0-default-r1"));
    assert!(stdout_publish.contains("[dry-run] output commit:"));
    assert!(!stdout_publish.contains("kslim-v1.0-dirty-worktree-r1"));
    assert!(!stdout_publish.contains("kslim-v1.0-private-metadata-r1"));
}

#[test]
fn test_publish_does_not_re_resolve_mutable_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "publish-no-reresolve", "1.0");
    let remote = "https://127.0.0.1:1/kslim-no-reresolve.git";
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

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

[publish]
remote = "{remote}"
"#,
            upstream = upstream,
            output = output_dir.display(),
            remote = remote,
        ),
    )
    .unwrap();

    let (ok_generate, _, stderr_generate) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok_generate, "generate failed: {}", stderr_generate);

    let local_candidate_published = output_meta_path(&output_dir, "published.toml");
    let local_candidate_contents = std::fs::read_to_string(&local_candidate_published).unwrap();
    std::fs::write(
        &local_candidate_published,
        local_candidate_contents.replace(
            "tag = \"kslim-v1.0-default-r1\"",
            "tag = \"kslim-v1.0-local-candidate-r1\"",
        ),
    )
    .unwrap();
    std::fs::remove_dir_all(std::path::Path::new(&upstream)).unwrap();
    std::fs::remove_file(kslim_dir.join("profiles/default.toml")).unwrap();

    std::fs::write(
        kslim_dir.join("kslim.toml"),
        format!(
            r#"[project]
name = "test-linux"

[upstream]
name = ""
url = ""
mode = "network"
cache = "obsolete"

[output]
path = "{output}"
branch_prefix = ""
branch = ""

[git]
user_email = ""
user_name = ""

[publish]
remote = "{remote}"
"#,
            output = output_dir.display(),
            remote = remote,
        ),
    )
    .unwrap();

    let (ok_publish, stdout_publish, stderr_publish) =
        kslim_in(&kslim_dir, &["publish", "--dry-run"]);
    assert!(
        ok_publish,
        "publish should not re-resolve mutable state: stdout={:?} stderr={:?}",
        stdout_publish, stderr_publish
    );
    assert!(stdout_publish.contains("[dry-run] would push tag:     kslim-v1.0-default-r1"));
    assert!(stdout_publish.contains(remote));
    assert!(!stdout_publish.contains("local-candidate"));
}

#[test]
fn test_publish_fails_when_published_snapshot_metadata_is_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "publish-missing-metadata", "1.0");
    let remote = tmp.path().join("publish-remote-missing.git");
    git_in(
        tmp.path().to_str().unwrap(),
        &["init", "--bare", remote.to_str().unwrap()],
    );
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

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

[publish]
remote = "{remote}"
"#,
            upstream = upstream,
            output = output_dir.display(),
            remote = remote.display(),
        ),
    )
    .unwrap();

    let (ok_generate, _, stderr_generate) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok_generate, "generate failed: {}", stderr_generate);

    let old_head = git_in(output_dir.to_str().unwrap(), &["rev-parse", "HEAD"]);
    std::fs::remove_file(committed_output_meta_path(&output_dir, "published.toml")).unwrap();
    git_in(output_dir.to_str().unwrap(), &["add", "-A"]);
    git_in(
        output_dir.to_str().unwrap(),
        &["commit", "-m", "remove committed published metadata"],
    );
    let bad_head = git_in(output_dir.to_str().unwrap(), &["rev-parse", "HEAD"]);
    let lockfile_path = kslim_dir.join("kslim.lock");
    let lockfile = std::fs::read_to_string(&lockfile_path).unwrap();
    std::fs::write(
        &lockfile_path,
        lockfile.replace(
            &format!("output_commit = \"{}\"", old_head),
            &format!("output_commit = \"{}\"", bad_head),
        ),
    )
    .unwrap();

    let (ok_publish, _, stderr_publish) = kslim_in(&kslim_dir, &["publish", "--dry-run"]);
    assert!(
        !ok_publish,
        "publish should fail when published metadata is missing"
    );
    assert!(stderr_publish.contains("required committed published metadata missing"));
    assert!(stderr_publish.contains("published.toml"));
}

#[test]
fn test_publish_fails_safe_when_committed_metadata_exists_without_lockfile() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "publish-metadata-no-lockfile", "1.0");
    let remote = tmp.path().join("publish-remote-no-lockfile.git");
    git_in(
        tmp.path().to_str().unwrap(),
        &["init", "--bare", remote.to_str().unwrap()],
    );
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

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

[publish]
remote = "{remote}"
"#,
            upstream = upstream,
            output = output_dir.display(),
            remote = remote.display(),
        ),
    )
    .unwrap();

    let (ok_generate, _, stderr_generate) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok_generate, "generate failed: {}", stderr_generate);
    assert!(committed_output_meta_path(&output_dir, "published.toml").exists());
    std::fs::remove_file(kslim_dir.join("kslim.lock")).unwrap();

    let (ok_publish, stdout_publish, stderr_publish) =
        kslim_in(&kslim_dir, &["publish", "--dry-run"]);

    assert!(
        !ok_publish,
        "publish must fail safe when output metadata exists without lockfile"
    );
    assert!(!stdout_publish.contains("would push"));
    assert!(stderr_publish.contains("committed published metadata exists"));
    assert!(stderr_publish.contains("kslim.lock is missing"));
    assert!(stderr_publish.contains("refusing to recover implicitly"));
}

#[test]
fn test_publish_fails_when_published_snapshot_metadata_is_inconsistent() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "publish-inconsistent-metadata", "1.0");
    let remote = tmp.path().join("publish-remote-inconsistent.git");
    git_in(
        tmp.path().to_str().unwrap(),
        &["init", "--bare", remote.to_str().unwrap()],
    );
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

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

[publish]
remote = "{remote}"
"#,
            upstream = upstream,
            output = output_dir.display(),
            remote = remote.display(),
        ),
    )
    .unwrap();

    let (ok_generate, _, stderr_generate) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok_generate, "generate failed: {}", stderr_generate);

    let old_head = git_in(output_dir.to_str().unwrap(), &["rev-parse", "HEAD"]);
    let published_path = committed_output_meta_path(&output_dir, "published.toml");
    let published = std::fs::read_to_string(&published_path).unwrap();
    std::fs::write(
        &published_path,
        published.replace(
            "branch = \"kslim/v1.0/default\"",
            "branch = \"kslim/v1.0/other\"",
        ),
    )
    .unwrap();
    git_in(output_dir.to_str().unwrap(), &["add", "-A"]);
    git_in(
        output_dir.to_str().unwrap(),
        &["commit", "-m", "corrupt committed published metadata"],
    );
    let bad_head = git_in(output_dir.to_str().unwrap(), &["rev-parse", "HEAD"]);
    let lockfile_path = kslim_dir.join("kslim.lock");
    let lockfile = std::fs::read_to_string(&lockfile_path).unwrap();
    std::fs::write(
        &lockfile_path,
        lockfile.replace(
            &format!("output_commit = \"{}\"", old_head),
            &format!("output_commit = \"{}\"", bad_head),
        ),
    )
    .unwrap();

    let (ok_publish, _, stderr_publish) = kslim_in(&kslim_dir, &["publish", "--dry-run"]);
    assert!(
        !ok_publish,
        "publish should fail when published metadata is inconsistent"
    );
    assert!(stderr_publish.contains("authoritative published state is inconsistent"));
}

#[test]
fn test_publish_fails_when_lockfile_published_state_is_inconsistent() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "publish-lockfile-inconsistent", "1.0");
    let remote = tmp.path().join("publish-remote-lockfile-inconsistent.git");
    git_in(
        tmp.path().to_str().unwrap(),
        &["init", "--bare", remote.to_str().unwrap()],
    );
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

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

[publish]
remote = "{remote}"
"#,
            upstream = upstream,
            output = output_dir.display(),
            remote = remote.display(),
        ),
    )
    .unwrap();

    let (ok_generate, _, stderr_generate) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok_generate, "generate failed: {}", stderr_generate);

    let lockfile_path = kslim_dir.join("kslim.lock");
    let lockfile = std::fs::read_to_string(&lockfile_path).unwrap();
    std::fs::write(
        &lockfile_path,
        lockfile.replace(
            "output_branch = \"kslim/v1.0/default\"",
            "output_branch = \"kslim/v1.0/other\"",
        ),
    )
    .unwrap();

    let (ok_publish, _, stderr_publish) = kslim_in(&kslim_dir, &["publish", "--dry-run"]);
    assert!(
        !ok_publish,
        "publish should fail when authoritative lockfile published state is inconsistent"
    );
    assert!(stderr_publish.contains("authoritative published state is inconsistent"));
    assert!(stderr_publish.contains("lockfile branch"));
}
