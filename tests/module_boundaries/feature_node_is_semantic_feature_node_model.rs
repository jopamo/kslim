use super::common::*;

#[test]
fn feature_node_is_semantic_feature_node_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let generate_state = state_source(root);
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    let node_section = feature
        .split("pub(crate) struct FeatureNode")
        .nth(1)
        .and_then(|rest| rest.split("pub(crate) enum FeatureEdgeKind").next())
        .expect("feature module should define FeatureNode before FeatureEdgeKind");

    for required in [
        "intent: FeatureIntent",
        "pub(crate) fn from_intent(intent: FeatureIntent) -> Self",
        "pub(crate) fn id(&self) -> &FeatureId",
        "pub(crate) fn intent(&self) -> &FeatureIntent",
        "pub(crate) fn stable_key(&self) -> String",
        "format!(\"feature:{}\", self.id().as_str())",
    ] {
        assert!(
            node_section.contains(required),
            "FeatureNode should own pre-resolution semantic node fact {required}"
        );
    }

    for forbidden in [
        "FeatureEdge",
        "FeatureOwnership",
        "CandidateTreeState",
        "PublishedSnapshotState",
        "GeneratePlan",
        "RemovalManifest",
        "LockfilePath",
        "OutputRepoPath",
    ] {
        assert!(
            !node_section.contains(forbidden),
            "FeatureNode must not own resolved edges, ownership, reducer, candidate, published, or lockfile state {forbidden}"
        );
    }

    assert!(
        feature.contains("nodes: BTreeMap<FeatureId, FeatureNode>")
            && feature.contains("FeatureNode::from_intent(intent)")
            && feature.contains("pub(crate) fn nodes(&self) -> impl Iterator<Item = &FeatureNode>")
            && feature.contains("self.nodes.values().map(FeatureNode::intent)"),
        "FeatureGraph should index FeatureNode values by FeatureId"
    );

    assert!(
        generate_state.contains(
            "FeatureConflictReport, FeatureGraph, FeatureIntent, FeatureIntentAction, FeatureNode,"
        ) && generate_state.contains("pub(crate) fn from_feature_node(node: &FeatureNode) -> Self")
            && generate_state.contains("Self::from_intent(node.intent())")
            && generate_state.contains(".map(FeatureIntentEntryPlan::from_feature_node)"),
        "generate state should derive legacy plan entries from FeatureNode"
    );

    assert!(
        architecture.contains("`FeatureNode` is the pre-resolution semantic graph node")
            && architecture.contains("stable node key derives from")
            && architecture.contains("`FeatureId`")
            && architecture.contains("does not")
            && architecture.contains("own ownership classifications")
            && kernel_build_guide
                .contains("`FeatureNode` entries in a pre-resolution `FeatureGraph`"),
        "docs should describe FeatureNode ownership"
    );
}
