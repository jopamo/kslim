use super::*;

#[test]
fn test_kslim_config_parse_defaults_project_root_fields() {
    let config: KslimConfig = toml::from_str(
        r#"
[project]
name = "demo"

[upstream]
name = "linux"
url = "/tmp/linux.git"

[output]
path = "/tmp/output"
"#,
    )
    .unwrap();

    assert_eq!(config.project.name, "demo");
    assert_eq!(config.upstream.name, "linux");
    assert_eq!(config.upstream.url, "/tmp/linux.git");
    assert_eq!(config.upstream.mode, None);
    assert_eq!(config.upstream.cache, None);
    assert_eq!(config.output.path, "/tmp/output");
    assert_eq!(config.output.branch_prefix, "kslim");
    assert_eq!(config.output.branch, None);
    assert_eq!(config.git.user_email, "kslim@localhost");
    assert_eq!(config.git.user_name, "kslim");
    assert_eq!(config.git.remote_name, "origin");
    assert!(config.publish.is_none());
    validate_config(&config).unwrap();
}
#[test]
fn test_kslim_config_parse_optional_git_output_and_publish_fields() {
    let config: KslimConfig = toml::from_str(
        r#"
[project]
name = "demo"

[upstream]
name = "linux-stable"
url = "/srv/linux.git"
mode = "direct"

[output]
path = "/srv/output"
branch_prefix = "reduced"
branch = "reduced/demo"

[git]
user_email = "builder@example.test"
user_name = "Builder"
remote_name = "published"

[publish]
remote = "mirror"
"#,
    )
    .unwrap();

    assert_eq!(config.project.name, "demo");
    assert_eq!(config.upstream.name, "linux-stable");
    assert_eq!(config.upstream.url, "/srv/linux.git");
    assert_eq!(config.upstream.mode.as_deref(), Some("direct"));
    assert_eq!(config.upstream.cache, None);
    assert_eq!(config.output.path, "/srv/output");
    assert_eq!(config.output.branch_prefix, "reduced");
    assert_eq!(config.output.branch.as_deref(), Some("reduced/demo"));
    assert_eq!(config.git.user_email, "builder@example.test");
    assert_eq!(config.git.user_name, "Builder");
    assert_eq!(config.git.remote_name, "published");
    assert_eq!(config.publish.as_ref().unwrap().remote, "mirror");
    validate_config(&config).unwrap();
}
#[test]
fn test_output_config_defaults_and_explicit_branch_intent() {
    let defaulted: OutputConfig = toml::from_str(
        r#"
path = "/srv/output"
"#,
    )
    .unwrap();

    assert_eq!(defaulted.path, "/srv/output");
    assert_eq!(defaulted.branch_prefix, "kslim");
    assert!(defaulted.branch.is_none());
    assert!(!defaulted.has_explicit_branch());

    let explicit: OutputConfig = toml::from_str(
        r#"
path = "/srv/output"
branch_prefix = "reduced"
branch = "release/linux-min"
"#,
    )
    .unwrap();

    assert_eq!(explicit.path, "/srv/output");
    assert_eq!(explicit.branch_prefix, "reduced");
    assert_eq!(explicit.branch.as_deref(), Some("release/linux-min"));
    assert!(explicit.has_explicit_branch());
}
#[test]
fn test_default_kslim_config_constructs_valid_project_root_model() {
    let config = default_kslim_config("demo", "/srv/output");

    assert_eq!(config.project.name, "demo");
    assert_eq!(config.upstream.name, "linux");
    assert_eq!(config.upstream.url, "/path/to/linux/.git");
    assert_eq!(config.output.path, "/srv/output");
    assert_eq!(config.output.branch_prefix, "kslim");
    assert_eq!(config.output.branch, None);
    assert!(config.publish.is_none());
    validate_config(&config).unwrap();
}
#[test]
fn test_profile_config_parse_defaults_profile_file_fields() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"
"#,
    )
    .unwrap();

    assert_eq!(profile.profile.name, "default");
    assert_eq!(profile.profile.description, "");
    assert_eq!(profile.base.r#ref, "v1.0");
    assert!(profile.slim.is_none());
    assert!(profile.features.is_empty());
    assert!(!profile.abi.allow_public_header_removal);
    assert!(!profile.abi.allow_uapi_header_removal);
    assert!(profile.arch.is_default());
    assert!(profile.build_matrix.is_default());
    assert!(profile.runtime_matrix.is_default());
    assert!(profile.reports.is_default());
    assert!(profile.security.is_default());
    assert!(profile.performance.is_default());
    assert!(profile.patches.is_none());
    assert!(profile.integrations.rtlmq.is_none());
    assert_eq!(profile.reducer, ReducerConfig::default());
    assert!(profile.selftests.enabled);
    assert!(profile.selftests.check_kconfig_sources);
    assert!(profile.selftests.check_makefiles);
    assert!(profile.selftests.kernel_builds.is_empty());
    assert!(profile.selftests.commands.is_empty());
    assert!(profile.removal_input().is_none());
    validate_profile(&profile).unwrap();
}
#[test]
fn test_profile_config_parse_optional_user_intent_sections() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "amdgpu-prune"
description = "Remove AMDGPU with build proof"

[base]
ref = "v6.15"

[slim]
remove_paths = ["drivers/gpu/drm/amd/amdgpu"]
remove_configs = ["DRM_AMDGPU", "DRM_AMDGPU_SI"]
set_defaults = { DRM_AMDGPU_WERROR = "n" }

[abi]
allow_public_header_removal = true
allow_uapi_header_removal = true

[patches]
source = "worktree"
path = "/tmp/linux-worktree"
base_remote = "upstream"
base_ref = "master"
require_clean = false

[integrations.rtlmq]
source = "/tmp/rtlmq"
tests_source = "/tmp/rtlmq-tests"

[reducer]
max_fixup_passes = 1
reject_unproven_fixups = false

[selftests]
enabled = true
check_kconfig_sources = true
check_makefiles = false
commands = ["make headers_check"]

[[selftests.kernel_builds]]
name = "x86-defconfig"
config_target = "defconfig"
targets = ["vmlinux"]
output_dir = ".kslim-selftest/x86"
jobs = 2
clean = false
make_program = "make"
make_args = ["LLVM=1"]
env = { ARCH = "x86" }
"#,
    )
    .unwrap();

    assert_eq!(profile.profile.name, "amdgpu-prune");
    assert_eq!(
        profile.profile.description,
        "Remove AMDGPU with build proof"
    );
    assert_eq!(profile.base.r#ref, "v6.15");

    let slim = profile.removal_input().unwrap();
    assert_eq!(slim.remove_paths, vec!["drivers/gpu/drm/amd/amdgpu"]);
    assert_eq!(slim.remove_configs, vec!["DRM_AMDGPU", "DRM_AMDGPU_SI"]);
    assert_eq!(
        slim.set_defaults.get("DRM_AMDGPU_WERROR"),
        Some(&String::from("n"))
    );
    assert!(profile.abi.allow_public_header_removal);
    assert!(profile.abi.allow_uapi_header_removal);

    let patch_source = profile.patches.as_ref().unwrap().sources()[0];
    assert_eq!(patch_source.source, "worktree");
    assert_eq!(patch_source.path, "/tmp/linux-worktree");
    assert_eq!(patch_source.base_remote, "upstream");
    assert_eq!(patch_source.base_ref, "master");
    assert!(!patch_source.require_clean);

    let rtlmq = profile.integrations.rtlmq.as_ref().unwrap();
    assert_eq!(rtlmq.source, "/tmp/rtlmq");
    assert_eq!(rtlmq.tests_source.as_deref(), Some("/tmp/rtlmq-tests"));
    assert_eq!(profile.reducer.max_fixup_passes, 1);
    assert!(!profile.reducer.reject_unproven_fixups);
    assert!(!profile.selftests.check_makefiles);
    assert_eq!(profile.selftests.commands, vec!["make headers_check"]);

    let build = &profile.selftests.kernel_builds[0];
    assert_eq!(build.name.as_deref(), Some("x86-defconfig"));
    assert_eq!(build.config_target.as_deref(), Some("defconfig"));
    assert_eq!(build.targets, vec!["vmlinux"]);
    assert_eq!(build.output_dir.as_deref(), Some(".kslim-selftest/x86"));
    assert_eq!(build.jobs, Some(2));
    assert!(!build.clean);
    assert_eq!(build.make_program.as_deref(), Some("make"));
    assert_eq!(build.make_args, vec!["LLVM=1"]);
    assert_eq!(build.env.get("ARCH"), Some(&String::from("x86")));
    validate_profile(&profile).unwrap();
}
#[test]
fn test_default_profile_config_constructs_valid_profile_file_model() {
    let profile = default_profile_config("v6.15");

    assert_eq!(profile.profile.name, "default");
    assert_eq!(
        profile.profile.description,
        "Unmodified upstream Linux emitted by kslim"
    );
    assert_eq!(profile.base.r#ref, "v6.15");
    assert!(profile.slim.is_none());
    assert!(profile.features.is_empty());
    assert!(profile.patches.is_none());
    assert!(profile.arch.is_default());
    assert!(profile.build_matrix.is_default());
    assert!(profile.runtime_matrix.is_default());
    assert!(profile.reports.is_default());
    assert!(profile.security.is_default());
    assert!(profile.performance.is_default());
    assert!(profile.integrations.rtlmq.is_none());
    assert_eq!(profile.reducer, ReducerConfig::default());
    assert!(profile.selftests.enabled);
    validate_profile(&profile).unwrap();
}
#[test]
fn test_slim_config_defaults_are_noop_removal_intent() {
    let slim = SlimConfig::default();

    assert!(slim.remove_paths.is_empty());
    assert!(slim.remove_configs.is_empty());
    assert!(slim.set_defaults.is_empty());
    assert!(!slim.unsafe_allow_root_path_removal);
    assert!(slim.is_noop());
}
#[test]
fn test_slim_config_parse_declared_removal_intent() {
    let slim: SlimConfig = toml::from_str(
        r#"
remove_paths = ["net/bluetooth", "drivers/bluetooth"]
remove_configs = ["BT", "BT_HCIBTUSB"]
set_defaults = { BT_DEBUGFS = "n" }
"#,
    )
    .unwrap();

    assert_eq!(
        slim.remove_paths,
        vec!["net/bluetooth", "drivers/bluetooth"]
    );
    assert_eq!(slim.remove_configs, vec!["BT", "BT_HCIBTUSB"]);
    assert_eq!(
        slim.set_defaults.get("BT_DEBUGFS"),
        Some(&String::from("n"))
    );
    assert!(!slim.unsafe_allow_root_path_removal);
    assert!(!slim.is_noop());

    let manifest = crate::removal_manifest::RemovalManifest::from_slim_config_with_abi_policy(
        &slim,
        &AbiPolicyConfig::default(),
    )
    .unwrap();
    assert_eq!(
        manifest
            .removed_paths_vec()
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<Vec<_>>(),
        vec!["drivers/bluetooth", "net/bluetooth"]
    );
    assert_eq!(
        manifest.removed_config_symbols_vec(),
        vec!["BT", "BT_HCIBTUSB"]
    );
    assert_eq!(
        manifest.default_overrides().get("BT_DEBUGFS"),
        Some(&String::from("n"))
    );
}
#[test]
fn test_slim_config_unsafe_root_opt_in_alone_stays_noop() {
    let slim: SlimConfig = toml::from_str(
        r#"
unsafe_allow_root_path_removal = true
"#,
    )
    .unwrap();

    assert!(slim.unsafe_allow_root_path_removal);
    assert!(slim.is_noop());

    let manifest = crate::removal_manifest::RemovalManifest::from_slim_config_with_abi_policy(
        &slim,
        &AbiPolicyConfig::default(),
    )
    .unwrap();
    assert!(manifest.is_noop());
    assert!(manifest.unsafe_allow_root_path_removal);
}
#[test]
fn test_reducer_config_defaults_are_strict_safety_policy() {
    let reducer = ReducerConfig::default();

    assert_eq!(reducer.max_fixup_passes, 3);
    assert!(reducer.report_unsupported_expressions);
    assert!(reducer.fail_on_unknown_diagnostics);
    assert!(reducer.reject_unproven_fixups);
    assert!(reducer.reject_unreasoned_edits);
    assert!(reducer.reject_speculative_fallout_edits);
    assert!(!reducer.fail_on_missing_prune_paths);
    assert!(!reducer.ignore_unsupported_special_removals);
    assert!(reducer.strict_mode());
}
#[test]
fn test_reducer_config_parse_defaults_and_overrides() {
    let defaults: ReducerConfig = toml::from_str("").unwrap();

    assert_eq!(defaults, ReducerConfig::default());
    assert!(defaults.strict_mode());

    let reducer: ReducerConfig = toml::from_str(
        r#"
max_fixup_passes = 0
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

    assert_eq!(reducer.max_fixup_passes, 0);
    assert!(!reducer.report_unsupported_expressions);
    assert!(!reducer.fail_on_unknown_diagnostics);
    assert!(!reducer.reject_unproven_fixups);
    assert!(!reducer.reject_unreasoned_edits);
    assert!(!reducer.reject_speculative_fallout_edits);
    assert!(reducer.fail_on_missing_prune_paths);
    assert!(reducer.ignore_unsupported_special_removals);
    assert!(!reducer.strict_mode());
}
#[test]
fn test_reducer_config_strict_mode_tracks_publish_safety_gates_only() {
    for reducer in [
        ReducerConfig {
            report_unsupported_expressions: false,
            ..ReducerConfig::default()
        },
        ReducerConfig {
            fail_on_unknown_diagnostics: false,
            ..ReducerConfig::default()
        },
        ReducerConfig {
            reject_unproven_fixups: false,
            ..ReducerConfig::default()
        },
        ReducerConfig {
            reject_unreasoned_edits: false,
            ..ReducerConfig::default()
        },
        ReducerConfig {
            reject_speculative_fallout_edits: false,
            ..ReducerConfig::default()
        },
    ] {
        assert!(!reducer.strict_mode());
    }

    let reducer = ReducerConfig {
        max_fixup_passes: 0,
        fail_on_missing_prune_paths: true,
        ignore_unsupported_special_removals: true,
        ..ReducerConfig::default()
    };

    assert!(reducer.strict_mode());
}
#[test]
fn test_feature_config_defaults_are_empty_named_intent() {
    let features = FeatureConfig::default();

    assert!(features.remove.is_empty());
    assert!(features.preserve.is_empty());
    assert!(features.is_empty());
}
#[test]
fn test_feature_config_parse_named_remove_and_preserve_intent() {
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
exported_symbols = ["bt_sock_register"]
remove_exported_symbols = ["bt_sock_unregister"]
module_names = ["btusb"]
remove_module_names = ["bt_debug"]
module_aliases = ["usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*"]
remove_module_aliases = ["pci:v00008086d00001572sv*sd*bc*sc*i*"]
device_compatibles = ["qcom,ipq8064"]
remove_device_compatibles = ["vendor,removed-device"]
acpi_ids = ["PNP0C09"]
remove_acpi_ids = ["ACPI0003"]
pci_ids = ["8086:1572"]
remove_pci_ids = ["10EC:8168"]
usb_ids = ["0BDA:8153"]
remove_usb_ids = ["046D:C52B"]
firmware_paths = ["amdgpu/polaris10_mc.bin"]
remove_firmware_paths = ["iwlwifi-7260-17.ucode"]
initcalls = ["bt_init"]
remove_initcalls = ["btusb_driver_init"]
runtime_registrations = ["module_init:bt_init"]
remove_runtime_registrations = ["module_platform_driver:btusb_driver"]
docs = ["Documentation/networking/bluetooth.rst"]
remove_docs = ["Documentation/driver-api/btusb.rst"]
tools = ["tools/perf"]
remove_tools = ["tools/objtool"]
samples = ["samples/bpf"]
remove_samples = ["samples/hidraw"]
kunit_suites = ["bt_test"]
remove_kunit_suites = ["btusb-test"]
kselftest_targets = ["net"]
remove_kselftest_targets = ["bpf"]
arch_scope = ["x86"]
safety = "surgical"
preserve_uapi = false
preserve_module_aliases = false
require_clean_boot = true
report_only = true

[features.preserve.netfilter]
kind = "subsystem"
roots = ["net/netfilter"]
configs = ["NETFILTER"]
exported_symbols = ["nf_register_net_hook"]
module_names = ["nf_conntrack"]
module_aliases = ["of:N*T*Cqcom,ipq8064"]
device_compatibles = ["brcm,bcm2835-aux-uart"]
acpi_ids = ["PRP0001"]
pci_ids = ["1AF4:1000"]
usb_ids = ["1D6B:0002"]
firmware_paths = ["qcom/venus-5.2/venus.mbn"]
initcalls = ["nf_conntrack_standalone_init"]
runtime_registrations = ["module_init:nf_conntrack_standalone_init"]
docs = ["Documentation/networking/nf_conntrack-sysctl.rst"]
tools = ["tools/testing/selftests/netfilter"]
samples = ["samples/kobject"]
kunit_suites = ["nf_conntrack_test"]
kselftest_targets = ["drivers/net"]
preserve_uapi = true
"#,
    )
    .unwrap();

    assert!(!profile.features.is_empty());
    let bluetooth = profile.features.remove.get("bluetooth").unwrap();
    assert_eq!(bluetooth.kind.as_deref(), Some("subsystem"));
    assert_eq!(bluetooth.roots, vec!["net/bluetooth", "drivers/bluetooth"]);
    assert_eq!(bluetooth.configs, vec!["BT"]);
    assert_eq!(bluetooth.exported_symbols, vec!["bt_sock_register"]);
    assert_eq!(
        bluetooth.remove_exported_symbols,
        vec!["bt_sock_unregister"]
    );
    assert_eq!(bluetooth.module_names, vec!["btusb"]);
    assert_eq!(bluetooth.remove_module_names, vec!["bt_debug"]);
    assert_eq!(
        bluetooth.module_aliases,
        vec!["usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*"]
    );
    assert_eq!(
        bluetooth.remove_module_aliases,
        vec!["pci:v00008086d00001572sv*sd*bc*sc*i*"]
    );
    assert_eq!(bluetooth.device_compatibles, vec!["qcom,ipq8064"]);
    assert_eq!(
        bluetooth.remove_device_compatibles,
        vec!["vendor,removed-device"]
    );
    assert_eq!(bluetooth.acpi_ids, vec!["PNP0C09"]);
    assert_eq!(bluetooth.remove_acpi_ids, vec!["ACPI0003"]);
    assert_eq!(bluetooth.pci_ids, vec!["8086:1572"]);
    assert_eq!(bluetooth.remove_pci_ids, vec!["10EC:8168"]);
    assert_eq!(bluetooth.usb_ids, vec!["0BDA:8153"]);
    assert_eq!(bluetooth.remove_usb_ids, vec!["046D:C52B"]);
    assert_eq!(bluetooth.firmware_paths, vec!["amdgpu/polaris10_mc.bin"]);
    assert_eq!(
        bluetooth.remove_firmware_paths,
        vec!["iwlwifi-7260-17.ucode"]
    );
    assert_eq!(bluetooth.initcalls, vec!["bt_init"]);
    assert_eq!(bluetooth.remove_initcalls, vec!["btusb_driver_init"]);
    assert_eq!(bluetooth.runtime_registrations, vec!["module_init:bt_init"]);
    assert_eq!(
        bluetooth.remove_runtime_registrations,
        vec!["module_platform_driver:btusb_driver"]
    );
    assert_eq!(
        bluetooth.docs,
        vec!["Documentation/networking/bluetooth.rst"]
    );
    assert_eq!(
        bluetooth.remove_docs,
        vec!["Documentation/driver-api/btusb.rst"]
    );
    assert_eq!(bluetooth.tools, vec!["tools/perf"]);
    assert_eq!(bluetooth.remove_tools, vec!["tools/objtool"]);
    assert_eq!(bluetooth.samples, vec!["samples/bpf"]);
    assert_eq!(bluetooth.remove_samples, vec!["samples/hidraw"]);
    assert_eq!(bluetooth.kunit_suites, vec!["bt_test"]);
    assert_eq!(bluetooth.remove_kunit_suites, vec!["btusb-test"]);
    assert_eq!(bluetooth.kselftest_targets, vec!["net"]);
    assert_eq!(bluetooth.remove_kselftest_targets, vec!["bpf"]);
    assert_eq!(bluetooth.arch_scope, vec!["x86"]);
    assert_eq!(bluetooth.safety, Some(FeatureSafetyLevel::Surgical));
    assert!(!bluetooth.preserve_uapi);
    assert!(!bluetooth.preserve_module_aliases);
    assert!(bluetooth.require_clean_boot);
    assert!(bluetooth.report_only);

    let netfilter = profile.features.preserve.get("netfilter").unwrap();
    assert_eq!(netfilter.kind.as_deref(), Some("subsystem"));
    assert_eq!(netfilter.roots, vec!["net/netfilter"]);
    assert_eq!(netfilter.configs, vec!["NETFILTER"]);
    assert_eq!(netfilter.exported_symbols, vec!["nf_register_net_hook"]);
    assert_eq!(netfilter.module_names, vec!["nf_conntrack"]);
    assert_eq!(netfilter.module_aliases, vec!["of:N*T*Cqcom,ipq8064"]);
    assert_eq!(netfilter.device_compatibles, vec!["brcm,bcm2835-aux-uart"]);
    assert_eq!(netfilter.acpi_ids, vec!["PRP0001"]);
    assert_eq!(netfilter.pci_ids, vec!["1AF4:1000"]);
    assert_eq!(netfilter.usb_ids, vec!["1D6B:0002"]);
    assert_eq!(netfilter.firmware_paths, vec!["qcom/venus-5.2/venus.mbn"]);
    assert_eq!(netfilter.initcalls, vec!["nf_conntrack_standalone_init"]);
    assert_eq!(
        netfilter.runtime_registrations,
        vec!["module_init:nf_conntrack_standalone_init"]
    );
    assert_eq!(
        netfilter.docs,
        vec!["Documentation/networking/nf_conntrack-sysctl.rst"]
    );
    assert_eq!(netfilter.tools, vec!["tools/testing/selftests/netfilter"]);
    assert_eq!(netfilter.samples, vec!["samples/kobject"]);
    assert_eq!(netfilter.kunit_suites, vec!["nf_conntrack_test"]);
    assert_eq!(netfilter.kselftest_targets, vec!["drivers/net"]);
    assert!(netfilter.preserve_uapi);

    let err = validate_profile(&profile).unwrap_err().to_string();
    assert!(err.contains("features.preserve.netfilter.preserve_uapi"));
}
#[test]
fn test_default_profile_uses_strict_reducer_defaults() {
    let profile = default_profile_config("v1.0");

    assert_eq!(profile.reducer.max_fixup_passes, 3);
    assert!(profile.reducer.report_unsupported_expressions);
    assert!(profile.reducer.fail_on_unknown_diagnostics);
    assert!(profile.reducer.reject_unproven_fixups);
    assert!(profile.reducer.reject_unreasoned_edits);
    assert!(profile.reducer.reject_speculative_fallout_edits);
    assert!(profile.reducer.strict_mode());
    assert!(!profile.reducer.fail_on_missing_prune_paths);
    assert!(!profile.reducer.ignore_unsupported_special_removals);
    assert!(!profile.abi.allow_public_header_removal);
    assert!(!profile.abi.allow_uapi_header_removal);
}
#[test]
fn test_profile_parse_defaults_reducer_to_strict_mode() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"
"#,
    )
    .unwrap();

    assert_eq!(profile.reducer.max_fixup_passes, 3);
    assert!(profile.reducer.report_unsupported_expressions);
    assert!(profile.reducer.fail_on_unknown_diagnostics);
    assert!(profile.reducer.reject_unproven_fixups);
    assert!(profile.reducer.reject_unreasoned_edits);
    assert!(profile.reducer.reject_speculative_fallout_edits);
    assert!(profile.reducer.strict_mode());
    assert!(!profile.reducer.fail_on_missing_prune_paths);
    assert!(!profile.reducer.ignore_unsupported_special_removals);
    assert!(!profile.abi.allow_public_header_removal);
    assert!(!profile.abi.allow_uapi_header_removal);
}
#[test]
fn test_profile_parse_defaults_root_path_removal_unsafe_mode_off() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[slim]
remove_paths = ["drivers/gpu/drm/amd/amdgpu"]
"#,
    )
    .unwrap();

    assert!(
        !profile
            .slim
            .as_ref()
            .unwrap()
            .unsafe_allow_root_path_removal
    );
}
#[test]
fn test_profile_parse_allows_explicit_root_path_removal_unsafe_mode() {
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

    assert!(
        profile
            .slim
            .as_ref()
            .unwrap()
            .unsafe_allow_root_path_removal
    );
}
#[test]
fn test_profile_parse_allows_explicit_abi_policy() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[abi]
allow_public_header_removal = true
allow_uapi_header_removal = true
"#,
    )
    .unwrap();

    assert!(profile.abi.allow_public_header_removal);
    assert!(profile.abi.allow_uapi_header_removal);
}
#[test]
fn test_abi_policy_config_defaults_fail_closed() {
    let policy = AbiPolicyConfig::default();

    assert!(!policy.allow_public_header_removal);
    assert!(!policy.allow_uapi_header_removal);
    assert!(policy.is_fail_closed());
}
#[test]
fn test_abi_policy_config_parse_explicit_approval_flags() {
    let policy: AbiPolicyConfig = toml::from_str(
        r#"
allow_public_header_removal = true
allow_uapi_header_removal = true
"#,
    )
    .unwrap();

    assert!(policy.allow_public_header_removal);
    assert!(policy.allow_uapi_header_removal);
    assert!(!policy.is_fail_closed());
}
#[test]
fn test_abi_policy_config_keeps_uapi_approval_separate_from_public_headers() {
    let public_header_only = AbiPolicyConfig {
        allow_public_header_removal: true,
        allow_uapi_header_removal: false,
    };

    assert!(crate::abi::allows_public_header_removal(
        std::path::Path::new("include/linux/public.h"),
        &public_header_only,
    ));
    assert!(!crate::abi::allows_public_header_removal(
        std::path::Path::new("include/uapi/linux/abi.h"),
        &public_header_only,
    ));
}
#[test]
fn test_arch_policy_config_defaults_are_unscoped_arch_intent() {
    let arch = ArchPolicyConfig::default();

    assert_eq!(arch.primary_arch, None);
    assert!(arch.secondary_arches.is_empty());
    assert!(arch.disabled_arches.is_empty());
    assert!(!arch.allow_arch_local_removal);
    assert!(arch.preserve_arch_shared);
    assert!(arch.is_default());
}
#[test]
fn test_arch_policy_config_parse_arch_selection_policy_fields() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[arch]
primary_arch = "x86"
secondary_arches = ["arm64", "riscv"]
disabled_arches = ["m68k"]
allow_arch_local_removal = true
preserve_arch_shared = false
"#,
    )
    .unwrap();

    assert_eq!(profile.arch.primary_arch.as_deref(), Some("x86"));
    assert_eq!(profile.arch.secondary_arches, vec!["arm64", "riscv"]);
    assert_eq!(profile.arch.disabled_arches, vec!["m68k"]);
    assert!(profile.arch.allow_arch_local_removal);
    assert!(!profile.arch.preserve_arch_shared);
    assert!(!profile.arch.is_default());
}
#[test]
fn test_build_matrix_config_defaults_are_inactive_policy() {
    let build_matrix = BuildMatrixConfig::default();

    assert!(!build_matrix.enabled);
    assert!(build_matrix.presets.is_empty());
    assert!(build_matrix.arches.is_empty());
    assert!(build_matrix.config_targets.is_empty());
    assert!(build_matrix.targets.is_empty());
    assert_eq!(build_matrix.randconfig_seed, None);
    assert_eq!(build_matrix.jobs, None);
    assert!(build_matrix.fail_on_error);
    assert!(build_matrix.is_default());
}
#[test]
fn test_build_matrix_config_parse_verification_matrix_fields() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[build_matrix]
enabled = true
presets = ["default", "hardening"]
arches = ["x86", "arm64"]
config_targets = ["defconfig", "allmodconfig"]
targets = ["vmlinux", "modules", "headers_install"]
randconfig_seed = "kslim-seed"
jobs = 16
fail_on_error = false
"#,
    )
    .unwrap();

    assert!(profile.build_matrix.enabled);
    assert_eq!(profile.build_matrix.presets, vec!["default", "hardening"]);
    assert_eq!(profile.build_matrix.arches, vec!["x86", "arm64"]);
    assert_eq!(
        profile.build_matrix.config_targets,
        vec!["defconfig", "allmodconfig"]
    );
    assert_eq!(
        profile.build_matrix.targets,
        vec!["vmlinux", "modules", "headers_install"]
    );
    assert_eq!(
        profile.build_matrix.randconfig_seed.as_deref(),
        Some("kslim-seed")
    );
    assert_eq!(profile.build_matrix.jobs, Some(16));
    assert!(!profile.build_matrix.fail_on_error);
    assert!(!profile.build_matrix.is_default());
}
#[test]
fn test_runtime_matrix_config_defaults_are_inactive_policy() {
    let runtime_matrix = RuntimeMatrixConfig::default();

    assert!(!runtime_matrix.enabled);
    assert!(runtime_matrix.boot_arches.is_empty());
    assert!(runtime_matrix.qemu_machines.is_empty());
    assert!(runtime_matrix.kunit_suites.is_empty());
    assert!(runtime_matrix.kselftest_targets.is_empty());
    assert!(!runtime_matrix.module_smoke);
    assert!(runtime_matrix.require_clean_dmesg);
    assert_eq!(runtime_matrix.boot_timeout_seconds, None);
    assert!(runtime_matrix.fail_on_error);
    assert!(runtime_matrix.is_default());
}
#[test]
fn test_runtime_matrix_config_parse_runtime_validation_fields() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[runtime_matrix]
enabled = true
boot_arches = ["x86", "arm64"]
qemu_machines = ["q35", "virt"]
kunit_suites = ["net_test"]
kselftest_targets = ["net", "bpf"]
module_smoke = true
require_clean_dmesg = false
boot_timeout_seconds = 120
fail_on_error = false
"#,
    )
    .unwrap();

    assert!(profile.runtime_matrix.enabled);
    assert_eq!(profile.runtime_matrix.boot_arches, vec!["x86", "arm64"]);
    assert_eq!(profile.runtime_matrix.qemu_machines, vec!["q35", "virt"]);
    assert_eq!(profile.runtime_matrix.kunit_suites, vec!["net_test"]);
    assert_eq!(profile.runtime_matrix.kselftest_targets, vec!["net", "bpf"]);
    assert!(profile.runtime_matrix.module_smoke);
    assert!(!profile.runtime_matrix.require_clean_dmesg);
    assert_eq!(profile.runtime_matrix.boot_timeout_seconds, Some(120));
    assert!(!profile.runtime_matrix.fail_on_error);
    assert!(!profile.runtime_matrix.is_default());
}
#[test]
fn test_report_config_defaults_are_current_committed_report_policy() {
    let reports = ReportConfig::default();

    assert_eq!(reports.formats, vec!["text", "markdown", "json"]);
    assert!(reports.include_edit_records);
    assert!(reports.include_diagnostics);
    assert!(!reports.include_source_map);
    assert!(reports.redact_host_paths);
    assert!(!reports.include_raw_logs);
    assert!(reports.fail_on_error);
    assert!(reports.is_default());
}
#[test]
fn test_report_config_parse_report_policy_fields() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[reports]
formats = ["json"]
include_edit_records = false
include_diagnostics = false
include_source_map = true
redact_host_paths = false
include_raw_logs = true
fail_on_error = false
"#,
    )
    .unwrap();

    assert_eq!(profile.reports.formats, vec!["json"]);
    assert!(!profile.reports.include_edit_records);
    assert!(!profile.reports.include_diagnostics);
    assert!(profile.reports.include_source_map);
    assert!(!profile.reports.redact_host_paths);
    assert!(profile.reports.include_raw_logs);
    assert!(!profile.reports.fail_on_error);
    assert!(!profile.reports.is_default());
}
#[test]
fn test_security_config_defaults_are_fail_closed_trust_boundary_policy() {
    let security = SecurityConfig::default();

    assert!(!security.allow_network);
    assert!(security.require_local_upstream);
    assert!(security.reject_host_paths_in_committed_metadata);
    assert!(security.reject_temp_paths_in_committed_metadata);
    assert!(security.reject_raw_logs_in_committed_metadata);
    assert!(security.require_reproducible_timestamps);
    assert!(security.require_phase_typed_metadata);
    assert_eq!(security.compatibility_mode, None);
    assert!(security.fail_on_policy_violation);
    assert!(security.is_default());
}
#[test]
fn test_security_config_parse_trust_boundary_policy_fields() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[security]
allow_network = true
require_local_upstream = false
reject_host_paths_in_committed_metadata = false
reject_temp_paths_in_committed_metadata = false
reject_raw_logs_in_committed_metadata = false
require_reproducible_timestamps = false
require_phase_typed_metadata = false
compatibility_mode = "legacy"
fail_on_policy_violation = false
"#,
    )
    .unwrap();

    assert!(profile.security.allow_network);
    assert!(!profile.security.require_local_upstream);
    assert!(!profile.security.reject_host_paths_in_committed_metadata);
    assert!(!profile.security.reject_temp_paths_in_committed_metadata);
    assert!(!profile.security.reject_raw_logs_in_committed_metadata);
    assert!(!profile.security.require_reproducible_timestamps);
    assert!(!profile.security.require_phase_typed_metadata);
    assert_eq!(
        profile.security.compatibility_mode.as_deref(),
        Some("legacy")
    );
    assert!(!profile.security.fail_on_policy_violation);
    assert!(!profile.security.is_default());
}
#[test]
fn test_performance_config_defaults_are_inactive_hot_path_policy() {
    let performance = PerformanceConfig::default();

    assert!(!performance.enabled);
    assert_eq!(performance.max_worker_threads, None);
    assert_eq!(performance.max_io_threads, None);
    assert!(!performance.cache_tree_index);
    assert!(!performance.incremental_reindex);
    assert!(!performance.collect_timing_metrics);
    assert!(!performance.profile_hot_paths);
    assert!(performance.fail_on_regression);
    assert!(performance.is_default());
}
#[test]
fn test_performance_config_parse_hot_path_policy_fields() {
    let profile: ProfileConfig = toml::from_str(
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[performance]
enabled = true
max_worker_threads = 16
max_io_threads = 4
cache_tree_index = true
incremental_reindex = true
collect_timing_metrics = true
profile_hot_paths = true
fail_on_regression = false
"#,
    )
    .unwrap();

    assert!(profile.performance.enabled);
    assert_eq!(profile.performance.max_worker_threads, Some(16));
    assert_eq!(profile.performance.max_io_threads, Some(4));
    assert!(profile.performance.cache_tree_index);
    assert!(profile.performance.incremental_reindex);
    assert!(profile.performance.collect_timing_metrics);
    assert!(profile.performance.profile_hot_paths);
    assert!(!profile.performance.fail_on_regression);
    assert!(!profile.performance.is_default());
}
