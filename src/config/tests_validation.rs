use super::*;

#[test]
fn test_validate_config_rejects_invalid_output_config() {
    let mut invalid_path = default_kslim_config("demo", "/srv/output");
    invalid_path.output.path = "../output".to_string();
    let err = format!("{:#}", validate_config(&invalid_path).unwrap_err());
    assert!(err.contains("output.path is invalid"));
    assert!(err.contains("parent components"));

    let mut empty_prefix = default_kslim_config("demo", "/srv/output");
    empty_prefix.output.branch_prefix = " ".to_string();
    let err = validate_config(&empty_prefix).unwrap_err().to_string();
    assert!(err.contains("output.branch_prefix must not be empty"));

    let mut empty_branch = default_kslim_config("demo", "/srv/output");
    empty_branch.output.branch = Some(" ".to_string());
    let err = validate_config(&empty_branch).unwrap_err().to_string();
    assert!(err.contains("output.branch must not be empty when specified"));

    let mut invalid_branch = default_kslim_config("demo", "/srv/output");
    invalid_branch.output.branch = Some("/release".to_string());
    let err = validate_config(&invalid_branch).unwrap_err().to_string();
    assert!(err.contains("output.branch must not contain empty branch path components"));
}
#[test]
fn test_slim_config_validation_rejects_invalid_user_intent() {
    let invalid_path: SlimConfig = toml::from_str(
        r#"
remove_paths = ["drivers/../net"]
"#,
    )
    .unwrap();
    let err = format!(
        "{:#}",
        crate::removal_manifest::RemovalManifest::from_slim_config_with_abi_policy(
            &invalid_path,
            &AbiPolicyConfig::default(),
        )
        .unwrap_err()
    );
    assert!(err.contains("invalid slim.remove_paths[0]"));
    assert!(err.contains("must not contain '..'"));

    let invalid_symbol: SlimConfig = toml::from_str(
        r#"
remove_configs = ["BT DEBUG"]
"#,
    )
    .unwrap();
    let err = format!(
        "{:#}",
        crate::removal_manifest::RemovalManifest::from_slim_config_with_abi_policy(
            &invalid_symbol,
            &AbiPolicyConfig::default(),
        )
        .unwrap_err()
    );
    assert!(err.contains("invalid Kconfig symbol"));

    let conflicting_default: SlimConfig = toml::from_str(
        r#"
remove_configs = ["BT"]
set_defaults = { BT = "n" }
"#,
    )
    .unwrap();
    let err = crate::removal_manifest::RemovalManifest::from_slim_config_with_abi_policy(
        &conflicting_default,
        &AbiPolicyConfig::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("both target 'BT'"));
}
#[test]
fn test_validate_profile_allows_named_feature_remove_intent() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
kind = "subsystem"
roots = ["net/bluetooth", "drivers/bluetooth"]
configs = ["BT"]
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
    let removal_input = profile.effective_removal_input().unwrap();

    assert_eq!(
        removal_input.remove_paths,
        vec!["net/bluetooth", "drivers/bluetooth"]
    );
    assert_eq!(removal_input.remove_configs, vec!["BT"]);
    assert!(!removal_input.unsafe_allow_root_path_removal);
}
#[test]
fn test_validate_profile_rejects_preserve_remove_runtime_registrations() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
runtime_registrations = ["module_init:bt_init"]
remove_runtime_registrations = ["module_platform_driver:btusb_driver"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_runtime_registrations"));
}
#[test]
fn test_validate_profile_rejects_preserve_remove_docs() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
docs = ["Documentation/networking/bluetooth.rst"]
remove_docs = ["Documentation/driver-api/btusb.rst"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_docs"));
}
#[test]
fn test_validate_profile_rejects_preserve_remove_tools() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
tools = ["tools/perf"]
remove_tools = ["tools/objtool"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_tools"));
}
#[test]
fn test_validate_profile_rejects_preserve_remove_samples() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
samples = ["samples/bpf"]
remove_samples = ["samples/hidraw"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_samples"));
}
#[test]
fn test_validate_profile_rejects_preserve_remove_kunit_suites() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
kunit_suites = ["bt_test"]
remove_kunit_suites = ["btusb-test"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_kunit_suites"));
}
#[test]
fn test_validate_profile_rejects_preserve_remove_kselftest_targets() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
kselftest_targets = ["net"]
remove_kselftest_targets = ["bpf"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_kselftest_targets"));
}
#[test]
fn test_validate_profile_rejects_conflicting_feature_remove_and_preserve() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
kind = "subsystem"

[features.preserve.bluetooth]
kind = "subsystem"
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("feature 'bluetooth' cannot be declared in both"));
}
#[test]
fn test_validate_profile_rejects_nondefault_arch_policy_until_supported() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[arch]
primary_arch = "x86"
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("arch policy config is parsed but not yet supported"));
    assert!(err.contains("[[selftests.kernel_builds]].env ARCH"));
}
#[test]
fn test_validate_profile_rejects_invalid_arch_policy_value() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[arch]
primary_arch = "x86/../../host"
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("arch.primary_arch is invalid"));
    assert!(err.contains("kernel architecture name contains invalid characters"));
}
#[test]
fn test_validate_profile_rejects_conflicting_arch_policy_scopes() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[arch]
secondary_arches = ["arm64"]
disabled_arches = ["arm64"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains(
        "architecture 'arm64' cannot be declared in both arch.secondary_arches and arch.disabled_arches"
    ));
}
#[test]
fn test_validate_profile_rejects_arch_local_removal_without_primary_arch() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[arch]
allow_arch_local_removal = true
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("arch.allow_arch_local_removal requires arch.primary_arch"));
}
#[test]
fn test_validate_profile_rejects_nondefault_build_matrix_until_supported() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[build_matrix]
enabled = true
presets = ["default"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("build matrix config is parsed but not yet supported"));
    assert!(err.contains("[[selftests.kernel_builds]]"));
}
#[test]
fn test_validate_profile_rejects_invalid_build_matrix_arch() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[build_matrix]
arches = ["x86/../../host"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("build_matrix.arches is invalid"));
    assert!(err.contains("kernel architecture name contains invalid characters"));
}
#[test]
fn test_validate_profile_rejects_unsupported_build_matrix_preset() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[build_matrix]
presets = ["runtime"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("build_matrix.presets contains unsupported preset 'runtime'"));
}
#[test]
fn test_validate_profile_rejects_zero_build_matrix_jobs() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[build_matrix]
jobs = 0
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("build_matrix.jobs must be greater than zero"));
}
#[test]
fn test_validate_profile_rejects_nondefault_runtime_matrix_until_supported() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[runtime_matrix]
enabled = true
boot_arches = ["x86"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("runtime matrix config is parsed but not yet supported"));
    assert!(err.contains("[selftests].commands"));
}
#[test]
fn test_validate_profile_rejects_invalid_runtime_matrix_arch() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[runtime_matrix]
boot_arches = ["x86/../../host"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("runtime_matrix.boot_arches is invalid"));
    assert!(err.contains("kernel architecture name contains invalid characters"));
}
#[test]
fn test_validate_profile_rejects_empty_runtime_matrix_kselftest_target() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[runtime_matrix]
kselftest_targets = [""]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("runtime_matrix.kselftest_targets must not contain empty values"));
}
#[test]
fn test_validate_profile_rejects_zero_runtime_matrix_boot_timeout() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[runtime_matrix]
boot_timeout_seconds = 0
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("runtime_matrix.boot_timeout_seconds must be greater than zero"));
}
#[test]
fn test_validate_profile_rejects_nondefault_report_config_until_supported() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[reports]
formats = ["json"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("report config is parsed but not yet supported"));
    assert!(err.contains("report planning lands"));
}
#[test]
fn test_validate_profile_rejects_unsupported_report_format() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[reports]
formats = ["xml"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("reports.formats contains unsupported format 'xml'"));
}
#[test]
fn test_validate_profile_rejects_duplicate_report_format() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[reports]
formats = ["json", "json"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("reports.formats must not contain duplicate value 'json'"));
}
#[test]
fn test_validate_profile_rejects_raw_logs_in_committed_reports() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[reports]
include_raw_logs = true
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("reports.include_raw_logs is not supported"));
    assert!(err.contains("raw logs must remain attempt metadata or CI artifacts"));
}
#[test]
fn test_validate_profile_rejects_nondefault_security_config_until_supported() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[security]
compatibility_mode = "legacy"
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("security config is parsed but not yet supported"));
    assert!(err.contains("security planning lands"));
}
#[test]
fn test_validate_profile_rejects_security_network_downgrade() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[security]
allow_network = true
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("security.allow_network is not supported"));
    assert!(err.contains("local read-only upstream inputs"));
}
#[test]
fn test_validate_profile_rejects_security_raw_log_downgrade() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[security]
reject_raw_logs_in_committed_metadata = false
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("security.reject_raw_logs_in_committed_metadata cannot be disabled"));
    assert!(err.contains("raw logs must remain attempt metadata or CI artifacts"));
}
#[test]
fn test_validate_profile_rejects_empty_security_compatibility_mode() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[security]
compatibility_mode = ""
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("security.compatibility_mode must not be empty"));
}
#[test]
fn test_validate_profile_rejects_unsupported_performance_config() {
    let enabled: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[performance]
enabled = true
"#,
    )
    .unwrap();
    let err = validate_profile(&enabled).unwrap_err().to_string();
    assert!(err.contains("performance config is parsed but not yet supported"));
    assert!(err.contains("performance planning lands"));

    let zero_threads: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[performance]
max_worker_threads = 0
"#,
    )
    .unwrap();
    let err = validate_profile(&zero_threads).unwrap_err().to_string();
    assert!(err.contains("performance.max_worker_threads must be greater than zero"));

    let relaxed: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[performance]
fail_on_regression = false
"#,
    )
    .unwrap();
    let err = validate_profile(&relaxed).unwrap_err().to_string();
    assert!(err.contains("performance.fail_on_regression cannot be disabled"));
}
