use super::*;

fn config_with_roots() -> FeatureIntentConfig {
    FeatureIntentConfig {
        kind: Some(String::from(" subsystem ")),
        roots: vec![
            String::from("drivers/zeta"),
            String::from("drivers/alpha"),
            String::from("drivers/alpha"),
        ],
        configs: vec![String::from("CONFIG_ZETA"), String::from("CONFIG_ALPHA")],
        exported_symbols: vec![String::from("zeta_api"), String::from("alpha_api")],
        remove_exported_symbols: vec![String::from("extra_removed_api")],
        module_names: vec![String::from("zeta-mod"), String::from("alpha_mod")],
        remove_module_names: vec![String::from("extra_removed_mod")],
        module_aliases: vec![
            String::from("usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*"),
            String::from("pci:v00008086d00001572sv*sd*bc*sc*i*"),
        ],
        remove_module_aliases: vec![String::from("of:N*T*Cqcom,ipq8064")],
        device_compatibles: vec![
            String::from("qcom,ipq8064"),
            String::from("brcm,bcm2835-aux-uart"),
        ],
        remove_device_compatibles: vec![String::from("vendor,removed-device")],
        acpi_ids: vec![String::from("PNP0C09"), String::from("ACPI0003")],
        remove_acpi_ids: vec![String::from("INT33A1")],
        pci_ids: vec![String::from("8086:1572"), String::from("10EC:8168")],
        remove_pci_ids: vec![String::from("1AF4:1000")],
        usb_ids: vec![String::from("0BDA:8153"), String::from("046D:C52B")],
        remove_usb_ids: vec![String::from("1D6B:0002")],
        firmware_paths: vec![
            String::from("amdgpu/polaris10_mc.bin"),
            String::from("iwlwifi-7260-17.ucode"),
        ],
        remove_firmware_paths: vec![String::from("qcom/venus-5.2/venus.mbn")],
        initcalls: vec![String::from("bt_init"), String::from("btusb_driver_init")],
        remove_initcalls: vec![String::from("rfkill_init")],
        runtime_registrations: vec![
            String::from("module_init:bt_init"),
            String::from("module_platform_driver:btusb_driver"),
        ],
        remove_runtime_registrations: vec![String::from("register_netdev:bt_netdev")],
        docs: vec![
            String::from("Documentation/networking/bluetooth.rst"),
            String::from("Documentation/driver-api/bluetooth.rst"),
        ],
        remove_docs: vec![String::from("Documentation/driver-api/btusb.rst")],
        tools: vec![String::from("tools/perf"), String::from("tools/objtool")],
        remove_tools: vec![String::from("tools/testing/selftests/bluetooth")],
        samples: vec![String::from("samples/bpf"), String::from("samples/hidraw")],
        remove_samples: vec![String::from("samples/auxdisplay")],
        kunit_suites: vec![String::from("bt_test"), String::from("btusb-test")],
        remove_kunit_suites: vec![String::from("bt_l2cap_test")],
        kselftest_targets: vec![String::from("net"), String::from("bpf")],
        remove_kselftest_targets: vec![String::from("drivers/net")],
        arch_scope: vec![String::from("x86"), String::from("arm64")],
        safety: Some(FeatureSafetyLevel::Surgical),
        require_clean_boot: true,
        report_only: true,
        ..FeatureIntentConfig::default()
    }
}

#[test]
fn feature_id_normalizes_named_feature_identity() {
    let id = FeatureId::new(" bluetooth ").unwrap();
    assert_eq!(id.as_str(), "bluetooth");

    let err = FeatureId::new(" ").unwrap_err();
    assert!(format!("{err:#}").contains("feature name must not be empty"));
}

#[test]
fn feature_kind_uses_stable_tokens() {
    assert_eq!(
        FeatureKind::ALL
            .into_iter()
            .map(FeatureKind::stable_name)
            .collect::<Vec<_>>(),
        vec![
            "subsystem",
            "driver",
            "bus",
            "filesystem",
            "network_protocol",
            "crypto_algorithm",
            "scheduler_feature",
            "security_feature",
            "tracing_feature",
            "bpf_feature",
            "arch_feature",
            "soc_platform",
            "board_platform_support",
            "firmware_loader_feature",
            "module_only_feature",
            "userspace_abi_feature",
            "generated_artifact_family",
            "docs_tests_tools_only_feature",
        ]
    );
    assert_eq!(
        FeatureKind::from_stable_name("network protocol")
            .unwrap()
            .stable_name(),
        "network_protocol"
    );
    assert_eq!(
        FeatureKind::from_stable_name("docs-tests-tools-only-feature")
            .unwrap()
            .stable_name(),
        "docs_tests_tools_only_feature"
    );

    let err = FeatureKind::from_stable_name("unknown").unwrap_err();
    assert!(format!("{err:#}").contains("unsupported feature kind"));
}

#[test]
fn feature_root_normalizes_relative_kernel_root() {
    let root = FeatureRoot::new("./drivers//bluetooth").unwrap();
    assert_eq!(root.as_path(), Path::new("drivers/bluetooth"));
    assert_eq!(
        root.as_relative_kernel_path().as_path(),
        Path::new("drivers/bluetooth")
    );

    let absolute = FeatureRoot::new("/drivers/bluetooth").unwrap_err();
    assert!(format!("{absolute:#}").contains("relative kernel path"));

    let traversal = FeatureRoot::new("../drivers/bluetooth").unwrap_err();
    assert!(format!("{traversal:#}").contains("parent components"));
}

#[test]
fn feature_scope_normalizes_arch_scope() {
    let scope = FeatureScope::from_arch_scope(&[
        String::from("x86"),
        String::from("arm64"),
        String::from("x86"),
    ])
    .unwrap();
    assert!(!scope.is_unscoped());
    assert_eq!(scope.stable_key(), "arm64|x86");
    assert_eq!(
        scope
            .arch_scope()
            .iter()
            .map(|arch| arch.as_str())
            .collect::<Vec<_>>(),
        vec!["arm64", "x86"]
    );

    assert!(FeatureScope::unscoped().is_unscoped());

    let invalid = FeatureScope::from_arch_scope(&[String::from("bad/arch")]).unwrap_err();
    assert!(format!("{invalid:#}").contains("kernel architecture name"));
}

#[test]
fn feature_node_carries_intent_identity_without_resolved_graph_state() {
    let intent = FeatureIntent::from_config(
        FeatureIntentAction::Remove,
        " bluetooth ",
        &config_with_roots(),
    )
    .unwrap();

    let node = FeatureNode::from_intent(intent);

    assert_eq!(node.id().as_str(), "bluetooth");
    assert_eq!(node.stable_key(), "feature:bluetooth");
    assert_eq!(node.intent().action, FeatureIntentAction::Remove);
    assert_eq!(node.intent().roots_key(), "drivers/alpha|drivers/zeta");
}

#[test]
fn feature_edge_uses_stable_directed_semantic_relationships() {
    let edge =
        FeatureEdge::from_names(FeatureEdgeKind::Dependency, " bluetooth ", " netfilter ").unwrap();

    assert_eq!(edge.kind().stable_name(), "dependency");
    assert_eq!(edge.from().as_str(), "bluetooth");
    assert_eq!(edge.to().as_str(), "netfilter");
    assert_eq!(edge.stable_key(), "dependency:bluetooth->netfilter");
    assert_eq!(
        FeatureEdgeKind::from_stable_name("preservation boundary")
            .unwrap()
            .stable_name(),
        "preservation_boundary"
    );

    let self_edge =
        FeatureEdge::from_names(FeatureEdgeKind::Conflict, "bluetooth", "bluetooth").unwrap_err();
    assert!(format!("{self_edge:#}").contains("endpoints must be distinct"));
}

#[test]
fn feature_ownership_uses_stable_semantic_classifications() {
    assert_eq!(
        FeatureOwnershipKind::ALL
            .into_iter()
            .map(FeatureOwnershipKind::stable_name)
            .collect::<Vec<_>>(),
        vec![
            "explicitly_removed",
            "explicitly_preserved",
            "owned_solely_by_removed_feature",
            "shared_with_live_feature",
            "generated_by_live_build",
            "public_abi_surface",
            "public_uapi_surface",
            "arch_local",
            "arch_shared",
            "runtime_only_surface",
            "test_only_surface",
            "documentation_only_surface",
            "unknown_ownership",
            "ambiguous_ownership",
            "unsupported_ownership",
        ]
    );

    let ownership = FeatureOwnership::from_name(
        FeatureOwnershipKind::ExplicitlyRemoved,
        " bluetooth ",
        " path:drivers/bluetooth ",
    )
    .unwrap();

    assert_eq!(ownership.kind().stable_name(), "explicitly_removed");
    assert_eq!(ownership.feature().as_str(), "bluetooth");
    assert_eq!(ownership.subject().as_str(), "path:drivers/bluetooth");
    assert_eq!(
        ownership.stable_key(),
        "explicitly_removed:bluetooth:path:drivers/bluetooth"
    );
    assert_eq!(
        FeatureOwnershipKind::from_stable_name("public ABI surface")
            .unwrap()
            .stable_name(),
        "public_abi_surface"
    );
}

#[test]
fn feature_ownership_rejects_invalid_subject_and_kind() {
    let empty = FeatureOwnershipSubject::new(" ").unwrap_err();
    assert!(format!("{empty:#}").contains("ownership subject must not be empty"));

    let control = FeatureOwnershipSubject::new("path:drivers\nbluetooth").unwrap_err();
    assert!(format!("{control:#}").contains("control characters"));

    let unknown = FeatureOwnershipKind::from_stable_name("not ownership").unwrap_err();
    assert!(format!("{unknown:#}").contains("unsupported feature ownership kind"));
}

#[test]
fn feature_impact_report_counts_profile_and_selected_feature_impact() {
    let mut profile = crate::config::default_profile_config("v1.0");
    profile.features.remove.insert(
        String::from("bluetooth"),
        FeatureIntentConfig {
            roots: vec![
                String::from("net/bluetooth"),
                String::from("drivers/bluetooth"),
            ],
            configs: vec![String::from("BT")],
            ..FeatureIntentConfig::default()
        },
    );
    profile.features.preserve.insert(
        String::from("netfilter"),
        FeatureIntentConfig {
            roots: vec![String::from("net/netfilter")],
            configs: vec![String::from("NETFILTER")],
            ..FeatureIntentConfig::default()
        },
    );

    let report = FeatureImpactReport::from_profile(&profile);

    assert_eq!(report.remove_paths(), 2);
    assert_eq!(report.remove_configs(), 1);
    assert_eq!(report.default_overrides(), 0);
    assert_eq!(report.preserve_paths(), 1);
    assert_eq!(report.preserve_configs(), 1);
    assert_eq!(report.ownership_count(), 0);
    assert!(!report.is_empty());

    let bluetooth = FeatureImpactReport::for_feature(&profile, " bluetooth ").unwrap();
    assert_eq!(bluetooth.remove_paths(), 2);
    assert_eq!(bluetooth.remove_configs(), 1);
    assert_eq!(bluetooth.preserve_paths(), 0);

    let netfilter = FeatureImpactReport::for_feature(&profile, "netfilter").unwrap();
    assert_eq!(netfilter.remove_paths(), 0);
    assert_eq!(netfilter.preserve_paths(), 1);
    assert_eq!(netfilter.preserve_configs(), 1);
}

#[test]
fn feature_impact_report_sorts_ownerships_and_rejects_invalid_filter() {
    let first = FeatureOwnership::from_name(
        FeatureOwnershipKind::ExplicitlyRemoved,
        "bluetooth",
        "path:drivers/bluetooth",
    )
    .unwrap();
    let second = FeatureOwnership::from_name(
        FeatureOwnershipKind::PublicAbiSurface,
        "bluetooth",
        "path:include/net/bluetooth.h",
    )
    .unwrap();

    let report = FeatureImpactReport::default().with_ownerships([second, first]);

    assert_eq!(report.ownership_count(), 2);
    assert_eq!(
        report
            .ownerships()
            .iter()
            .map(FeatureOwnership::stable_key)
            .collect::<Vec<_>>(),
        vec![
            "explicitly_removed:bluetooth:path:drivers/bluetooth",
            "public_abi_surface:bluetooth:path:include/net/bluetooth.h",
        ]
    );

    let profile = crate::config::default_profile_config("v1.0");
    let invalid = FeatureImpactReport::for_feature(&profile, " ").unwrap_err();
    assert!(format!("{invalid:#}").contains("feature name must not be empty"));
}

#[test]
fn feature_conflict_report_sorts_actionable_conflicts() {
    assert_eq!(
        FeatureConflictKind::ALL
            .into_iter()
            .map(FeatureConflictKind::stable_name)
            .collect::<Vec<_>>(),
        vec![
            "removed_feature_owns_live_dependency",
            "removed_feature_selected_by_live_kconfig",
            "removed_feature_referenced_by_live_kbuild",
            "removed_feature_exports_consumed_symbol",
            "removed_feature_device_id_referenced_by_live_table",
            "removed_feature_uapi_referenced_by_userspace_facing_code",
            "removed_feature_runtime_registration_reachable",
            "shared_file_between_removed_and_preserved_features",
        ]
    );

    let live_dependency = FeatureConflict::from_name(
        FeatureConflictKind::RemovedFeatureOwnsLiveDependency,
        "bluetooth",
        "feature:rfkill",
        "removed feature owns a live dependency",
        "preserve the dependency or remove the live consumer",
    )
    .unwrap();
    let kconfig = FeatureConflict::from_name(
        FeatureConflictKind::RemovedFeatureSelectedByLiveKconfig,
        "bluetooth",
        "kconfig:BT",
        "removed feature is still selected by live Kconfig",
        "remove the selector or preserve the feature",
    )
    .unwrap()
    .non_blocking();

    let report = FeatureConflictReport::new([kconfig, live_dependency]).unwrap();

    assert_eq!(report.len(), 2);
    assert!(!report.is_empty());
    assert!(report.has_blocking_conflicts());
    assert_eq!(report.blocking_count(), 1);
    assert_eq!(
        report
            .conflicts()
            .iter()
            .map(FeatureConflict::stable_key)
            .collect::<Vec<_>>(),
        vec![
            "removed_feature_owns_live_dependency:bluetooth:feature:rfkill",
            "removed_feature_selected_by_live_kconfig:bluetooth:kconfig:BT",
        ]
    );
    let first = &report.conflicts()[0];
    assert_eq!(
        first.kind().stable_name(),
        "removed_feature_owns_live_dependency"
    );
    assert_eq!(first.feature().as_str(), "bluetooth");
    assert_eq!(first.subject().as_str(), "feature:rfkill");
    assert_eq!(first.summary(), "removed feature owns a live dependency");
    assert_eq!(
        first.suggested_action(),
        "preserve the dependency or remove the live consumer"
    );
}

#[test]
fn feature_conflict_report_rejects_duplicate_and_invalid_conflicts() {
    let conflict = FeatureConflict::from_name(
        FeatureConflictKind::RemovedFeatureReferencedByLiveKbuild,
        "bluetooth",
        "kbuild:drivers/bluetooth/",
        "removed feature is still referenced by live kbuild",
        "remove stale kbuild references",
    )
    .unwrap();

    let duplicate = FeatureConflictReport::new([conflict.clone(), conflict]).unwrap_err();
    assert!(format!("{duplicate:#}").contains("duplicate conflict"));

    let empty_summary = FeatureConflict::from_name(
        FeatureConflictKind::RemovedFeatureExportsConsumedSymbol,
        "bluetooth",
        "symbol:bt_sock_register",
        " ",
        "preserve provider or remove consumer",
    )
    .unwrap_err();
    assert!(format!("{empty_summary:#}").contains("conflict summary must not be empty"));

    let unknown = FeatureConflictKind::from_stable_name("not conflict").unwrap_err();
    assert!(format!("{unknown:#}").contains("unsupported feature conflict kind"));
}

#[test]
fn feature_conflict_report_rejects_blocking_conflicts_in_strict_mode() {
    let blocking = FeatureConflict::from_name(
        FeatureConflictKind::SharedFileBetweenRemovedAndPreservedFeatures,
        "bluetooth",
        "path:drivers/bluetooth/btusb.c",
        "removed feature shares a file with a preserved feature",
        "split the shared file or preserve the removed feature",
    )
    .unwrap();
    let relaxed = FeatureConflict::from_name(
        FeatureConflictKind::RemovedFeatureSelectedByLiveKconfig,
        "bluetooth",
        "kconfig:BT",
        "removed feature is still selected by live Kconfig",
        "remove the selector or preserve the feature",
    )
    .unwrap()
    .non_blocking();
    let report = FeatureConflictReport::new([relaxed, blocking]).unwrap();

    report
        .reject_blocking_conflicts_in_strict_mode(false)
        .unwrap();
    let err = report
        .reject_blocking_conflicts_in_strict_mode(true)
        .unwrap_err();
    let err = format!("{err:#}");

    assert!(err.contains("unresolved feature conflicts block strict mutation"));
    assert!(err.contains(
        "shared_file_between_removed_and_preserved_features:bluetooth:path:drivers/bluetooth/btusb.c"
    ));
    assert!(err.contains("summary: removed feature shares a file with a preserved feature"));
    assert!(err.contains("action: split the shared file or preserve the removed feature"));
    assert!(!err.contains("removed_feature_selected_by_live_kconfig"));
}

#[test]
fn feature_graph_collects_profile_intents_by_feature_id() {
    let mut profile = crate::config::default_profile_config("v1.0");
    profile.features.remove.insert(
        String::from("bluetooth"),
        FeatureIntentConfig {
            roots: vec![String::from("net/bluetooth")],
            configs: vec![String::from("BT")],
            ..FeatureIntentConfig::default()
        },
    );
    profile.features.preserve.insert(
        String::from("netfilter"),
        FeatureIntentConfig {
            roots: vec![String::from("net/netfilter")],
            configs: vec![String::from("NETFILTER")],
            ..FeatureIntentConfig::default()
        },
    );

    let graph = FeatureGraph::from_profile(&profile).unwrap();

    assert_eq!(graph.len(), 2);
    assert!(!graph.is_empty());
    assert_eq!(graph.edge_count(), 0);
    assert_eq!(
        graph
            .nodes()
            .map(|node| node.id().as_str())
            .collect::<Vec<_>>(),
        vec!["bluetooth", "netfilter"]
    );
    assert_eq!(
        graph
            .intents()
            .map(|intent| intent.id.as_str())
            .collect::<Vec<_>>(),
        vec!["bluetooth", "netfilter"]
    );
    assert_eq!(
        graph
            .get(&FeatureId::new("bluetooth").unwrap())
            .unwrap()
            .intent()
            .action,
        FeatureIntentAction::Remove
    );
    assert_eq!(
        graph
            .get(&FeatureId::new("netfilter").unwrap())
            .unwrap()
            .intent()
            .action,
        FeatureIntentAction::Preserve
    );
}

#[test]
fn feature_graph_validates_feature_edges() {
    let bluetooth = FeatureIntent::from_config(
        FeatureIntentAction::Remove,
        "bluetooth",
        &config_with_roots(),
    )
    .unwrap();
    let netfilter = FeatureIntent::from_config(
        FeatureIntentAction::Remove,
        "netfilter",
        &FeatureIntentConfig {
            roots: vec![String::from("net/netfilter")],
            configs: vec![String::from("NETFILTER")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap();
    let edge =
        FeatureEdge::from_names(FeatureEdgeKind::Dependency, "bluetooth", "netfilter").unwrap();

    let graph =
        FeatureGraph::with_edges([bluetooth.clone(), netfilter.clone()], [edge.clone()]).unwrap();

    assert_eq!(graph.edge_count(), 1);
    assert_eq!(
        graph
            .edges()
            .map(FeatureEdge::stable_key)
            .collect::<Vec<_>>(),
        vec!["dependency:bluetooth->netfilter"]
    );

    let unknown_target = FeatureGraph::with_edges([bluetooth.clone()], [edge.clone()]).unwrap_err();
    assert!(format!("{unknown_target:#}").contains("unknown target feature: netfilter"));

    let duplicate =
        FeatureGraph::with_edges([bluetooth, netfilter], [edge.clone(), edge]).unwrap_err();
    assert!(format!("{duplicate:#}").contains("duplicate feature edge"));
}

#[test]
fn feature_graph_rejects_duplicate_feature_ids() {
    let intent = FeatureIntent::from_config(
        FeatureIntentAction::Remove,
        "bluetooth",
        &config_with_roots(),
    )
    .unwrap();

    let duplicate = FeatureGraph::new([intent.clone(), intent]).unwrap_err();

    assert!(format!("{duplicate:#}").contains("duplicate feature id: bluetooth"));
}

#[test]
fn feature_intent_normalizes_profile_config_into_typed_sorted_intent() {
    let intent = FeatureIntent::from_config(
        FeatureIntentAction::Remove,
        " bluetooth ",
        &config_with_roots(),
    )
    .unwrap();

    assert_eq!(intent.action.stable_name(), "remove");
    assert_eq!(intent.id.as_str(), "bluetooth");
    assert_eq!(intent.kind.map(FeatureKind::stable_name), Some("subsystem"));
    assert_eq!(intent.roots_key(), "drivers/alpha|drivers/zeta");
    assert_eq!(intent.configs_key(), "CONFIG_ALPHA|CONFIG_ZETA");
    assert_eq!(intent.exported_symbols_key(), "alpha_api|zeta_api");
    assert_eq!(intent.remove_exported_symbols_key(), "extra_removed_api");
    assert_eq!(intent.module_names_key(), "alpha_mod|zeta_mod");
    assert_eq!(intent.remove_module_names_key(), "extra_removed_mod");
    assert_eq!(
        intent.module_aliases_key(),
        "pci:v00008086d00001572sv*sd*bc*sc*i*|usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*"
    );
    assert_eq!(intent.remove_module_aliases_key(), "of:N*T*Cqcom,ipq8064");
    assert_eq!(
        intent.device_compatibles_key(),
        "brcm,bcm2835-aux-uart|qcom,ipq8064"
    );
    assert_eq!(
        intent.remove_device_compatibles_key(),
        "vendor,removed-device"
    );
    assert_eq!(intent.acpi_ids_key(), "ACPI0003|PNP0C09");
    assert_eq!(intent.remove_acpi_ids_key(), "INT33A1");
    assert_eq!(intent.pci_ids_key(), "10EC:8168|8086:1572");
    assert_eq!(intent.remove_pci_ids_key(), "1AF4:1000");
    assert_eq!(intent.usb_ids_key(), "046D:C52B|0BDA:8153");
    assert_eq!(intent.remove_usb_ids_key(), "1D6B:0002");
    assert_eq!(
        intent.firmware_paths_key(),
        "amdgpu/polaris10_mc.bin|iwlwifi-7260-17.ucode"
    );
    assert_eq!(
        intent.remove_firmware_paths_key(),
        "qcom/venus-5.2/venus.mbn"
    );
    assert_eq!(intent.initcalls_key(), "bt_init|btusb_driver_init");
    assert_eq!(intent.remove_initcalls_key(), "rfkill_init");
    assert_eq!(
        intent.runtime_registrations_key(),
        "module_init:bt_init|module_platform_driver:btusb_driver"
    );
    assert_eq!(
        intent.remove_runtime_registrations_key(),
        "register_netdev:bt_netdev"
    );
    assert_eq!(
        intent.docs_key(),
        "Documentation/driver-api/bluetooth.rst|Documentation/networking/bluetooth.rst"
    );
    assert_eq!(
        intent.remove_docs_key(),
        "Documentation/driver-api/btusb.rst"
    );
    assert_eq!(intent.tools_key(), "tools/objtool|tools/perf");
    assert_eq!(
        intent.remove_tools_key(),
        "tools/testing/selftests/bluetooth"
    );
    assert_eq!(intent.samples_key(), "samples/bpf|samples/hidraw");
    assert_eq!(intent.remove_samples_key(), "samples/auxdisplay");
    assert_eq!(intent.kunit_suites_key(), "bt_test|btusb-test");
    assert_eq!(intent.remove_kunit_suites_key(), "bt_l2cap_test");
    assert_eq!(intent.kselftest_targets_key(), "bpf|net");
    assert_eq!(intent.remove_kselftest_targets_key(), "drivers/net");
    assert_eq!(intent.arch_scope_key(), "arm64|x86");
    assert_eq!(intent.safety, Some(FeatureSafetyLevel::Surgical));
    assert!(intent.require_clean_boot);
    assert!(intent.report_only);
}

#[test]
fn feature_intent_rejects_empty_and_action_invalid_intent() {
    let empty_name =
        FeatureIntent::from_config(FeatureIntentAction::Remove, " ", &config_with_roots())
            .unwrap_err();
    assert!(format!("{empty_name:#}").contains("feature name must not be empty"));

    let empty_remove = FeatureIntent::from_config(
        FeatureIntentAction::Remove,
        "empty",
        &FeatureIntentConfig::default(),
    )
    .unwrap_err();
    assert!(format!("{empty_remove:#}").contains("must declare roots"));

    let preserve_remove_path = FeatureIntent::from_config(
        FeatureIntentAction::Preserve,
        "keep",
        &FeatureIntentConfig {
            roots: vec![String::from("drivers/keep")],
            remove_paths: vec![String::from("drivers/remove")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap_err();
    assert!(format!("{preserve_remove_path:#}").contains("remove_paths is removal-only"));

    let preserve_remove_export = FeatureIntent::from_config(
        FeatureIntentAction::Preserve,
        "keep",
        &FeatureIntentConfig {
            exported_symbols: vec![String::from("keep_api")],
            remove_exported_symbols: vec![String::from("remove_api")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap_err();
    assert!(
        format!("{preserve_remove_export:#}").contains("remove_exported_symbols is removal-only")
    );

    let preserve_remove_module = FeatureIntent::from_config(
        FeatureIntentAction::Preserve,
        "keep",
        &FeatureIntentConfig {
            module_names: vec![String::from("keep_mod")],
            remove_module_names: vec![String::from("remove_mod")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap_err();
    assert!(format!("{preserve_remove_module:#}").contains("remove_module_names is removal-only"));

    let preserve_remove_alias = FeatureIntent::from_config(
        FeatureIntentAction::Preserve,
        "keep",
        &FeatureIntentConfig {
            module_aliases: vec![String::from("usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*")],
            remove_module_aliases: vec![String::from("pci:v00008086d00001572sv*sd*bc*sc*i*")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap_err();
    assert!(format!("{preserve_remove_alias:#}").contains("remove_module_aliases is removal-only"));

    let preserve_remove_compatible = FeatureIntent::from_config(
        FeatureIntentAction::Preserve,
        "keep",
        &FeatureIntentConfig {
            device_compatibles: vec![String::from("qcom,ipq8064")],
            remove_device_compatibles: vec![String::from("vendor,removed-device")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap_err();
    assert!(format!("{preserve_remove_compatible:#}")
        .contains("remove_device_compatibles is removal-only"));

    let preserve_remove_acpi = FeatureIntent::from_config(
        FeatureIntentAction::Preserve,
        "keep",
        &FeatureIntentConfig {
            acpi_ids: vec![String::from("PNP0C09")],
            remove_acpi_ids: vec![String::from("ACPI0003")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap_err();
    assert!(format!("{preserve_remove_acpi:#}").contains("remove_acpi_ids is removal-only"));

    let preserve_remove_pci = FeatureIntent::from_config(
        FeatureIntentAction::Preserve,
        "keep",
        &FeatureIntentConfig {
            pci_ids: vec![String::from("8086:1572")],
            remove_pci_ids: vec![String::from("10EC:8168")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap_err();
    assert!(format!("{preserve_remove_pci:#}").contains("remove_pci_ids is removal-only"));

    let preserve_remove_usb = FeatureIntent::from_config(
        FeatureIntentAction::Preserve,
        "keep",
        &FeatureIntentConfig {
            usb_ids: vec![String::from("0BDA:8153")],
            remove_usb_ids: vec![String::from("046D:C52B")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap_err();
    assert!(format!("{preserve_remove_usb:#}").contains("remove_usb_ids is removal-only"));

    let preserve_remove_firmware = FeatureIntent::from_config(
        FeatureIntentAction::Preserve,
        "keep",
        &FeatureIntentConfig {
            firmware_paths: vec![String::from("amdgpu/polaris10_mc.bin")],
            remove_firmware_paths: vec![String::from("iwlwifi-7260-17.ucode")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap_err();
    assert!(
        format!("{preserve_remove_firmware:#}").contains("remove_firmware_paths is removal-only")
    );

    let preserve_remove_initcall = FeatureIntent::from_config(
        FeatureIntentAction::Preserve,
        "keep",
        &FeatureIntentConfig {
            initcalls: vec![String::from("bt_init")],
            remove_initcalls: vec![String::from("btusb_driver_init")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap_err();
    assert!(format!("{preserve_remove_initcall:#}").contains("remove_initcalls is removal-only"));

    let preserve_remove_runtime_registration = FeatureIntent::from_config(
        FeatureIntentAction::Preserve,
        "keep",
        &FeatureIntentConfig {
            runtime_registrations: vec![String::from("module_init:bt_init")],
            remove_runtime_registrations: vec![String::from("module_platform_driver:btusb_driver")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap_err();
    assert!(format!("{preserve_remove_runtime_registration:#}")
        .contains("remove_runtime_registrations is removal-only"));

    let preserve_remove_docs = FeatureIntent::from_config(
        FeatureIntentAction::Preserve,
        "keep",
        &FeatureIntentConfig {
            docs: vec![String::from("Documentation/networking/bluetooth.rst")],
            remove_docs: vec![String::from("Documentation/driver-api/btusb.rst")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap_err();
    assert!(format!("{preserve_remove_docs:#}").contains("remove_docs is removal-only"));

    let preserve_remove_tools = FeatureIntent::from_config(
        FeatureIntentAction::Preserve,
        "keep",
        &FeatureIntentConfig {
            tools: vec![String::from("tools/perf")],
            remove_tools: vec![String::from("tools/objtool")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap_err();
    assert!(format!("{preserve_remove_tools:#}").contains("remove_tools is removal-only"));

    let preserve_remove_samples = FeatureIntent::from_config(
        FeatureIntentAction::Preserve,
        "keep",
        &FeatureIntentConfig {
            samples: vec![String::from("samples/bpf")],
            remove_samples: vec![String::from("samples/hidraw")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap_err();
    assert!(format!("{preserve_remove_samples:#}").contains("remove_samples is removal-only"));

    let preserve_remove_kunit_suites = FeatureIntent::from_config(
        FeatureIntentAction::Preserve,
        "keep",
        &FeatureIntentConfig {
            kunit_suites: vec![String::from("bt_test")],
            remove_kunit_suites: vec![String::from("btusb-test")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap_err();
    assert!(
        format!("{preserve_remove_kunit_suites:#}").contains("remove_kunit_suites is removal-only")
    );

    let preserve_remove_kselftest_targets = FeatureIntent::from_config(
        FeatureIntentAction::Preserve,
        "keep",
        &FeatureIntentConfig {
            kselftest_targets: vec![String::from("net")],
            remove_kselftest_targets: vec![String::from("bpf")],
            ..FeatureIntentConfig::default()
        },
    )
    .unwrap_err();
    assert!(format!("{preserve_remove_kselftest_targets:#}")
        .contains("remove_kselftest_targets is removal-only"));
}
