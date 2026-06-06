use super::common::*;

#[test]
fn generate_publish_module_defines_publication_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let generate = production_source(&root.join("src/generate.rs"));
    let publish = production_source(&root.join("src/generate/publish.rs"));
    let publish_stage = production_source(&root.join("src/generate/publish/stage.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod stage;",
        "pub(crate) const FIXED_PUBLISH_PIPELINE",
        "pub(crate) struct VerifiedPublishedSnapshotMetadata",
        "enum PublishedSnapshotMetadataProof",
        "pub(in crate::generate) fn from_candidate_verification(",
        "pub(crate) struct SuccessfulCommitResult",
        "pub(crate) fn commit_verified_candidate",
        "pub(in crate::generate) fn commit_output_repo_state",
        "pub(in crate::generate) fn write_authoritative_lockfile",
        "fn write_authoritative_lockfile_from_committed_publish",
        "fn authoritative_lockfile_from_committed_output",
        "fn render_commit_reducer_summary",
        "fn render_commit_selftest_summary",
        "pub(in crate::generate) fn write_output_metadata_report_and_manifest",
        "fn ensure_output_repo_matches_plan",
        "fn ensure_verification_still_matches_candidate",
        "fn verified_published_metadata_from_candidate_verification",
    ] {
        assert!(
            publish.contains(required),
            "generate/publish.rs should own publication boundary item {required}"
        );
    }

    for required in [
        "pub(in crate::generate) use publish::{",
        "commit_output_repo_state",
        "write_authoritative_lockfile",
        "SuccessfulCommitResult",
        "pub(crate) use publish::VerifiedPublishedSnapshotMetadata",
    ] {
        assert!(
            generate.contains(required),
            "src/generate.rs should delegate publication glue through publish.rs re-export {required}"
        );
    }

    for forbidden in [
        "\nstruct SuccessfulCommitResult",
        "\npub(crate) struct VerifiedPublishedSnapshotMetadata",
        "\nfn write_authoritative_lockfile(",
        "\nfn authoritative_lockfile_from_committed_output(",
        "\nfn render_commit_reducer_summary(",
        "\nfn render_commit_selftest_summary(",
        "\nfn commit_output_repo_state(",
        "\nfn write_output_metadata_report_and_manifest(",
    ] {
        assert!(
            !generate.contains(forbidden),
            "src/generate.rs should not retain extracted publication glue {forbidden}"
        );
    }

    for required in [
        "pub(crate) enum PublishStage",
        "CheckOutputPlan",
        "CheckOutputSafety",
        "ReverifyCandidate",
        "StageOutputCandidate",
        "ReverifyStagedCandidate",
        "BuildPublishedMetadata",
        "CaptureOutputRollback",
        "CreateOutputBranch",
        "SyncOutputTree",
        "SyncOutputMetadata",
        "WritePublishedMetadata",
        "CommitOutput",
        "BuildPublishedSnapshot",
        "UpdateAuthoritativeLockfile",
        "pub(crate) const ALL",
        "pub(crate) const fn stable_name",
        "#[serde(rename = \"check_output_plan\")]",
        "#[serde(rename = \"update_authoritative_lockfile\")]",
    ] {
        assert!(
            publish_stage.contains(required),
            "generate/publish/stage.rs should define stable publish stage item {required}"
        );
    }

    for required in [
        "`src/generate/publish.rs`",
        "Generate publication glue",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document publication module ownership through {required}"
        );
    }
}
