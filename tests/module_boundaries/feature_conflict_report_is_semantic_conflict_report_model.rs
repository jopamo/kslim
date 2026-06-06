use super::common::*;

#[test]
fn feature_conflict_report_is_semantic_conflict_report_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    let conflict_section = feature
        .split("pub(crate) enum FeatureConflictKind")
        .nth(1)
        .and_then(|rest| rest.split("fn normalize_feature_kind_token").next())
        .expect("feature module should define FeatureConflictReport before feature helpers");

    for required in [
        "pub(crate) const ALL: [Self; 8]",
        "RemovedFeatureOwnsLiveDependency",
        "RemovedFeatureSelectedByLiveKconfig",
        "RemovedFeatureReferencedByLiveKbuild",
        "RemovedFeatureExportsConsumedSymbol",
        "RemovedFeatureDeviceIdReferencedByLiveTable",
        "RemovedFeatureUapiReferencedByUserspaceFacingCode",
        "RemovedFeatureRuntimeRegistrationReachable",
        "SharedFileBetweenRemovedAndPreservedFeatures",
        "pub(crate) fn from_stable_name(value: &str) -> Result<Self>",
        "pub(crate) const fn stable_name(self) -> &'static str",
        "\"removed_feature_owns_live_dependency\"",
        "\"removed_feature_selected_by_live_kconfig\"",
        "\"removed_feature_referenced_by_live_kbuild\"",
        "\"removed_feature_exports_consumed_symbol\"",
        "\"removed_feature_device_id_referenced_by_live_table\"",
        "\"removed_feature_uapi_referenced_by_userspace_facing_code\"",
        "\"removed_feature_runtime_registration_reachable\"",
        "\"shared_file_between_removed_and_preserved_features\"",
        "pub(crate) struct FeatureConflict",
        "kind: FeatureConflictKind",
        "feature: FeatureId",
        "subject: FeatureOwnershipSubject",
        "summary: String",
        "suggested_action: String",
        "strict_blocking: bool",
        "pub(crate) fn from_name(",
        "FeatureId::new(feature)?",
        "FeatureOwnershipSubject::new(subject)?",
        "pub(crate) fn non_blocking(mut self) -> Self",
        "pub(crate) fn stable_key(&self) -> String",
        "pub(crate) struct FeatureConflictReport",
        "conflicts: Vec<FeatureConflict>",
        "pub(crate) fn new(conflicts: impl IntoIterator<Item = FeatureConflict>) -> Result<Self>",
        "left.stable_key().cmp(&right.stable_key())",
        "duplicate conflict",
        "pub(crate) fn conflicts(&self) -> &[FeatureConflict]",
        "pub(crate) fn blocking_count(&self) -> usize",
        "pub(crate) fn has_blocking_conflicts(&self) -> bool",
    ] {
        assert!(
            conflict_section.contains(required),
            "FeatureConflictReport should own semantic conflict report fact {required}"
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
    ] {
        assert!(
            !conflict_section.contains(forbidden),
            "FeatureConflictReport must not own reducer, candidate, published, lockfile, or mutation state {forbidden}"
        );
    }

    assert!(
        architecture.contains("`FeatureConflictReport` is the semantic report model")
            && architecture.contains("actionable")
            && architecture.contains("stable conflict kinds")
            && architecture.contains("strict-mode blocking flag")
            && architecture.contains("without owning conflict detection passes")
            && kernel_build_guide.contains("`FeatureConflictReport`")
            && kernel_build_guide.contains("typed actionable-conflict model")
            && kernel_build_guide.contains("strict-mode mutation gates"),
        "docs should describe FeatureConflictReport ownership"
    );
}
