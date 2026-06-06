use super::common::*;

#[test]
fn feature_roots_resolve_to_pci_ids() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let pci_id_resolution = production_source(&root.join("src/feature/pci_id_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod pci_id_resolution;")
            && feature.contains("FeaturePciIdResolution")
            && feature.contains("FeatureResolvedPciId")
            && feature.contains("FeatureResolvedPciIdKind"),
        "feature module should expose the feature PCI ID resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedPciIdKind",
        "RemovePciIdRoot",
        "ExplicitRemovePciId",
        "PreservePciIdRoot",
        "\"remove_pci_id_root\"",
        "\"explicit_remove_pci_id\"",
        "\"preserve_pci_id_root\"",
        "FeatureOwnershipKind::OwnedSolelyByRemovedFeature",
        "FeatureOwnershipKind::SharedWithLiveFeature",
        "pub(crate) struct FeatureResolvedPciId",
        "feature: FeatureId",
        "id: PciId",
        "kind: FeatureResolvedPciIdKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"pci_id:{}\", self.id.as_str()))?",
        "pub(crate) struct FeaturePciIdResolution",
        "ids: Vec<FeatureResolvedPciId>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "ids.extend(pci_ids_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "intent.pci_ids.iter().cloned()",
        "intent.remove_pci_ids.iter().cloned()",
        "pub(crate) fn remove_pci_ids(&self) -> Vec<PciId>",
        "pub(crate) fn preserve_pci_ids(&self) -> Vec<PciId>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            pci_id_resolution.contains(required),
            "feature PCI ID resolution should own root-to-PCI-ID fact {required}"
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
        "UsbId",
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
        "pci_match_table",
        "module_pci_driver",
    ] {
        assert!(
            !pci_id_resolution.contains(forbidden),
            "FeaturePciIdResolution must only resolve typed PCI ID intent, not own device-table extraction, proof, scanning, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeaturePciIdResolution` is the fourteenth feature-graph")
            && architecture.contains("resolves typed feature PCI ID roots")
            && architecture.contains("`PciId` facts")
            && architecture.contains("PCI ID ownership assertions")
            && architecture.contains("without scanning PCI device tables")
            && architecture.contains("PCI device-table proof")
            && kernel_build_guide
                .contains("`FeaturePciIdResolution` resolves typed feature PCI ID roots")
            && kernel_build_guide.contains("sorted PCI ID facts")
            && kernel_build_guide.contains("before PCI device-table proof"),
        "docs should describe feature root-to-PCI-ID resolution"
    );
}
