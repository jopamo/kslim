use super::common::*;

#[test]
fn strict_mode_blocks_feature_conflict_mutation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature = production_source(&root.join("src/feature/mod.rs"));
    let state = state_source(root);
    let plan = plan_source(root);
    let generate = production_source(&root.join("src/generate.rs"));
    let candidate_write = production_source(&root.join("src/generate/candidate/write.rs"));
    let commands = commands_source(root);
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);

    for required in [
        "pub(crate) fn reject_blocking_conflicts_in_strict_mode(&self, strict_mode: bool) -> Result<()>",
        "unresolved feature conflicts block strict mutation",
        "conflict.stable_key()",
        "conflict.summary()",
        "conflict.suggested_action()",
    ] {
        assert!(
            feature.contains(required),
            "FeatureConflictReport should expose actionable strict-mutation rejection {required}"
        );
    }

    for required in [
        "feature_conflicts: FeatureConflictReport",
        "let feature_conflicts = FeatureConflictReport::from_profile(profile)?",
        "feature_conflicts,",
        "pub(crate) fn reject_unresolved_feature_conflicts_in_strict_mode(&self) -> Result<()>",
        ".reject_blocking_conflicts_in_strict_mode(self.reducer_plan.strict_mode())",
        "pub(crate) fn strict_mode(&self) -> bool",
        "self.report_unsupported_expressions",
        "self.fail_on_unknown_diagnostics",
        "self.reject_unproven_fixups",
        "self.reject_unreasoned_edits",
        "self.reject_speculative_fallout_edits",
    ] {
        assert!(
            state.contains(required),
            "ResolvedCandidateState should carry feature conflicts and expose a strict gate {required}"
        );
    }

    for required in [
        "resolved.feature_conflicts.total",
        "resolved.feature_conflicts.blocking",
        "resolved.feature_conflicts.key",
        "resolved.feature_conflicts.strict_blocking",
        "resolved.feature_conflicts.summary",
        "resolved.feature_conflicts.action",
    ] {
        assert!(
            plan.contains(required),
            "generate plan fingerprint should serialize resolved feature conflicts {required}"
        );
    }

    assert!(
        generate.contains("resolved.reject_unresolved_feature_conflicts_in_strict_mode()?;")
            && generate.contains("// ── materialize"),
        "generate should reject strict feature conflicts before materializing a candidate"
    );

    for required in [
        "plan.resolved\n        .reject_unresolved_feature_conflicts_in_strict_mode()?;",
        "FeatureConflictReport::from_profile(profile)?",
        ".reject_blocking_conflicts_in_strict_mode(profile.reducer.strict_mode())",
    ] {
        assert!(
            candidate_write.contains(required),
            "candidate mutation entrypoints should reject strict feature conflicts {required}"
        );
    }

    assert!(
        commands.contains("reject_profile_feature_conflicts_in_strict_mode(&profile)?;")
            && commands.contains("fn reject_profile_feature_conflicts_in_strict_mode(")
            && commands.contains("FeatureConflictReport::from_profile(profile)?")
            && commands.contains(
                ".reject_blocking_conflicts_in_strict_mode(profile.reducer.strict_mode())"
            ),
        "reduce-tree should reject strict feature conflicts before direct tree mutation"
    );

    assert!(
        architecture.contains("Strict mutation paths reject unresolved blocking feature conflicts")
            && architecture.contains("before candidate materialization, candidate reduction, or direct `reduce-tree` mutation")
            && kernel_build_guide
                .contains("Strict mutation paths reject unresolved blocking feature conflicts")
            && kernel_build_guide.contains(
                "before candidate materialization, candidate reduction, or direct `reduce-tree` mutation"
            ),
        "docs should describe strict feature conflict mutation gates"
    );
}
