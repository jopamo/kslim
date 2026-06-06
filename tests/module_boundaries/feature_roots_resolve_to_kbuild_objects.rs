use super::common::*;

#[test]
fn feature_roots_resolve_to_kbuild_objects() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let kbuild_resolution = production_source(&root.join("src/feature/kbuild_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod kbuild_resolution;")
            && feature.contains("FeatureKbuildResolution")
            && feature.contains("FeatureResolvedKbuildObject")
            && feature.contains("FeatureResolvedKbuildObjectKind"),
        "feature module should expose the feature kbuild-object resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedKbuildObjectKind",
        "RemoveRootObject",
        "RemoveRootDirectory",
        "ExplicitRemoveObject",
        "ExplicitRemoveDirectory",
        "PreserveRootObject",
        "PreserveRootDirectory",
        "\"remove_root_object\"",
        "\"remove_root_directory\"",
        "\"explicit_remove_object\"",
        "\"explicit_remove_directory\"",
        "\"preserve_root_object\"",
        "\"preserve_root_directory\"",
        "FeatureOwnershipKind::ExplicitlyRemoved",
        "FeatureOwnershipKind::ExplicitlyPreserved",
        "pub(crate) struct FeatureResolvedKbuildObject",
        "feature: FeatureId",
        "object: KbuildObject",
        "kind: FeatureResolvedKbuildObjectKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"kbuild:{}\", self.object.as_str()))?",
        "pub(crate) struct FeatureKbuildResolution",
        "objects: Vec<FeatureResolvedKbuildObject>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Result<Self>",
        "for node in graph.nodes()",
        "objects.extend(objects_from_intent(node.intent())?)",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "root.as_relative_kernel_path()",
        "intent.remove_paths",
        "KbuildObject::new(path.with_extension(\"o\")",
        "KbuildObject::new(format!(\"{}/\", path.to_string_lossy()))?",
        "is_kbuild_source_path",
        "is_kbuild_metadata_file",
        "pub(crate) fn remove_objects(&self) -> Vec<KbuildObject>",
        "pub(crate) fn preserve_objects(&self) -> Vec<KbuildObject>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            kbuild_resolution.contains(required),
            "feature kbuild resolution should own root-to-object fact {required}"
        );
    }

    for forbidden in [
        "KconfigSymbol",
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
        "build_kbuild_index",
        "std::fs::",
    ] {
        assert!(
            !kbuild_resolution.contains(forbidden),
            "FeatureKbuildResolution must only resolve typed roots/paths to kbuild objects, not own deeper resolution or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureKbuildResolution` is the third feature-graph")
            && architecture.contains("resolves typed feature roots")
            && architecture.contains("sorted")
            && architecture.contains("`KbuildObject` facts")
            && architecture.contains("kbuild-object ownership assertions")
            && architecture.contains("without scanning")
            && architecture.contains("Makefiles")
            && kernel_build_guide
                .contains("`FeatureKbuildResolution` resolves typed feature roots")
            && kernel_build_guide.contains("sorted kbuild object")
            && kernel_build_guide.contains("before")
            && kernel_build_guide.contains("Makefile/Kbuild graph parsing"),
        "docs should describe feature root-to-kbuild-object resolution"
    );
}
