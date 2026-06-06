use super::common::*;

#[test]
fn generate_failure_reporting_and_rollback_glue_lives_in_failure_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let generate = production_source(&root.join("src/generate.rs"));
    let failure = production_source(&root.join("src/generate/failure.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "pub(in crate::generate) use failure::{",
        "FailureReportContext",
        "capture_output_repo_failure_atomic_state",
        "rollback_output_repo_failure_atomic_state",
        "write_project_last_attempt",
    ] {
        assert!(
            generate.contains(required),
            "src/generate.rs should delegate failure reporting/rollback through {required}"
        );
    }

    for required in [
        "pub(in crate::generate) struct FailureReportContext",
        "pub(in crate::generate) fn record_generate_attempt_failure(",
        "pub(in crate::generate) fn ensure_no_attempt_failure_before_publication(",
        "pub(in crate::generate) fn project_attempt_metadata_dir(",
        "pub(in crate::generate) fn write_project_last_attempt(",
        "pub(in crate::generate) fn write_project_reducer_failure_report(",
        "fn render_last_attempt_json(",
        "fn render_reducer_failure_json(",
        "pub(in crate::generate) enum OutputRepoFailureAtomicState",
        "pub(in crate::generate) fn capture_output_repo_failure_atomic_state(",
        "pub(in crate::generate) fn capture_published_metadata_failure_atomic_state(",
        "pub(in crate::generate) fn rollback_published_metadata_failure_atomic_state(",
        "pub(in crate::generate) fn rollback_failed_run_lockfile_state(",
        "pub(in crate::generate) fn rollback_output_repo_failure_atomic_state(",
        "fn sync_plain_dir(",
        "fn remove_plain_path(",
    ] {
        assert!(
            failure.contains(required),
            "src/generate/failure.rs should own failure reporting/rollback item {required}"
        );
    }

    for forbidden in [
        "\nstruct FailureReportContext",
        "\nfn write_project_last_attempt(",
        "\nfn write_project_reducer_failure_report(",
        "\nfn render_last_attempt_json(",
        "\nfn capture_output_repo_failure_atomic_state(",
        "\nfn rollback_output_repo_failure_atomic_state(",
        "\nfn sync_plain_dir(",
    ] {
        assert!(
            !generate.contains(forbidden),
            "src/generate.rs should not retain extracted failure reporting/rollback glue {forbidden}"
        );
    }

    for required in [
        "`src/generate/failure.rs`",
        "Generate failure reporting and rollback glue",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document extracted generate failure ownership through {required}"
        );
    }
}
