use super::common::*;

#[test]
fn feature_graph_is_semantic_feature_graph_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let generate_state = state_source(root);
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    let graph_section = feature
        .split("pub(crate) struct FeatureGraph")
        .nth(1)
        .and_then(|rest| rest.split("pub(crate) enum FeatureIntentAction").next())
        .expect("feature module should define FeatureGraph before FeatureIntentAction");

    assert!(
        feature.contains("pub(crate) struct FeatureGraph")
            && graph_section.contains("nodes: BTreeMap<FeatureId, FeatureNode>")
            && graph_section.contains("edges: BTreeMap<String, FeatureEdge>")
            && graph_section.contains(
                "pub(crate) fn new(intents: impl IntoIterator<Item = FeatureIntent>) -> Result<Self>"
            )
            && graph_section.contains("pub(crate) fn with_edges(")
            && graph_section.contains("FeatureNode::from_intent(intent)")
            && graph_section.contains("validate_feature_edge_endpoints(&nodes, &edge)?")
            && graph_section.contains("pub(crate) fn from_profile(profile: &ProfileConfig)")
            && graph_section.contains("FeatureIntentAction::Remove")
            && graph_section.contains("FeatureIntentAction::Preserve")
            && graph_section.contains("duplicate feature id")
            && graph_section.contains("duplicate feature edge")
            && graph_section.contains("pub(crate) fn edge_count(&self) -> usize")
            && graph_section
                .contains("pub(crate) fn nodes(&self) -> impl Iterator<Item = &FeatureNode>")
            && graph_section
                .contains("pub(crate) fn intents(&self) -> impl Iterator<Item = &FeatureIntent>")
            && graph_section
                .contains("pub(crate) fn edges(&self) -> impl Iterator<Item = &FeatureEdge>"),
        "FeatureGraph should own the pre-resolution feature intent graph"
    );

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
            !graph_section.contains(forbidden),
            "FeatureGraph must not own resolved graph, reducer, candidate, published, or lockfile state {forbidden}"
        );
    }

    assert!(
        generate_state.contains(
            "FeatureConflictReport, FeatureGraph, FeatureIntent, FeatureIntentAction, FeatureNode,"
        ) && generate_state.contains("let graph = FeatureGraph::from_profile(profile)?;")
            && generate_state.contains("FeatureIntentPlan::from_feature_graph(&graph)")
            && generate_state.contains(".map(FeatureIntentEntryPlan::from_feature_node)"),
        "generate state should consume FeatureGraph before building resolved plan entries"
    );

    assert!(
        architecture.contains("`FeatureGraph` is the")
            && architecture.contains("pre-resolution semantic graph")
            && architecture.contains("container keyed by")
            && architecture.contains("keyed by")
            && architecture.contains("`FeatureId`")
            && architecture.contains("typed feature nodes")
            && architecture.contains("semantic")
            && architecture.contains("edges without owning")
            && architecture.contains("ownership classifications")
            && kernel_build_guide.contains("pre-resolution `FeatureGraph`"),
        "docs should describe FeatureGraph ownership"
    );
}
