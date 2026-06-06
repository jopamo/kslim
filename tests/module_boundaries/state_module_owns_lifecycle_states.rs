use super::common::*;

#[test]
fn state_module_owns_lifecycle_states() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let state = state_source(root);
    let generate_state_facade = production_source(&root.join("src/generate/state.rs"));
    let fingerprint = production_source(&root.join("src/state/fingerprint.rs"));
    let docs = production_source(&root.join("docs/architecture.md"));

    assert!(
        main.contains("mod state;") && main.contains("mod generate;"),
        "main.rs should register lifecycle state before generate consumers use it"
    );
    assert!(
        generate_state_facade.contains("pub(crate) use crate::state::*;")
            && !generate_state_facade.contains("pub(crate) struct RequestedGenerateState")
            && !generate_state_facade.contains("pub(crate) struct CandidateTreeState"),
        "src/generate/state.rs should be a compatibility facade over src/state/* ownership"
    );

    for required in [
        "mod fingerprint;",
        "pub(crate) enum GenerateStatePhase",
        "pub(crate) struct GenerateStateIdentity",
        "pub(crate) struct RequestedGenerateState",
        "pub(crate) struct ResolvedCandidateState",
        "pub(crate) enum FeatureResolutionSource",
        "pub(crate) struct FeatureResolutionState",
        "pub(crate) struct AbiDecisionState",
        "pub(crate) struct CandidateTreeState",
        "pub(crate) struct PublishedSnapshotState",
        "pub(crate) struct CommittedOutputSnapshot",
        "pub(crate) enum GenerateErrorKind",
        "pub(crate) struct GenerateAttemptFailure",
    ] {
        assert!(
            state.contains(required),
            "src/state/mod.rs should own lifecycle state item {required}"
        );
    }

    for required in [
        "FeatureGraphFingerprint",
        "RemovalManifestFingerprint",
        "AbiPolicyFingerprint",
        "ArchPolicyFingerprint",
    ] {
        assert!(
            fingerprint.contains(required),
            "src/state/fingerprint.rs should own state fingerprint item {required}"
        );
    }

    for phase in [
        "Requested",
        "Resolved",
        "Candidate",
        "OutputTarget",
        "Published",
        "Failure",
    ] {
        assert!(
            state.contains(phase),
            "GenerateStatePhase should name lifecycle phase {phase}"
        );
    }

    for required in [
        "Resolved, candidate, published, and failure state",
        "must stay in separate state objects",
        "feature resolution without removal input cannot contain removal facts",
        "candidate tree state cannot advance before materialization",
        "cannot be converted into published state",
        "CommittedOutputSnapshot::from_successful_commit",
    ] {
        assert!(
            state.contains(required),
            "src/state/mod.rs should encode lifecycle separation invariant {required}"
        );
    }

    for forbidden in [
        "std::fs::write",
        "std::fs::remove_",
        "commit_if_changed",
        "write_authoritative_lockfile",
        "write_verified_published_snapshot_metadata",
    ] {
        assert!(
            !state.contains(forbidden),
            "src/state/mod.rs must model state without mutating or publishing side effects; found {forbidden}"
        );
    }

    assert!(
        docs.contains("`state/*` | Requested, resolved, candidate, attempt, published, and failure state models")
            && docs.contains("`generate/state.rs` is a compatibility facade only"),
        "docs/architecture.md should document state/* ownership and generate/state facade"
    );
}
