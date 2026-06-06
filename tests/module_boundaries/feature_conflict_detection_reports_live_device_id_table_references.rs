use super::common::*;

#[test]
fn feature_conflict_detection_reports_live_device_id_table_references() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let detection = production_source(&root.join("src/feature/conflict_detection.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        feature.contains("pub(crate) use conflict_detection::FeatureDeviceIdTableReference;"),
        "feature module should expose semantic device-id table reference facts for conflict detection"
    );

    for required in [
        "pub(crate) enum FeatureDeviceId",
        "DeviceCompatible(DeviceCompatible)",
        "AcpiId(AcpiId)",
        "PciId(PciId)",
        "UsbId(UsbId)",
        "pub(crate) struct FeatureDeviceIdTableReference",
        "table_owner: FeatureId",
        "id: FeatureDeviceId",
        "pub(crate) fn from_device_compatible_names(",
        "FeatureDeviceId::from_device_compatible_name(compatible)?",
        "pub(crate) fn from_acpi_id_names(table_owner: &str, id: &str) -> Result<Self>",
        "pub(crate) fn from_pci_id_names(table_owner: &str, id: &str) -> Result<Self>",
        "pub(crate) fn from_usb_id_names(table_owner: &str, id: &str) -> Result<Self>",
        "device_id_table_ref:{}->{}",
        "pub(crate) fn from_graph_and_device_id_table_references(",
        "removed_feature_device_id_live_table_conflicts(",
        "fn removed_feature_device_id_live_table_conflicts(",
        "fn removed_feature_device_ids_by_id(",
        "FeatureDeviceCompatibleResolution::from_graph(graph)",
        "FeatureAcpiIdResolution::from_graph(graph)",
        "FeaturePciIdResolution::from_graph(graph)",
        "FeatureUsbIdResolution::from_graph(graph)",
        "removed_features_by_id",
        "live_features",
        "compatible.kind().is_removal()",
        "id.kind().is_removal()",
        "live_features.contains_key(reference.table_owner())",
        "removed_features_by_id.get(reference.id())",
        "FeatureConflictKind::RemovedFeatureDeviceIdReferencedByLiveTable",
        "FeatureOwnershipSubject::new(reference.id().stable_key())?",
        "removed feature owns device ID",
        "referenced by live feature",
        "table reference to",
    ] {
        assert!(
            detection.contains(required),
            "feature conflict detection should report live device-id table references to removed feature IDs {required}"
        );
    }

    for forbidden in [
        "crate::hardware",
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
            "Feature device-id conflict detection must consume semantic facts, not scan tables or own lifecycle state {forbidden}"
        );
    }

    assert!(
        architecture.contains("device-id table reference facts")
            && architecture
                .contains("live device table references an ID owned by a removed feature")
            && architecture.contains("`removed_feature_device_id_referenced_by_live_table`")
            && kernel_build_guide.contains("device-id table reference facts")
            && kernel_build_guide
                .contains("live device table references an ID owned by a removed feature")
            && kernel_build_guide.contains("`removed_feature_device_id_referenced_by_live_table`"),
        "docs should describe live device-id table conflict detection"
    );
}
