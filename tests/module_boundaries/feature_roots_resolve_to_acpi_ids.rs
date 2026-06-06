use super::common::*;

#[test]
fn feature_roots_resolve_to_acpi_ids() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let acpi_id_resolution = production_source(&root.join("src/feature/acpi_id_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod acpi_id_resolution;")
            && feature.contains("FeatureAcpiIdResolution")
            && feature.contains("FeatureResolvedAcpiId")
            && feature.contains("FeatureResolvedAcpiIdKind"),
        "feature module should expose the feature ACPI ID resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedAcpiIdKind",
        "RemoveAcpiIdRoot",
        "ExplicitRemoveAcpiId",
        "PreserveAcpiIdRoot",
        "\"remove_acpi_id_root\"",
        "\"explicit_remove_acpi_id\"",
        "\"preserve_acpi_id_root\"",
        "FeatureOwnershipKind::OwnedSolelyByRemovedFeature",
        "FeatureOwnershipKind::SharedWithLiveFeature",
        "pub(crate) struct FeatureResolvedAcpiId",
        "feature: FeatureId",
        "id: AcpiId",
        "kind: FeatureResolvedAcpiIdKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"acpi_id:{}\", self.id.as_str()))?",
        "pub(crate) struct FeatureAcpiIdResolution",
        "ids: Vec<FeatureResolvedAcpiId>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "ids.extend(acpi_ids_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "intent.acpi_ids.iter().cloned()",
        "intent.remove_acpi_ids.iter().cloned()",
        "pub(crate) fn remove_acpi_ids(&self) -> Vec<AcpiId>",
        "pub(crate) fn preserve_acpi_ids(&self) -> Vec<AcpiId>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            acpi_id_resolution.contains(required),
            "feature ACPI ID resolution should own root-to-ACPI-ID fact {required}"
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
        "PciId",
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
        "acpi_match_table",
        "module_acpi_driver",
    ] {
        assert!(
            !acpi_id_resolution.contains(forbidden),
            "FeatureAcpiIdResolution must only resolve typed ACPI ID intent, not own device-table extraction, proof, scanning, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureAcpiIdResolution` is the thirteenth feature-graph")
            && architecture.contains("resolves typed feature ACPI ID roots")
            && architecture.contains("`AcpiId` facts")
            && architecture.contains("ACPI ID ownership assertions")
            && architecture.contains("without scanning ACPI device tables")
            && architecture.contains("ACPI device-table proof")
            && kernel_build_guide
                .contains("`FeatureAcpiIdResolution` resolves typed feature ACPI ID roots")
            && kernel_build_guide.contains("sorted ACPI ID facts")
            && kernel_build_guide.contains("before ACPI device-table proof"),
        "docs should describe feature root-to-ACPI-ID resolution"
    );
}
