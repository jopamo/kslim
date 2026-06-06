mod common;
use common::*;

fn write_runtime_matrix_profile(kslim_dir: &std::path::Path) {
    std::fs::write(
        kslim_dir.join("profiles/default.toml"),
        r#"[profile]
name = "default"
description = "Runtime matrix CLI override test"

[base]
ref = "v1.0"

[selftests]
enabled = true
check_kconfig_sources = true
check_makefiles = true
commands = ["true"]

[[selftests.kernel_builds]]
name = "must-not-run-in-runtime-matrix"
config_target = "defconfig"
make_program = "/bin/false"
"#,
    )
    .unwrap();
}

#[test]
fn test_matrix_flag_selects_runtime_selftest_view_without_mutating_output() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("linux.git");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        upstream.to_str().unwrap(),
    );
    write_runtime_matrix_profile(&kslim_dir);

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["matrix", "--matrix", "runtime"]);

    assert!(ok, "matrix failed: stdout={stdout:?} stderr={stderr:?}");
    assert!(stdout.contains("selected matrix: runtime"));
    assert!(stdout.contains("  check kconfig sources: false"));
    assert!(stdout.contains("  check makefiles: false"));
    assert!(stdout.contains("  kernel builds: 0"));
    assert!(stdout.contains("  commands: 1"));
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
}

#[test]
fn test_generate_matrix_runtime_uses_runtime_selftest_selection() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "matrix-runtime", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );
    write_runtime_matrix_profile(&kslim_dir);

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["generate", "--matrix", "runtime"]);

    assert!(
        ok,
        "runtime matrix generate failed: stdout={stdout:?} stderr={stderr:?}"
    );
    assert!(stdout.contains("selftests: 0 built-in, 1 custom"));
    assert!(output_dir.exists());
}
