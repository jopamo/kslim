use super::common::*;

#[test]
fn feature_conflict_detection_reports_live_userspace_uapi_references() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let detection = production_source(&root.join("src/feature/conflict_detection.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("pub(crate) use conflict_detection::FeatureUserspaceUapiReference;"),
        "feature module should expose semantic userspace UAPI reference facts for conflict detection"
    );

    for required in [
        "pub(crate) struct FeatureUserspaceUapiReference",
        "referrer: FeatureId",
        "path: UapiPath",
        "pub(crate) fn new(referrer: FeatureId, path: UapiPath) -> Self",
        "pub(crate) fn from_names(referrer: &str, path: &str) -> Result<Self>",
        "FeatureId::new(referrer)?",
        "UapiPath::new(PathBuf::from(path))?",
        "pub(crate) fn stable_key(&self) -> String",
        "userspace_uapi_ref:{}->{}",
        "pub(crate) fn from_graph_and_userspace_uapi_references(",
        "removed_feature_uapi_live_userspace_reference_conflicts(",
        "fn removed_feature_uapi_live_userspace_reference_conflicts(",
        "FeatureUapiHeaderResolution::from_graph(graph)?",
        "removed_features_by_path",
        "live_features",
        "header.kind().is_removal()",
        "live_features.contains_key(reference.referrer())",
        "removed_features_by_path.get(reference.path())",
        "FeatureConflictKind::RemovedFeatureUapiReferencedByUserspaceFacingCode",
        "uapi_header:{}",
        "removed feature UAPI",
        "referenced by live userspace-facing code",
        "userspace UAPI reference to",
    ] {
        assert!(
            detection.contains(required),
            "feature conflict detection should report live userspace UAPI references to removed feature UAPI {required}"
        );
    }

    for forbidden in [
        "crate::abi",
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
            "Feature UAPI conflict detection must consume semantic facts, not scan userspace references or own lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("userspace UAPI reference facts")
            && architecture
                .contains("live userspace-facing code references UAPI owned by a removed feature")
            && architecture.contains("`removed_feature_uapi_referenced_by_userspace_facing_code`")
            && kernel_build_guide.contains("userspace UAPI reference facts")
            && kernel_build_guide
                .contains("live userspace-facing code references UAPI owned by a removed feature")
            && kernel_build_guide
                .contains("`removed_feature_uapi_referenced_by_userspace_facing_code`"),
        "docs should describe live userspace UAPI reference conflict detection"
    );
}
