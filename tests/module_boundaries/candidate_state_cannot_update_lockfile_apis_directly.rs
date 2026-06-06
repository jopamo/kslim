use super::common::*;

#[test]
fn candidate_state_cannot_update_lockfile_apis_directly() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = [
        production_source(&root.join("src/generate/candidate.rs")),
        production_source(&root.join("src/generate/candidate/errors.rs")),
        production_source(&root.join("src/generate/candidate/metadata.rs")),
        production_source(&root.join("src/generate/candidate/model.rs")),
        production_source(&root.join("src/generate/candidate/write.rs")),
    ]
    .join("\n");
    let lockfile = production_source(&root.join("src/lockfile.rs"));
    let docs = std::fs::read_to_string(root.join("docs/architecture.md"))
        .expect("architecture doc should be readable");

    for required in [
        "pub(crate) struct ResolvedBaseLockfileUpdate",
        "pub(crate) struct PublishedLockfileUpdate",
        "pub(crate) fn write_resolved_base_lockfile",
        "pub(crate) fn write_published_lockfile",
        "fn write_lockfile_contents",
    ] {
        assert!(
            lockfile.contains(required),
            "lockfile.rs should expose phase-specific lockfile update item {required}"
        );
    }
    assert!(
        !lockfile.contains("pub fn write_lockfile"),
        "lockfile.rs must not expose raw lockfile writes that candidate state could call directly"
    );

    let forbidden_lockfile_update_tokens = [
        "crate::lockfile",
        "lockfile::",
        "ResolvedBaseLockfileUpdate",
        "PublishedLockfileUpdate",
        "LockfilePath",
        "LockfileFailureAtomicState",
        "capture_lockfile_failure_atomic_state",
        "rollback_lockfile_failure_atomic_state",
        "load_lockfile",
        "write_resolved_base_lockfile",
        "write_published_lockfile",
        "write_lockfile",
        "Lockfile {",
        "PublishedLockfile",
        "kslim.lock",
    ];

    for forbidden in forbidden_lockfile_update_tokens {
        assert!(
            !candidate.contains(forbidden),
            "generate/candidate.rs must not update authoritative lockfile state directly; found forbidden token {forbidden}"
        );
    }

    for forbidden in [
        "crate::generate::candidate",
        "CandidateTreeState",
        "CandidateTreePath",
        "CandidateMetadataDir",
        "CandidateMutationTarget",
        "MaterializedTree",
    ] {
        assert!(
            !lockfile.contains(forbidden),
            "lockfile APIs must not accept candidate state as lockfile authority; found forbidden token {forbidden}"
        );
    }

    for required in [
        "`generate/candidate/*` builds and mutates the private candidate tree",
        "must not open the output repo or update `kslim.lock`",
        "It must not import",
        "`lockfile.rs` update APIs",
    ] {
        assert!(
            docs.contains(required),
            "architecture docs should document candidate/lockfile authority boundary; missing {required}"
        );
    }
}
