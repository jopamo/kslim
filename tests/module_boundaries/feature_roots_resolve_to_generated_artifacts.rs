use super::common::*;

#[test]
fn feature_roots_resolve_to_generated_artifacts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let generated_artifact_resolution =
        production_source(&root.join("src/feature/generated_artifact_resolution.rs"));
    let generated_artifacts = production_source(&root.join("src/generated/artifact.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod generated_artifact_resolution;")
            && feature.contains("FeatureGeneratedArtifactResolution")
            && feature.contains("FeatureResolvedGeneratedArtifact")
            && feature.contains("FeatureResolvedGeneratedArtifactKind"),
        "feature module should expose the feature generated-artifact resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedGeneratedArtifactKind",
        "RemoveRoot",
        "ExplicitRemovePath",
        "PreserveRoot",
        "\"remove_root_generated_artifact\"",
        "\"explicit_remove_generated_artifact\"",
        "\"preserve_root_generated_artifact\"",
        "FeatureOwnershipKind::GeneratedByLiveBuild",
        "pub(crate) struct FeatureResolvedGeneratedArtifact",
        "feature: FeatureId",
        "artifact: GeneratedArtifactPath",
        "kind: FeatureResolvedGeneratedArtifactKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"generated_artifact:{}\", self.artifact.as_str()))?",
        "pub(crate) struct FeatureGeneratedArtifactResolution",
        "artifacts: Vec<FeatureResolvedGeneratedArtifact>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Result<Self>",
        "for node in graph.nodes()",
        "artifacts.extend(artifacts_from_intent(node.intent())?)",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "root.as_relative_kernel_path()",
        "intent.remove_paths",
        "GeneratedArtifactPath::new(path.as_path().to_path_buf())?",
        "crate::generated::is_generated_artifact_like_path(path.as_path())",
        "pub(crate) fn remove_generated_artifacts(&self) -> Vec<GeneratedArtifactPath>",
        "pub(crate) fn preserve_generated_artifacts(&self) -> Vec<GeneratedArtifactPath>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            generated_artifact_resolution.contains(required),
            "feature generated-artifact resolution should own root-to-generated-artifact fact {required}"
        );
    }

    for required in [
        "pub(crate) fn is_generated_artifact_path(path: &Path) -> bool",
        "GeneratedArtifactPath::matches_path(path)",
        "pub(crate) fn raw_generated_artifact_path_parts_match(path: &Path) -> bool",
        "*child != \"uapi\"",
        "pub(crate) fn discover_generated_artifacts",
    ] {
        assert!(
            generated_artifacts.contains(required),
            "src/generated/artifact.rs should own generated artifact classification/discovery item {required}"
        );
    }

    for forbidden in [
        "KconfigSymbol",
        "KbuildObject",
        "SourceFilePath",
        "HeaderPath",
        "UapiPath",
        "ExportedSymbol",
        "ModuleName",
        "DeviceCompatible",
        "FirmwarePath",
        "Initcall",
        "GeneratedArtifactIndex",
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
            !generated_artifact_resolution.contains(forbidden),
            "FeatureGeneratedArtifactResolution must only resolve typed roots/paths to generated artifacts, not own generated indexes, ABI policy gates, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureGeneratedArtifactResolution` is the eighth feature-graph")
            && architecture.contains("resolves typed feature roots")
            && architecture.contains("generated artifacts")
            && architecture.contains("`GeneratedArtifactPath` facts")
            && architecture.contains("generated-artifact")
            && architecture.contains("ownership assertions")
            && architecture.contains("without scanning the source tree")
            && architecture.contains("generated-artifact indexes")
            && kernel_build_guide
                .contains("`FeatureGeneratedArtifactResolution` resolves typed feature roots")
            && kernel_build_guide.contains("sorted generated-artifact facts")
            && kernel_build_guide.contains("before generated-artifact indexes"),
        "docs should describe feature root-to-generated-artifact resolution"
    );
}
