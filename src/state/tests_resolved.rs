use super::*;

#[test]
fn test_resolved_candidate_state_captures_plans_without_candidate_or_published_state() {
    let mut config = config::default_kslim_config("demo", "/tmp/output");
    config.output.branch = Some(String::from("kslim/custom"));
    let mut profile = config::default_profile_config("v1.0");
    profile.integrations.rtlmq = Some(RtlmqIntegrationConfig {
        source: String::from("/tmp/rtlmq"),
        tests_source: Some(String::from("/tmp/rtlmq-tests")),
    });
    profile.slim = Some(SlimConfig {
        remove_paths: vec![
            String::from("./drivers//remove/file.c"),
            String::from("drivers/remove"),
        ],
        remove_configs: vec![String::from("CONFIG_REMOVE")],
        set_defaults: BTreeMap::from([(String::from("CONFIG_KEEP"), String::from("y"))]),
        unsafe_allow_root_path_removal: false,
    });
    profile.reducer.max_fixup_passes = 7;
    profile.reducer.report_unsupported_expressions = false;
    profile.reducer.fail_on_unknown_diagnostics = false;
    profile.reducer.reject_unproven_fixups = false;
    profile.reducer.reject_unreasoned_edits = false;
    profile.reducer.reject_speculative_fallout_edits = false;
    profile.reducer.fail_on_missing_prune_paths = true;
    profile.reducer.ignore_unsupported_special_removals = true;
    profile.selftests.commands = vec![String::from("make test")];
    profile.selftests.kernel_builds = vec![KernelBuildConfig {
        name: Some(String::from("tiny")),
        config_target: Some(String::from("tinyconfig")),
        targets: vec![String::from("drivers/foo/bar.o")],
        output_dir: Some(String::from("build/tiny")),
        jobs: Some(2),
        clean: false,
        make_program: Some(String::from("gmake")),
        make_args: vec![String::from("V=1")],
        env: BTreeMap::from([(String::from("ARCH"), String::from("x86"))]),
    }];
    let patch_infos = vec![PatchInfo {
        source: String::from("worktree"),
        worktree_path: String::from("/tmp/patches"),
        branch: String::from("topic"),
        head_commit: String::from("abc123"),
        merge_base: String::from("base123"),
        base_remote: String::from("origin"),
        base_ref: String::from("main"),
        patch_count: 3,
    }];

    let resolved = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &profile,
        test_resolved_base(),
        Some(&patch_infos),
        "slimmed",
        "kslim/custom",
    )
    .unwrap();

    assert_eq!(resolved.base.commit, "deadbeef");
    assert_eq!(resolved.patch_plan.total_patch_count, 3);
    assert!(resolved.patch_plan.sources[0]
        .stable_id
        .starts_with("patch-source-"));
    assert_eq!(resolved.patch_plan.sources[0].branch, "topic");
    assert_eq!(resolved.integration_plan.entries.len(), 1);
    assert_eq!(resolved.integration_plan.entries[0].kind, "rtlmq");
    assert!(resolved.integration_plan.entries[0]
        .stable_id
        .starts_with("integration-rtlmq-"));
    assert_eq!(
        resolved.integration_plan.rtlmq.as_ref().unwrap().source,
        "/tmp/rtlmq"
    );
    assert_eq!(
        resolved.integration_plan.entries[0].stable_id,
        resolved.integration_plan.rtlmq.as_ref().unwrap().stable_id
    );
    assert_eq!(
        resolved.feature_resolution.source(),
        FeatureResolutionSource::DirectSlim
    );
    assert!(!resolved.feature_resolution.is_noop());
    assert_eq!(
        resolved
            .feature_resolution
            .remove_paths()
            .iter()
            .map(|path| path.as_path())
            .collect::<Vec<_>>(),
        vec![Path::new("drivers/remove")]
    );
    assert_eq!(
        resolved
            .feature_resolution
            .remove_configs()
            .iter()
            .map(|symbol| symbol.as_str())
            .collect::<Vec<_>>(),
        vec!["CONFIG_REMOVE"]
    );
    assert_eq!(
        resolved
            .feature_resolution
            .set_defaults()
            .get("CONFIG_KEEP")
            .map(String::as_str),
        Some("y")
    );
    assert!(
        !resolved
            .feature_resolution
            .abi_policy()
            .allow_public_header_removal
    );
    assert!(!resolved.feature_resolution.unsafe_allow_root_path_removal());
    assert!(!resolved.abi_decision.allow_public_header_removal());
    assert!(!resolved.abi_decision.allow_uapi_header_removal());
    assert!(!resolved.abi_decision.has_abi_sensitive_removals());
    assert_eq!(
        resolved
            .prune_plan
            .remove_paths
            .iter()
            .map(|path| path.as_path())
            .collect::<Vec<_>>(),
        vec![Path::new("drivers/remove")]
    );
    assert_eq!(
        resolved
            .prune_plan
            .remove_configs
            .iter()
            .map(|symbol| symbol.as_str())
            .collect::<Vec<_>>(),
        vec!["CONFIG_REMOVE"]
    );
    assert_eq!(
        resolved
            .prune_plan
            .set_defaults
            .get("CONFIG_KEEP")
            .map(String::as_str),
        Some("y")
    );
    assert_eq!(resolved.reducer_plan.max_fixup_passes, 7);
    assert!(!resolved.reducer_plan.report_unsupported_expressions);
    assert!(!resolved.reducer_plan.fail_on_unknown_diagnostics);
    assert!(!resolved.reducer_plan.reject_unproven_fixups);
    assert!(!resolved.reducer_plan.reject_unreasoned_edits);
    assert!(!resolved.reducer_plan.reject_speculative_fallout_edits);
    assert!(resolved.reducer_plan.fail_on_missing_prune_paths);
    assert!(resolved.reducer_plan.ignore_unsupported_special_removals);
    assert_eq!(resolved.selftest_plan.commands, ["make test"]);
    assert_eq!(
        resolved.selftest_plan.kernel_builds[0]
            .output_dir
            .as_ref()
            .map(|dir| dir.as_path()),
        Some(Path::new("build/tiny"))
    );
    assert_eq!(
        resolved.selftest_plan.kernel_builds[0]
            .arch
            .as_ref()
            .map(|arch| arch.as_str()),
        Some("x86")
    );
    assert_eq!(
        resolved.selftest_plan.kernel_builds[0]
            .env
            .get("ARCH")
            .map(String::as_str),
        Some("x86")
    );
    assert_eq!(
        resolved.output_plan.output_path.as_path(),
        Path::new("/tmp/output")
    );
    assert_eq!(resolved.output_plan.branch, "kslim/custom");
    assert_eq!(resolved.output_plan.mode, "slimmed");
    assert_eq!(resolved.output_plan.naming.project_name, "demo");
    assert_eq!(resolved.output_plan.naming.profile_name, "default");
    assert_eq!(resolved.output_plan.naming.branch_prefix, "kslim");
    assert_eq!(
        resolved.output_plan.naming.explicit_branch.as_deref(),
        Some("kslim/custom")
    );
    assert_eq!(resolved.output_plan.naming.base_ref, "v1.0");
    assert_eq!(resolved.output_plan.naming.base_commit, "deadbeef");
}

#[test]
fn test_feature_resolution_state_captures_absent_removal_input_as_noop() {
    let profile = config::default_profile_config("v1.0");

    let resolution = FeatureResolutionState::from_profile(&profile).unwrap();

    assert_eq!(resolution.source(), FeatureResolutionSource::NoRemoval);
    assert!(resolution.is_noop());
    assert!(resolution.remove_paths().is_empty());
    assert!(resolution.remove_configs().is_empty());
    assert!(resolution.preserve_paths().is_empty());
    assert!(resolution.preserve_configs().is_empty());
    assert!(resolution.set_defaults().is_empty());
    assert!(!resolution.abi_policy().allow_public_header_removal);
    assert!(resolution.feature_safety_levels().is_empty());
    assert!(resolution.feature_arch_scopes().is_empty());
    assert!(resolution.feature_test_matrices().is_empty());
    assert!(resolution.feature_report_modes().is_empty());
    assert!(!resolution.unsafe_allow_root_path_removal());
}

#[test]
fn test_resolved_candidate_state_captures_feature_intent_plan() {
    let config = config::default_kslim_config("demo", "/tmp/output");
    let mut profile = config::default_profile_config("v1.0");
    profile.features.remove.insert(
        String::from("bluetooth"),
        config::FeatureIntentConfig {
            kind: Some(String::from("subsystem")),
            roots: vec![
                String::from("net/bluetooth"),
                String::from("drivers/bluetooth"),
            ],
            remove_paths: vec![String::from("net/rfkill")],
            configs: vec![String::from("BT")],
            remove_configs: vec![String::from("BT_HCIBTUSB")],
            exported_symbols: vec![String::from("bt_sock_register")],
            remove_exported_symbols: vec![String::from("bt_debugfs_init")],
            module_names: vec![String::from("btusb")],
            remove_module_names: vec![String::from("bt_debug")],
            module_aliases: vec![String::from("usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*")],
            remove_module_aliases: vec![String::from("pci:v00008086d00001572sv*sd*bc*sc*i*")],
            device_compatibles: vec![String::from("qcom,ipq8064")],
            remove_device_compatibles: vec![String::from("vendor,removed-device")],
            acpi_ids: vec![String::from("PNP0C09")],
            remove_acpi_ids: vec![String::from("ACPI0003")],
            pci_ids: vec![String::from("8086:1572")],
            remove_pci_ids: vec![String::from("10EC:8168")],
            usb_ids: vec![String::from("0BDA:8153")],
            remove_usb_ids: vec![String::from("046D:C52B")],
            firmware_paths: vec![String::from("amdgpu/polaris10_mc.bin")],
            remove_firmware_paths: vec![String::from("iwlwifi-7260-17.ucode")],
            initcalls: vec![String::from("bt_init")],
            remove_initcalls: vec![String::from("btusb_driver_init")],
            runtime_registrations: vec![String::from("module_init:bt_init")],
            remove_runtime_registrations: vec![String::from("module_platform_driver:btusb_driver")],
            docs: vec![String::from("Documentation/networking/bluetooth.rst")],
            remove_docs: vec![String::from("Documentation/driver-api/btusb.rst")],
            tools: vec![String::from("tools/perf")],
            remove_tools: vec![String::from("tools/objtool")],
            samples: vec![String::from("samples/bpf")],
            remove_samples: vec![String::from("samples/hidraw")],
            kunit_suites: vec![String::from("bt_test")],
            remove_kunit_suites: vec![String::from("btusb-test")],
            kselftest_targets: vec![String::from("net")],
            remove_kselftest_targets: vec![String::from("bpf")],
            allow_uapi_header_removal: true,
            safety: Some(config::FeatureSafetyLevel::Surgical),
            arch_scope: vec![String::from("x86"), String::from("arm64")],
            require_clean_boot: true,
            report_only: true,
            ..config::FeatureIntentConfig::default()
        },
    );
    profile.features.preserve.insert(
        String::from("netfilter"),
        config::FeatureIntentConfig {
            roots: vec![String::from("net/netfilter")],
            configs: vec![String::from("NETFILTER")],
            exported_symbols: vec![String::from("nf_register_net_hook")],
            module_names: vec![String::from("nf-conntrack")],
            module_aliases: vec![String::from("of:N*T*Cqcom,ipq8064")],
            device_compatibles: vec![String::from("brcm,bcm2835-aux-uart")],
            acpi_ids: vec![String::from("PRP0001")],
            pci_ids: vec![String::from("1AF4:1000")],
            usb_ids: vec![String::from("1D6B:0002")],
            firmware_paths: vec![String::from("qcom/venus-5.2/venus.mbn")],
            initcalls: vec![String::from("nf_conntrack_standalone_init")],
            runtime_registrations: vec![String::from("module_init:nf_conntrack_standalone_init")],
            docs: vec![String::from(
                "Documentation/networking/nf_conntrack-sysctl.rst",
            )],
            tools: vec![String::from("tools/testing/selftests/netfilter")],
            samples: vec![String::from("samples/kobject")],
            kunit_suites: vec![String::from("nf_conntrack_test")],
            kselftest_targets: vec![String::from("drivers/net")],
            ..config::FeatureIntentConfig::default()
        },
    );

    let resolved = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap();

    assert!(resolved
        .feature_graph_fingerprint
        .as_str()
        .starts_with("feature-graph-"));
    assert!(resolved
        .removal_manifest_fingerprint
        .as_str()
        .starts_with("removal-manifest-"));
    assert!(resolved
        .abi_policy_fingerprint
        .as_str()
        .starts_with("abi-policy-"));
    assert!(resolved
        .arch_policy_fingerprint
        .as_str()
        .starts_with("arch-policy-"));
    assert_eq!(resolved.feature_intent_plan.intents.len(), 2);
    let bluetooth = resolved
        .feature_intent_plan
        .intents
        .iter()
        .find(|intent| intent.name == "bluetooth")
        .unwrap();
    assert!(bluetooth.stable_id.starts_with("feature-intent-"));
    assert_eq!(bluetooth.action, "remove");
    assert_eq!(bluetooth.kind.as_deref(), Some("subsystem"));
    assert_eq!(
        bluetooth
            .roots
            .iter()
            .map(|path| path.as_path())
            .collect::<Vec<_>>(),
        vec![Path::new("drivers/bluetooth"), Path::new("net/bluetooth")]
    );
    assert_eq!(
        bluetooth
            .configs
            .iter()
            .map(|symbol| symbol.as_str())
            .collect::<Vec<_>>(),
        vec!["BT"]
    );
    assert_eq!(
        bluetooth
            .exported_symbols
            .iter()
            .map(|symbol| symbol.as_str())
            .collect::<Vec<_>>(),
        vec!["bt_sock_register"]
    );
    assert_eq!(
        bluetooth
            .remove_exported_symbols
            .iter()
            .map(|symbol| symbol.as_str())
            .collect::<Vec<_>>(),
        vec!["bt_debugfs_init"]
    );
    assert_eq!(
        bluetooth
            .module_names
            .iter()
            .map(|module| module.as_str())
            .collect::<Vec<_>>(),
        vec!["btusb"]
    );
    assert_eq!(
        bluetooth
            .remove_module_names
            .iter()
            .map(|module| module.as_str())
            .collect::<Vec<_>>(),
        vec!["bt_debug"]
    );
    assert_eq!(
        bluetooth
            .module_aliases
            .iter()
            .map(|alias| alias.as_str())
            .collect::<Vec<_>>(),
        vec!["usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*"]
    );
    assert_eq!(
        bluetooth
            .remove_module_aliases
            .iter()
            .map(|alias| alias.as_str())
            .collect::<Vec<_>>(),
        vec!["pci:v00008086d00001572sv*sd*bc*sc*i*"]
    );
    assert_eq!(
        bluetooth
            .device_compatibles
            .iter()
            .map(|compatible| compatible.as_str())
            .collect::<Vec<_>>(),
        vec!["qcom,ipq8064"]
    );
    assert_eq!(
        bluetooth
            .remove_device_compatibles
            .iter()
            .map(|compatible| compatible.as_str())
            .collect::<Vec<_>>(),
        vec!["vendor,removed-device"]
    );
    assert_eq!(
        bluetooth
            .acpi_ids
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        vec!["PNP0C09"]
    );
    assert_eq!(
        bluetooth
            .remove_acpi_ids
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        vec!["ACPI0003"]
    );
    assert_eq!(
        bluetooth
            .pci_ids
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        vec!["8086:1572"]
    );
    assert_eq!(
        bluetooth
            .remove_pci_ids
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        vec!["10EC:8168"]
    );
    assert_eq!(
        bluetooth
            .usb_ids
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        vec!["0BDA:8153"]
    );
    assert_eq!(
        bluetooth
            .remove_usb_ids
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        vec!["046D:C52B"]
    );
    assert_eq!(
        bluetooth
            .firmware_paths
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        vec!["amdgpu/polaris10_mc.bin"]
    );
    assert_eq!(
        bluetooth
            .remove_firmware_paths
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        vec!["iwlwifi-7260-17.ucode"]
    );
    assert_eq!(
        bluetooth
            .initcalls
            .iter()
            .map(|initcall| initcall.as_str())
            .collect::<Vec<_>>(),
        vec!["bt_init"]
    );
    assert_eq!(
        bluetooth
            .remove_initcalls
            .iter()
            .map(|initcall| initcall.as_str())
            .collect::<Vec<_>>(),
        vec!["btusb_driver_init"]
    );
    assert_eq!(
        bluetooth
            .runtime_registrations
            .iter()
            .map(|surface| surface.as_str())
            .collect::<Vec<_>>(),
        vec!["module_init:bt_init"]
    );
    assert_eq!(
        bluetooth
            .remove_runtime_registrations
            .iter()
            .map(|surface| surface.as_str())
            .collect::<Vec<_>>(),
        vec!["module_platform_driver:btusb_driver"]
    );
    assert_eq!(
        bluetooth
            .docs
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        vec!["Documentation/networking/bluetooth.rst"]
    );
    assert_eq!(
        bluetooth
            .remove_docs
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        vec!["Documentation/driver-api/btusb.rst"]
    );
    assert_eq!(
        bluetooth
            .tools
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        vec!["tools/perf"]
    );
    assert_eq!(
        bluetooth
            .remove_tools
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        vec!["tools/objtool"]
    );
    assert_eq!(
        bluetooth
            .samples
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        vec!["samples/bpf"]
    );
    assert_eq!(
        bluetooth
            .remove_samples
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        vec!["samples/hidraw"]
    );
    assert_eq!(
        bluetooth
            .kunit_suites
            .iter()
            .map(|suite| suite.as_str())
            .collect::<Vec<_>>(),
        vec!["bt_test"]
    );
    assert_eq!(
        bluetooth
            .remove_kunit_suites
            .iter()
            .map(|suite| suite.as_str())
            .collect::<Vec<_>>(),
        vec!["btusb-test"]
    );
    assert_eq!(
        bluetooth
            .kselftest_targets
            .iter()
            .map(|target| target.as_str())
            .collect::<Vec<_>>(),
        vec!["net"]
    );
    assert_eq!(
        bluetooth
            .remove_kselftest_targets
            .iter()
            .map(|target| target.as_str())
            .collect::<Vec<_>>(),
        vec!["bpf"]
    );
    assert_eq!(
        bluetooth
            .arch_scope
            .iter()
            .map(|arch| arch.as_str())
            .collect::<Vec<_>>(),
        vec!["arm64", "x86"]
    );
    assert_eq!(bluetooth.safety, Some(config::FeatureSafetyLevel::Surgical));
    assert!(bluetooth.allow_uapi_header_removal);
    assert!(bluetooth.require_clean_boot);
    assert!(bluetooth.report_only);

    let netfilter = resolved
        .feature_intent_plan
        .intents
        .iter()
        .find(|intent| intent.name == "netfilter")
        .unwrap();
    assert_eq!(netfilter.action, "preserve");
    assert!(netfilter.stable_id.starts_with("feature-intent-"));
    assert_eq!(
        netfilter
            .configs
            .iter()
            .map(|symbol| symbol.as_str())
            .collect::<Vec<_>>(),
        vec!["NETFILTER"]
    );
    assert_eq!(
        netfilter
            .exported_symbols
            .iter()
            .map(|symbol| symbol.as_str())
            .collect::<Vec<_>>(),
        vec!["nf_register_net_hook"]
    );
    assert_eq!(
        netfilter
            .module_names
            .iter()
            .map(|module| module.as_str())
            .collect::<Vec<_>>(),
        vec!["nf_conntrack"]
    );
    assert_eq!(
        netfilter
            .module_aliases
            .iter()
            .map(|alias| alias.as_str())
            .collect::<Vec<_>>(),
        vec!["of:N*T*Cqcom,ipq8064"]
    );
    assert_eq!(
        netfilter
            .device_compatibles
            .iter()
            .map(|compatible| compatible.as_str())
            .collect::<Vec<_>>(),
        vec!["brcm,bcm2835-aux-uart"]
    );
    assert_eq!(
        netfilter
            .acpi_ids
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        vec!["PRP0001"]
    );
    assert_eq!(
        netfilter
            .pci_ids
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        vec!["1AF4:1000"]
    );
    assert_eq!(
        netfilter
            .usb_ids
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        vec!["1D6B:0002"]
    );
    assert_eq!(
        netfilter
            .firmware_paths
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        vec!["qcom/venus-5.2/venus.mbn"]
    );
    assert_eq!(
        netfilter
            .initcalls
            .iter()
            .map(|initcall| initcall.as_str())
            .collect::<Vec<_>>(),
        vec!["nf_conntrack_standalone_init"]
    );
    assert_eq!(
        netfilter
            .runtime_registrations
            .iter()
            .map(|surface| surface.as_str())
            .collect::<Vec<_>>(),
        vec!["module_init:nf_conntrack_standalone_init"]
    );
    assert_eq!(
        netfilter
            .docs
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        vec!["Documentation/networking/nf_conntrack-sysctl.rst"]
    );
    assert_eq!(
        netfilter
            .tools
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        vec!["tools/testing/selftests/netfilter"]
    );
    assert_eq!(
        netfilter
            .samples
            .iter()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        vec!["samples/kobject"]
    );
    assert_eq!(
        netfilter
            .kunit_suites
            .iter()
            .map(|suite| suite.as_str())
            .collect::<Vec<_>>(),
        vec!["nf_conntrack_test"]
    );
    assert_eq!(
        netfilter
            .kselftest_targets
            .iter()
            .map(|target| target.as_str())
            .collect::<Vec<_>>(),
        vec!["drivers/net"]
    );
}

#[test]
fn test_resolved_feature_graph_fingerprint_tracks_feature_graph() {
    let config = config::default_kslim_config("demo", "/tmp/output");
    let mut first_profile = config::default_profile_config("v1.0");
    first_profile.features.remove.insert(
        String::from("bluetooth"),
        config::FeatureIntentConfig {
            roots: vec![String::from("net/bluetooth")],
            configs: vec![String::from("BT")],
            safety: Some(config::FeatureSafetyLevel::Surgical),
            ..config::FeatureIntentConfig::default()
        },
    );
    let mut reordered_profile = config::default_profile_config("v1.0");
    reordered_profile.features.remove.insert(
        String::from("bluetooth"),
        config::FeatureIntentConfig {
            configs: vec![String::from("BT")],
            roots: vec![String::from("./net//bluetooth")],
            safety: Some(config::FeatureSafetyLevel::Surgical),
            ..config::FeatureIntentConfig::default()
        },
    );
    let mut changed_profile = first_profile.clone();
    changed_profile
        .features
        .remove
        .get_mut("bluetooth")
        .unwrap()
        .safety = Some(config::FeatureSafetyLevel::Conservative);

    let first = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &first_profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap();
    let reordered = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &reordered_profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap();
    let changed = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &changed_profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap();

    assert_eq!(
        first.feature_graph_fingerprint,
        reordered.feature_graph_fingerprint
    );
    assert_ne!(
        first.feature_graph_fingerprint,
        changed.feature_graph_fingerprint
    );
}

#[test]
fn test_removal_manifest_fingerprint_tracks_normalized_manifest() {
    let config = config::default_kslim_config("demo", "/tmp/output");
    let mut first_profile = config::default_profile_config("v1.0");
    first_profile.slim = Some(SlimConfig {
        remove_paths: vec![
            String::from("./drivers//net/wireless/foo.c"),
            String::from("drivers/net/wireless/foo.c"),
        ],
        remove_configs: vec![String::from("CONFIG_BT"), String::from("CONFIG_BT")],
        set_defaults: BTreeMap::from([(String::from("CONFIG_FOO"), String::from("n"))]),
        unsafe_allow_root_path_removal: false,
    });
    first_profile.features.preserve.insert(
        String::from("netfilter"),
        config::FeatureIntentConfig {
            roots: vec![String::from("net/netfilter")],
            configs: vec![String::from("NETFILTER")],
            ..config::FeatureIntentConfig::default()
        },
    );

    let mut reordered_profile = config::default_profile_config("v1.0");
    reordered_profile.slim = Some(SlimConfig {
        remove_paths: vec![String::from("drivers/net/wireless/foo.c")],
        remove_configs: vec![String::from("CONFIG_BT")],
        set_defaults: BTreeMap::from([(String::from("CONFIG_FOO"), String::from("n"))]),
        unsafe_allow_root_path_removal: false,
    });
    reordered_profile.features.preserve.insert(
        String::from("netfilter"),
        config::FeatureIntentConfig {
            configs: vec![String::from("NETFILTER")],
            roots: vec![String::from("./net//netfilter")],
            ..config::FeatureIntentConfig::default()
        },
    );

    let mut changed_profile = first_profile.clone();
    changed_profile
        .slim
        .as_mut()
        .unwrap()
        .remove_configs
        .push(String::from("CONFIG_BT_CHANGED"));

    let first = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &first_profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap();
    let reordered = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &reordered_profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap();
    let changed = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &changed_profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap();

    assert_eq!(
        first.removal_manifest_fingerprint,
        reordered.removal_manifest_fingerprint
    );
    assert_ne!(
        first.removal_manifest_fingerprint,
        changed.removal_manifest_fingerprint
    );
}

#[test]
fn test_abi_policy_fingerprint_tracks_effective_abi_policy() {
    let config = config::default_kslim_config("demo", "/tmp/output");
    let mut direct_profile = config::default_profile_config("v1.0");
    direct_profile.abi.allow_public_header_removal = true;
    direct_profile.abi.allow_uapi_header_removal = true;

    let mut named_profile = config::default_profile_config("v1.0");
    named_profile.abi.allow_public_header_removal = true;
    named_profile.features.remove.insert(
        String::from("uapi"),
        config::FeatureIntentConfig {
            remove_paths: vec![String::from("include/uapi/linux/uapi-demo.h")],
            allow_uapi_header_removal: true,
            ..config::FeatureIntentConfig::default()
        },
    );

    let mut changed_profile = direct_profile.clone();
    changed_profile.abi.allow_uapi_header_removal = false;

    let direct = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &direct_profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap();
    let named = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &named_profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap();
    let changed = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &changed_profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap();

    assert_eq!(direct.abi_policy_fingerprint, named.abi_policy_fingerprint);
    assert_ne!(
        direct.abi_policy_fingerprint,
        changed.abi_policy_fingerprint
    );
}

#[test]
fn test_arch_policy_fingerprint_tracks_normalized_arch_policy() {
    let config = config::default_kslim_config("demo", "/tmp/output");
    let mut first_profile = config::default_profile_config("v1.0");
    first_profile.arch = config::ArchPolicyConfig {
        primary_arch: Some(String::from(" x86 ")),
        secondary_arches: vec![String::from("riscv"), String::from("arm64")],
        disabled_arches: vec![String::from("m68k")],
        allow_arch_local_removal: true,
        preserve_arch_shared: false,
    };

    let mut reordered_profile = config::default_profile_config("v1.0");
    reordered_profile.arch = config::ArchPolicyConfig {
        primary_arch: Some(String::from("x86")),
        secondary_arches: vec![String::from("arm64"), String::from("riscv")],
        disabled_arches: vec![String::from("m68k")],
        allow_arch_local_removal: true,
        preserve_arch_shared: false,
    };

    let mut changed_profile = first_profile.clone();
    changed_profile.arch.disabled_arches = vec![String::from("s390")];

    let first = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &first_profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap();
    let reordered = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &reordered_profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap();
    let changed = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &changed_profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap();

    assert_eq!(
        first.arch_policy_fingerprint,
        reordered.arch_policy_fingerprint
    );
    assert_ne!(
        first.arch_policy_fingerprint,
        changed.arch_policy_fingerprint
    );
}

#[test]
fn test_feature_resolution_state_resolves_named_feature_remove_input() {
    let mut profile = config::default_profile_config("v1.0");
    profile.features.remove.insert(
        String::from("bluetooth"),
        config::FeatureIntentConfig {
            roots: vec![
                String::from("net/bluetooth"),
                String::from("drivers/bluetooth"),
            ],
            remove_paths: vec![String::from("net/rfkill")],
            configs: vec![String::from("BT")],
            remove_configs: vec![String::from("BT_HCIBTUSB")],
            safety: Some(config::FeatureSafetyLevel::Surgical),
            arch_scope: vec![String::from("arm64"), String::from("x86")],
            require_clean_boot: true,
            report_only: true,
            ..config::FeatureIntentConfig::default()
        },
    );

    let resolution = FeatureResolutionState::from_profile(&profile).unwrap();

    assert_eq!(
        resolution.source(),
        FeatureResolutionSource::NamedFeatureRemove
    );
    assert_eq!(
        resolution
            .remove_paths()
            .iter()
            .map(|path| path.as_path())
            .collect::<Vec<_>>(),
        vec![
            Path::new("drivers/bluetooth"),
            Path::new("net/bluetooth"),
            Path::new("net/rfkill")
        ]
    );
    assert_eq!(
        resolution
            .remove_configs()
            .iter()
            .map(|symbol| symbol.as_str())
            .collect::<Vec<_>>(),
        vec!["BT", "BT_HCIBTUSB"]
    );
    assert_eq!(
        resolution.feature_safety_levels().get("bluetooth"),
        Some(&config::FeatureSafetyLevel::Surgical)
    );
    assert_eq!(
        resolution
            .feature_arch_scopes()
            .get("bluetooth")
            .unwrap()
            .iter()
            .map(|arch| arch.as_str())
            .collect::<Vec<_>>(),
        vec!["arm64", "x86"]
    );
    assert_eq!(
        resolution
            .feature_test_matrices()
            .get("bluetooth")
            .map(|matrix| matrix.require_clean_boot),
        Some(true)
    );
    assert_eq!(
        resolution
            .feature_report_modes()
            .get("bluetooth")
            .map(|mode| mode.report_only),
        Some(true)
    );
}

#[test]
fn test_feature_resolution_state_resolves_named_feature_preserve_input() {
    let mut profile = config::default_profile_config("v1.0");
    profile.features.preserve.insert(
        String::from("netfilter"),
        config::FeatureIntentConfig {
            roots: vec![String::from("net/netfilter")],
            configs: vec![String::from("NETFILTER")],
            arch_scope: vec![String::from("x86")],
            require_clean_boot: true,
            report_only: true,
            ..config::FeatureIntentConfig::default()
        },
    );

    let resolution = FeatureResolutionState::from_profile(&profile).unwrap();

    assert_eq!(resolution.source(), FeatureResolutionSource::NoRemoval);
    assert!(resolution.is_noop());
    assert_eq!(
        resolution
            .preserve_paths()
            .iter()
            .map(|path| path.as_path())
            .collect::<Vec<_>>(),
        vec![Path::new("net/netfilter")]
    );
    assert_eq!(
        resolution
            .preserve_configs()
            .iter()
            .map(|symbol| symbol.as_str())
            .collect::<Vec<_>>(),
        vec!["NETFILTER"]
    );
    assert_eq!(
        resolution
            .feature_arch_scopes()
            .get("netfilter")
            .unwrap()
            .iter()
            .map(|arch| arch.as_str())
            .collect::<Vec<_>>(),
        vec!["x86"]
    );
    assert_eq!(
        resolution
            .feature_test_matrices()
            .get("netfilter")
            .map(|matrix| matrix.require_clean_boot),
        Some(true)
    );
    assert_eq!(
        resolution
            .feature_report_modes()
            .get("netfilter")
            .map(|mode| mode.report_only),
        Some(true)
    );
}

#[test]
fn test_feature_resolution_state_rejects_arch_scope_without_feature_input() {
    let mut profile = config::default_profile_config("v1.0");
    profile.features.remove.insert(
        String::from("bluetooth"),
        config::FeatureIntentConfig {
            arch_scope: vec![String::from("x86")],
            ..config::FeatureIntentConfig::default()
        },
    );

    let err = FeatureResolutionState::from_profile(&profile)
        .unwrap_err()
        .to_string();

    assert!(err.contains("features.remove.bluetooth.arch_scope requires feature input"));
}

#[test]
fn test_feature_resolution_state_rejects_test_matrix_without_feature_input() {
    let mut profile = config::default_profile_config("v1.0");
    profile.features.remove.insert(
        String::from("bluetooth"),
        config::FeatureIntentConfig {
            require_clean_boot: true,
            ..config::FeatureIntentConfig::default()
        },
    );

    let err = FeatureResolutionState::from_profile(&profile)
        .unwrap_err()
        .to_string();

    assert!(err.contains("features.remove.bluetooth.require_clean_boot requires feature input"));
}

#[test]
fn test_feature_resolution_state_rejects_report_only_without_feature_input() {
    let mut profile = config::default_profile_config("v1.0");
    profile.features.remove.insert(
        String::from("bluetooth"),
        config::FeatureIntentConfig {
            report_only: true,
            ..config::FeatureIntentConfig::default()
        },
    );

    let err = FeatureResolutionState::from_profile(&profile)
        .unwrap_err()
        .to_string();

    assert!(err.contains("features.remove.bluetooth.report_only requires feature input"));
}

#[test]
fn test_feature_resolution_state_rejects_safety_without_named_removal_input() {
    let mut profile = config::default_profile_config("v1.0");
    profile.slim = Some(config::SlimConfig {
        remove_paths: vec![String::from("drivers/gpu/drm/amd/amdgpu")],
        remove_configs: Vec::new(),
        set_defaults: BTreeMap::new(),
        unsafe_allow_root_path_removal: false,
    });
    profile.features.remove.insert(
        String::from("bluetooth"),
        config::FeatureIntentConfig {
            safety: Some(config::FeatureSafetyLevel::Surgical),
            ..config::FeatureIntentConfig::default()
        },
    );

    let err = FeatureResolutionState::from_profile(&profile)
        .unwrap_err()
        .to_string();

    assert!(err.contains("features.remove.bluetooth.safety requires removal input"));
}

#[test]
fn test_feature_resolution_state_rejects_no_input_with_removal_facts() {
    let err = FeatureResolutionState::new(
        FeatureResolutionSource::NoRemoval,
        vec![RelativeKernelPath::new("drivers/remove").unwrap()],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        BTreeMap::new(),
        AbiPolicyConfig::default(),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        false,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("without removal input cannot contain removal facts"));
}

#[test]
fn test_abi_decision_state_rejects_public_header_or_uapi_without_matching_policy() {
    let err = AbiDecisionState::new(
        AbiPolicyConfig::default(),
        &[RelativeKernelPath::new("include/linux/public.h").unwrap()],
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("ABI decision rejected public header removal"));
    assert!(err.contains("abi.allow_public_header_removal"));

    let public_only = AbiPolicyConfig {
        allow_public_header_removal: true,
        allow_uapi_header_removal: false,
    };
    let err = AbiDecisionState::new(
        public_only,
        &[RelativeKernelPath::new("include/uapi/linux/abi.h").unwrap()],
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("ABI decision rejected UAPI removal"));
    assert!(err.contains("abi.allow_uapi_header_removal"));
}

#[test]
fn test_abi_decision_state_records_approved_abi_sensitive_removals() {
    let policy = AbiPolicyConfig {
        allow_public_header_removal: true,
        allow_uapi_header_removal: true,
    };
    let decision = AbiDecisionState::new(
        policy.clone(),
        &[
            RelativeKernelPath::new("include/uapi/linux/abi.h").unwrap(),
            RelativeKernelPath::new("include/linux/public.h").unwrap(),
            RelativeKernelPath::new("include/linux/public.h").unwrap(),
            RelativeKernelPath::new("drivers/private.c").unwrap(),
        ],
    )
    .unwrap();

    assert_eq!(decision.policy(), &policy);
    assert!(decision.allow_public_header_removal());
    assert!(decision.allow_uapi_header_removal());
    assert!(decision.has_abi_sensitive_removals());
    assert_eq!(
        decision
            .approved_public_headers()
            .iter()
            .map(HeaderPath::as_str)
            .collect::<Vec<_>>(),
        vec!["include/linux/public.h"]
    );
    assert_eq!(
        decision
            .approved_uapi_paths()
            .iter()
            .map(UapiPath::as_str)
            .collect::<Vec<_>>(),
        vec!["include/uapi/linux/abi.h"]
    );
}

#[test]
fn test_resolved_candidate_state_rejects_empty_output_plan() {
    let config = config::default_kslim_config("demo", "");
    let profile = config::default_profile_config("v1.0");
    let err = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("resolved output path is empty"));
}

#[test]
fn test_resolved_candidate_state_rejects_invalid_kernel_build_dir() {
    let config = config::default_kslim_config("demo", "/tmp/output");
    let mut profile = config::default_profile_config("v1.0");
    profile.selftests.kernel_builds = vec![KernelBuildConfig {
        name: Some(String::from("bad-build-dir")),
        config_target: Some(String::from("defconfig")),
        targets: Vec::new(),
        output_dir: Some(String::from("../build")),
        jobs: None,
        clean: true,
        make_program: None,
        make_args: Vec::new(),
        env: BTreeMap::new(),
    }];

    let err = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("kernel build dir path"));
    assert!(err.contains("parent components"));
}

#[test]
fn test_resolved_candidate_state_rejects_invalid_arch_name() {
    let config = config::default_kslim_config("demo", "/tmp/output");
    let mut profile = config::default_profile_config("v1.0");
    profile.selftests.kernel_builds = vec![KernelBuildConfig {
        name: Some(String::from("bad-arch")),
        config_target: Some(String::from("defconfig")),
        targets: Vec::new(),
        output_dir: None,
        jobs: None,
        clean: true,
        make_program: None,
        make_args: Vec::new(),
        env: BTreeMap::from([(String::from("ARCH"), String::from("x86/../../host"))]),
    }];

    let err = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("kernel architecture name"));
    assert!(err.contains("invalid characters"));
}

#[test]
fn test_resolved_candidate_state_rejects_invalid_kconfig_symbol() {
    let config = config::default_kslim_config("demo", "/tmp/output");
    let mut profile = config::default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: Vec::new(),
        remove_configs: vec![String::from("DRM_AMDGPU || DRM_RADEON")],
        set_defaults: BTreeMap::new(),
        unsafe_allow_root_path_removal: false,
    });

    let err = format!(
        "{:#}",
        ResolvedCandidateState::from_resolved_inputs(
            &config,
            &profile,
            test_resolved_base(),
            None,
            "slimmed",
            "kslim/test",
        )
        .unwrap_err()
    );

    assert!(err.contains("Kconfig symbol"));
    assert!(err.contains("invalid characters"));
}

#[test]
fn test_resolved_candidate_state_allows_root_removal_only_with_unsafe_mode() {
    let config = config::default_kslim_config("demo", "/tmp/output");
    let mut profile = config::default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec![String::from(".")],
        remove_configs: Vec::new(),
        set_defaults: BTreeMap::new(),
        unsafe_allow_root_path_removal: false,
    });

    let err = format!(
        "{:#}",
        ResolvedCandidateState::from_resolved_inputs(
            &config,
            &profile,
            test_resolved_base(),
            None,
            "slimmed",
            "kslim/test",
        )
        .unwrap_err()
    );
    assert!(err.contains("slim.unsafe_allow_root_path_removal"));

    profile
        .slim
        .as_mut()
        .unwrap()
        .unsafe_allow_root_path_removal = true;
    let resolved = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &profile,
        test_resolved_base(),
        None,
        "slimmed",
        "kslim/test",
    )
    .unwrap();
    assert!(resolved.prune_plan.unsafe_allow_root_path_removal);
    assert_eq!(
        resolved
            .prune_plan
            .remove_paths
            .iter()
            .map(|path| path.as_path())
            .collect::<Vec<_>>(),
        vec![Path::new(".")]
    );
}

