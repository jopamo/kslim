use super::common::*;

#[test]
fn output_repo_report_writer_module_owns_report_writes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output_repo = production_source(&root.join("src/output_repo.rs"));
    let writer = production_source(&root.join("src/output_repo/report_writer.rs"));
    let report = production_source(&root.join("src/output_repo/report.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod report_writer;",
        "pub use report_writer::{",
        "reducer_artifact_path",
        "write_reducer_artifact",
        "write_reducer_metadata",
        "write_reducer_metadata_at_dir",
        "write_reducer_metadata_at_dir_with_context",
        "write_report",
        "write_failure_report",
    ] {
        assert!(
            output_repo.contains(required),
            "output_repo.rs should expose report writer API item {required}"
        );
    }

    for required in [
        "pub fn reducer_artifact_path(output_path: &Path, artifact_name: &str) -> Result<PathBuf>",
        "pub fn write_reducer_artifact(",
        "pub fn write_reducer_metadata(",
        "pub fn write_reducer_metadata_with_config(",
        "pub fn write_reducer_metadata_with_context(",
        "pub fn write_reducer_metadata_at_dir(",
        "pub fn write_reducer_metadata_at_dir_with_config(",
        "pub fn write_reducer_metadata_at_dir_with_context(",
        "crate::reducer::render_reducer_stats_report_artifacts_with_manifest",
        "ReducerReportArtifactNames",
        "pub fn write_report(",
        "pub fn write_failure_report(",
        "fn render_generate_stage_for_report(stage: GenerateStage) -> &'static str",
        "stage.as_str()",
        "fn write_reducer_artifact_at_dir(",
        "fn remove_reducer_artifact_at_dir(",
        "report::metadata_report_path",
        "metadata::render_patch_section",
        "metadata::MetadataPathPolicy::Committed",
        "metadata::MetadataPathPolicy::Attempt",
        "branch_name(config, profile, resolved)",
    ] {
        assert!(
            writer.contains(required),
            "output_repo/report_writer.rs should own report write item {required}"
        );
    }

    for moved in [
        "\npub fn reducer_artifact_path",
        "\npub fn write_reducer_artifact",
        "\npub fn write_reducer_metadata",
        "\npub fn write_reducer_metadata_with_config",
        "\npub fn write_reducer_metadata_with_context",
        "\npub fn write_reducer_metadata_at_dir",
        "\npub fn write_reducer_metadata_at_dir_with_config",
        "\npub fn write_reducer_metadata_at_dir_with_context",
        "\npub fn write_report",
        "\npub fn write_failure_report",
        "\nfn render_generate_stage_for_report",
        "\nfn write_reducer_artifact_at_dir",
        "\nfn remove_reducer_artifact_at_dir",
    ] {
        assert!(
            !output_repo.contains(moved),
            "output_repo.rs should not retain moved report writer implementation {moved}"
        );
    }

    for forbidden in [
        "ReducerStats",
        "SelfTestResult",
        "KslimConfig",
        "ProfileConfig",
        "std::fs::write",
        "std::fs::remove",
        "render_reducer",
        "write_reducer_metadata",
    ] {
        assert!(
            !report.contains(forbidden),
            "output_repo/report.rs must not own report writing/rendering token {forbidden}"
        );
    }

    for required in [
        "`src/output_repo/report_writer.rs`",
        "Output repo report writers",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document report writer ownership through {required}"
        );
    }
}
