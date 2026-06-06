use super::common::*;

#[test]
fn selftest_modules_execute_tests_without_reducer_policy() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let modules = [
        "src/selftest/mod.rs",
        "src/selftest/built_in.rs",
        "src/selftest/kernel_build.rs",
        "src/selftest/commands.rs",
    ];

    let mod_rs = production_source(&root.join("src/selftest/mod.rs"));
    assert!(
        mod_rs.contains("mod built_in;")
            && mod_rs.contains("mod kernel_build;")
            && mod_rs.contains("mod commands;"),
        "selftest should be split by execution responsibility"
    );

    let required_execution_entrypoints = [
        ("src/selftest/mod.rs", "pub fn run_capture"),
        (
            "src/selftest/built_in.rs",
            "pub(super) fn validate_kconfig_sources",
        ),
        (
            "src/selftest/built_in.rs",
            "pub(super) fn validate_makefiles",
        ),
        (
            "src/selftest/kernel_build.rs",
            "pub(super) fn run_kernel_build",
        ),
        ("src/selftest/commands.rs", "pub(super) fn run_command"),
    ];
    for (module, required) in required_execution_entrypoints {
        let production = production_source(&root.join(module));
        assert!(
            production.contains(required),
            "{module} should own test execution item {required}"
        );
    }

    let forbidden_reducer_policy = [
        "crate::reducer",
        "crate::fixups",
        "crate::removal_manifest",
        "crate::edit_reason",
        "crate::generate",
        "crate::output_repo",
        "ReducerStats",
        "AppliedFixup",
        "SkippedFixup",
        "FixupProof",
        "EditRecord",
        "EditReason",
        "RemovalManifest",
        "apply_selftest_fixup",
        "max_fixup_passes",
        "fail_on_unknown_diagnostics",
        "ensure_supported_fallout",
        "run_reducer",
        "ProfileConfig",
        "KslimConfig",
    ];

    for module in modules {
        let production = production_source(&root.join(module));
        for forbidden in forbidden_reducer_policy {
            assert!(
                !production.contains(forbidden),
                "{module} must execute tests without deciding reducer policy; found forbidden token {forbidden}"
            );
        }
    }
}
