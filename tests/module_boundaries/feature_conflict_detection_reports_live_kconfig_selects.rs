use super::common::*;

#[test]
fn feature_conflict_detection_reports_live_kconfig_selects() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let detection = production_source(&root.join("src/feature/conflict_detection.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("pub(crate) use conflict_detection::FeatureKconfigSelection;"),
        "feature module should expose semantic Kconfig selection facts for conflict detection"
    );

    for required in [
        "pub(crate) struct FeatureKconfigSelection",
        "selector: KconfigSymbol",
        "selected: KconfigSymbol",
        "pub(crate) fn new(selector: KconfigSymbol, selected: KconfigSymbol) -> Result<Self>",
        "feature Kconfig selection endpoints must be distinct",
        "pub(crate) fn from_names(selector: &str, selected: &str) -> Result<Self>",
        "KconfigSymbol::new(selector)?",
        "KconfigSymbol::new(selected)?",
        "pub(crate) fn stable_key(&self) -> String",
        "select:{}->{}",
        "pub(crate) fn from_graph_and_kconfig_selections(",
        "removed_feature_live_kconfig_selection_conflicts(",
        "fn removed_feature_live_kconfig_selection_conflicts(",
        "FeatureKconfigResolution::from_graph(graph)",
        "removed_features_by_symbol",
        "live_features_by_symbol",
        "symbol.kind().is_removal()",
        "symbol.kind().is_preservation()",
        "live_features_by_symbol.contains_key(selection.selector())",
        "removed_features_by_symbol.get(selection.selected())",
        "FeatureConflictKind::RemovedFeatureSelectedByLiveKconfig",
        "kconfig:{}",
        "still selected by live Kconfig symbol",
        "remove the '{}' selector or preserve feature '{}'",
    ] {
        assert!(
            detection.contains(required),
            "feature conflict detection should report live Kconfig selects of removed feature symbols {required}"
        );
    }

    for forbidden in [
        "crate::kconfig",
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
            "Feature Kconfig conflict detection must consume semantic facts, not parse trees or own lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("Kconfig selection facts")
            && architecture.contains("preserved Kconfig symbol selects a removed feature symbol")
            && architecture.contains("`removed_feature_selected_by_live_kconfig`")
            && kernel_build_guide.contains("Kconfig selection facts")
            && kernel_build_guide
                .contains("preserved Kconfig symbol selects a removed feature symbol")
            && kernel_build_guide.contains("`removed_feature_selected_by_live_kconfig`"),
        "docs should describe live-Kconfig-select conflict detection"
    );
}
