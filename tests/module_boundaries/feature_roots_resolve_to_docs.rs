use super::common::*;

#[test]
fn feature_roots_resolve_to_docs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let documentation_resolution =
        production_source(&root.join("src/feature/documentation_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod documentation_resolution;")
            && feature.contains("FeatureDocumentationResolution")
            && feature.contains("FeatureResolvedDocumentation")
            && feature.contains("FeatureResolvedDocumentationKind"),
        "feature module should expose the feature documentation resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedDocumentationKind",
        "RemoveDocumentationRoot",
        "ExplicitRemoveDocumentation",
        "PreserveDocumentationRoot",
        "\"remove_documentation_root\"",
        "\"explicit_remove_documentation\"",
        "\"preserve_documentation_root\"",
        "FeatureOwnershipKind::OwnedSolelyByRemovedFeature",
        "FeatureOwnershipKind::SharedWithLiveFeature",
        "pub(crate) struct FeatureResolvedDocumentation",
        "feature: FeatureId",
        "path: DocumentationPath",
        "kind: FeatureResolvedDocumentationKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"documentation_path:{}\", self.path.as_str()))?",
        "pub(crate) struct FeatureDocumentationResolution",
        "paths: Vec<FeatureResolvedDocumentation>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "paths.extend(documentation_paths_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "intent.docs.iter().cloned()",
        "intent.remove_docs.iter().cloned()",
        "pub(crate) fn remove_docs(&self) -> Vec<DocumentationPath>",
        "pub(crate) fn preserve_docs(&self) -> Vec<DocumentationPath>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            documentation_resolution.contains(required),
            "feature documentation resolution should own root-to-documentation fact {required}"
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
        "FirmwarePath",
        "ToolPath",
        "SamplePath",
        "KunitSuite",
        "KselftestTarget",
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
        "link-check",
        "sphinx",
    ] {
        assert!(
            !documentation_resolution.contains(forbidden),
            "FeatureDocumentationResolution must only resolve typed documentation intent, not own doc indexing, link checking, scanning, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureDocumentationResolution` is the nineteenth feature-graph")
            && architecture.contains("resolves typed feature documentation roots")
            && architecture.contains("`DocumentationPath` facts")
            && architecture.contains("documentation ownership assertions")
            && architecture.contains("without scanning documentation indexes")
            && architecture.contains("link graphs")
            && kernel_build_guide.contains(
                "`FeatureDocumentationResolution` resolves typed feature documentation roots"
            )
            && kernel_build_guide.contains("sorted documentation facts")
            && kernel_build_guide.contains("before doc index or link-check proof"),
        "docs should describe feature root-to-documentation resolution"
    );
}
