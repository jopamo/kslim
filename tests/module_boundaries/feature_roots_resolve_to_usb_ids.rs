use super::common::*;

#[test]
fn feature_roots_resolve_to_usb_ids() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let usb_id_resolution = production_source(&root.join("src/feature/usb_id_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod usb_id_resolution;")
            && feature.contains("FeatureUsbIdResolution")
            && feature.contains("FeatureResolvedUsbId")
            && feature.contains("FeatureResolvedUsbIdKind"),
        "feature module should expose the feature USB ID resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedUsbIdKind",
        "RemoveUsbIdRoot",
        "ExplicitRemoveUsbId",
        "PreserveUsbIdRoot",
        "\"remove_usb_id_root\"",
        "\"explicit_remove_usb_id\"",
        "\"preserve_usb_id_root\"",
        "FeatureOwnershipKind::OwnedSolelyByRemovedFeature",
        "FeatureOwnershipKind::SharedWithLiveFeature",
        "pub(crate) struct FeatureResolvedUsbId",
        "feature: FeatureId",
        "id: UsbId",
        "kind: FeatureResolvedUsbIdKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"usb_id:{}\", self.id.as_str()))?",
        "pub(crate) struct FeatureUsbIdResolution",
        "ids: Vec<FeatureResolvedUsbId>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "ids.extend(usb_ids_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "intent.usb_ids.iter().cloned()",
        "intent.remove_usb_ids.iter().cloned()",
        "pub(crate) fn remove_usb_ids(&self) -> Vec<UsbId>",
        "pub(crate) fn preserve_usb_ids(&self) -> Vec<UsbId>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            usb_id_resolution.contains(required),
            "feature USB ID resolution should own root-to-USB-ID fact {required}"
        );
    }

    for forbidden in [
        "KconfigSymbol",
        "KbuildObject",
        "SourceFilePath",
        "HeaderPath",
        "UapiPath",
        "GeneratedArtifactPath",
        "ExportedSymbol",
        "ModuleName",
        "ModuleAlias",
        "DeviceCompatible",
        "AcpiId",
        "PciId",
        "FirmwarePath",
        "Initcall",
        "RuntimeRegistrationSurface",
        "DocumentationPath",
        "ToolPath",
        "SamplePath",
        "KunitSuite",
        "KselftestTarget",
        "CandidateTreeState",
        "PublishedSnapshotState",
        "GeneratePlan",
        "RemovalManifest",
        "LockfilePath",
        "OutputRepoPath",
        "crate::hardware::",
        "crate::kbuild::",
        "crate::tree_index::",
        "std::fs::",
        "walkdir",
        "MODULE_DEVICE_TABLE",
        "usb_match_table",
        "module_usb_driver",
    ] {
        assert!(
            !usb_id_resolution.contains(forbidden),
            "FeatureUsbIdResolution must only resolve typed USB ID intent, not own device-table extraction, proof, scanning, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureUsbIdResolution` is the fifteenth feature-graph")
            && architecture.contains("resolves typed feature USB ID roots")
            && architecture.contains("`UsbId` facts")
            && architecture.contains("USB ID ownership assertions")
            && architecture.contains("without scanning USB device tables")
            && architecture.contains("USB device-table proof")
            && kernel_build_guide
                .contains("`FeatureUsbIdResolution` resolves typed feature USB ID roots")
            && kernel_build_guide.contains("sorted USB ID facts")
            && kernel_build_guide.contains("before USB device-table proof"),
        "docs should describe feature root-to-USB-ID resolution"
    );
}
