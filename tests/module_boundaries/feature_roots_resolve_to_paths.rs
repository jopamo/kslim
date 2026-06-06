use super::common::*;

#[test]
fn feature_roots_resolve_to_paths() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let path_resolution = production_source(&root.join("src/feature/path_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod path_resolution;")
            && feature.contains("FeaturePathResolution")
            && feature.contains("FeatureResolvedPath")
            && feature.contains("FeatureResolvedPathKind"),
        "feature module should expose the feature path-resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedPathKind",
        "RemoveRoot",
        "ExplicitRemovePath",
        "PreserveRoot",
        "\"remove_root\"",
        "\"explicit_remove_path\"",
        "\"preserve_root\"",
        "FeatureOwnershipKind::ExplicitlyRemoved",
        "FeatureOwnershipKind::ExplicitlyPreserved",
        "pub(crate) struct FeatureResolvedPath",
        "feature: FeatureId",
        "path: RelativeKernelPath",
        "kind: FeatureResolvedPathKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(",
        "pub(crate) struct FeaturePathResolution",
        "paths: Vec<FeatureResolvedPath>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "paths.extend(paths_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "root.as_relative_kernel_path().clone()",
        "intent.remove_paths.iter().cloned()",
        "pub(crate) fn remove_paths(&self) -> Vec<RelativeKernelPath>",
        "pub(crate) fn preserve_paths(&self) -> Vec<RelativeKernelPath>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            path_resolution.contains(required),
            "feature path resolution should own root-to-path fact {required}"
        );
    }

    for forbidden in [
        "KconfigSymbol::new",
        "KbuildObject",
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
        "std::fs::",
    ] {
        assert!(
            !path_resolution.contains(forbidden),
            "FeaturePathResolution must only resolve typed roots to paths, not own deeper resolution or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeaturePathResolution` is the first feature-graph")
            && architecture.contains("resolves typed feature roots")
            && architecture.contains("sorted `RelativeKernelPath` facts")
            && architecture.contains("path ownership assertions")
            && architecture.contains("without")
            && architecture.contains("scanning source files")
            && kernel_build_guide.contains("`FeaturePathResolution` resolves typed feature roots")
            && kernel_build_guide.contains("relative kernel paths")
            && kernel_build_guide.contains("before deeper Kconfig"),
        "docs should describe feature root-to-path resolution"
    );
}
