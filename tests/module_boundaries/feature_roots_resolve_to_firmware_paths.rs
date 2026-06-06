use super::common::*;

#[test]
fn feature_roots_resolve_to_firmware_paths() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let firmware_path_resolution =
        production_source(&root.join("src/feature/firmware_path_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod firmware_path_resolution;")
            && feature.contains("FeatureFirmwarePathResolution")
            && feature.contains("FeatureResolvedFirmwarePath")
            && feature.contains("FeatureResolvedFirmwarePathKind"),
        "feature module should expose the feature firmware-path resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedFirmwarePathKind",
        "RemoveFirmwarePathRoot",
        "ExplicitRemoveFirmwarePath",
        "PreserveFirmwarePathRoot",
        "\"remove_firmware_path_root\"",
        "\"explicit_remove_firmware_path\"",
        "\"preserve_firmware_path_root\"",
        "FeatureOwnershipKind::OwnedSolelyByRemovedFeature",
        "FeatureOwnershipKind::SharedWithLiveFeature",
        "pub(crate) struct FeatureResolvedFirmwarePath",
        "feature: FeatureId",
        "path: FirmwarePath",
        "kind: FeatureResolvedFirmwarePathKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"firmware_path:{}\", self.path.as_str()))?",
        "pub(crate) struct FeatureFirmwarePathResolution",
        "paths: Vec<FeatureResolvedFirmwarePath>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "paths.extend(firmware_paths_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "intent.firmware_paths.iter().cloned()",
        "intent.remove_firmware_paths.iter().cloned()",
        "pub(crate) fn remove_firmware_paths(&self) -> Vec<FirmwarePath>",
        "pub(crate) fn preserve_firmware_paths(&self) -> Vec<FirmwarePath>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            firmware_path_resolution.contains(required),
            "feature firmware-path resolution should own root-to-firmware-path fact {required}"
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
        "UsbId",
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
        "request_firmware",
        "MODULE_FIRMWARE",
        "firmware_request_nowarn",
    ] {
        assert!(
            !firmware_path_resolution.contains(forbidden),
            "FeatureFirmwarePathResolution must only resolve typed firmware path intent, not own firmware-loader extraction, proof, scanning, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureFirmwarePathResolution` is the sixteenth feature-graph")
            && architecture.contains("resolves typed feature firmware references")
            && architecture.contains("`FirmwarePath` facts")
            && architecture.contains("firmware-path ownership assertions")
            && architecture.contains("without scanning firmware loader call sites")
            && architecture.contains("firmware-loader proof")
            && kernel_build_guide.contains(
                "`FeatureFirmwarePathResolution` resolves typed feature firmware references"
            )
            && kernel_build_guide.contains("sorted firmware-path facts")
            && kernel_build_guide.contains("before firmware-loader proof"),
        "docs should describe feature root-to-firmware-path resolution"
    );
}
