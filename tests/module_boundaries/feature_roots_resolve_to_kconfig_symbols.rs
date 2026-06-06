use super::common::*;

#[test]
fn feature_roots_resolve_to_kconfig_symbols() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let kconfig_resolution = production_source(&root.join("src/feature/kconfig_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod kconfig_resolution;")
            && feature.contains("FeatureKconfigResolution")
            && feature.contains("FeatureResolvedKconfig")
            && feature.contains("FeatureResolvedKconfigKind"),
        "feature module should expose the feature Kconfig-symbol resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedKconfigKind",
        "RemoveConfigRoot",
        "ExplicitRemoveConfig",
        "PreserveConfigRoot",
        "\"remove_config_root\"",
        "\"explicit_remove_config\"",
        "\"preserve_config_root\"",
        "FeatureOwnershipKind::ExplicitlyRemoved",
        "FeatureOwnershipKind::ExplicitlyPreserved",
        "pub(crate) struct FeatureResolvedKconfig",
        "feature: FeatureId",
        "symbol: KconfigSymbol",
        "kind: FeatureResolvedKconfigKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"kconfig:{}\", self.symbol.as_str()))?",
        "pub(crate) struct FeatureKconfigResolution",
        "symbols: Vec<FeatureResolvedKconfig>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "symbols.extend(symbols_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "intent.configs.iter().cloned()",
        "intent.remove_configs.iter().cloned()",
        "pub(crate) fn remove_configs(&self) -> Vec<KconfigSymbol>",
        "pub(crate) fn preserve_configs(&self) -> Vec<KconfigSymbol>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            kconfig_resolution.contains(required),
            "feature Kconfig resolution should own root-to-symbol fact {required}"
        );
    }

    for forbidden in [
        "KconfigSymbol::new",
        "RelativeKernelPath",
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
        "std::fs::",
    ] {
        assert!(
            !kconfig_resolution.contains(forbidden),
            "FeatureKconfigResolution must only resolve typed Kconfig roots to symbols, not own deeper resolution or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureKconfigResolution` is the second feature-graph")
            && architecture.contains("resolves typed feature Kconfig roots")
            && architecture.contains("sorted `KconfigSymbol` facts")
            && architecture.contains("symbol ownership assertions")
            && architecture.contains("without parsing")
            && architecture.contains("Kconfig files")
            && kernel_build_guide
                .contains("`FeatureKconfigResolution` resolves typed feature Kconfig roots")
            && kernel_build_guide.contains("sorted")
            && kernel_build_guide.contains("Kconfig symbols")
            && kernel_build_guide.contains("before Kconfig AST solving"),
        "docs should describe feature root-to-Kconfig-symbol resolution"
    );
}
