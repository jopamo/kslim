use super::common::*;

#[test]
fn feature_conflict_detection_reports_live_kbuild_references() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let detection = production_source(&root.join("src/feature/conflict_detection.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("pub(crate) use conflict_detection::FeatureKbuildReference;"),
        "feature module should expose semantic kbuild reference facts for conflict detection"
    );

    for required in [
        "pub(crate) struct FeatureKbuildReference",
        "referencer: KbuildObject",
        "referenced: KbuildObject",
        "pub(crate) fn new(referencer: KbuildObject, referenced: KbuildObject) -> Result<Self>",
        "feature kbuild reference endpoints must be distinct",
        "pub(crate) fn from_names(referencer: &str, referenced: &str) -> Result<Self>",
        "KbuildObject::new(referencer)?",
        "KbuildObject::new(referenced)?",
        "pub(crate) fn stable_key(&self) -> String",
        "kbuild_ref:{}->{}",
        "pub(crate) fn from_graph_and_kbuild_references(",
        "pub(crate) fn from_graph_and_feature_facts(",
        "removed_feature_live_kbuild_reference_conflicts(",
        "fn removed_feature_live_kbuild_reference_conflicts(",
        "FeatureKbuildResolution::from_graph(graph)?",
        "removed_features_by_object",
        "live_features_by_object",
        "object.kind().is_removal()",
        "object.kind().is_preservation()",
        "features_covering_kbuild_object(&live_features_by_object, reference.referencer())",
        "features_covering_kbuild_object(&removed_features_by_object, reference.referenced())",
        "FeatureConflictKind::RemovedFeatureReferencedByLiveKbuild",
        "kbuild:{}",
        "still referenced by live kbuild object",
        "remove the '{}' kbuild reference or preserve the removed feature",
        "fn kbuild_object_covers(owner: &KbuildObject, object: &KbuildObject) -> bool",
        "owner.is_directory_ref() && object.as_str().starts_with(owner.as_str())",
    ] {
        assert!(
            detection.contains(required),
            "feature conflict detection should report live kbuild references to removed feature objects {required}"
        );
    }

    for forbidden in [
        "crate::kbuild",
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
            "Feature kbuild conflict detection must consume semantic facts, not parse makefiles or own lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("kbuild reference facts")
            && architecture
                .contains("live kbuild object references an object owned by a removed feature")
            && architecture.contains("`removed_feature_referenced_by_live_kbuild`")
            && kernel_build_guide.contains("kbuild reference facts")
            && kernel_build_guide
                .contains("live kbuild object references an object owned by a removed feature")
            && kernel_build_guide.contains("`removed_feature_referenced_by_live_kbuild`"),
        "docs should describe live-kbuild-reference conflict detection"
    );
}
