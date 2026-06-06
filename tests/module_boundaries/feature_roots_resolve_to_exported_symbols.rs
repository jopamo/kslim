use super::common::*;

#[test]
fn feature_roots_resolve_to_exported_symbols() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let exported_symbol_resolution =
        production_source(&root.join("src/feature/exported_symbol_resolution.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod exported_symbol_resolution;")
            && feature.contains("FeatureExportedSymbolResolution")
            && feature.contains("FeatureResolvedExportedSymbol")
            && feature.contains("FeatureResolvedExportedSymbolKind"),
        "feature module should expose the feature exported-symbol resolution slice"
    );

    for required in [
        "pub(crate) enum FeatureResolvedExportedSymbolKind",
        "RemoveExportedSymbolRoot",
        "ExplicitRemoveExportedSymbol",
        "PreserveExportedSymbolRoot",
        "\"remove_exported_symbol_root\"",
        "\"explicit_remove_exported_symbol\"",
        "\"preserve_exported_symbol_root\"",
        "FeatureOwnershipKind::OwnedSolelyByRemovedFeature",
        "FeatureOwnershipKind::SharedWithLiveFeature",
        "pub(crate) struct FeatureResolvedExportedSymbol",
        "feature: FeatureId",
        "symbol: ExportedSymbol",
        "kind: FeatureResolvedExportedSymbolKind",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) fn ownership(&self) -> Result<FeatureOwnership>",
        "FeatureOwnershipSubject::new(format!(\"exported_symbol:{}\", self.symbol.as_str()))?",
        "pub(crate) struct FeatureExportedSymbolResolution",
        "symbols: Vec<FeatureResolvedExportedSymbol>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>",
        "let graph = FeatureGraph::from_profile(profile)?",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Self",
        "for node in graph.nodes()",
        "symbols.extend(symbols_from_intent(node.intent()))",
        "FeatureIntentAction::Remove",
        "FeatureIntentAction::Preserve",
        "intent.exported_symbols.iter().cloned()",
        "intent.remove_exported_symbols.iter().cloned()",
        "pub(crate) fn remove_exported_symbols(&self) -> Vec<ExportedSymbol>",
        "pub(crate) fn preserve_exported_symbols(&self) -> Vec<ExportedSymbol>",
        "pub(crate) fn ownerships(&self) -> Result<Vec<FeatureOwnership>>",
    ] {
        assert!(
            exported_symbol_resolution.contains(required),
            "feature exported-symbol resolution should own root-to-exported-symbol fact {required}"
        );
    }

    for forbidden in [
        "KconfigSymbol",
        "KbuildObject",
        "SourceFilePath",
        "HeaderPath",
        "UapiPath",
        "GeneratedArtifactPath",
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
        "crate::exported_symbols::",
        "crate::tree_index::",
        "std::fs::",
        "walkdir",
        "EXPORT_SYMBOL",
    ] {
        assert!(
            !exported_symbol_resolution.contains(forbidden),
            "FeatureExportedSymbolResolution must only resolve typed exported-symbol intent, not own source scanning, consumer proof, or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureExportedSymbolResolution` is the ninth feature-graph")
            && architecture.contains("resolves typed feature exported-symbol roots")
            && architecture.contains("`ExportedSymbol` facts")
            && architecture.contains("exported-symbol")
            && architecture.contains("ownership assertions")
            && architecture.contains("without scanning source files")
            && architecture.contains("exported-symbol consumer proof")
            && kernel_build_guide.contains(
                "`FeatureExportedSymbolResolution` resolves typed feature exported-symbol roots"
            )
            && kernel_build_guide.contains("sorted exported-symbol facts")
            && kernel_build_guide.contains("before exported-symbol graph"),
        "docs should describe feature root-to-exported-symbol resolution"
    );
}
