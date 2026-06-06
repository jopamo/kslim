use super::*;
use crate::config::validate_profile;

fn assert_duplicate_feature_definition_rejected(contents: &str) {
    let err = toml::from_str::<ProfileConfig>(contents)
        .expect_err("duplicate feature definitions must be rejected before validation")
        .to_string();
    let normalized = err.to_ascii_lowercase();
    assert!(
        normalized.contains("duplicate") || normalized.contains("redefinition"),
        "expected duplicate feature definition parse error, got: {err}"
    );
}

#[test]
fn feature_remove_explicit_paths_resolve_into_effective_removal_input() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
kind = "subsystem"
remove_paths = ["net/rfkill", "drivers/bluetooth"]
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
    let removal_input = profile.effective_removal_input().unwrap();

    assert_eq!(
        removal_input.remove_paths,
        vec!["net/rfkill", "drivers/bluetooth"]
    );
    assert!(removal_input.remove_configs.is_empty());
}

#[test]
fn duplicate_feature_remove_definition_is_rejected() {
    assert_duplicate_feature_definition_rejected(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
roots = ["net/bluetooth"]

[features.remove.bluetooth]
configs = ["BT"]
"#,
    );
}

#[test]
fn duplicate_feature_preserve_definition_is_rejected() {
    assert_duplicate_feature_definition_rejected(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve]
bluetooth = { roots = ["net/bluetooth"] }

[features.preserve.bluetooth]
configs = ["BT"]
"#,
    );
}

#[test]
fn profile_inheritance_intent_is_parsed_and_rejected_until_supported() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "child"
inherits = "base"

[base]
ref = "v1.0"
"#,
    )
    .unwrap();

    assert_eq!(profile.profile.inherits.as_deref(), Some("base"));

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("profile.inherits"));
    assert!(err.contains("not yet supported"));
}

#[test]
fn profile_inheritance_parent_name_must_not_be_empty() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "child"
inherits = " "

[base]
ref = "v1.0"
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("profile.inherits must not be empty"));
}

#[test]
fn feature_preserve_rejects_explicit_remove_paths() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
roots = ["net/bluetooth"]
remove_paths = ["drivers/bluetooth"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_paths"));
}

#[test]
fn feature_remove_explicit_paths_validate_through_manifest() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
kind = "subsystem"
remove_paths = ["drivers/../net"]
"#,
    )
    .unwrap();

    let err = format!("{:#}", validate_profile(&profile).unwrap_err());

    assert!(err.contains("must not contain '..'"));
}

#[test]
fn feature_remove_explicit_configs_resolve_into_effective_removal_input() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
kind = "subsystem"
remove_configs = ["BT", "BT_HCIBTUSB"]
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
    let removal_input = profile.effective_removal_input().unwrap();

    assert!(removal_input.remove_paths.is_empty());
    assert_eq!(removal_input.remove_configs, vec!["BT", "BT_HCIBTUSB"]);
}

#[test]
fn feature_preserve_rejects_explicit_remove_configs() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
configs = ["BT"]
remove_configs = ["BT_HCIBTUSB"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_configs"));
}

#[test]
fn feature_preserve_rejects_explicit_remove_exported_symbols() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
exported_symbols = ["bt_sock_register"]
remove_exported_symbols = ["bt_debugfs_init"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_exported_symbols"));
}

#[test]
fn feature_preserve_rejects_explicit_remove_module_names() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
module_names = ["btusb"]
remove_module_names = ["bt_debug"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_module_names"));
}

#[test]
fn feature_preserve_rejects_explicit_remove_module_aliases() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
module_aliases = ["usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*"]
remove_module_aliases = ["pci:v00008086d00001572sv*sd*bc*sc*i*"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_module_aliases"));
}

#[test]
fn feature_preserve_rejects_explicit_remove_device_compatibles() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
device_compatibles = ["qcom,ipq8064"]
remove_device_compatibles = ["vendor,removed-device"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_device_compatibles"));
}

#[test]
fn feature_preserve_rejects_explicit_remove_acpi_ids() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
acpi_ids = ["PNP0C09"]
remove_acpi_ids = ["ACPI0003"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_acpi_ids"));
}

#[test]
fn feature_preserve_rejects_explicit_remove_pci_ids() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
pci_ids = ["8086:1572"]
remove_pci_ids = ["10EC:8168"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_pci_ids"));
}

#[test]
fn feature_preserve_rejects_explicit_remove_usb_ids() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
usb_ids = ["0BDA:8153"]
remove_usb_ids = ["046D:C52B"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_usb_ids"));
}

#[test]
fn feature_preserve_rejects_explicit_remove_firmware_paths() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
firmware_paths = ["amdgpu/polaris10_mc.bin"]
remove_firmware_paths = ["iwlwifi-7260-17.ucode"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_firmware_paths"));
}

#[test]
fn feature_preserve_rejects_explicit_remove_initcalls() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.bluetooth]
kind = "subsystem"
initcalls = ["bt_init"]
remove_initcalls = ["btusb_driver_init"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.bluetooth.remove_initcalls"));
}

#[test]
fn feature_remove_explicit_configs_validate_through_manifest() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
kind = "subsystem"
remove_configs = ["BT DEBUG"]
"#,
    )
    .unwrap();

    let err = format!("{:#}", validate_profile(&profile).unwrap_err());

    assert!(err.contains("invalid Kconfig symbol"));
}

#[test]
fn feature_remove_explicit_abi_approval_allows_public_header_removal() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.driver_headers]
kind = "driver"
remove_paths = ["include/linux/driver_abi.h"]
allow_public_header_removal = true
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
    let policy = profile.effective_abi_policy();

    assert!(policy.allow_public_header_removal);
    assert!(!policy.allow_uapi_header_removal);
}

#[test]
fn feature_remove_explicit_uapi_approval_allows_uapi_removal() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.driver_uapi]
kind = "driver"
remove_paths = ["include/uapi/linux/driver_abi.h"]
allow_uapi_header_removal = true
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
    let policy = profile.effective_abi_policy();

    assert!(!policy.allow_public_header_removal);
    assert!(policy.allow_uapi_header_removal);
}

#[test]
fn feature_remove_rejects_abi_sensitive_path_without_explicit_approval() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.driver_uapi]
kind = "driver"
remove_paths = ["include/uapi/linux/driver_abi.h"]
"#,
    )
    .unwrap();

    let err = format!("{:#}", validate_profile(&profile).unwrap_err());

    assert!(err.contains("abi.allow_uapi_header_removal"));
}

#[test]
fn feature_preserve_rejects_abi_removal_approval() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.driver_uapi]
kind = "driver"
configs = ["DRIVER_UAPI"]
allow_uapi_header_removal = true
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.driver_uapi.allow_uapi_header_removal"));
}

#[test]
fn feature_remove_accepts_and_normalizes_safety_level() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
kind = "subsystem"
roots = ["net/bluetooth"]
safety = "surgical"
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
    let safety = profile.effective_feature_safety_levels();

    assert_eq!(safety.get("bluetooth"), Some(&FeatureSafetyLevel::Surgical));
}

#[test]
fn feature_remove_defaults_safety_level_to_normal() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
kind = "subsystem"
roots = ["net/bluetooth"]
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
    let safety = profile.effective_feature_safety_levels();

    assert_eq!(safety.get("bluetooth"), Some(&FeatureSafetyLevel::Normal));
}

#[test]
fn feature_preserve_rejects_safety_level() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.netfilter]
kind = "subsystem"
roots = ["net/netfilter"]
safety = "conservative"
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.preserve.netfilter.safety"));
}

#[test]
fn feature_safety_level_rejects_unknown_value() {
    let err = toml::from_str::<ProfileConfig>(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
kind = "subsystem"
roots = ["net/bluetooth"]
safety = "reckless"
"#,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("reckless"));
}

#[test]
fn feature_remove_accepts_arch_scope() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
kind = "subsystem"
roots = ["net/bluetooth"]
arch_scope = ["x86", "arm64"]
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
    let scopes = profile.effective_feature_arch_scopes();

    assert_eq!(
        scopes.get("bluetooth"),
        Some(&vec![String::from("x86"), String::from("arm64")])
    );
}

#[test]
fn feature_preserve_accepts_arch_scope() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.netfilter]
kind = "subsystem"
roots = ["net/netfilter"]
arch_scope = ["x86"]
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
    let scopes = profile.effective_feature_arch_scopes();

    assert_eq!(scopes.get("netfilter"), Some(&vec![String::from("x86")]));
}

#[test]
fn feature_remove_accepts_test_matrix() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
kind = "subsystem"
roots = ["net/bluetooth"]
require_clean_boot = true
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
    let matrices = profile.effective_feature_test_matrices();

    assert_eq!(
        matrices.get("bluetooth"),
        Some(&FeatureTestMatrixConfig {
            require_clean_boot: true
        })
    );
}

#[test]
fn feature_preserve_accepts_test_matrix() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.netfilter]
kind = "subsystem"
roots = ["net/netfilter"]
require_clean_boot = true
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
    let matrices = profile.effective_feature_test_matrices();

    assert_eq!(
        matrices.get("netfilter"),
        Some(&FeatureTestMatrixConfig {
            require_clean_boot: true
        })
    );
}

#[test]
fn feature_remove_accepts_report_only_mode() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
kind = "subsystem"
roots = ["net/bluetooth"]
report_only = true
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
    let modes = profile.effective_feature_report_modes();

    assert_eq!(
        modes.get("bluetooth"),
        Some(&FeatureReportModeConfig { report_only: true })
    );
}

#[test]
fn feature_preserve_accepts_report_only_mode() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.preserve.netfilter]
kind = "subsystem"
roots = ["net/netfilter"]
report_only = true
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
    let modes = profile.effective_feature_report_modes();

    assert_eq!(
        modes.get("netfilter"),
        Some(&FeatureReportModeConfig { report_only: true })
    );
}

#[test]
fn feature_arch_scope_rejects_invalid_arch() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
kind = "subsystem"
roots = ["net/bluetooth"]
arch_scope = ["x86/../../host"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.remove.bluetooth.arch_scope is invalid"));
}

#[test]
fn feature_arch_scope_rejects_duplicate_arch() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
kind = "subsystem"
roots = ["net/bluetooth"]
arch_scope = ["x86", "x86"]
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();

    assert!(err.contains("features.remove.bluetooth.arch_scope"));
    assert!(err.contains("duplicate architecture"));
}
