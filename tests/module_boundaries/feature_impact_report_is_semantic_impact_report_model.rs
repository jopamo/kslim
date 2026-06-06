use super::common::*;

#[test]
fn feature_impact_report_is_semantic_impact_report_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let commands = commands_source(root);
    let command_render = production_source(&root.join("src/command_render.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    let report_section = feature
        .split("pub(crate) struct FeatureImpactReport")
        .nth(1)
        .and_then(|rest| rest.split("fn normalize_feature_kind_token").next())
        .expect("feature module should define FeatureImpactReport before feature helpers");

    for required in [
        "remove_paths: usize",
        "remove_configs: usize",
        "default_overrides: usize",
        "preserve_paths: usize",
        "preserve_configs: usize",
        "ownerships: Vec<FeatureOwnership>",
        "pub(crate) fn from_profile(profile: &ProfileConfig) -> Self",
        "profile.effective_removal_input()",
        "profile.effective_preservation_input()",
        "pub(crate) fn for_feature(profile: &ProfileConfig, feature: &str) -> Result<Self>",
        "FeatureId::new(feature)?",
        "pub(crate) fn with_ownerships(",
        "left.stable_key().cmp(&right.stable_key())",
        "pub(crate) fn remove_paths(&self) -> usize",
        "pub(crate) fn remove_configs(&self) -> usize",
        "pub(crate) fn default_overrides(&self) -> usize",
        "pub(crate) fn preserve_paths(&self) -> usize",
        "pub(crate) fn preserve_configs(&self) -> usize",
        "pub(crate) fn ownerships(&self) -> &[FeatureOwnership]",
        "pub(crate) fn ownership_count(&self) -> usize",
        "pub(crate) fn is_empty(&self) -> bool",
    ] {
        assert!(
            report_section.contains(required),
            "FeatureImpactReport should own semantic impact report fact {required}"
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
            !report_section.contains(forbidden),
            "FeatureImpactReport must not own reducer, candidate, published, lockfile, or mutation state {forbidden}"
        );
    }

    assert!(
        commands.contains("FeatureImpactReport")
            && commands.contains("FeatureImpactReport::from_profile(profile)")
            && command_render.contains("FeatureImpactReport")
            && command_render.contains("FeatureImpactReport::for_feature(profile, feature)?")
            && command_render.contains("impact.remove_paths()")
            && command_render.contains("impact.preserve_configs()"),
        "commands should render FeatureImpactReport without owning its model"
    );

    assert!(
        architecture.contains("`FeatureImpactReport` is the semantic report model")
            && architecture.contains("path/config/default")
            && architecture.contains("sorted ownership assertions")
            && architecture.contains("without owning command")
            && kernel_build_guide.contains("`FeatureImpactReport` is the typed impact model")
            && kernel_build_guide.contains("`feature-impact` counts"),
        "docs should describe FeatureImpactReport ownership"
    );
}
