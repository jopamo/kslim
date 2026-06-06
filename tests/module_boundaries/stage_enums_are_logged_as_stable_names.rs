use super::common::*;

#[test]
fn stage_enums_are_logged_as_stable_names() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let generate = production_sources(
        &root,
        &[
            "src/generate.rs",
            "src/generate/orchestration.rs",
            "src/generate/failure.rs",
            "src/generate/publish.rs",
        ],
    );
    let candidate_write = production_source(&root.join("src/generate/candidate/write.rs"));
    let verify = production_source(&root.join("src/generate/verify.rs"));
    let publish = production_source(&root.join("src/generate/publish.rs"));
    let reducer_pipeline = production_source(&root.join("src/reducer/pipeline.rs"));
    let reducer_engine = production_source(&root.join("src/reducer/engine.rs"));

    for required in [
        "fn log_generate_stage(stage: GenerateStage",
        "stage.as_str()",
        "log_generate_stage(stage, \"enter\")",
        "log_generate_stage(GenerateStage::Resolve, \"prepare\")",
        "log_generate_stage(GenerateStage::Commit, \"commit_output_repo_state\")",
    ] {
        assert!(
            generate.contains(required),
            "generate logs should store stable GenerateStage names through {required}"
        );
    }

    for required in [
        "fn log_candidate_generate_stage(stage: GenerateStage",
        "stage.as_str()",
        "log_candidate_generate_stage(GenerateStage::Integrate, \"apply_patch_sources\")",
        "log_candidate_generate_stage(GenerateStage::Prune, \"prune_candidate_paths\")",
        "log_candidate_generate_stage(GenerateStage::Reduce, \"run_candidate_reducer\")",
    ] {
        assert!(
            candidate_write.contains(required),
            "candidate logs should store stable GenerateStage names through {required}"
        );
    }

    for required in [
        "fn log_verification_stage(stage: VerificationStage",
        "stage.as_str()",
        "log_verification_stage(VerificationStage::EnsureCandidateObservable)",
        "log_verification_stage(VerificationStage::ReadCandidateMetadata)",
        "log_verification_stage(VerificationStage::FingerprintCandidateMetadata)",
    ] {
        assert!(
            verify.contains(required),
            "verification logs should store stable VerificationStage names through {required}"
        );
    }

    for required in [
        "fn log_publish_stage(stage: PublishStage",
        "stage.as_str()",
        "log_publish_stage(PublishStage::CheckOutputPlan)",
        "log_publish_stage(PublishStage::WritePublishedMetadata)",
        "log_publish_stage(PublishStage::UpdateAuthoritativeLockfile)",
    ] {
        assert!(
            publish.contains(required),
            "publish logs should store stable PublishStage names through {required}"
        );
    }

    for required in [
        "fn log_reducer_stage(stage: ReducerStage",
        "stage.as_str()",
        "log_reducer_stage(ReducerStage::BuildManifest)",
        "log_reducer_stage(ReducerStage::RewriteKconfig)",
        "log_reducer_stage(ReducerStage::RewriteIncludes)",
    ] {
        assert!(
            reducer_pipeline.contains(required),
            "reducer pipeline logs should store stable ReducerStage names through {required}"
        );
    }

    for required in [
        "fn log_reducer_stage(stage: ReducerStage",
        "stage.as_str()",
        "log_reducer_stage(ReducerStage::RunSelftests)",
        "log_reducer_stage(ReducerStage::ClassifyDiagnostics)",
        "log_reducer_stage(ReducerStage::ApplyFixups)",
        "log_reducer_stage(ReducerStage::ReindexAndRepeat)",
    ] {
        assert!(
            reducer_engine.contains(required),
            "reducer engine logs should store stable ReducerStage names through {required}"
        );
    }
}
