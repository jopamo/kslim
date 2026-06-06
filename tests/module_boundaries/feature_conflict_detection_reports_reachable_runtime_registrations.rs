use super::common::*;

#[test]
fn feature_conflict_detection_reports_reachable_runtime_registrations() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let detection = production_source(&root.join("src/feature/conflict_detection.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature
            .contains("pub(crate) use conflict_detection::FeatureRuntimeRegistrationReachability;"),
        "feature module should expose semantic runtime-registration reachability facts for conflict detection"
    );

    for required in [
        "pub(crate) struct FeatureRuntimeRegistrationReachability",
        "reachable_from: FeatureId",
        "runtime_registration: RuntimeRegistrationSurface",
        "pub(crate) fn new(",
        "reachable_from: FeatureId",
        "runtime_registration: RuntimeRegistrationSurface",
        "pub(crate) fn from_names(reachable_from: &str, runtime_registration: &str) -> Result<Self>",
        "FeatureId::new(reachable_from)?",
        "RuntimeRegistrationSurface::new(runtime_registration)?",
        "pub(crate) fn stable_key(&self) -> String",
        "runtime_registration_reachability:{}->{}",
        "pub(crate) fn from_graph_and_runtime_registration_reachability(",
        "removed_feature_runtime_registration_reachable_conflicts(",
        "fn removed_feature_runtime_registration_reachable_conflicts(",
        "FeatureRuntimeRegistrationResolution::from_graph(graph)",
        "removed_features_by_registration",
        "live_features",
        "runtime_registration.kind().is_removal()",
        "live_features.contains_key(reachable.reachable_from())",
        "removed_features_by_registration.get(reachable.runtime_registration())",
        "FeatureConflictKind::RemovedFeatureRuntimeRegistrationReachable",
        "runtime_registration_surface:{}",
        "removed feature runtime registration",
        "still reachable from live feature",
        "runtime reachability to",
    ] {
        assert!(
            detection.contains(required),
            "feature conflict detection should report live reachability to removed runtime registrations {required}"
        );
    }

    for forbidden in [
        "crate::runtime",
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
            "Feature runtime-registration conflict detection must consume semantic facts, not scan source files or own lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("runtime-registration reachability facts")
            && architecture.contains(
                "live runtime path can still reach a registration owned by a removed feature"
            )
            && architecture.contains("`removed_feature_runtime_registration_reachable`")
            && kernel_build_guide.contains("runtime-registration reachability facts")
            && kernel_build_guide.contains(
                "live runtime path can still reach a registration owned by a removed feature"
            )
            && kernel_build_guide.contains("`removed_feature_runtime_registration_reachable`"),
        "docs should describe reachable runtime-registration conflict detection"
    );
}
