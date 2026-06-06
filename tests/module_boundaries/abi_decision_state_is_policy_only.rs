use super::common::*;

#[test]
fn abi_decision_state_is_policy_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let state = state_source(root);
    let plan = plan_source(root);

    let decision_section = state
        .split("pub(crate) struct AbiDecisionState")
        .nth(1)
        .and_then(|rest| rest.split("pub(crate) struct PrunePlan").next())
        .expect("generate/state.rs should define AbiDecisionState before PrunePlan");

    for required in [
        "policy: AbiPolicyConfig",
        "allow_public_header_removal: bool",
        "allow_uapi_header_removal: bool",
        "approved_public_headers: Vec<HeaderPath>",
        "approved_uapi_paths: Vec<UapiPath>",
        "pub(crate) fn new(",
        "remove_paths: &[RelativeKernelPath]",
        "UapiPath::new(path.to_path_buf())",
        "ABI decision rejected UAPI removal without explicit approval",
        "abi.allow_uapi_header_removal = true",
        "crate::abi::is_public_header_path(path)",
        "HeaderPath::new(path.to_string_lossy().into_owned())",
        "ABI decision rejected public header removal without explicit approval",
        "abi.allow_public_header_removal = true",
        "approved_public_headers.sort()",
        "approved_public_headers.dedup()",
        "approved_uapi_paths.sort()",
        "approved_uapi_paths.dedup()",
        "fn from_feature_resolution(resolution: &FeatureResolutionState)",
        "Self::new(resolution.abi_policy().clone(), resolution.remove_paths())",
        "pub(crate) fn policy(&self)",
        "pub(crate) fn allow_public_header_removal(&self)",
        "pub(crate) fn allow_uapi_header_removal(&self)",
        "pub(crate) fn approved_public_headers(&self)",
        "pub(crate) fn approved_uapi_paths(&self)",
        "pub(crate) fn has_abi_sensitive_removals(&self)",
    ] {
        assert!(
            decision_section.contains(required),
            "AbiDecisionState should capture explicit ABI policy decisions; missing {required}"
        );
    }

    for forbidden in [
        "pub(crate) policy",
        "pub(crate) allow_public_header_removal",
        "pub(crate) allow_uapi_header_removal",
        "pub(crate) approved_public_headers",
        "pub(crate) approved_uapi_paths",
        "RequestedGenerateState",
        "CandidateTreeState",
        "CandidateTreePath",
        "CandidateMetadataDir",
        "OutputRepoPath",
        "PublishedMetadataDir",
        "PublishedSnapshotState",
        "GenerateAttemptFailure",
        "AttemptMetadataDir",
        "CommittedOutputSnapshot",
        "SuccessfulCommitResult",
        "LockfilePath",
        "commit_if_changed",
        "write_authoritative_lockfile",
        "write_verified_published_snapshot_metadata",
        "std::fs::write",
    ] {
        assert!(
            !decision_section.contains(forbidden),
            "AbiDecisionState must not expose mutable fields or contain requested, candidate, published, failure, commit, lockfile, or mutation state; found {forbidden}"
        );
    }

    assert!(
        state.contains("abi_decision: AbiDecisionState")
            && state.contains(
                "let abi_decision = AbiDecisionState::from_feature_resolution(&feature_resolution)?"
            ),
        "ResolvedCandidateState should carry ABI decisions resolved from feature state"
    );
    assert!(
        plan.contains("resolved.abi_decision.allow_public_header_removal")
            && plan.contains("resolved.abi_decision.allow_uapi_header_removal")
            && plan.contains("resolved.abi_decision.approved_public_headers")
            && plan.contains("resolved.abi_decision.approved_uapi_paths"),
        "generate plan fingerprint should include ABI decision state"
    );
}
