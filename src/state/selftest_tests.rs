use super::*;
use crate::config::{self, KernelBuildConfig};
use std::collections::BTreeMap;
use std::path::Path;

fn test_resolved_base() -> ResolvedBase {
    ResolvedBase {
        upstream: String::from("linux"),
        url: String::from("/tmp/linux.git"),
        r#ref: String::from("v1.0"),
        commit: String::from("deadbeef"),
        resolved_at: String::from("2026-01-01T00:00:00Z"),
    }
}

fn default_cli_overrides() -> CliOverrides {
    CliOverrides {
        dry_run: false,
        deep_dry_run: false,
        report_only: false,
        force: false,
        offline: false,
        base_ref: None,
        feature: None,
        remove_feature: None,
        preserve_feature: None,
        arch: None,
        primary_arch: None,
        secondary_arch: None,
        safety: None,
        max_fixup_passes: None,
        matrix: None,
        strict: false,
        no_strict: false,
        run_selftests: true,
    }
}

#[test]
fn test_resolved_candidate_state_captures_full_selftest_matrix() {
    let config = config::default_kslim_config("demo", "/tmp/output");
    let mut profile = config::default_profile_config("v1.0");
    profile.selftests.enabled = false;
    profile.selftests.check_kconfig_sources = false;
    profile.selftests.check_makefiles = true;
    profile.selftests.commands = vec![
        String::from("make -C tools/testing/selftests TARGETS=net run_tests"),
        String::from("scripts/kunit.py run"),
    ];
    profile.selftests.kernel_builds = vec![KernelBuildConfig {
        name: Some(String::from("x86-defconfig")),
        config_target: Some(String::from("defconfig")),
        targets: vec![String::from("vmlinux"), String::from("modules")],
        output_dir: Some(String::from("build/x86")),
        jobs: Some(8),
        clean: false,
        make_program: Some(String::from("gmake")),
        make_args: vec![String::from("LLVM=1"), String::from("W=1")],
        env: BTreeMap::from([
            (String::from("ARCH"), String::from("x86")),
            (
                String::from("CROSS_COMPILE"),
                String::from("x86_64-linux-gnu-"),
            ),
        ]),
    }];

    let resolved = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap();

    assert!(!resolved.selftest_plan.enabled);
    assert!(!resolved.selftest_plan.check_kconfig_sources);
    assert!(resolved.selftest_plan.check_makefiles);
    assert_eq!(
        resolved.selftest_plan.commands,
        [
            "make -C tools/testing/selftests TARGETS=net run_tests",
            "scripts/kunit.py run"
        ]
    );
    let build = &resolved.selftest_plan.kernel_builds[0];
    assert_eq!(build.name.as_deref(), Some("x86-defconfig"));
    assert_eq!(build.config_target.as_deref(), Some("defconfig"));
    assert_eq!(build.targets, ["vmlinux", "modules"]);
    assert_eq!(
        build.output_dir.as_ref().map(|dir| dir.as_path()),
        Some(Path::new("build/x86"))
    );
    assert_eq!(build.jobs, Some(8));
    assert!(!build.clean);
    assert_eq!(build.make_program.as_deref(), Some("gmake"));
    assert_eq!(build.make_args, ["LLVM=1", "W=1"]);
    assert_eq!(build.arch.as_ref().map(|arch| arch.as_str()), Some("x86"));
    assert_eq!(
        build.env.get("CROSS_COMPILE").map(String::as_str),
        Some("x86_64-linux-gnu-")
    );
}

#[test]
fn test_resolved_candidate_state_captures_cli_selftest_matrix_override() {
    let config = config::default_kslim_config("demo", "/tmp/output");
    let mut profile = config::default_profile_config("v1.0");
    profile.selftests.enabled = false;
    profile.selftests.check_kconfig_sources = true;
    profile.selftests.check_makefiles = true;
    profile.selftests.commands = vec![String::from("true")];
    profile.selftests.kernel_builds = vec![KernelBuildConfig {
        name: Some(String::from("must-not-run")),
        config_target: Some(String::from("defconfig")),
        targets: Vec::new(),
        output_dir: None,
        jobs: None,
        clean: true,
        make_program: None,
        make_args: Vec::new(),
        env: BTreeMap::new(),
    }];
    let mut cli = default_cli_overrides();
    cli.matrix = Some(String::from("runtime"));
    let profile = cli.apply_profile_overrides(profile).unwrap();

    let resolved = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap();

    assert!(resolved.selftest_plan.enabled);
    assert!(!resolved.selftest_plan.check_kconfig_sources);
    assert!(!resolved.selftest_plan.check_makefiles);
    assert!(resolved.selftest_plan.kernel_builds.is_empty());
    assert_eq!(resolved.selftest_plan.commands, ["true"]);
}
