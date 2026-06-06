use super::common::*;

#[test]
fn feature_roots_resolve_to_samples() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let sample_resolution = production_source(&root.join("src/feature/sample_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod sample_resolution;")
            && feature.contains("FeatureSampleResolution")
            && feature.contains("FeatureResolvedSample")
            && feature.contains("FeatureResolvedSampleKind"),
        "feature module should expose the feature sample resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedSampleKind",
        "RemoveSampleRoot",
        "ExplicitRemoveSample",
        "PreserveSampleRoot",
        "\"remove_sample_root\"",
        "\"explicit_remove_sample\"",
        "\"preserve_sample_root\"",
        "FeatureOwnershipKind::OwnedSolelyByRemovedFeature",
        "FeatureOwnershipKind::SharedWithLiveFeature",
        "pub(crate) struct FeatureResolvedSample",
        "feature: FeatureId",
        "path: SamplePath",
        "kind: FeatureResolvedSampleKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"sample_path:{}\", self.path.as_str()))?",
        "pub(crate) struct FeatureSampleResolution",
        "paths: Vec<FeatureResolvedSample>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "paths.extend(sample_paths_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "intent.samples.iter().cloned()",
        "intent.remove_samples.iter().cloned()",
        "pub(crate) fn remove_samples(&self) -> Vec<SamplePath>",
        "pub(crate) fn preserve_samples(&self) -> Vec<SamplePath>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            sample_resolution.contains(required),
            "feature sample resolution should own root-to-sample fact {required}"
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
        "ToolPath",
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
        "make samples",
        "selftest",
        "kselftest",
    ] {
        assert!(
            !sample_resolution.contains(forbidden),
            "FeatureSampleResolution must only resolve typed sample intent, not own sample build indexing, runtime proof, scanning, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureSampleResolution` is the twenty-first feature-graph")
            && architecture.contains("resolves typed feature sample roots")
            && architecture.contains("`SamplePath` facts")
            && architecture.contains("sample ownership assertions")
            && architecture.contains("without scanning sample build files")
            && architecture.contains("runtime harnesses")
            && kernel_build_guide
                .contains("`FeatureSampleResolution` resolves typed feature sample roots")
            && kernel_build_guide.contains("sorted sample facts")
            && kernel_build_guide.contains("before sample build or runtime proof"),
        "docs should describe feature root-to-sample resolution"
    );
}
