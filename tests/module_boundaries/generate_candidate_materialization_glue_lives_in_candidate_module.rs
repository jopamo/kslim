use super::common::*;

#[test]
fn generate_candidate_materialization_glue_lives_in_candidate_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let generate = production_source(&root.join("src/generate.rs"));
    let candidate = production_source(&root.join("src/generate/candidate.rs"));
    let candidate_write = production_source(&root.join("src/generate/candidate/write.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        candidate.contains("materialize_integrate_and_reduce_candidate_tree"),
        "generate/candidate.rs should expose the candidate materialization glue"
    );
    assert!(
        generate.contains("candidate::materialize_integrate_and_reduce_candidate_tree("),
        "src/generate.rs should delegate candidate materialization glue to generate/candidate"
    );

    for required in [
        "pub(in crate::generate) struct CandidateMaterialization",
        "pub(in crate::generate) enum CandidateMaterializationEvent",
        "pub(in crate::generate) fn materialize_integrate_and_reduce_candidate_tree(",
        "materialize_resolved_candidate_tree(plan, keep_temp)",
        "apply_patch_sources(profile, &mutation_target)",
        "ensure_patch_application_matches_plan(planned_patch_infos, applied_patch_infos.as_deref())",
        "apply_integrations(profile, &mutation_target)",
        "reduce_tree(&mutation_target, profile)",
        "log_candidate_generate_stage(GenerateStage::Reduce, \"reducer_summary\")",
    ] {
        assert!(
            candidate_write.contains(required),
            "generate/candidate/write.rs should own candidate materialization glue item {required}"
        );
    }

    for forbidden in [
        "candidate::apply_patch_sources(",
        "candidate::apply_integrations(",
        "candidate::reduce_tree(",
        "\nfn ensure_patch_application_matches_plan(",
        "log_generate_stage(GenerateStage::Reduce, \"reducer_summary\")",
    ] {
        assert!(
            !generate.contains(forbidden),
            "src/generate.rs should not retain extracted candidate materialization glue {forbidden}"
        );
    }

    for required in [
        "`src/generate/candidate/write.rs`",
        "Candidate materialization glue",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document extracted candidate materialization ownership through {required}"
        );
    }
}
