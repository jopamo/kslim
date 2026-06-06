mod common;
use common::*;

fn write_project_config(
    kslim_dir: &std::path::Path,
    project_name: &str,
    upstream: &str,
    output: &std::path::Path,
    publish_remote: Option<&str>,
) {
    let mut config = format!(
        r#"[project]
name = "{project_name}"

[upstream]
name = "linux"
url = "{upstream}"

[output]
path = "{output}"
branch_prefix = "kslim"
"#,
        output = output.display(),
    );
    if let Some(remote) = publish_remote {
        config.push_str(&format!(
            r#"
[publish]
remote = "{remote}"
"#
        ));
    }
    std::fs::write(kslim_dir.join("kslim.toml"), config).unwrap();
}

#[test]
fn test_no_network_rejects_network_upstream_endpoint() {
    let tmp = tempfile::tempdir().unwrap();
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        "https://example.invalid/linux.git",
    );

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["upstream", "sync", "--no-network"]);

    assert!(!ok, "network upstream should fail: stdout={stdout:?}");
    assert!(stderr.contains("--no-network"));
    assert!(stderr.contains("upstream.url"));
}

#[test]
fn test_no_network_allows_local_upstream_and_local_publish_remote() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "no-network-local", "1.0");
    let remote = tmp.path().join("publish-local.git");
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
    write_project_config(
        &kslim_dir,
        "test-linux",
        &upstream,
        &output_dir,
        Some(remote.to_str().unwrap()),
    );

    let (ok_generate, stdout_generate, stderr_generate) =
        kslim_in(&kslim_dir, &["generate", "--no-network"]);
    assert!(
        ok_generate,
        "generate should allow local upstream: stdout={stdout_generate:?} stderr={stderr_generate:?}"
    );

    let (ok_publish, stdout_publish, stderr_publish) =
        kslim_in(&kslim_dir, &["publish", "--no-network"]);
    assert!(
        ok_publish,
        "publish should allow local remote: stdout={stdout_publish:?} stderr={stderr_publish:?}"
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
}

#[test]
fn test_no_network_rejects_network_publish_remote_before_remote_config_mutation() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "no-network-publish", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok_generate, stdout_generate, stderr_generate) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok_generate,
        "generate failed: stdout={stdout_generate:?} stderr={stderr_generate:?}"
    );
    write_project_config(
        &kslim_dir,
        "test-linux",
        &upstream,
        &output_dir,
        Some("ssh://127.0.0.1:1/forbidden.git"),
    );

    let (ok_publish, stdout_publish, stderr_publish) =
        kslim_in(&kslim_dir, &["publish", "--no-network"]);

    assert!(
        !ok_publish,
        "network publish should fail: stdout={stdout_publish:?}"
    );
    assert!(stderr_publish.contains("--no-network"));
    assert!(stderr_publish.contains("publish.remote"));
    assert_eq!(git_in(output_dir.to_str().unwrap(), &["remote"]).trim(), "");
}
