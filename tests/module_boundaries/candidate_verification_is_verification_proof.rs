use super::common::*;

#[test]
fn candidate_verification_is_verification_proof() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let verify = production_source(&root.join("src/generate/verify.rs"));
    let publish = production_source(&root.join("src/generate/publish.rs"));

    let verification_section = verify
        .split("pub(crate) struct CandidateVerification")
        .nth(1)
        .and_then(|rest| rest.split("fn plan_requires_reducer").next())
        .expect("generate/verify.rs should define CandidateVerification before helper functions");

    for required in [
        "tree_fingerprint: TreeFingerprint",
        "metadata_fingerprint: MetadataFingerprint",
        "reducer_ok: bool",
        "selftest_ok: bool",
        "report_ok: bool",
        "fn new(",
        "candidate verification cannot be constructed from failed checks",
        "pub(crate) fn tree_fingerprint(&self)",
        "pub(crate) fn metadata_fingerprint(&self)",
        "pub(crate) fn reducer_ok(&self)",
        "pub(crate) fn selftest_ok(&self)",
        "pub(crate) fn report_ok(&self)",
        "pub(crate) fn all_checks_ok(&self)",
        "pub(crate) fn verify_candidate(",
        "ensure_candidate_is_observable(candidate)?",
        "fingerprint_candidate_tree(candidate.tree.as_path())?",
        "fingerprint_candidate_metadata(candidate.metadata_dir.as_path())?",
        "CandidateVerification::new(",
    ] {
        assert!(
            verification_section.contains(required),
            "CandidateVerification should be a private-field proof produced by verification; missing {required}"
        );
    }

    for forbidden in [
        "pub(crate) tree_fingerprint",
        "pub(crate) metadata_fingerprint",
        "pub(crate) reducer_ok",
        "pub(crate) selftest_ok",
        "pub(crate) report_ok",
        "RequestedGenerateState",
        "ResolvedCandidateState",
        "OutputRepoPath",
        "PublishedMetadataDir",
        "PublishedSnapshotState",
        "GenerateAttemptFailure",
        "AttemptMetadataDir",
        "SuccessfulCommitResult",
        "LockfilePath",
        "commit_if_changed",
        "write_authoritative_lockfile",
        "write_verified_published_snapshot_metadata",
        "std::fs::write",
    ] {
        assert!(
            !verification_section.contains(forbidden),
            "CandidateVerification must not expose mutable fields or contain requested, resolved, published, failure, commit, lockfile, or mutation state; found {forbidden}"
        );
    }

    assert!(
        publish.contains("verification: CandidateVerification")
            && publish.contains("ensure_verification_still_matches_candidate")
            && publish.contains("verified_published_metadata_from_candidate_verification"),
        "publish should consume CandidateVerification as proof and reverify before publication"
    );
}
