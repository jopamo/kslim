use super::common::*;

#[test]
fn feature_conflict_detection_reports_live_exported_symbol_consumers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let detection = production_source(&root.join("src/feature/conflict_detection.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("pub(crate) use conflict_detection::FeatureExportedSymbolConsumer;"),
        "feature module should expose semantic exported-symbol consumer facts for conflict detection"
    );

    for required in [
        "pub(crate) struct FeatureExportedSymbolConsumer",
        "consumer: FeatureId",
        "symbol: ExportedSymbol",
        "pub(crate) fn new(consumer: FeatureId, symbol: ExportedSymbol) -> Self",
        "pub(crate) fn from_names(consumer: &str, symbol: &str) -> Result<Self>",
        "FeatureId::new(consumer)?",
        "ExportedSymbol::new(symbol)?",
        "pub(crate) fn stable_key(&self) -> String",
        "exported_symbol_consumer:{}->{}",
        "pub(crate) fn from_graph_and_exported_symbol_consumers(",
        "removed_feature_exported_symbol_live_consumer_conflicts(",
        "fn removed_feature_exported_symbol_live_consumer_conflicts(",
        "FeatureExportedSymbolResolution::from_graph(graph)",
        "removed_features_by_symbol",
        "live_features",
        "symbol.kind().is_removal()",
        "live_features.contains_key(consumer.consumer())",
        "removed_features_by_symbol.get(consumer.symbol())",
        "FeatureConflictKind::RemovedFeatureExportsConsumedSymbol",
        "symbol:{}",
        "removed feature exports symbol",
        "consumed by live code",
        "remove live consumers of '{}' or preserve feature '{}'",
    ] {
        assert!(
            detection.contains(required),
            "feature conflict detection should report live consumers of removed exported symbols {required}"
        );
    }

    for forbidden in [
        "crate::exported_symbols",
        "crate::tree_index",
        "CandidateTreeState",
        "PublishedSnapshotState",
        "GeneratePlan",
        "RemovalManifest",
        "LockfilePath",
        "OutputRepoPath",
        "std::fs::",
        "walkdir",
        "crate::reducer",
        "crate::generate",
    ] {
        assert!(
            !detection.contains(forbidden),
            "Feature exported-symbol conflict detection must consume semantic facts, not scan sources or own lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("exported-symbol consumer facts")
            && architecture
                .contains("live feature consumes a symbol exported by a removed feature")
            && architecture.contains("`removed_feature_exports_consumed_symbol`")
            && kernel_build_guide.contains("exported-symbol consumer facts")
            && kernel_build_guide
                .contains("live feature consumes a symbol exported by a removed feature")
            && kernel_build_guide.contains("`removed_feature_exports_consumed_symbol`"),
        "docs should describe live exported-symbol consumer conflict detection"
    );
}
