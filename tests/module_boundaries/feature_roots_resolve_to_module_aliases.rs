use super::common::*;

#[test]
fn feature_roots_resolve_to_module_aliases() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let module_alias_resolution =
        production_source(&root.join("src/feature/module_alias_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod module_alias_resolution;")
            && feature.contains("FeatureModuleAliasResolution")
            && feature.contains("FeatureResolvedModuleAlias")
            && feature.contains("FeatureResolvedModuleAliasKind"),
        "feature module should expose the feature module-alias resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedModuleAliasKind",
        "RemoveModuleAliasRoot",
        "ExplicitRemoveModuleAlias",
        "PreserveModuleAliasRoot",
        "\"remove_module_alias_root\"",
        "\"explicit_remove_module_alias\"",
        "\"preserve_module_alias_root\"",
        "FeatureOwnershipKind::OwnedSolelyByRemovedFeature",
        "FeatureOwnershipKind::SharedWithLiveFeature",
        "pub(crate) struct FeatureResolvedModuleAlias",
        "feature: FeatureId",
        "alias: ModuleAlias",
        "kind: FeatureResolvedModuleAliasKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"module_alias:{}\", self.alias.as_str()))?",
        "pub(crate) struct FeatureModuleAliasResolution",
        "aliases: Vec<FeatureResolvedModuleAlias>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "aliases.extend(aliases_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "intent.module_aliases.iter().cloned()",
        "intent.remove_module_aliases.iter().cloned()",
        "pub(crate) fn remove_module_aliases(&self) -> Vec<ModuleAlias>",
        "pub(crate) fn preserve_module_aliases(&self) -> Vec<ModuleAlias>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            module_alias_resolution.contains(required),
            "feature module-alias resolution should own root-to-module-alias fact {required}"
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
        "DeviceCompatible",
        "ModuleName",
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
            !module_alias_resolution.contains(forbidden),
            "FeatureModuleAliasResolution must only resolve typed module-alias intent, not own kbuild scanning, source extraction, proof, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureModuleAliasResolution` is the eleventh feature-graph")
            && architecture.contains("resolves typed feature module-alias roots")
            && architecture.contains("`ModuleAlias` facts")
            && architecture.contains("module-alias ownership assertions")
            && architecture.contains("without scanning module source files")
            && architecture.contains("module alias extraction")
            && kernel_build_guide.contains(
                "`FeatureModuleAliasResolution` resolves typed feature module-alias roots"
            )
            && kernel_build_guide.contains("sorted module-alias facts")
            && kernel_build_guide.contains("before module-alias extraction"),
        "docs should describe feature root-to-module-alias resolution"
    );
}
