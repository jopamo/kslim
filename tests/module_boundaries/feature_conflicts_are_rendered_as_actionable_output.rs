use super::common::*;

#[test]
fn feature_conflicts_are_rendered_as_actionable_output() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands = commands_source(root);
    let command_render = production_source(&root.join("src/command_render.rs"));
    let detection = production_source(&root.join("src/feature/conflict_detection.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    assert!(
        detection.contains("pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self>")
            && detection.contains("let graph = FeatureGraph::from_profile(profile)?;")
            && detection.contains("Self::from_graph(&graph)"),
        "FeatureConflictReport should expose profile-level semantic conflict detection"
    );

    for required in [
        "use crate::feature::{FeatureConflictReport, FeatureImpactReport};",
        "print_effective_feature_conflicts(profile)?",
        "fn print_effective_feature_conflicts(profile: &config::ProfileConfig) -> Result<()>",
        "FeatureConflictReport::from_profile(profile)?",
        "command_render::print_feature_conflicts(&conflicts);",
    ] {
        assert!(
            commands.contains(required),
            "feature-impact should emit semantic actionable conflicts {required}"
        );
    }

    for required in [
        "use crate::feature::{FeatureConflictReport, FeatureImpactReport};",
        "pub(crate) fn print_feature_conflicts(report: &FeatureConflictReport)",
        "feature conflicts:",
        "report.blocking_count()",
        "report.is_empty()",
        "conflict.stable_key()",
        "conflict.kind().stable_name()",
        "conflict.feature().as_str()",
        "conflict.subject().as_str()",
        "conflict.summary()",
        "conflict.suggested_action()",
        "conflict.strict_blocking()",
    ] {
        assert!(
            command_render.contains(required),
            "command rendering should print actionable conflict fields {required}"
        );
    }

    assert!(
        architecture.contains("`feature-impact` emits actionable feature conflicts")
            && architecture
                .contains("stable key, summary, suggested action, and strict-blocking status")
            && kernel_build_guide.contains("`feature-impact` emits actionable feature conflicts")
            && kernel_build_guide
                .contains("stable key, summary, suggested action, and strict-blocking status"),
        "docs should describe actionable feature conflict emission"
    );
}
