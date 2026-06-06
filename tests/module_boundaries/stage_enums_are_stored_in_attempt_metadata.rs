use super::common::*;

#[test]
fn stage_enums_are_stored_in_attempt_metadata() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let generate_failure = production_source(&root.join("src/generate/failure.rs"));
    let candidate_metadata = production_source(&root.join("src/generate/candidate/metadata.rs"));

    for required in [
        "stage: GenerateStage",
        "render_generate_stage_json(stage)",
        "serde_json::to_string(&stage)",
    ] {
        assert!(
            generate_failure.contains(required),
            "last-attempt attempt metadata should store typed GenerateStage through {required}"
        );
    }

    for required in [
        "struct CandidateFailureAttemptFile",
        "stage: GenerateStage",
        "stage,",
    ] {
        assert!(
            candidate_metadata.contains(required),
            "candidate attempt metadata should store typed GenerateStage through {required}"
        );
    }

    for required in [
        "struct GenerateFailureFile",
        "stage: GenerateStage",
        "stage,",
    ] {
        assert!(
            generate_failure.contains(required),
            "generate failure attempt metadata should store typed GenerateStage through {required}"
        );
    }
}
