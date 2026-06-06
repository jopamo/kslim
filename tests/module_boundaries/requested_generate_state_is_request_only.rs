use super::common::*;

#[test]
fn requested_generate_state_is_request_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let state = state_source(root);
    let plan = plan_source(root);

    let requested_section = state
        .split("pub(crate) struct RequestedGenerateState")
        .nth(1)
        .and_then(|rest| {
            rest.split("pub(crate) struct ResolvedCandidateState")
                .next()
        })
        .expect("generate/state.rs should define RequestedGenerateState before resolved state");

    for required in [
        "config_path: RequestedConfigPath",
        "selected_profile: ProfileName",
        "cli_overrides: CliOverrides",
        "pub(crate) fn new(",
        "pub(crate) fn from_inputs(",
        "RequestedConfigPath::new(config_path)?",
        "ProfileName::new(profile.profile.name.clone())?",
        "CliOverrides::from_options(opts)",
        "GenerateStatePhase::Requested",
    ] {
        assert!(
            requested_section.contains(required),
            "RequestedGenerateState should capture only request inputs through typed fields; missing {required}"
        );
    }

    for forbidden in [
        "ResolvedBase",
        "ResolvedCandidateState",
        "CandidateTreePath",
        "CandidateMetadataDir",
        "OutputRepoPath",
        "PublishedMetadataDir",
        "PublishedSnapshotState",
        "LockfilePath",
        "GenerateAttemptFailure",
    ] {
        assert!(
            !requested_section.contains(forbidden),
            "RequestedGenerateState must not contain resolved, candidate, published, or failure state; found {forbidden}"
        );
    }

    assert!(
        plan.contains("pub(crate) requested: RequestedGenerateState")
            && plan.contains("pub(crate) resolved: ResolvedCandidateState"),
        "GeneratePlan should tie requested state to resolved candidate state without merging phases"
    );
}
