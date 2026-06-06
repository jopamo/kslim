use super::common::*;

#[test]
fn plan_module_owns_generate_plans_and_frozen_verification() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let plan = plan_source(root);
    let frozen_plan = production_source(&root.join("src/plan/frozen_plan.rs"));
    let summary = production_source(&root.join("src/plan/summary.rs"));
    let generate_plan_facade = production_source(&root.join("src/generate/plan.rs"));
    let frozen_plan_facade = production_source(&root.join("src/generate/frozen_plan.rs"));
    let summary_facade = production_source(&root.join("src/generate/plan_summary.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        main.contains("mod plan;"),
        "main.rs should register the top-level immutable plan module"
    );

    for required in [
        "mod frozen_plan;",
        "mod summary;",
        "pub(crate) use frozen_plan::{",
        "FrozenPlanInputs",
        "pub(crate) use summary::{resolve_plan_summary, GeneratePlanSummary};",
        "pub(crate) struct GeneratePlan",
        "pub(crate) fn resolve_generate_plan(",
        "pub(crate) struct CandidatePlan",
        "pub(crate) fn resolve_candidate_plan_with_source_maps(",
        "frozen.verify_resolved_plan(&generate_plan)?;",
    ] {
        assert!(
            plan.contains(required),
            "src/plan/mod.rs should own immutable generate plan construction and verification hook {required}"
        );
    }

    for required in [
        "struct FrozenPlanDocument",
        "pub(crate) struct FrozenPlanInputs",
        "pub(crate) struct LoadedFrozenPlan",
        "pub(crate) fn to_generate_options(",
        "pub(crate) fn verify_resolved_plan(",
        "pub(crate) fn load_frozen_plan(",
        "pub(crate) fn write_frozen_plan_for_request(",
        "pub(crate) fn ensure_tree_matches_frozen_base(",
        "fn validate_document_header(",
        "fn validate_frozen_cli_overrides(",
        "ensure_equal(",
    ] {
        assert!(
            frozen_plan.contains(required),
            "src/plan/frozen_plan.rs should own frozen-plan document and verification behavior {required}"
        );
    }

    for required in [
        "pub(crate) struct GeneratePlanSummary",
        "pub(crate) fn from_plan(plan: &GeneratePlan) -> Self",
        "pub(crate) fn resolve_plan_summary(",
    ] {
        assert!(
            summary.contains(required),
            "src/plan/summary.rs should own immutable plan summary behavior {required}"
        );
    }

    for forbidden in [
        "CandidateTreeState",
        "CandidateTreePath",
        "CandidateMetadataDir",
        "PublishedSnapshotState",
        "PublishedMetadataDir",
        "GenerateAttemptFailure",
        "AttemptMetadataDir",
        "SuccessfulCommitResult",
        "write_authoritative_lockfile",
        "commit_output_repo_state",
        "materialize_resolved_candidate_tree",
        "write_verified_published_snapshot_metadata",
    ] {
        assert!(
            !frozen_plan.contains(forbidden),
            "frozen-plan verification must not own candidate, published, failure, commit, or materialization state; found {forbidden}"
        );
    }

    assert!(
        generate_plan_facade.contains("pub(crate) use crate::plan::*;")
            && frozen_plan_facade.contains("pub(crate) use crate::plan::{")
            && frozen_plan_facade.contains("FrozenPlanInputs")
            && summary_facade.contains(
                "pub(crate) use crate::plan::{resolve_plan_summary, GeneratePlanSummary};"
            ),
        "generate plan modules should remain compatibility facades over crate::plan"
    );

    assert!(
        architecture.contains("`plan/*`")
            && architecture.contains("immutable generate plans")
            && architecture.contains("frozen-plan")
            && architecture.contains("`generate/plan.rs`, `generate/frozen_plan.rs`, and `generate/plan_summary.rs` are compatibility facades"),
        "docs/architecture.md should document plan ownership and generate facades"
    );
}
