use super::common::*;

#[test]
fn feature_roots_resolve_to_devicetree_compatibles() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let device_compatible_resolution =
        production_source(&root.join("src/feature/device_compatible_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod device_compatible_resolution;")
            && feature.contains("FeatureDeviceCompatibleResolution")
            && feature.contains("FeatureResolvedDeviceCompatible")
            && feature.contains("FeatureResolvedDeviceCompatibleKind"),
        "feature module should expose the feature devicetree-compatible resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedDeviceCompatibleKind",
        "RemoveDeviceCompatibleRoot",
        "ExplicitRemoveDeviceCompatible",
        "PreserveDeviceCompatibleRoot",
        "\"remove_device_compatible_root\"",
        "\"explicit_remove_device_compatible\"",
        "\"preserve_device_compatible_root\"",
        "FeatureOwnershipKind::OwnedSolelyByRemovedFeature",
        "FeatureOwnershipKind::SharedWithLiveFeature",
        "pub(crate) struct FeatureResolvedDeviceCompatible",
        "feature: FeatureId",
        "compatible: DeviceCompatible",
        "kind: FeatureResolvedDeviceCompatibleKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(",
        "\"device_compatible:{}\"",
        "self.compatible.as_str()",
        "pub(crate) struct FeatureDeviceCompatibleResolution",
        "compatibles: Vec<FeatureResolvedDeviceCompatible>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "compatibles.extend(compatibles_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "intent.device_compatibles.iter().cloned()",
        "intent.remove_device_compatibles.iter().cloned()",
        "pub(crate) fn remove_device_compatibles(&self) -> Vec<DeviceCompatible>",
        "pub(crate) fn preserve_device_compatibles(&self) -> Vec<DeviceCompatible>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            device_compatible_resolution.contains(required),
            "feature devicetree-compatible resolution should own root-to-compatible fact {required}"
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
        "AcpiId",
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
        "compatible_strings_in_content",
        "schema_reference_tokens",
        "DTS",
        "DTSI",
    ] {
        assert!(
            !device_compatible_resolution.contains(forbidden),
            "FeatureDeviceCompatibleResolution must only resolve typed devicetree-compatible intent, not own binding extraction, live-reference proof, scanning, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureDeviceCompatibleResolution` is the twelfth feature-graph")
            && architecture.contains("resolves typed feature devicetree-compatible roots")
            && architecture.contains("`DeviceCompatible` facts")
            && architecture.contains("devicetree-compatible ownership assertions")
            && architecture.contains("without scanning devicetree source files")
            && architecture.contains("device-binding proof")
            && kernel_build_guide.contains(
                "`FeatureDeviceCompatibleResolution` resolves typed feature devicetree-compatible roots"
            )
            && kernel_build_guide.contains("sorted devicetree-compatible facts")
            && kernel_build_guide.contains("before device-binding proof"),
        "docs should describe feature root-to-devicetree-compatible resolution"
    );
}
