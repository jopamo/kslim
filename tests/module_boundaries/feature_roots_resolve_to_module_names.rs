use super::common::*;

#[test]
fn feature_roots_resolve_to_module_names() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let module_name_resolution =
        production_source(&root.join("src/feature/module_name_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod module_name_resolution;")
            && feature.contains("FeatureModuleNameResolution")
            && feature.contains("FeatureResolvedModuleName")
            && feature.contains("FeatureResolvedModuleNameKind"),
        "feature module should expose the feature module-name resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedModuleNameKind",
        "RemoveModuleNameRoot",
        "ExplicitRemoveModuleName",
        "PreserveModuleNameRoot",
        "\"remove_module_name_root\"",
        "\"explicit_remove_module_name\"",
        "\"preserve_module_name_root\"",
        "FeatureOwnershipKind::OwnedSolelyByRemovedFeature",
        "FeatureOwnershipKind::SharedWithLiveFeature",
        "pub(crate) struct FeatureResolvedModuleName",
        "feature: FeatureId",
        "module: ModuleName",
        "kind: FeatureResolvedModuleNameKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"module_name:{}\", self.module.as_str()))?",
        "pub(crate) struct FeatureModuleNameResolution",
        "modules: Vec<FeatureResolvedModuleName>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "modules.extend(modules_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "intent.module_names.iter().cloned()",
        "intent.remove_module_names.iter().cloned()",
        "pub(crate) fn remove_module_names(&self) -> Vec<ModuleName>",
        "pub(crate) fn preserve_module_names(&self) -> Vec<ModuleName>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            module_name_resolution.contains(required),
            "feature module-name resolution should own root-to-module-name fact {required}"
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
        "ModuleAlias",
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
        "std::fs::",
        "walkdir",
        "modules.order",
        "MODULE_ALIAS",
    ] {
        assert!(
            !module_name_resolution.contains(forbidden),
            "FeatureModuleNameResolution must only resolve typed module-name intent, not own kbuild scanning, alias metadata, proof, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureModuleNameResolution` is the tenth feature-graph")
            && architecture.contains("resolves typed feature module-name roots")
            && architecture.contains("`ModuleName` facts")
            && architecture.contains("module-name")
            && architecture.contains("ownership assertions")
            && architecture.contains("without scanning kbuild files")
            && architecture.contains("module alias metadata")
            && kernel_build_guide
                .contains("`FeatureModuleNameResolution` resolves typed feature module-name roots")
            && kernel_build_guide.contains("sorted module-name facts")
            && kernel_build_guide.contains("before module-alias metadata"),
        "docs should describe feature root-to-module-name resolution"
    );
}
