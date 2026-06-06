use super::common::*;

#[test]
fn feature_roots_resolve_to_tools() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let tool_resolution = production_source(&root.join("src/feature/tool_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod tool_resolution;")
            && feature.contains("FeatureToolResolution")
            && feature.contains("FeatureResolvedTool")
            && feature.contains("FeatureResolvedToolKind"),
        "feature module should expose the feature tool resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedToolKind",
        "RemoveToolRoot",
        "ExplicitRemoveTool",
        "PreserveToolRoot",
        "\"remove_tool_root\"",
        "\"explicit_remove_tool\"",
        "\"preserve_tool_root\"",
        "FeatureOwnershipKind::OwnedSolelyByRemovedFeature",
        "FeatureOwnershipKind::SharedWithLiveFeature",
        "pub(crate) struct FeatureResolvedTool",
        "feature: FeatureId",
        "path: ToolPath",
        "kind: FeatureResolvedToolKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"tool_path:{}\", self.path.as_str()))?",
        "pub(crate) struct FeatureToolResolution",
        "paths: Vec<FeatureResolvedTool>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "paths.extend(tool_paths_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "intent.tools.iter().cloned()",
        "intent.remove_tools.iter().cloned()",
        "pub(crate) fn remove_tools(&self) -> Vec<ToolPath>",
        "pub(crate) fn preserve_tools(&self) -> Vec<ToolPath>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            tool_resolution.contains(required),
            "feature tool resolution should own root-to-tool fact {required}"
        );
    }

    for forbidden in [
        "KconfigSymbol",
        "KbuildObject",
        "SourceFilePath",
        "HeaderPath",
        "UapiPath",
        "GeneratedArtifactPath",
        "DocumentationPath",
        "SamplePath",
        "KunitSuite",
        "KselftestTarget",
        "ExportedSymbol",
        "ModuleName",
        "ModuleAlias",
        "DeviceCompatible",
        "AcpiId",
        "PciId",
        "UsbId",
        "FirmwarePath",
        "Initcall",
        "RuntimeRegistrationSurface",
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
        "rustdoc",
        "make tools",
        "selftest",
        "kselftest",
    ] {
        assert!(
            !tool_resolution.contains(forbidden),
            "FeatureToolResolution must only resolve typed tool intent, not own tool build indexing, test proof, scanning, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureToolResolution` is the twentieth feature-graph")
            && architecture.contains("resolves typed feature tool roots")
            && architecture.contains("`ToolPath` facts")
            && architecture.contains("tool ownership assertions")
            && architecture.contains("without scanning tool build files")
            && architecture.contains("test harnesses")
            && kernel_build_guide
                .contains("`FeatureToolResolution` resolves typed feature tool roots")
            && kernel_build_guide.contains("sorted tool facts")
            && kernel_build_guide.contains("before tool build or test proof"),
        "tools should describe feature root-to-tool resolution"
    );
}
