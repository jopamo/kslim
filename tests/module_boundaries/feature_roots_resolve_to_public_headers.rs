use super::common::*;

#[test]
fn feature_roots_resolve_to_public_headers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let public_header_resolution =
        production_source(&root.join("src/feature/public_header_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod public_header_resolution;")
            && feature.contains("FeaturePublicHeaderResolution")
            && feature.contains("FeatureResolvedPublicHeader")
            && feature.contains("FeatureResolvedPublicHeaderKind"),
        "feature module should expose the feature public-header resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedPublicHeaderKind",
        "RemoveRoot",
        "ExplicitRemovePath",
        "PreserveRoot",
        "\"remove_root_public_header\"",
        "\"explicit_remove_public_header\"",
        "\"preserve_root_public_header\"",
        "FeatureOwnershipKind::PublicAbiSurface",
        "pub(crate) struct FeatureResolvedPublicHeader",
        "feature: FeatureId",
        "header: HeaderPath",
        "kind: FeatureResolvedPublicHeaderKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"public_header:{}\", self.header.as_str()))?",
        "pub(crate) struct FeaturePublicHeaderResolution",
        "headers: Vec<FeatureResolvedPublicHeader>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Result<Self>",
        "for node in graph.nodes()",
        "headers.extend(headers_from_intent(node.intent())?)",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "root.as_relative_kernel_path()",
        "intent.remove_paths",
        "HeaderPath::new(path.as_path().to_string_lossy().into_owned())?",
        "is_public_header_path",
        "path.starts_with(\"include/linux\")",
        "path.starts_with(\"include/net\")",
        "pub(crate) fn remove_public_headers(&self) -> Vec<HeaderPath>",
        "pub(crate) fn preserve_public_headers(&self) -> Vec<HeaderPath>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            public_header_resolution.contains(required),
            "feature public-header resolution should own root-to-public-header fact {required}"
        );
    }

    for forbidden in [
        "KconfigSymbol",
        "KbuildObject",
        "SourceFilePath",
        "UapiPath",
        "ExportedSymbol",
        "ModuleName",
        "DeviceCompatible",
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
        "crate::abi::",
        "crate::tree_index::",
        "std::fs::",
    ] {
        assert!(
            !public_header_resolution.contains(forbidden),
            "FeaturePublicHeaderResolution must only resolve typed roots/paths to public headers, not own UAPI/generated resolution, ABI policy gates, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeaturePublicHeaderResolution` is the sixth feature-graph")
            && architecture.contains("resolves typed feature roots")
            && architecture.contains("public")
            && architecture.contains("kernel headers")
            && architecture.contains("`HeaderPath` facts")
            && architecture.contains("public-header")
            && architecture.contains("ownership assertions")
            && architecture.contains("without scanning the source tree")
            && kernel_build_guide
                .contains("`FeaturePublicHeaderResolution` resolves typed feature roots")
            && kernel_build_guide.contains("sorted public-header facts")
            && kernel_build_guide.contains("before UAPI-header"),
        "docs should describe feature root-to-public-header resolution"
    );
}
