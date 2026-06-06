use super::common::*;

#[test]
fn feature_roots_resolve_to_uapi_headers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let uapi_header_resolution =
        production_source(&root.join("src/feature/uapi_header_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod uapi_header_resolution;")
            && feature.contains("FeatureUapiHeaderResolution")
            && feature.contains("FeatureResolvedUapiHeader")
            && feature.contains("FeatureResolvedUapiHeaderKind"),
        "feature module should expose the feature UAPI-header resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedUapiHeaderKind",
        "RemoveRoot",
        "ExplicitRemovePath",
        "PreserveRoot",
        "\"remove_root_uapi_header\"",
        "\"explicit_remove_uapi_header\"",
        "\"preserve_root_uapi_header\"",
        "FeatureOwnershipKind::PublicUapiSurface",
        "pub(crate) struct FeatureResolvedUapiHeader",
        "feature: FeatureId",
        "header: UapiPath",
        "kind: FeatureResolvedUapiHeaderKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"uapi_header:{}\", self.header.as_str()))?",
        "pub(crate) struct FeatureUapiHeaderResolution",
        "headers: Vec<FeatureResolvedUapiHeader>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Result<Self>",
        "for node in graph.nodes()",
        "headers.extend(headers_from_intent(node.intent())?)",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "root.as_relative_kernel_path()",
        "intent.remove_paths",
        "UapiPath::new(path.as_path().to_path_buf())?",
        "is_uapi_header_path",
        "UapiPath::matches_path(path)",
        "has_header_extension",
        "pub(crate) fn remove_uapi_headers(&self) -> Vec<UapiPath>",
        "pub(crate) fn preserve_uapi_headers(&self) -> Vec<UapiPath>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            uapi_header_resolution.contains(required),
            "feature UAPI-header resolution should own root-to-UAPI-header fact {required}"
        );
    }

    for forbidden in [
        "KconfigSymbol",
        "KbuildObject",
        "SourceFilePath",
        "HeaderPath",
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
            !uapi_header_resolution.contains(forbidden),
            "FeatureUapiHeaderResolution must only resolve typed roots/paths to UAPI headers, not own public/private/generated resolution, ABI policy gates, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureUapiHeaderResolution` is the seventh feature-graph")
            && architecture.contains("resolves typed feature roots")
            && architecture.contains("UAPI")
            && architecture.contains("`UapiPath` facts")
            && architecture.contains("uapi-header")
            && architecture.contains("ownership assertions")
            && architecture.contains("without scanning the source tree")
            && kernel_build_guide
                .contains("`FeatureUapiHeaderResolution` resolves typed feature roots")
            && kernel_build_guide.contains("sorted UAPI-header facts")
            && kernel_build_guide.contains("before generated-artifact"),
        "docs should describe feature root-to-UAPI-header resolution"
    );
}
