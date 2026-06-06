mod common;
use common::*;

#[test]
fn test_feature_impact_reports_named_feature_intent_without_mutating_output() {
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
description = "Feature impact test"

[base]
ref = "v1.0"

[features.remove.bluetooth]
kind = "subsystem"
roots = ["net/bluetooth", "drivers/bluetooth"]
configs = ["BT"]
exported_symbols = ["bt_sock_register"]
module_names = ["btusb"]
module_aliases = ["usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*"]
device_compatibles = ["qcom,ipq8064"]
acpi_ids = ["PNP0C09"]
pci_ids = ["8086:1572"]
usb_ids = ["0BDA:8153"]
firmware_paths = ["amdgpu/polaris10_mc.bin"]
initcalls = ["bt_init"]
runtime_registrations = ["module_init:bt_init"]
docs = ["Documentation/networking/bluetooth.rst"]
tools = ["tools/perf"]
samples = ["samples/bpf"]
kunit_suites = ["bt_test"]
kselftest_targets = ["net"]
arch_scope = ["x86"]
safety = "surgical"
require_clean_boot = true
report_only = true

[features.preserve.netfilter]
kind = "subsystem"
roots = ["net/netfilter"]
configs = ["NETFILTER"]
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["feature-impact"]);

    assert!(
        ok,
        "feature-impact failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains("profile: default"));
    assert!(stdout.contains("feature filter: <all>"));
    assert!(stdout.contains("effective impact:"));
    assert!(stdout.contains("  remove paths: 2"));
    assert!(stdout.contains("  remove configs: 1"));
    assert!(stdout.contains("  preserve paths: 1"));
    assert!(stdout.contains("  preserve configs: 1"));
    assert!(stdout.contains("  - bluetooth (remove)"));
    assert!(stdout.contains("    kind: subsystem"));
    assert!(stdout.contains("    roots: net/bluetooth, drivers/bluetooth"));
    assert!(stdout.contains("    configs: BT"));
    assert!(stdout.contains("    arch scope: x86"));
    assert!(stdout.contains("    safety: surgical"));
    assert!(stdout.contains("    require clean boot: true"));
    assert!(stdout.contains("    report only: true"));
    assert!(stdout.contains("  - netfilter (preserve)"));
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
    assert!(!kslim_dir.join(".kslim").exists());

    let (ok, stdout, stderr) = kslim_in(
        &kslim_dir,
        &[
            "feature-impact",
            "--safety",
            "aggressive",
            "--feature",
            "bluetooth",
        ],
    );
    assert!(
        ok,
        "filtered feature-impact failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains("  remove paths: 2"));
    assert!(stdout.contains("  remove configs: 1"));
    assert!(stdout.contains("  - bluetooth (remove)"));
    assert!(!stdout.contains("  - netfilter (preserve)"));
    assert!(stdout.contains("    safety: aggressive"));
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
}

#[test]
fn test_feature_impact_emits_actionable_conflicts_without_mutating_output() {
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
description = "Feature conflict emission test"

[base]
ref = "v1.0"

[features.remove.bluetooth]
roots = ["drivers/bluetooth"]
configs = ["BT"]

[features.preserve.rfkill]
roots = ["drivers/bluetooth/btusb.c"]
configs = ["RFKILL"]
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["feature-impact"]);

    assert!(
        ok,
        "feature-impact failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains("feature conflicts:"));
    assert!(stdout.contains("  total: 1"));
    assert!(stdout.contains("  blocking: 1"));
    assert!(stdout.contains(
        "  - shared_file_between_removed_and_preserved_features:bluetooth:path:drivers/bluetooth/btusb.c"
    ));
    assert!(stdout.contains("    kind: shared_file_between_removed_and_preserved_features"));
    assert!(stdout.contains("    feature: bluetooth"));
    assert!(stdout.contains("    subject: path:drivers/bluetooth/btusb.c"));
    assert!(stdout.contains(
        "    summary: removed feature 'bluetooth' shares path 'drivers/bluetooth/btusb.c' with preserved feature 'rfkill'"
    ));
    assert!(stdout.contains(
        "    action: split shared path 'drivers/bluetooth/btusb.c', narrow feature roots, or preserve feature 'bluetooth'"
    ));
    assert!(stdout.contains("    strict blocking: true"));
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
    assert!(!kslim_dir.join(".kslim").exists());
}

#[test]
fn test_generate_refuses_strict_feature_conflicts_before_candidate_mutation() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "feature-conflict-gate", "1.0");
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
description = "Feature strict conflict gate test"

[base]
ref = "v1.0"

[features.remove.bluetooth]
roots = ["drivers/bluetooth"]

[features.preserve.rfkill]
roots = ["drivers/bluetooth/btusb.c"]
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(
        &kslim_dir,
        &["generate", "--deep-dry-run", "--no-selftests"],
    );

    assert!(
        !ok,
        "generate should reject strict feature conflicts: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stderr.contains("unresolved feature conflicts block strict mutation"));
    assert!(stderr.contains(
        "shared_file_between_removed_and_preserved_features:bluetooth:path:drivers/bluetooth/btusb.c"
    ));
    assert!(stderr.contains(
        "action: split shared path 'drivers/bluetooth/btusb.c', narrow feature roots, or preserve feature 'bluetooth'"
    ));
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
    assert!(!kslim_dir.join(".kslim").exists());
}

#[test]
fn test_reduce_tree_refuses_strict_feature_conflicts_before_tree_mutation() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("linux.git");
    let output_dir = tmp.path().join("output");
    let tree = tmp.path().join("tree");
    std::fs::create_dir_all(tree.join("drivers/bluetooth")).unwrap();
    std::fs::create_dir_all(tree.join("arch")).unwrap();
    std::fs::create_dir_all(tree.join("include")).unwrap();
    std::fs::create_dir_all(tree.join("kernel")).unwrap();
    std::fs::create_dir_all(tree.join("scripts")).unwrap();
    std::fs::write(tree.join("Makefile"), "# test\n").unwrap();
    std::fs::write(tree.join("Kconfig"), "# test\n").unwrap();
    std::fs::write(tree.join("drivers/bluetooth/btusb.c"), "int btusb;\n").unwrap();
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
description = "Reduce-tree strict conflict gate test"

[base]
ref = "v1.0"

[features.remove.bluetooth]
roots = ["drivers/bluetooth"]

[features.preserve.rfkill]
roots = ["drivers/bluetooth/btusb.c"]
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(
        &kslim_dir,
        &["reduce-tree", "--tree", tree.to_str().unwrap()],
    );

    assert!(
        !ok,
        "reduce-tree should reject strict feature conflicts: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stderr.contains("unresolved feature conflicts block strict mutation"));
    assert!(tree.join("drivers/bluetooth/btusb.c").exists());
    assert!(!output_dir.exists());
}

#[test]
fn test_feature_flag_selects_named_feature_for_plan_and_generate() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "feature-flag", "1.0");
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
description = "Feature flag test"

[base]
ref = "v1.0"

[features.remove.bluetooth]
roots = ["net/bluetooth", "drivers/bluetooth"]
configs = ["BT"]

[features.remove.wifi]
roots = ["drivers/net/wireless"]

[features.preserve.netfilter]
roots = ["net/netfilter"]
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["plan", "--feature", " wifi "]);

    assert!(
        ok,
        "feature-selected plan failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains("profile: default"));
    assert!(stdout.contains("  source: named_feature_remove_input"));
    assert!(stdout.contains("  remove paths: 1"));
    assert!(stdout.contains("  remove configs: 0"));
    assert!(stdout.contains("  preserve paths: 0"));
    assert!(stdout.contains("  preserve configs: 0"));

    let (ok, stdout, stderr) = kslim_in(
        &kslim_dir,
        &[
            "generate",
            "--report-only",
            "--feature",
            "wifi",
            "--safety",
            "aggressive",
            "--no-strict",
        ],
    );

    assert!(
        ok,
        "feature-selected report-only generate failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains("target branch:        kslim/v1.0/default"));
    let report = std::fs::read_to_string(project_failure_report_path(&kslim_dir)).unwrap();
    assert!(report.contains("features.selected: cli (cli --feature)"));
    assert!(report.contains("features.remove.safety: cli (cli --safety)"));
    assert!(report.contains("reducer.reject_unreasoned_edits: cli (cli --no-strict)"));
    let (ok, _, stderr) = kslim_in(
        &kslim_dir,
        &["plan", "--feature", "wifi", "--safety", "reckless"],
    );
    assert!(!ok, "invalid safety flag should be rejected");
    assert!(stderr.contains("cli --safety is invalid"));
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
}

#[test]
fn test_feature_flag_rejects_empty_or_unknown_feature() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "feature-flag-reject", "1.0");
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
description = "Feature flag reject test"

[base]
ref = "v1.0"

[features.remove.bluetooth]
roots = ["net/bluetooth"]
"#,
    )
    .unwrap();

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["plan", "--feature", " "]);

    assert!(!ok, "empty feature flag should be rejected");
    assert!(
        stderr.contains("cli --feature must not be empty"),
        "unexpected stderr: {}",
        stderr
    );

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["plan", "--feature", "wifi"]);

    assert!(!ok, "unknown feature flag should be rejected");
    assert!(
        stderr.contains("feature 'wifi' is not declared"),
        "unexpected stderr: {}",
        stderr
    );

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["plan", "--remove-feature", " "]);

    assert!(!ok, "empty remove-feature flag should be rejected");
    assert!(
        stderr.contains("cli --remove-feature must not be empty"),
        "unexpected stderr: {}",
        stderr
    );

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["plan", "--preserve-feature", " "]);

    assert!(!ok, "empty preserve-feature flag should be rejected");
    assert!(
        stderr.contains("cli --preserve-feature must not be empty"),
        "unexpected stderr: {}",
        stderr
    );
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
}

#[test]
fn test_remove_feature_flag_selects_declared_remove_feature_only() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "remove-feature-flag", "1.0");
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
description = "Remove feature flag test"

[base]
ref = "v1.0"

[features.remove.bluetooth]
roots = ["net/bluetooth", "drivers/bluetooth"]
configs = ["BT"]

[features.remove.wifi]
roots = ["drivers/net/wireless"]

[features.preserve.netfilter]
roots = ["net/netfilter"]
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["plan", "--remove-feature", " bluetooth "]);

    assert!(
        ok,
        "remove-feature-selected plan failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains("  source: named_feature_remove_input"));
    assert!(stdout.contains("  remove paths: 2"));
    assert!(stdout.contains("  remove configs: 1"));
    assert!(stdout.contains("  preserve paths: 0"));
    assert!(stdout.contains("  preserve configs: 0"));

    let (ok, stdout, stderr) = kslim_in(
        &kslim_dir,
        &[
            "generate",
            "--reducer-report-only",
            "--remove-feature",
            "bluetooth",
        ],
    );

    assert!(
        ok,
        "remove-feature-selected report-only generate failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    let report = std::fs::read_to_string(project_failure_report_path(&kslim_dir)).unwrap();
    assert!(report.contains("features.remove.selected: cli (cli --remove-feature)"));

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["plan", "--remove-feature", "netfilter"]);

    assert!(!ok, "preserve feature should not satisfy --remove-feature");
    assert!(
        stderr.contains("remove feature 'netfilter' is not declared in features.remove"),
        "unexpected stderr: {}",
        stderr
    );
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
}

#[test]
fn test_preserve_feature_flag_selects_declared_preserve_feature_only() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "preserve-feature-flag", "1.0");
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
description = "Preserve feature flag test"

[base]
ref = "v1.0"

[features.remove.bluetooth]
roots = ["net/bluetooth"]

[features.preserve.netfilter]
roots = ["net/netfilter"]
configs = ["NETFILTER"]

[features.preserve.wifi]
roots = ["drivers/net/wireless"]
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["plan", "--preserve-feature", " netfilter "]);

    assert!(
        ok,
        "preserve-feature-selected plan failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains("  source: no_removal_input"));
    assert!(stdout.contains("  remove paths: 0"));
    assert!(stdout.contains("  remove configs: 0"));
    assert!(stdout.contains("  preserve paths: 1"));
    assert!(stdout.contains("  preserve configs: 1"));

    let (ok, stdout, stderr) = kslim_in(
        &kslim_dir,
        &[
            "generate",
            "--reducer-report-only",
            "--preserve-feature",
            "netfilter",
        ],
    );

    assert!(
        ok,
        "preserve-feature-selected report-only generate failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    let report = std::fs::read_to_string(project_failure_report_path(&kslim_dir)).unwrap();
    assert!(report.contains("features.preserve.selected: cli (cli --preserve-feature)"));

    let (ok, _, stderr) = kslim_in(&kslim_dir, &["plan", "--preserve-feature", "bluetooth"]);

    assert!(!ok, "remove feature should not satisfy --preserve-feature");
    assert!(
        stderr.contains("preserve feature 'bluetooth' is not declared in features.preserve"),
        "unexpected stderr: {}",
        stderr
    );
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
}

#[test]
fn test_arch_flag_filters_named_feature_arch_scope() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "arch-flag", "1.0");
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
description = "Arch flag test"

[base]
ref = "v1.0"

[features.remove.bluetooth]
roots = ["net/bluetooth"]
arch_scope = ["x86"]

[features.remove.wifi]
roots = ["drivers/net/wireless"]
arch_scope = ["arm64"]

[features.remove.usb]
roots = ["drivers/usb"]

[features.preserve.netfilter]
roots = ["net/netfilter"]
configs = ["NETFILTER"]
arch_scope = ["x86"]

[features.preserve.sound]
roots = ["sound"]
arch_scope = ["arm64"]
"#,
    )
    .unwrap();

    for (flag, value, preserve_configs) in [
        ("--arch", " x86 ", 1),
        ("--primary-arch", " arm64 ", 0),
        ("--secondary-arch", " arm64 ", 0),
    ] {
        let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["plan", flag, value]);
        assert!(
            ok,
            "{flag}-filtered plan failed: stdout={:?} stderr={:?}",
            stdout, stderr
        );
        for expected in [
            "  source: named_feature_remove_input",
            "  remove paths: 2",
            "  remove configs: 0",
            "  preserve paths: 1",
        ] {
            assert!(stdout.contains(expected), "missing {expected} in {stdout}");
        }
        assert!(stdout.contains(&format!("  preserve configs: {preserve_configs}")));
    }
    let (ok, stdout, stderr) = kslim_in(
        &kslim_dir,
        &["plan", "--primary-arch", "x86", "--secondary-arch", "arm64"],
    );
    assert!(
        ok,
        "primary+secondary plan failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains("  remove paths: 3"));
    assert!(stdout.contains("  preserve paths: 2"));
    assert!(stdout.contains("  preserve configs: 1"));

    for (flag, value, source) in [
        ("--arch", "x86", "arch.selected: cli (cli --arch)"),
        (
            "--primary-arch",
            "arm64",
            "arch.primary_arch: cli (cli --primary-arch)",
        ),
        (
            "--secondary-arch",
            "arm64",
            "arch.secondary_arches: cli (cli --secondary-arch)",
        ),
    ] {
        let (ok, stdout, stderr) = kslim_in(
            &kslim_dir,
            &["generate", "--reducer-report-only", flag, value],
        );
        assert!(
            ok,
            "{flag}-filtered report-only generate failed: stdout={:?} stderr={:?}",
            stdout, stderr
        );
        let report = std::fs::read_to_string(project_failure_report_path(&kslim_dir)).unwrap();
        assert!(report.contains(source), "missing {source} in {report}");
    }

    for (flag, message) in [
        ("--arch", "cli --arch is invalid"),
        ("--primary-arch", "cli --primary-arch is invalid"),
        ("--secondary-arch", "cli --secondary-arch is invalid"),
    ] {
        let (ok, _, stderr) = kslim_in(&kslim_dir, &["plan", flag, "x86/../../host"]);
        assert!(!ok, "invalid {flag} flag should be rejected");
        assert!(stderr.contains(message), "unexpected stderr: {}", stderr);
    }
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
}

#[test]
fn test_explain_feature_reports_named_intent_without_mutating_output() {
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
description = "Feature explain test"

[base]
ref = "v1.0"

[features.remove.bluetooth]
kind = "subsystem"
roots = ["net/bluetooth", "drivers/bluetooth"]
configs = ["BT"]
exported_symbols = ["bt_sock_register"]
module_names = ["btusb"]
module_aliases = ["usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*"]
device_compatibles = ["qcom,ipq8064"]
acpi_ids = ["PNP0C09"]
pci_ids = ["8086:1572"]
usb_ids = ["0BDA:8153"]
firmware_paths = ["amdgpu/polaris10_mc.bin"]
initcalls = ["bt_init"]
runtime_registrations = ["module_init:bt_init"]
docs = ["Documentation/networking/bluetooth.rst"]
tools = ["tools/perf"]
samples = ["samples/bpf"]
kunit_suites = ["bt_test"]
kselftest_targets = ["net"]
arch_scope = ["x86"]
safety = "surgical"
allow_public_header_removal = true
require_clean_boot = true
report_only = true
"#,
    )
    .unwrap();

    let (ok, stdout, stderr) = kslim_in(&kslim_dir, &["explain-feature", "bluetooth"]);

    assert!(
        ok,
        "explain-feature failed: stdout={:?} stderr={:?}",
        stdout, stderr
    );
    assert!(stdout.contains("explain-feature: bluetooth"));
    assert!(stdout.contains("profile: default"));
    assert!(stdout.contains("decision: removed"));
    assert!(stdout.contains("owner: profile features.remove.bluetooth"));
    assert!(stdout.contains("proof source: profile feature intent features.remove.bluetooth"));
    assert!(stdout.contains("kind: subsystem"));
    assert!(stdout.contains("roots: net/bluetooth, drivers/bluetooth"));
    assert!(stdout.contains("configs: BT"));
    assert!(stdout.contains("exported symbols: bt_sock_register"));
    assert!(stdout.contains("module names: btusb"));
    assert!(stdout.contains("module aliases: usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*"));
    assert!(stdout.contains("device compatibles: qcom,ipq8064"));
    assert!(stdout.contains("ACPI IDs: PNP0C09"));
    assert!(stdout.contains("PCI IDs: 8086:1572"));
    assert!(stdout.contains("USB IDs: 0BDA:8153"));
    assert!(stdout.contains("firmware paths: amdgpu/polaris10_mc.bin"));
    assert!(stdout.contains("initcalls: bt_init"));
    assert!(stdout.contains("runtime registrations: module_init:bt_init"));
    assert!(stdout.contains("docs: Documentation/networking/bluetooth.rst"));
    assert!(stdout.contains("tools: tools/perf"));
    assert!(stdout.contains("samples: samples/bpf"));
    assert!(stdout.contains("KUnit suites: bt_test"));
    assert!(stdout.contains("kselftest targets: net"));
    assert!(stdout.contains("arch scope: x86"));
    assert!(stdout.contains("safety: surgical"));
    assert!(stdout.contains("allow public headers: true"));
    assert!(stdout.contains("require clean boot: true"));
    assert!(stdout.contains("report only: true"));
    assert!(stdout.contains("effective impact:"));
    assert!(stdout.contains("  remove paths: 2"));
    assert!(stdout.contains("  remove configs: 1"));
    assert!(!output_dir.exists());
    assert!(!kslim_dir.join("kslim.lock").exists());
    assert!(!kslim_dir.join(".kslim").exists());
}
