mod common;
use common::*;

#[test]
fn test_generate_frozen_plan_uses_plan_without_mutable_config_or_ref_resolution() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "frozen-generate", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );
    let locked_commit = git_in(&upstream, &["rev-parse", "v1.0^{commit}"]);
    let plan_path = kslim_dir.join("frozen-plan.toml");

    let (ok_plan, stdout_plan, stderr_plan) = kslim_in(
        &kslim_dir,
        &["plan", "--frozen-plan", plan_path.to_str().unwrap()],
    );
    assert!(
        ok_plan,
        "frozen plan write failed: stdout={stdout_plan:?} stderr={stderr_plan:?}"
    );
    assert!(plan_path.exists());

    std::fs::write(kslim_dir.join("kslim.toml"), "not valid toml = [\n").unwrap();
    std::fs::remove_file(kslim_dir.join("profiles/default.toml")).unwrap();
    git_in(&upstream, &["tag", "-d", "v1.0"]);

    let (ok_generate, stdout_generate, stderr_generate) = kslim_in(
        &kslim_dir,
        &["generate", "--frozen-plan", plan_path.to_str().unwrap()],
    );
    assert!(
        ok_generate,
        "frozen generate should use the frozen document only: stdout={stdout_generate:?} stderr={stderr_generate:?}"
    );
    assert!(stdout_generate.contains("frozen-plan: verified"));

    let metadata =
        std::fs::read_to_string(committed_output_meta_path(&output_dir, "base.toml")).unwrap();
    assert!(metadata.contains(&format!("commit = \"{}\"", locked_commit.trim())));
}

#[test]
fn test_reduce_tree_frozen_plan_rejects_base_commit_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "frozen-reduce", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );
    let plan_path = kslim_dir.join("frozen-plan.toml");
    let (ok_plan, stdout_plan, stderr_plan) = kslim_in(
        &kslim_dir,
        &["plan", "--frozen-plan", plan_path.to_str().unwrap()],
    );
    assert!(
        ok_plan,
        "frozen plan write failed: stdout={stdout_plan:?} stderr={stderr_plan:?}"
    );

    let tree = tmp.path().join("linux-tree");
    git_in(
        tmp.path().to_str().unwrap(),
        &["clone", &upstream, tree.to_str().unwrap()],
    );
    git_in(
        tree.to_str().unwrap(),
        &["config", "user.email", "test@kslim.local"],
    );
    git_in(
        tree.to_str().unwrap(),
        &["config", "user.name", "kslim test"],
    );
    std::fs::write(tree.join("Makefile"), "# changed after frozen plan\n").unwrap();
    git_in(tree.to_str().unwrap(), &["add", "-A"]);
    git_in(tree.to_str().unwrap(), &["commit", "-m", "new base"]);

    let (ok_reduce, stdout_reduce, stderr_reduce) = kslim_in(
        &kslim_dir,
        &[
            "reduce-tree",
            "--tree",
            tree.to_str().unwrap(),
            "--frozen-plan",
            plan_path.to_str().unwrap(),
        ],
    );
    assert!(
        !ok_reduce,
        "mismatched tree should fail: stdout={stdout_reduce:?}"
    );
    assert!(stderr_reduce.contains("--frozen-plan tree HEAD"));
    assert!(stderr_reduce.contains("plan base commit"));
}

#[test]
fn test_frozen_plan_rejects_schema_and_tool_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "frozen-header", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );
    let plan_path = kslim_dir.join("frozen-plan.toml");
    let (ok_plan, stdout_plan, stderr_plan) = kslim_in(
        &kslim_dir,
        &["plan", "--frozen-plan", plan_path.to_str().unwrap()],
    );
    assert!(
        ok_plan,
        "frozen plan write failed: stdout={stdout_plan:?} stderr={stderr_plan:?}"
    );
    let document = std::fs::read_to_string(&plan_path).unwrap();

    let bad_schema = kslim_dir.join("bad-schema.toml");
    std::fs::write(
        &bad_schema,
        document.replacen("schema_version = 1", "schema_version = 999", 1),
    )
    .unwrap();
    let (ok_schema, stdout_schema, stderr_schema) = kslim_in(
        &kslim_dir,
        &["generate", "--frozen-plan", bad_schema.to_str().unwrap()],
    );
    assert!(
        !ok_schema,
        "bad schema should fail: stdout={stdout_schema:?}"
    );
    assert!(stderr_schema.contains("unsupported frozen plan schema_version"));

    let bad_tool = kslim_dir.join("bad-tool.toml");
    std::fs::write(
        &bad_tool,
        document.replacen(
            &format!("tool_version = \"{}\"", env!("CARGO_PKG_VERSION")),
            "tool_version = \"999.0.0\"",
            1,
        ),
    )
    .unwrap();
    let (ok_tool, stdout_tool, stderr_tool) = kslim_in(
        &kslim_dir,
        &["generate", "--frozen-plan", bad_tool.to_str().unwrap()],
    );
    assert!(!ok_tool, "bad tool should fail: stdout={stdout_tool:?}");
    assert!(stderr_tool.contains("frozen plan tool_version"));
}
