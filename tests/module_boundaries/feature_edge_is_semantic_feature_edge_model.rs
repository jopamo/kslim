use super::common::*;

#[test]
fn feature_edge_is_semantic_feature_edge_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    let edge_section = feature
        .split("pub(crate) enum FeatureEdgeKind")
        .nth(1)
        .and_then(|rest| rest.split("pub(crate) struct FeatureGraph").next())
        .expect("feature module should define FeatureEdgeKind and FeatureEdge before FeatureGraph");

    assert!(
        feature.contains("pub(crate) enum FeatureEdgeKind"),
        "feature module should define FeatureEdgeKind"
    );

    for required in [
        "Dependency",
        "Conflict",
        "PreservationBoundary",
        "pub(crate) fn from_stable_name(value: &str) -> Result<Self>",
        "pub(crate) const fn stable_name(self) -> &'static str",
        "\"dependency\"",
        "\"conflict\"",
        "\"preservation_boundary\"",
        "pub(crate) struct FeatureEdge",
        "from: FeatureId",
        "to: FeatureId",
        "kind: FeatureEdgeKind",
        "pub(crate) fn new(kind: FeatureEdgeKind, from: FeatureId, to: FeatureId)",
        "pub(crate) fn from_names(kind: FeatureEdgeKind, from: &str, to: &str)",
        "FeatureId::new(from)?",
        "FeatureId::new(to)?",
        "pub(crate) fn from(&self) -> &FeatureId",
        "pub(crate) fn to(&self) -> &FeatureId",
        "pub(crate) fn kind(&self) -> FeatureEdgeKind",
        "pub(crate) fn stable_key(&self) -> String",
        "format!(",
        "feature edge endpoints must be distinct",
    ] {
        assert!(
            edge_section.contains(required),
            "FeatureEdge should own pre-resolution semantic edge fact {required}"
        );
    }

    for forbidden in [
        "FeatureOwnership",
        "CandidateTreeState",
        "PublishedSnapshotState",
        "GeneratePlan",
        "RemovalManifest",
        "LockfilePath",
        "OutputRepoPath",
    ] {
        assert!(
            !edge_section.contains(forbidden),
            "FeatureEdge must not own ownership, reducer, candidate, published, or lockfile state {forbidden}"
        );
    }

    assert!(
        feature.contains("edges: BTreeMap<String, FeatureEdge>")
            && feature.contains("validate_feature_edge_endpoints(&nodes, &edge)?")
            && feature.contains("feature edge references unknown source feature")
            && feature.contains("feature edge references unknown target feature")
            && feature.contains("pub(crate) fn edges(&self) -> impl Iterator<Item = &FeatureEdge>"),
        "FeatureGraph should validate and expose FeatureEdge values"
    );

    assert!(
        architecture.contains("`FeatureEdge` is the pre-resolution directed")
            && architecture.contains("semantic relationship between two feature nodes")
            && architecture.contains("stable edge key derives")
            && architecture.contains("not")
            && architecture.contains("resolved ownership truth")
            && kernel_build_guide.contains("`FeatureEdge` models explicit directed semantic")
            && kernel_build_guide.contains("without treating them as resolved")
            && kernel_build_guide.contains("ownership truth"),
        "docs should describe FeatureEdge ownership"
    );
}
