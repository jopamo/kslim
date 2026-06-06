use super::common::*;

#[test]
fn generate_candidate_modules_define_candidate_build_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = production_source(&root.join("src/generate/candidate.rs"));
    let candidate_model = production_source(&root.join("src/generate/candidate/model.rs"));
    let candidate_write = production_source(&root.join("src/generate/candidate/write.rs"));
    let candidate_metadata = production_source(&root.join("src/generate/candidate/metadata.rs"));
    let candidate_errors = production_source(&root.join("src/generate/candidate/errors.rs"));

    for required in ["mod model;", "mod write;", "mod metadata;", "mod errors;"] {
        assert!(
            candidate.contains(required),
            "generate/candidate.rs should register candidate build submodule {required}"
        );
    }

    for required in [
        "pub(super) struct WorkspacePaths",
        "candidate_tree: CandidateTreePath",
        "pub(in crate::generate) struct MaterializedTree",
        "pub(in crate::generate) struct CandidateMutationTarget",
        "tree_path: CandidateTreePath",
        "pub(super) fn ensure_candidate_mutation_target",
        "pub(super) fn path_aliases_across_lifecycle",
        "pub(super) fn normalize_candidate_boundary_path",
    ] {
        assert!(
            candidate_model.contains(required),
            "generate/candidate/model.rs should own candidate model/boundary item {required}"
        );
    }

    for required in [
        "pub(in crate::generate) fn materialize_resolved_candidate_tree",
        "pub(in crate::generate) fn materialize_integrate_and_reduce_candidate_tree",
        "pub(in crate::generate) struct CandidateMaterialization",
        "pub(in crate::generate) enum CandidateMaterializationEvent",
        "pub(in crate::generate) fn ensure_patch_application_matches_plan",
        "pub(in crate::generate) fn apply_patch_sources",
        "pub(in crate::generate) fn apply_integrations",
        "pub(in crate::generate) fn reduce_tree",
        "fn prune_candidate_paths",
        "fn run_candidate_reducer",
    ] {
        assert!(
            candidate_write.contains(required),
            "generate/candidate/write.rs should own candidate mutation item {required}"
        );
    }

    for required in [
        "struct CandidateMetadataFile",
        "struct CandidateFailureAttemptFile",
        "pub(super) fn write_candidate_metadata",
        "pub(super) fn write_candidate_failure_attempt_metadata",
        "pub(super) fn record_partial_candidate_reducer_reports",
    ] {
        assert!(
            candidate_metadata.contains(required),
            "generate/candidate/metadata.rs should own candidate metadata item {required}"
        );
    }

    for required in [
        "pub(super) struct CandidateBuildStageFailure",
        "pub(super) fn record_candidate_stage",
        "pub(super) fn record_candidate_failure_attempt",
    ] {
        assert!(
            candidate_errors.contains(required),
            "generate/candidate/errors.rs should own candidate error item {required}"
        );
    }

    for moved_item in [
        "struct CandidateMetadataFile",
        "struct CandidateBuildStageFailure",
        "struct WorkspacePaths",
        "struct CandidateMutationTarget",
        "fn materialize_resolved_candidate_tree",
        "fn write_candidate_metadata",
        "fn ensure_candidate_mutation_target",
    ] {
        assert!(
            !candidate.contains(moved_item),
            "generate/candidate.rs must stay a thin candidate build facade; found moved item {moved_item}"
        );
    }
}
