mod common;
use common::*;

#[test]
fn test_init_creates_files() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = tmp.path().join("my-project");
    std::fs::create_dir(&project_dir).unwrap();

    let (ok, stdout, _) = kslim_in(
        &project_dir,
        &[
            "init",
            "--name",
            "test-linux",
            "--output",
            "/tmp/test-out",
            "--upstream-name",
            "linux-next",
            "--upstream-url",
            "https://example.com/linux-next.git",
        ],
    );
    assert!(ok, "init failed: {}", stdout);
    assert!(project_dir.join("kslim.toml").exists());
    assert!(project_dir.join("profiles/default.toml").exists());
    assert!(project_dir.join("manifests").exists());
    assert!(project_dir
        .join("profiles/amdgpu-prune.toml.example")
        .exists());
    assert!(project_dir.join("docs/kernel-build-iteration.md").exists());
    let config = std::fs::read_to_string(project_dir.join("kslim.toml")).unwrap();
    assert!(
        config.contains("name = \"linux-next\""),
        "init should persist custom upstream name"
    );
    assert!(
        config.contains("url = \"https://example.com/linux-next.git\""),
        "init should persist custom upstream url"
    );

    let amdgpu =
        std::fs::read_to_string(project_dir.join("profiles/amdgpu-prune.toml.example")).unwrap();
    assert!(amdgpu.contains("remove_paths = [\"drivers/gpu/drm/amd/amdgpu\"]"));
    assert!(amdgpu.contains("[[selftests.kernel_builds]]"));

    let guide =
        std::fs::read_to_string(project_dir.join("docs/kernel-build-iteration.md")).unwrap();
    assert!(guide.contains("kslim generate"));
}

#[test]
fn test_init_refuses_double_init() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = tmp.path().join("my-project");
    std::fs::create_dir(&project_dir).unwrap();

    let (ok, _, _) = kslim_in(
        &project_dir,
        &["init", "--name", "test-linux", "--output", "/tmp/test-out"],
    );
    assert!(ok);
    let (ok2, _, _) = kslim_in(
        &project_dir,
        &["init", "--name", "test-linux", "--output", "/tmp/test-out"],
    );
    assert!(!ok2, "second init should fail");
}

#[test]
fn test_config_validation_empty_url() {
    let tmp = tempfile::tempdir().unwrap();
    let kslim_dir = tmp.path().join("my-project");
    std::fs::create_dir(&kslim_dir).unwrap();

    let config = r#"[project]
name = "test"

[upstream]
name = "linux"
url = ""

[output]
path = "/tmp/out"
"#;
    std::fs::write(kslim_dir.join("kslim.toml"), config).unwrap();
    std::fs::create_dir_all(kslim_dir.join("profiles")).unwrap();
    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        "[profile]\nname = \"default\"\n\n[base]\nref = \"v1.0\"\n",
    )
    .unwrap();

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["status"]);
    assert!(!ok, "should reject empty upstream URL");
    assert!(
        stderr.contains("upstream.url"),
        "expected upstream.url error, got: {}",
        stderr
    );
}

#[test]
fn test_config_validation_empty_output_path() {
    let tmp = tempfile::tempdir().unwrap();
    let kslim_dir = tmp.path().join("my-project");
    std::fs::create_dir(&kslim_dir).unwrap();

    let config = r#"[project]
name = "test"

[upstream]
name = "linux"
url = "https://example.com/linux.git"

[output]
path = ""
"#;
    std::fs::write(kslim_dir.join("kslim.toml"), config).unwrap();
    std::fs::create_dir_all(kslim_dir.join("profiles")).unwrap();
    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        "[profile]\nname = \"default\"\n\n[base]\nref = \"v1.0\"\n",
    )
    .unwrap();

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["status"]);
    assert!(!ok, "should reject empty output path");
    assert!(
        stderr.contains("output.path"),
        "expected output.path error, got: {}",
        stderr
    );
}

#[test]
fn test_validate_config_accepts_project_config_and_profiles() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("linux.git");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        upstream.to_str().unwrap(),
    );

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["validate-config"]);

    assert!(
        ok,
        "validate-config failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains("config: ok"));
    assert!(stdout.contains("project: test-linux"));
    assert!(stdout.contains("profiles:"));
    assert!(stdout.contains("  - default: ok (base: v1.0)"));
}

#[test]
fn test_validate_config_rejects_invalid_profile() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("linux.git");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        upstream.to_str().unwrap(),
    );
    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"
[profile]
name = "default"

[base]
ref = ""
"#,
    )
    .unwrap();

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["validate-config"]);

    assert!(!ok, "invalid profile should fail validation");
    assert!(stderr.contains("failed to validate profile 'default'"));
    assert!(stderr.contains("base.ref must not be empty"));
}

#[test]
fn test_plan_resolves_generate_plan_without_mutating_output() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "plan", "1.0");
    let work = tmp.path().join("upstream-work-plan");
    std::fs::write(work.join("Makefile"), "# Linux planned head\n").unwrap();
    git_in(work.to_str().unwrap(), &["add", "-A"]);
    git_in(
        work.to_str().unwrap(),
        &["commit", "-m", "Linux planned head"],
    );
    git_in(
        work.to_str().unwrap(),
        &["push", &upstream, "HEAD:refs/heads/plan-head"],
    );
    let plan_commit = git_in(&upstream, &["rev-parse", "plan-head"])
        .trim()
        .to_string();
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["plan", "--base", "plan-head"]);

    assert!(ok, "plan failed: stdout={:?} stderr={:?}", stdout, stderr);
    assert!(stdout.contains("plan: plan-"));
    assert!(stdout.contains("fingerprint: fingerprint-"));
    assert!(stdout.contains("config hash: config-"));
    assert!(stdout.contains("profile: default"));
    assert!(stdout.contains(&format!("base: plan-head -> {plan_commit}")));
    assert!(stdout.contains(&format!("  path: {}", output_dir.display())));
    assert!(stdout.contains("  patches: 0"));
    assert!(stdout.contains("  source: no_removal_input"));
    assert!(stdout.contains("  enabled: true"));
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
    assert!(!kslim_dir.join(".kslim").exists());
}
#[test]
fn test_explain_abi_reports_policy_without_mutating_output() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("linux.git");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        upstream.to_str().unwrap(),
    );
    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "ABI explain test"

[base]
ref = "v1.0"

[abi]
allow_public_header_removal = true

[slim]
remove_paths = ["include/linux/old.h"]

[features.remove.netlink]
roots = ["include/uapi/linux/netlink.h"]
allow_uapi_header_removal = true
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["explain-abi"]);

    assert!(
        ok,
        "explain-abi failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains("explain-abi"));
    assert!(stdout.contains("profile: default"));
    assert!(stdout.contains("decision: approved"));
    assert!(stdout.contains("owner: profile default"));
    assert!(stdout.contains("proof source: profile ABI/UAPI policy"));
    assert!(stdout.contains("abi.allow_public_header_removal: true (profile"));
    assert!(stdout.contains("abi.allow_uapi_header_removal: false (default"));
    assert!(stdout.contains("effective policy:"));
    assert!(stdout.contains("  allow public header removal: true"));
    assert!(stdout.contains("  allow uapi header removal: true"));
    assert!(stdout.contains("feature-scoped approvals:"));
    assert!(stdout.contains("features.remove.netlink.allow_uapi_header_removal: true (profile"));
    assert!(stdout.contains("ABI-sensitive removal candidates:"));
    assert!(stdout.contains("  - include/linux/old.h (public_header)"));
    assert!(stdout.contains("    owner: profile slim.remove_paths"));
    assert!(stdout.contains("    proof source: abi.allow_public_header_removal"));
    assert!(stdout.contains("  - include/uapi/linux/netlink.h (uapi)"));
    assert!(stdout.contains("    owner: features.remove.netlink"));
    assert!(stdout.contains("    proof source: features.remove.netlink.allow_uapi_header_removal"));
    assert!(stdout.contains("fail-closed behavior:"));
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
    assert!(!kslim_dir.join(".kslim").exists());
}

#[test]
fn test_matrix_reports_selected_selftest_matrix_without_mutating_output() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("linux.git");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        upstream.to_str().unwrap(),
    );
    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Matrix report test"

[base]
ref = "v1.0"

[selftests]
enabled = true
check_kconfig_sources = true
check_makefiles = false
commands = ["make -C tools/testing/selftests TARGETS=net run_tests"]

[[selftests.kernel_builds]]
name = "x86-defconfig"
config_target = "defconfig"
targets = ["vmlinux", "modules"]
output_dir = "build/x86"
jobs = 4
clean = false
make_program = "gmake"
make_args = ["LLVM=1"]

[selftests.kernel_builds.env]
ARCH = "x86"
CROSS_COMPILE = "x86_64-linux-gnu-"
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["matrix"]);

    assert!(ok, "matrix failed: stdout={:?} stderr={:?}", stdout, stderr);
    assert!(stdout.contains("matrix"));
    assert!(stdout.contains("profile: default"));
    assert!(stdout.contains("effective source: selftests"));
    assert!(stdout.contains("future build matrix:"));
    assert!(stdout.contains("  enabled: false"));
    assert!(stdout.contains("future runtime matrix:"));
    assert!(stdout.contains("selected selftests:"));
    assert!(stdout.contains("  enabled: true"));
    assert!(stdout.contains("  check kconfig sources: true"));
    assert!(stdout.contains("  check makefiles: false"));
    assert!(stdout.contains("  kernel builds: 1"));
    assert!(stdout.contains("    - x86-defconfig"));
    assert!(stdout.contains("      arch: x86"));
    assert!(stdout.contains("      config target: defconfig"));
    assert!(stdout.contains("      targets: vmlinux, modules"));
    assert!(stdout.contains("      output dir: build/x86"));
    assert!(stdout.contains("      jobs: 4"));
    assert!(stdout.contains("      clean: false"));
    assert!(stdout.contains("      make program: gmake"));
    assert!(stdout.contains("      make args: LLVM=1"));
    assert!(stdout.contains("      env: ARCH=x86, CROSS_COMPILE=x86_64-linux-gnu-"));
    assert!(stdout.contains("  commands: 1"));
    assert!(stdout.contains("    - make -C tools/testing/selftests TARGETS=net run_tests"));
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
    assert!(!kslim_dir.join(".kslim").exists());
}

#[test]
fn test_selftest_runs_profile_selftests_without_mutating_output() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("linux.git");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        upstream.to_str().unwrap(),
    );
    let kernel = tmp.path().join("kernel");
    std::fs::create_dir_all(&kernel).unwrap();
    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Selftest CLI test"

[base]
ref = "v1.0"

[selftests]
check_kconfig_sources = false
check_makefiles = false
commands = ["touch selftest-marker"]
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(
        &kslim_dir,
        &["selftest", "--tree", kernel.to_str().unwrap()],
    );

    assert!(
        ok,
        "selftest failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains("selftest: ok"));
    assert!(stdout.contains("profile: default"));
    assert!(stdout.contains(&format!("tree: {}", kernel.display())));
    assert!(stdout.contains("enabled: true"));
    assert!(stdout.contains("built-in checks: 0"));
    assert!(stdout.contains("kernel builds: 0"));
    assert!(stdout.contains("commands: 1"));
    assert!(kernel.join("selftest-marker").exists());
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
    assert!(!kslim_dir.join(".kslim").exists());
}

#[test]
fn test_fuzz_fixtures_writes_deterministic_seed_corpus_without_mutating_output() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("linux.git");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        upstream.to_str().unwrap(),
    );

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["fuzz-fixtures"]);

    assert!(
        ok,
        "fuzz-fixtures failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains("fuzz-fixtures: written"));
    assert!(stdout.contains("files: 12"));
    assert!(stdout.contains("  - kconfig/malformed.Kconfig"));
    assert!(stdout.contains("  - cpp/fake-includes.c"));
    assert!(stdout.contains("  - metadata/reducer-report.json"));
    let fixtures = kslim_dir.join("fuzz-fixtures");
    assert!(fixtures.join("README.md").exists());
    assert!(fixtures.join("kconfig/malformed.Kconfig").exists());
    assert!(fixtures.join("kbuild/Makefile.multiline").exists());
    assert!(fixtures.join("cpp/nested-branches.c").exists());
    assert!(fixtures.join("metadata/reducer-report.json").exists());
    let before = std::fs::read_to_string(fixtures.join("metadata/reducer-report.json")).unwrap();

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["fuzz-fixtures"]);

    assert!(ok, "idempotent fuzz-fixtures failed: {}", stderr);
    let after = std::fs::read_to_string(fixtures.join("metadata/reducer-report.json")).unwrap();
    assert_eq!(before, after);

    let (ok, _, stderr) = kslim_in(
        &kslim_dir,
        &["fuzz-fixtures", "--out", output_dir.to_str().unwrap()],
    );

    assert!(
        !ok,
        "fuzz-fixtures should reject the configured output repo"
    );
    assert!(
        stderr.contains("refusing to write fuzz fixtures inside configured output repo"),
        "unexpected stderr: {}",
        stderr
    );
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
    assert!(!kslim_dir.join(".kslim").exists());
}

#[test]
fn test_reduce_tree_applies_profile_reducer_without_mutating_project_output() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("linux.git");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        upstream.to_str().unwrap(),
    );
    let kernel = tmp.path().join("kernel");
    std::fs::create_dir_all(kernel.join("drivers/foo")).unwrap();
    std::fs::write(kernel.join("Makefile"), "# test kernel\n").unwrap();
    std::fs::write(kernel.join("Kconfig"), "source \"drivers/Kconfig\"\n").unwrap();
    std::fs::write(kernel.join("drivers/Makefile"), "obj-y += foo/\n").unwrap();
    std::fs::write(
        kernel.join("drivers/Kconfig"),
        "source \"drivers/foo/Kconfig\"\n",
    )
    .unwrap();
    std::fs::write(
        kernel.join("drivers/foo/Kconfig"),
        "config FOO\n\ttristate \"Foo\"\n",
    )
    .unwrap();
    std::fs::write(kernel.join("drivers/foo/Makefile"), "obj-y += foo.o\n").unwrap();
    std::fs::write(kernel.join("drivers/foo/foo.c"), "int foo;\n").unwrap();
    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Reduce tree test"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/foo"]
remove_configs = ["FOO"]
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(
        &kslim_dir,
        &["reduce-tree", "--tree", kernel.to_str().unwrap()],
    );

    assert!(
        ok,
        "reduce-tree failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains("reduce-tree: done"));
    assert!(stdout.contains("profile: default"));
    assert!(stdout.contains("status: success"));
    assert!(stdout.contains("  files removed: 3"));
    assert!(stdout.contains("  kconfig refs removed: 1"));
    assert!(stdout.contains("  makefile refs removed: 1"));
    assert!(!kernel.join("drivers/foo").exists());
    let drivers_makefile = std::fs::read_to_string(kernel.join("drivers/Makefile")).unwrap();
    assert!(!drivers_makefile.contains("foo/"));
    let drivers_kconfig = std::fs::read_to_string(kernel.join("drivers/Kconfig")).unwrap();
    assert!(drivers_kconfig.contains("# kslim: removed source \"drivers/foo/Kconfig\""));
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
    assert!(!kslim_dir.join(".kslim").exists());
}

#[test]
fn test_unknown_profile_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["generate", "--profile", "nope", "--dry-run"]);
    assert!(!ok, "should reject unknown profile");
    assert!(
        stderr.contains("unknown profile"),
        "expected 'unknown profile' error, got: {}",
        stderr
    );
}

#[test]
fn test_profile_flag_selects_named_profile_for_plan_and_generate() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "profile-flag", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );
    std::fs::write(
        kslim_dir.join("profiles/custom.toml"),
        r#"[profile]
name = "custom"
description = "Custom profile selected by --profile"

[base]
ref = "v1.0"

[selftests]
enabled = false
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["plan", "--profile", "custom"]);

    assert!(ok, "plan failed: stdout={:?} stderr={:?}", stdout, stderr);
    assert!(stdout.contains("profile: custom"));
    assert!(stdout.contains("  branch: kslim/v1.0/custom"));
    assert!(stdout.contains("  enabled: false"));

    let (ok, stdout, stderr) = kslim_in(
        &kslim_dir,
        &["generate", "--profile", " custom ", "--dry-run"],
    );

    assert!(
        ok,
        "generate dry-run failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains("  would resolve base:   v1.0 ->"));
    assert!(stdout.contains("  target branch: kslim/v1.0/custom"));
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
    assert!(!kslim_dir.join(".kslim").exists());
}

#[test]
fn test_profile_flag_rejects_path_like_names() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("linux.git");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        upstream.to_str().unwrap(),
    );

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["plan", "--profile", "../default"]);

    assert!(!ok, "path-like profile name should be rejected");
    assert!(
        stderr.contains("profile name must not contain path separators"),
        "unexpected stderr: {}",
        stderr
    );

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["plan", "--profile", " "]);

    assert!(!ok, "empty profile name should be rejected");
    assert!(
        stderr.contains("profile name must not be empty"),
        "unexpected stderr: {}",
        stderr
    );
}

#[test]
fn test_generate_cli_base_override_takes_precedence_over_profile_base() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "override", "1.0");
    let work = tmp.path().join("upstream-work-override");
    std::fs::write(work.join("Makefile"), "# Linux override head\n").unwrap();
    git_in(work.to_str().unwrap(), &["add", "-A"]);
    git_in(
        work.to_str().unwrap(),
        &["commit", "-m", "Linux override head"],
    );
    git_in(
        work.to_str().unwrap(),
        &["push", &upstream, "HEAD:refs/heads/override-head"],
    );
    let profile_commit = git_in(&upstream, &["rev-parse", "v1.0"]).trim().to_string();
    let override_commit = git_in(&upstream, &["rev-parse", "override-head"])
        .trim()
        .to_string();
    assert_ne!(profile_commit, override_commit);

    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok, stdout, stderr) = kslim_in(
        &kslim_dir,
        &[
            "generate",
            "--reducer-report-only",
            "--base",
            "override-head",
        ],
    );
    assert!(
        ok,
        "generate report-only failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains(&format!(
        "base:                 override-head -> {override_commit}"
    )));
    assert!(!stdout.contains(&profile_commit));

    let report = std::fs::read_to_string(project_failure_report_path(&kslim_dir)).unwrap();
    assert!(report.contains("Base ref: override-head"));
    assert!(report.contains(&format!("Base commit: {override_commit}")));
    assert!(!report.contains(&format!("Base commit: {profile_commit}")));
    assert!(report.contains("    base.ref: profile ("));
    assert!(report.contains("    base.ref: cli (cli --base)"));
}

#[test]
fn test_generate_cli_base_override_is_normalized_before_resolution() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "normalized", "1.0");
    let work = tmp.path().join("upstream-work-normalized");
    std::fs::write(work.join("Makefile"), "# Linux normalized head\n").unwrap();
    git_in(work.to_str().unwrap(), &["add", "-A"]);
    git_in(
        work.to_str().unwrap(),
        &["commit", "-m", "Linux normalized head"],
    );
    git_in(
        work.to_str().unwrap(),
        &["push", &upstream, "HEAD:refs/heads/override-head"],
    );
    let override_commit = git_in(&upstream, &["rev-parse", "override-head"])
        .trim()
        .to_string();

    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok, stdout, stderr) = kslim_in(
        &kslim_dir,
        &[
            "generate",
            "--reducer-report-only",
            "--base",
            "  override-head  ",
        ],
    );
    assert!(
        ok,
        "generate report-only failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains(&format!(
        "base:                 override-head -> {override_commit}"
    )));
    assert!(!stdout.contains("  override-head  "));

    let report = std::fs::read_to_string(project_failure_report_path(&kslim_dir)).unwrap();
    assert!(report.contains("Base ref: override-head"));
    assert!(report.contains(&format!("Base commit: {override_commit}")));
    assert!(!report.contains("Base ref:   override-head  "));
    assert!(report.contains("    base.ref: cli (cli --base)"));
}

#[test]
fn test_refuses_non_kslim_output_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    std::fs::create_dir_all(&output_dir).unwrap();
    std::fs::write(output_dir.join("somefile.txt"), "hello").unwrap();

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(!ok, "should refuse non-kslim output dir");
    assert!(
        stderr.contains("output path exists but is not a git repository")
            || stderr.contains("not managed by kslim"),
        "expected safety error, got: {}",
        stderr
    );
}

#[test]
fn test_generates_output_repo() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok,
        "generate failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(
        stdout.contains("stage: publish"),
        "generate stdout should expose final stage for CI summaries: {}",
        stdout
    );
    assert!(output_dir.exists());
    assert!(output_dir.join(".git").exists());
    assert!(output_dir.join(".kslim").exists());
    assert!(output_dir.join("Makefile").exists());
    assert!(output_meta_path(&output_dir, "managed.toml").exists());
    assert!(output_meta_path(&output_dir, "base.toml").exists());
    assert!(output_meta_path(&output_dir, "generated.toml").exists());
    assert!(output_meta_path(&output_dir, "manifest.txt").exists());
    assert!(output_meta_path(&output_dir, "published.toml").exists());
    assert!(output_meta_path(&output_dir, "report.txt").exists());
    let report = std::fs::read_to_string(output_meta_path(&output_dir, "report.txt")).unwrap();
    assert!(report.contains("Stage: metadata"));
    assert!(committed_output_meta_path(&output_dir, "managed.toml").exists());
    assert!(committed_output_meta_path(&output_dir, "published.toml").exists());
}

#[test]
fn test_generate_accepts_worktree_git_dir_upstream() {
    let tmp = tempfile::tempdir().unwrap();
    let _bare = create_fake_upstream(tmp.path(), "test", "1.0");
    let upstream_gitdir = tmp.path().join("upstream-work-test").join(".git");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        upstream_gitdir.to_str().unwrap(),
    );

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok,
        "generate with direct .git upstream failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(output_dir.join("Makefile").exists());
    assert!(output_meta_path(&output_dir, "report.txt").exists());
}

#[test]
fn test_generate_integrates_rtlmq_natively() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream_with_realtek(tmp.path(), "realtek", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );
    let rtlmq_src = create_fake_rtlmq_source(tmp.path());

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        format!(
            r#"[profile]
name = "default"
description = "Test profile"

[base]
ref = "v1.0"

[integrations.rtlmq]
source = "{source}"
"#,
            source = rtlmq_src.to_string_lossy()
        ),
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok,
        "generate with rtlmq integration failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );

    assert!(output_dir
        .join("drivers/net/ethernet/realtek/rtlmq/rtlmq_main.c")
        .exists());
    assert!(!output_dir
        .join("drivers/net/ethernet/realtek/rtlmq/rtlmq_kunit_refine.c")
        .exists());
    assert!(output_dir
        .join("drivers/net/ethernet/realtek/rtlmq/scripts/helper.sh")
        .exists());
    assert!(!output_dir
        .join("tools/testing/selftests/drivers/net/rtlmq/smoke.sh")
        .exists());

    let realtek_kconfig =
        std::fs::read_to_string(output_dir.join("drivers/net/ethernet/realtek/Kconfig")).unwrap();
    assert!(realtek_kconfig.contains(r#"source "drivers/net/ethernet/realtek/rtlmq/Kconfig""#));

    let realtek_makefile =
        std::fs::read_to_string(output_dir.join("drivers/net/ethernet/realtek/Makefile")).unwrap();
    assert!(realtek_makefile.contains("obj-$(CONFIG_RTLMQ) += rtlmq/"));
}

#[test]
fn test_generate_integrates_explicit_rtlmq_tests_source() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream_with_realtek(tmp.path(), "realtek", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );
    let rtlmq_src = create_fake_rtlmq_source(tmp.path());
    let rtlmq_tests = rtlmq_src.parent().unwrap().join("rtlmq-tests");

    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        format!(
            r#"[profile]
name = "default"
description = "Test profile"

[base]
ref = "v1.0"

[integrations.rtlmq]
source = "{source}"
tests_source = "{tests_source}"
"#,
            source = rtlmq_src.to_string_lossy(),
            tests_source = rtlmq_tests.to_string_lossy()
        ),
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok,
        "generate with rtlmq tests integration failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );

    assert!(output_dir
        .join("drivers/net/ethernet/realtek/rtlmq/rtlmq_kunit_refine.c")
        .exists());
    assert!(output_dir
        .join("tools/testing/selftests/drivers/net/rtlmq/smoke.sh")
        .exists());
}

#[test]
fn test_generate_applies_latest_worktree_patches() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "patched", "1.0");
    let patch_worktree = create_patch_worktree(
        tmp.path(),
        &upstream,
        "topic/latest-makefile",
        "Makefile",
        "1.0 + patch stack",
    );
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
description = "Apply latest worktree patches"

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

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok,
        "generate failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(
        stdout.contains("patches: 1"),
        "generate should report applied patch count: {}",
        stdout
    );

    let makefile = std::fs::read_to_string(output_dir.join("Makefile")).unwrap();
    assert!(makefile.contains("1.0 + patch stack"));

    let patch_meta =
        std::fs::read_to_string(output_meta_path(&output_dir, "patches.toml")).unwrap();
    assert!(patch_meta.contains("source_count = 1"));
    assert!(patch_meta.contains("total_patch_count = 1"));
    assert!(patch_meta.contains("source = \"worktree\""));
    assert!(patch_meta.contains("patch_count = 1"));
    assert!(patch_meta.contains("branch = \"topic/latest-makefile\""));

    let report = std::fs::read_to_string(output_meta_path(&output_dir, "report.txt")).unwrap();
    assert!(report.contains("Patches:"));
    assert!(report.contains("Total count: 1"));
}

#[test]
fn test_generate_applies_multiple_worktree_patch_sources_in_order() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "patched-multi", "1.0");
    let patch_worktree_one = create_patch_worktree(
        tmp.path(),
        &upstream,
        "topic/patch-one",
        "Makefile",
        "1.0 + patch one",
    );
    let patch_worktree_two = create_patch_worktree_replace(
        tmp.path(),
        &upstream,
        "topic/patch-two",
        "Kconfig",
        "# Kconfig",
        "# Kconfig + patch two",
    );

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
description = "Apply multiple worktree patch sources"

[base]
ref = "v1.0"

[patches]

[[patches.sources]]
source = "worktree"
path = "{}"
base_remote = "origin"
base_ref = "master"
require_clean = true

[[patches.sources]]
source = "worktree"
path = "{}"
base_remote = "origin"
base_ref = "master"
require_clean = true
"#,
            patch_worktree_one.display(),
            patch_worktree_two.display()
        ),
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(
        ok,
        "generate failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(
        stdout.contains("patches: 2"),
        "generate should report total applied patch count: {}",
        stdout
    );

    let makefile = std::fs::read_to_string(output_dir.join("Makefile")).unwrap();
    assert!(makefile.contains("1.0 + patch one"));
    let kconfig = std::fs::read_to_string(output_dir.join("Kconfig")).unwrap();
    assert!(kconfig.contains("# Kconfig + patch two"));

    let patch_meta =
        std::fs::read_to_string(output_meta_path(&output_dir, "patches.toml")).unwrap();
    assert!(patch_meta.contains("source_count = 2"));
    assert!(patch_meta.contains("total_patch_count = 2"));
    assert!(patch_meta.contains("branch = \"topic/patch-one\""));
    assert!(patch_meta.contains("branch = \"topic/patch-two\""));

    let report = std::fs::read_to_string(output_meta_path(&output_dir, "report.txt")).unwrap();
    assert!(report.contains("Sources: 2"));
    assert!(report.contains("Total count: 2"));
    assert!(report.contains("topic/patch-one"));
    assert!(report.contains("topic/patch-two"));
}

#[test]
fn test_branch_naming_is_predictable() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok, _, _) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok);

    let branch = git_in(output_dir.to_str().unwrap(), &["branch", "--show-current"]);
    assert_eq!(branch.trim(), "kslim/v1.0/default");
}

#[test]
fn test_fixed_output_branch_name_is_used() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test-fixed-branch", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let config = format!(
        r#"[project]
name = "test-linux"

[upstream]
name = "linux"
url = "{up}"

[output]
path = "{out}"
branch_prefix = "kslim"
branch = "snapshot"
"#,
        up = upstream,
        out = output_dir.to_str().unwrap(),
    );
    std::fs::write(kslim_dir.join("kslim.toml"), config).unwrap();

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok, "generate failed: {}", stderr);

    let branch = git_in(output_dir.to_str().unwrap(), &["branch", "--show-current"]);
    assert_eq!(branch.trim(), "snapshot");

    let branches = git_in(
        output_dir.to_str().unwrap(),
        &["branch", "--format=%(refname:short)"],
    );
    assert!(
        branches.lines().all(|line| line.trim() == "snapshot"),
        "expected only snapshot branch, got: {}",
        branches
    );
}

#[test]
fn test_output_git_config_is_built_from_project_config() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test-git-config", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let config = format!(
        r#"[project]
name = "test-linux"

[upstream]
name = "linux"
url = "{up}"

[output]
path = "{out}"
branch_prefix = "kslim"
branch = "snapshot"

[git]
user_email = "kslim@localhost"
user_name = "kslim"
remote_name = "origin"

[publish]
remote = "gitlab-pjo:pjo/kone.git"
"#,
        up = upstream,
        out = output_dir.to_str().unwrap(),
    );
    std::fs::write(kslim_dir.join("kslim.toml"), config).unwrap();

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok, "generate failed: {}", stderr);

    let git_config = std::fs::read_to_string(output_dir.join(".git/config")).unwrap();
    assert_eq!(
        git_config.trim(),
        r#"[core]
	repositoryformatversion = 0
	filemode = true
	bare = false
	logallrefupdates = true
[user]
	email = kslim@localhost
	name = kslim
[remote "origin"]
	url = gitlab-pjo:pjo/kone.git
	fetch = +refs/heads/*:refs/remotes/origin/*
[branch "snapshot"]
	remote = origin
	merge = refs/heads/snapshot"#
    );
}

#[test]
fn test_report_exists_after_generation() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok, _, _) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok);

    let report_path = output_meta_path(&output_dir, "report.txt");
    assert!(report_path.exists());
    let contents = std::fs::read_to_string(&report_path).unwrap();
    assert!(contents.contains("kslim report"));
    assert!(contents.contains("Files:"));
    assert!(contents.contains("Bytes:"));

    let (ok_report, stdout_report, stderr_report) = kslim_in(&kslim_dir, &["report"]);
    assert!(
        ok_report,
        "report failed: stdout={:?} stderr={:?}",
        stdout_report, stderr_report
    );
    assert!(stdout_report.contains("report scope: published"));
    assert!(stdout_report.contains("authoritative: true"));
    assert!(stdout_report.contains("artifact: report.txt"));
    assert!(stdout_report.contains("kslim report"));
    assert!(stdout_report.contains("Profile: default"));
    assert!(stdout_report.contains("Files:"));
}

#[test]
fn test_report_prints_non_authoritative_attempt_report_when_no_published_report() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "attempt-report", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok_generate, stdout_generate, stderr_generate) =
        kslim_in(&kslim_dir, &["generate", "--reducer-report-only"]);
    assert!(
        ok_generate,
        "report-only generate failed: stdout={:?} stderr={:?}",
        stdout_generate, stderr_generate
    );
    assert!(!output_dir.exists());

    let (ok_report, stdout_report, stderr_report) = kslim_in(&kslim_dir, &["report"]);
    assert!(
        ok_report,
        "report failed: stdout={:?} stderr={:?}",
        stdout_report, stderr_report
    );
    assert!(stdout_report.contains("report scope: attempt"));
    assert!(stdout_report.contains("authoritative: false"));
    assert!(stdout_report.contains("artifact: report.txt"));
    assert!(stdout_report.contains("kslim report"));
    assert!(stdout_report.contains("Status: report-only"));
    assert!(stdout_report.contains("Metadata scope: non-authoritative-attempt"));
    assert!(!kslim_dir.join("kslim.lock").exists());
}

#[test]
fn test_status_shows_project_info() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok, stdout, _) = kslim_in(&kslim_dir, &["status"]);
    assert!(ok);
    assert!(
        stdout.contains("test-linux"),
        "status should show project name: {}",
        stdout
    );
    assert!(
        stdout.contains(&upstream),
        "status should show upstream URL: {}",
        stdout
    );
    assert!(
        stdout.contains("direct read-only"),
        "status should show direct upstream mode: {}",
        stdout
    );
    assert!(
        stdout.contains("profiles:"),
        "status should list profiles: {}",
        stdout
    );
    assert!(
        stdout.contains("default"),
        "status should mention default profile: {}",
        stdout
    );
}

#[test]
fn test_compare_detects_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream1 = create_fake_upstream(tmp.path(), "v1", "1.0");

    // Create a v2.0 upstream with an extra file
    let work2 = tmp.path().join("upstream-work-v2");
    let bare2 = tmp.path().join("upstream-v2.git");

    std::fs::create_dir_all(&work2).unwrap();
    for dir in &[
        "arch", "drivers", "fs", "include", "kernel", "mm", "net", "scripts",
    ] {
        std::fs::create_dir_all(work2.join(dir)).unwrap();
    }
    std::fs::write(work2.join("Makefile"), "# Linux 2.0 Makefile\n").unwrap();
    std::fs::write(work2.join("Kconfig"), "# Kconfig v2\n").unwrap();
    std::fs::write(work2.join("NEWFILE"), "new content\n").unwrap();
    for d in &[
        "arch", "drivers", "fs", "include", "kernel", "mm", "net", "scripts",
    ] {
        std::fs::write(work2.join(d).join(".keep"), "").unwrap();
    }

    git_in(work2.to_str().unwrap(), &["init"]);
    git_in(
        work2.to_str().unwrap(),
        &["config", "user.email", "test@kslim.local"],
    );
    git_in(
        work2.to_str().unwrap(),
        &["config", "user.name", "kslim test"],
    );
    git_in(work2.to_str().unwrap(), &["add", "-A"]);
    git_in(work2.to_str().unwrap(), &["commit", "-m", "Linux 2.0"]);
    git_in(work2.to_str().unwrap(), &["tag", "v2.0"]);
    git_in(
        tmp.path().to_str().unwrap(),
        &[
            "clone",
            "--bare",
            work2.to_str().unwrap(),
            bare2.to_str().unwrap(),
        ],
    );
    git_in(
        bare2.to_str().unwrap(),
        &["remote", "set-url", "origin", bare2.to_str().unwrap()],
    );

    // Push v2.0 refs into the v1.0 upstream repo as a remote
    git_in(
        &upstream1,
        &["remote", "add", "v2", bare2.to_str().unwrap()],
    );
    git_in(&upstream1, &["fetch", "--tags", "v2"]);

    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream1,
    );

    let (ok, stdout, _) = kslim_in(&kslim_dir, &["compare", "--from", "v1.0", "--to", "v2.0"]);
    assert!(ok, "compare failed: {}", stdout);
    assert!(
        stdout.contains("files added"),
        "should show added files: {}",
        stdout
    );
    assert!(
        stdout.contains("NEWFILE"),
        "should show NEWFILE: {}",
        stdout
    );
}
