use super::common::*;

#[test]
fn feature_conflict_detection_reports_removed_live_dependency_edges() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let detection = production_source(&root.join("src/feature/conflict_detection.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("mod conflict_detection;"),
        "feature module should expose the feature conflict detection slice"
    );

    for required in [
        "impl FeatureConflictReport",
        "pub(crate) fn from_graph(graph: &FeatureGraph) -> Result<Self>",
        "removed_feature_live_dependency_conflicts(graph)?",
        "fn removed_feature_live_dependency_conflicts(graph: &FeatureGraph) -> Result<Vec<FeatureConflict>>",
        "for edge in graph.edges()",
        "edge.kind() != FeatureEdgeKind::Dependency",
        "let live_consumer = graph",
        "get(edge.from())",
        "let removed_dependency = graph",
        "get(edge.to())",
        "FeatureIntentAction::Preserve",
        "FeatureIntentAction::Remove",
        "FeatureConflictKind::RemovedFeatureOwnsLiveDependency",
        "FeatureOwnershipSubject::new(format!(\"feature:{}\", edge.from().as_str()))?",
        "removed feature '{}' is required by live feature '{}'",
        "preserve feature '{}' or remove live consumer '{}'",
    ] {
        assert!(
            detection.contains(required),
            "feature conflict detection should report removed live-dependency edge conflict {required}"
        );
    }

    for forbidden in [
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
            "Feature conflict detection must stay semantic and avoid reducer, candidate, published, lockfile, or filesystem state {forbidden}"
        );
    }

    assert!(
        architecture.contains("feature conflict")
            && architecture.contains("detection slice")
            && architecture.contains("dependency edge as live consumer to dependency owner")
            && architecture.contains("`removed_feature_owns_live_dependency`")
            && kernel_build_guide.contains("conflict-detection slice")
            && kernel_build_guide.contains("preserved")
            && kernel_build_guide.contains("removed")
            && kernel_build_guide.contains("`removed_feature_owns_live_dependency`"),
        "docs should describe removed-live-dependency conflict detection"
    );
}
