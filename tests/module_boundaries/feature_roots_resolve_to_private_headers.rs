use super::common::*;

#[test]
fn feature_roots_resolve_to_private_headers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let private_header_resolution =
        production_source(&root.join("src/feature/private_header_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod private_header_resolution;")
            && feature.contains("FeaturePrivateHeaderResolution")
            && feature.contains("FeatureResolvedPrivateHeader")
            && feature.contains("FeatureResolvedPrivateHeaderKind"),
        "feature module should expose the feature private-header resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedPrivateHeaderKind",
        "RemoveRoot",
        "ExplicitRemovePath",
        "PreserveRoot",
        "\"remove_root_private_header\"",
        "\"explicit_remove_private_header\"",
        "\"preserve_root_private_header\"",
        "FeatureOwnershipKind::ExplicitlyRemoved",
        "FeatureOwnershipKind::ExplicitlyPreserved",
        "pub(crate) struct FeatureResolvedPrivateHeader",
        "feature: FeatureId",
        "header: HeaderPath",
        "kind: FeatureResolvedPrivateHeaderKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"private_header:{}\", self.header.as_str()))?",
        "pub(crate) struct FeaturePrivateHeaderResolution",
        "headers: Vec<FeatureResolvedPrivateHeader>",
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
        "is_private_header_path",
        "is_public_header_path",
        "is_uapi_like_header_path",
        "is_generated_header_path",
        "pub(crate) fn remove_private_headers(&self) -> Vec<HeaderPath>",
        "pub(crate) fn preserve_private_headers(&self) -> Vec<HeaderPath>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            private_header_resolution.contains(required),
            "feature private-header resolution should own root-to-private-header fact {required}"
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
            !private_header_resolution.contains(forbidden),
            "FeaturePrivateHeaderResolution must only resolve typed roots/paths to private headers, not own public/UAPI/generated resolution or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeaturePrivateHeaderResolution` is the fifth feature-graph")
            && architecture.contains("resolves typed feature roots")
            && architecture.contains("explicit path intent")
            && architecture.contains("private")
            && architecture.contains("`HeaderPath` facts")
            && architecture.contains("private-header")
            && architecture.contains("ownership assertions")
            && architecture.contains("without scanning the source tree")
            && kernel_build_guide
                .contains("`FeaturePrivateHeaderResolution` resolves typed feature roots")
            && kernel_build_guide.contains("sorted private-header facts")
            && kernel_build_guide.contains("before public-header"),
        "docs should describe feature root-to-private-header resolution"
    );
}
