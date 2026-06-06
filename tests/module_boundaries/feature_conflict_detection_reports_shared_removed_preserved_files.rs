use super::common::*;

#[test]
fn feature_conflict_detection_reports_shared_removed_preserved_files() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let detection = production_source(&root.join("src/feature/conflict_detection.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "removed_feature_shared_file_conflicts(graph)?",
        "fn removed_feature_shared_file_conflicts(graph: &FeatureGraph) -> Result<Vec<FeatureConflict>>",
        "FeaturePathResolution::from_graph(graph)",
        "let removed_paths = path_resolution",
        "let preserved_paths = path_resolution",
        "path.kind().is_removal()",
        "path.kind().is_preservation()",
        "fn feature_paths_overlap(left: &RelativeKernelPath, right: &RelativeKernelPath) -> bool",
        "fn shared_feature_path_subject<'a>(",
        "crate::path_policy::normalized_relative_path_covers",
        "FeatureConflictKind::SharedFileBetweenRemovedAndPreservedFeatures",
        "FeatureOwnershipSubject::new(format!(",
        "\"path:{}\"",
        "removed feature '{}' shares path '{}' with preserved feature '{}'",
        "split shared path '{}', narrow feature roots, or preserve feature '{}'",
    ] {
        assert!(
            detection.contains(required),
            "feature conflict detection should report shared removed/preserved files {required}"
        );
    }

    for forbidden in [
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
            "Shared-file conflict detection must use semantic paths, not filesystem scans or lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("shared-file conflicts from feature path resolution")
            && architecture.contains(
                "removed path overlaps a preserved path as `shared_file_between_removed_and_preserved_features`"
            )
            && kernel_build_guide.contains("shared-file conflicts from feature path resolution")
            && kernel_build_guide.contains(
                "removed path overlaps a preserved path as `shared_file_between_removed_and_preserved_features`"
            ),
        "docs should describe shared removed/preserved file conflict detection"
    );
}
