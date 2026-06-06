use super::common::*;

#[test]
fn generate_verify_modules_define_candidate_verification_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let verify = production_source(&root.join("src/generate/verify.rs"));
    let verify_fs = production_source(&root.join("src/generate/verify/fs.rs"));
    let verify_metadata = production_source(&root.join("src/generate/verify/metadata.rs"));
    let verify_invariants = production_source(&root.join("src/generate/verify/invariants.rs"));
    let verify_report = production_source(&root.join("src/generate/verify/report.rs"));
    let verify_stage = production_source(&root.join("src/generate/verify/stage.rs"));

    for required in [
        "mod fs;",
        "mod metadata;",
        "mod invariants;",
        "mod report;",
        "mod stage;",
    ] {
        assert!(
            verify.contains(required),
            "generate/verify.rs should register candidate verification submodule {required}"
        );
    }

    for required in [
        "pub(super) fn ensure_candidate_is_observable",
        "pub(super) fn fingerprint_candidate_tree",
        "pub(super) fn fingerprint_candidate_metadata",
        "fn normalize_boundary_path",
    ] {
        assert!(
            verify_fs.contains(required),
            "generate/verify/fs.rs should own filesystem verification item {required}"
        );
    }

    for required in [
        "pub(super) struct CandidateMetadataSummary",
        "pub(super) fn read_candidate_metadata_summary",
        "pub(super) fn verify_candidate_metadata_complete",
        "pub(super) fn verify_no_host_only_absolute_paths_in_committed_candidate_metadata",
        "pub(super) fn verify_no_raw_logs_in_committed_candidate_metadata",
        "pub(super) fn verify_no_temporary_paths_in_committed_candidate_metadata",
        "pub(super) fn verify_only_reproducible_timestamps_in_committed_candidate_metadata",
    ] {
        assert!(
            verify_metadata.contains(required),
            "generate/verify/metadata.rs should own candidate metadata verification item {required}"
        );
    }

    for required in [
        "pub(super) struct ReducerReportFile",
        "pub(super) struct ReducerDiagnosticsFile",
        "pub(super) fn verify_candidate_reports",
        "pub(super) fn verify_report_paths_are_relative_and_normalized",
        "pub(super) fn read_reducer_report",
    ] {
        assert!(
            verify_report.contains(required),
            "generate/verify/report.rs should own report verification item {required}"
        );
    }

    for required in [
        "pub(super) fn verify_reducer_success",
        "pub(super) fn verify_selftest_policy",
        "pub(super) fn verify_no_unreasoned_edits",
        "pub(super) fn verify_no_broad_speculative_fallout_edits",
        "pub(super) fn verify_no_unknown_diagnostics_in_strict_mode",
        "pub(super) fn verify_no_unsupported_syntax_in_strict_mode",
        "fn verify_edit_record_byte_evidence",
    ] {
        assert!(
            verify_invariants.contains(required),
            "generate/verify/invariants.rs should own candidate invariant verification item {required}"
        );
    }

    for required in [
        "pub(crate) struct VerifiedGeneratedOutput",
        "pub(super) fn verify_generated_output(",
        "fn verify_required_metadata(",
        "pub(super) fn write_candidate_metadata_and_verify(",
    ] {
        assert!(
            verify.contains(required),
            "generate/verify.rs should own legacy verification glue item {required}"
        );
    }

    for required in [
        "pub(crate) enum VerificationStage",
        "EnsureCandidateObservable",
        "RejectTemporaryMetadataPaths",
        "RejectHostMetadataPaths",
        "RejectRawMetadataLogs",
        "VerifyReproducibleMetadataTimestamps",
        "ReadCandidateMetadata",
        "FingerprintCandidateTree",
        "VerifyCandidateMetadata",
        "VerifyReducerSuccess",
        "VerifyCandidateReports",
        "VerifyReportPaths",
        "VerifyReasonedEdits",
        "VerifyNoSpeculativeFallout",
        "VerifyNoUnknownDiagnostics",
        "VerifyNoUnsupportedSyntax",
        "VerifySelftestPolicy",
        "FingerprintCandidateMetadata",
        "pub(crate) const ALL",
        "pub(crate) const fn stable_name",
        "#[serde(rename = \"ensure_candidate_observable\")]",
        "#[serde(rename = \"fingerprint_candidate_metadata\")]",
    ] {
        assert!(
            verify_stage.contains(required),
            "generate/verify/stage.rs should define stable verification stage item {required}"
        );
    }

    for moved_item in [
        "struct CandidateMetadataSummary",
        "struct ReducerReportFile",
        "fn ensure_candidate_is_observable",
        "fn verify_candidate_metadata_complete",
        "fn verify_report_paths_are_relative_and_normalized",
        "fn verify_no_unreasoned_edits",
    ] {
        assert!(
            !verify.contains(moved_item),
            "generate/verify.rs must stay a thin verification entrypoint; found moved item {moved_item}"
        );
    }
}
