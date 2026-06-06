use super::*;
#[test]
fn test_profile_parse_allows_explicit_reducer_overrides() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[reducer]
max_fixup_passes = 1
report_unsupported_expressions = false
fail_on_unknown_diagnostics = false
reject_unproven_fixups = false
reject_unreasoned_edits = false
reject_speculative_fallout_edits = false
fail_on_missing_prune_paths = true
ignore_unsupported_special_removals = true
"#,
    )
    .unwrap();

    assert_eq!(profile.reducer.max_fixup_passes, 1);
    assert!(!profile.reducer.report_unsupported_expressions);
    assert!(!profile.reducer.fail_on_unknown_diagnostics);
    assert!(!profile.reducer.reject_unproven_fixups);
    assert!(!profile.reducer.reject_unreasoned_edits);
    assert!(!profile.reducer.reject_speculative_fallout_edits);
    assert!(!profile.reducer.strict_mode());
    assert!(profile.reducer.fail_on_missing_prune_paths);
    assert!(profile.reducer.ignore_unsupported_special_removals);
}

#[test]
fn test_profile_removal_input_reads_from_slim_section() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/gpu/drm/amd/amdgpu"]
remove_configs = ["DRM_AMDGPU", "DRM_AMDGPU_SI"]
set_defaults = { DRM_AMDGPU_WERROR = "n" }
"#,
    )
    .unwrap();

    let slim = profile.removal_input().unwrap();
    assert_eq!(slim.remove_paths, vec!["drivers/gpu/drm/amd/amdgpu"]);
    assert_eq!(slim.remove_configs, vec!["DRM_AMDGPU", "DRM_AMDGPU_SI"]);
    assert_eq!(
        slim.set_defaults.get("DRM_AMDGPU_WERROR"),
        Some(&String::from("n"))
    );
}

#[test]
fn test_default_profile_keeps_removal_input_under_slim_only() {
    let profile = default_profile_config("v1.0");
    let toml = toml::to_string_pretty(&profile).unwrap();

    assert!(!toml.contains("[reducer]\nremove_"));
    assert!(!toml.contains("[reducer]\nremove_paths"));
    assert!(!toml.contains("[reducer]\nremove_configs"));
    assert!(profile.removal_input().is_none());
}

#[test]
fn test_amdgpu_template_uses_slim_for_user_facing_removal_input() {
    let template = amdgpu_prune_profile_template("v1.0");

    assert!(template.contains("[slim]"));
    assert!(template.contains("remove_paths = [\"drivers/gpu/drm/amd/amdgpu\"]"));
    assert!(template.contains("remove_configs = [\"DRM_AMDGPU\", \"DRM_AMDGPU_SI\"]"));
}

#[test]
fn test_validate_profile_rejects_custom_reducer_without_removal_input() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[reducer]
fail_on_unknown_diagnostics = false
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();
    assert!(err.contains("reducer settings may only be customized"));
}

#[test]
fn test_validate_profile_rejects_custom_reducer_with_noop_slim() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[slim]

[reducer]
reject_unproven_fixups = false
"#,
    )
    .unwrap();

    let err = validate_profile(&profile).unwrap_err().to_string();
    assert!(err.contains("reducer settings may only be customized"));
}

#[test]
fn test_validate_profile_allows_custom_reducer_with_real_slim_input() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/gpu/drm/amd/amdgpu"]

[reducer]
reject_unproven_fixups = false
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
}

#[test]
fn test_validate_profile_rejects_parent_dir_removal_path() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/../net"]
"#,
    )
    .unwrap();

    let err = format!("{:#}", validate_profile(&profile).unwrap_err());
    assert!(err.contains("must not contain '..'"));
}

#[test]
fn test_validate_profile_rejects_root_removal_without_unsafe_mode() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[slim]
remove_paths = ["."]
"#,
    )
    .unwrap();

    let err = format!("{:#}", validate_profile(&profile).unwrap_err());
    assert!(err.contains("tree root"));
    assert!(err.contains("slim.unsafe_allow_root_path_removal"));
}

#[test]
fn test_validate_profile_allows_root_removal_with_unsafe_mode() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[slim]
remove_paths = ["."]
unsafe_allow_root_path_removal = true
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
}

#[test]
fn test_validate_profile_rejects_public_header_removal_without_abi_policy() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[slim]
remove_paths = ["include/linux/public.h"]
"#,
    )
    .unwrap();

    let err = format!("{:#}", validate_profile(&profile).unwrap_err());
    assert!(err.contains("explicit ABI policy approval"));
    assert!(err.contains("abi.allow_public_header_removal"));
}

#[test]
fn test_validate_profile_allows_public_header_removal_with_abi_policy() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[slim]
remove_paths = ["include/linux/public.h"]

[abi]
allow_public_header_removal = true
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
}

#[test]
fn test_validate_profile_rejects_uapi_removal_without_uapi_abi_policy() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[slim]
remove_paths = ["include/uapi/linux/abi.h"]

[abi]
allow_public_header_removal = true
"#,
    )
    .unwrap();

    let err = format!("{:#}", validate_profile(&profile).unwrap_err());
    assert!(err.contains("UAPI removal requires explicit ABI policy approval"));
    assert!(err.contains("abi.allow_uapi_header_removal"));
}

#[test]
fn test_validate_profile_allows_uapi_removal_with_uapi_abi_policy() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[slim]
remove_paths = ["include/uapi/linux/abi.h"]

[abi]
allow_uapi_header_removal = true
"#,
    )
    .unwrap();

    validate_profile(&profile).unwrap();
}

#[test]
fn test_validate_profile_rejects_invalid_kernel_build_arch_env() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[[selftests.kernel_builds]]
name = "bad-arch"
config_target = "defconfig"
env = { ARCH = "x86/../../host" }
"#,
    )
    .unwrap();
    let err = format!("{:#}", validate_profile(&profile).unwrap_err());
    assert!(err.contains("ARCH env is invalid") && err.contains("invalid characters"));
}
