use super::common::*;

#[test]
fn feature_intent_is_semantic_intent_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let generate_state = state_source(root);
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        main.contains("mod feature;"),
        "main.rs should register the feature semantics module"
    );

    for required in [
        "pub(crate) struct FeatureId",
        "pub(crate) fn new(id: impl Into<String>)",
        "pub(crate) fn as_str(&self) -> &str",
        "pub(crate) enum FeatureKind",
        "pub(crate) fn from_stable_name(value: &str) -> Result<Self>",
        "pub(crate) const fn stable_name(self) -> &'static str",
        "pub(crate) struct FeatureRoot",
        "pub(crate) fn as_relative_kernel_path(&self) -> &RelativeKernelPath",
        "pub(crate) struct FeatureScope",
        "pub(crate) fn from_arch_scope(arches: &[String])",
        "pub(crate) fn arch_scope(&self) -> &[ArchName]",
        "pub(crate) enum FeatureIntentAction",
        "pub(crate) struct FeatureIntent",
        "pub(crate) fn from_config(",
        "action: FeatureIntentAction",
        "id: FeatureId",
        "kind: Option<FeatureKind>",
        "roots: Vec<FeatureRoot>",
        "remove_paths: Vec<RelativeKernelPath>",
        "configs: Vec<KconfigSymbol>",
        "remove_configs: Vec<KconfigSymbol>",
        "exported_symbols: Vec<ExportedSymbol>",
        "remove_exported_symbols: Vec<ExportedSymbol>",
        "module_names: Vec<ModuleName>",
        "remove_module_names: Vec<ModuleName>",
        "module_aliases: Vec<ModuleAlias>",
        "remove_module_aliases: Vec<ModuleAlias>",
        "device_compatibles: Vec<DeviceCompatible>",
        "remove_device_compatibles: Vec<DeviceCompatible>",
        "acpi_ids: Vec<AcpiId>",
        "remove_acpi_ids: Vec<AcpiId>",
        "pci_ids: Vec<PciId>",
        "remove_pci_ids: Vec<PciId>",
        "usb_ids: Vec<UsbId>",
        "remove_usb_ids: Vec<UsbId>",
        "firmware_paths: Vec<FirmwarePath>",
        "remove_firmware_paths: Vec<FirmwarePath>",
        "initcalls: Vec<Initcall>",
        "remove_initcalls: Vec<Initcall>",
        "runtime_registrations: Vec<RuntimeRegistrationSurface>",
        "remove_runtime_registrations: Vec<RuntimeRegistrationSurface>",
        "docs: Vec<DocumentationPath>",
        "remove_docs: Vec<DocumentationPath>",
        "tools: Vec<ToolPath>",
        "remove_tools: Vec<ToolPath>",
        "samples: Vec<SamplePath>",
        "remove_samples: Vec<SamplePath>",
        "kunit_suites: Vec<KunitSuite>",
        "remove_kunit_suites: Vec<KunitSuite>",
        "kselftest_targets: Vec<KselftestTarget>",
        "remove_kselftest_targets: Vec<KselftestTarget>",
        "scope: FeatureScope",
        "safety: Option<FeatureSafetyLevel>",
        "allow_public_header_removal: bool",
        "allow_uapi_header_removal: bool",
        "require_clean_boot: bool",
        "report_only: bool",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "features.preserve.{name}.remove_paths is removal-only",
        "features.preserve.{name}.remove_exported_symbols is removal-only",
        "features.preserve.{name}.remove_module_names is removal-only",
        "features.preserve.{name}.remove_module_aliases is removal-only",
        "features.preserve.{name}.remove_device_compatibles is removal-only",
        "features.preserve.{name}.remove_acpi_ids is removal-only",
        "features.preserve.{name}.remove_pci_ids is removal-only",
        "features.preserve.{name}.remove_usb_ids is removal-only",
        "features.preserve.{name}.remove_firmware_paths is removal-only",
        "features.preserve.{name}.remove_initcalls is removal-only",
        "features.preserve.{name}.remove_runtime_registrations is removal-only",
        "features.preserve.{name}.remove_docs is removal-only",
        "features.preserve.{name}.remove_tools is removal-only",
        "features.preserve.{name}.remove_samples is removal-only",
        "features.preserve.{name}.remove_kunit_suites is removal-only",
        "features.preserve.{name}.remove_kselftest_targets is removal-only",
    ] {
        assert!(
            feature.contains(required),
            "feature module should own typed feature intent fact {required}"
        );
    }

    for forbidden in [
        "CandidateTreeState",
        "PublishedSnapshotState",
        "GeneratePlan",
        "PrunePlan",
        "RemovalManifest",
        "LockfilePath",
        "OutputRepoPath",
    ] {
        assert!(
            !feature.contains(forbidden),
            "FeatureIntent must stay semantic intent, not lifecycle or reducer state {forbidden}"
        );
    }

    assert!(
        generate_state
            .contains("FeatureConflictReport, FeatureGraph, FeatureIntent, FeatureIntentAction, FeatureNode,")
            && generate_state.contains("let graph = FeatureGraph::from_profile(profile)?;")
            && generate_state.contains("FeatureIntentPlan::from_feature_graph(&graph)")
            && generate_state.contains("FeatureIntentEntryPlan::from_feature_node")
            && generate_state.contains("Self::from_intent(node.intent())"),
        "generate state should consume typed FeatureGraph/FeatureIntent before building plan entries"
    );

    assert!(
        architecture.contains("`feature/*` | Typed feature semantics intent")
            && architecture.contains("`FeatureId` is the typed stable identity")
            && architecture.contains("`FeatureKind` is the stable enum")
            && architecture.contains("`FeatureRoot`")
            && architecture.contains("`FeatureScope`")
            && architecture.contains("`FeatureIntent`")
            && architecture.contains("semantic intent")
            && architecture.contains("without owning graph")
            && architecture.contains("reducer, candidate, published")
            && architecture.contains("lockfile state"),
        "architecture docs should describe FeatureIntent ownership"
    );
}
