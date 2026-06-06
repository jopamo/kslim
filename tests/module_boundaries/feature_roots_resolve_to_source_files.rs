use super::common::*;

#[test]
fn feature_roots_resolve_to_source_files() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let source_resolution = production_source(&root.join("src/feature/source_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod source_resolution;")
            && feature.contains("FeatureSourceResolution")
            && feature.contains("FeatureResolvedSourceFile")
            && feature.contains("FeatureResolvedSourceFileKind"),
        "feature module should expose the feature source-file resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedSourceFileKind",
        "RemoveRoot",
        "ExplicitRemovePath",
        "PreserveRoot",
        "\"remove_root_source_file\"",
        "\"explicit_remove_source_file\"",
        "\"preserve_root_source_file\"",
        "FeatureOwnershipKind::ExplicitlyRemoved",
        "FeatureOwnershipKind::ExplicitlyPreserved",
        "pub(crate) struct FeatureResolvedSourceFile",
        "feature: FeatureId",
        "source: SourceFilePath",
        "kind: FeatureResolvedSourceFileKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"source:{}\", self.source.as_str()))?",
        "pub(crate) struct FeatureSourceResolution",
        "sources: Vec<FeatureResolvedSourceFile>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Result<Self>",
        "for node in graph.nodes()",
        "sources.extend(sources_from_intent(node.intent())?)",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "root.as_relative_kernel_path()",
        "intent.remove_paths",
        "SourceFilePath::new(path.as_path().to_path_buf())?",
        "is_source_file_path",
        "pub(crate) fn remove_source_files(&self) -> Vec<SourceFilePath>",
        "pub(crate) fn preserve_source_files(&self) -> Vec<SourceFilePath>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            source_resolution.contains(required),
            "feature source resolution should own root-to-source-file fact {required}"
        );
    }

    for forbidden in [
        "KconfigSymbol",
        "KbuildObject",
        "HeaderPath",
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
        "crate::kbuild::",
        "crate::tree_index::",
        "build_kbuild_index",
        "std::fs::",
    ] {
        assert!(
            !source_resolution.contains(forbidden),
            "FeatureSourceResolution must only resolve typed roots/paths to source files, not own deeper resolution or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureSourceResolution` is the fourth feature-graph")
            && architecture.contains("resolves typed feature roots")
            && architecture.contains("explicit path intent")
            && architecture.contains("`SourceFilePath` facts")
            && architecture.contains("source-file ownership assertions")
            && architecture.contains("without scanning the source tree")
            && kernel_build_guide
                .contains("`FeatureSourceResolution` resolves typed feature roots")
            && kernel_build_guide.contains("sorted source-file facts")
            && kernel_build_guide.contains("before source indexing"),
        "docs should describe feature root-to-source-file resolution"
    );
}
