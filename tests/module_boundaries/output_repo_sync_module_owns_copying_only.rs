use super::common::*;

#[test]
fn output_repo_sync_module_owns_copying_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output_repo = production_source(&root.join("src/output_repo.rs"));
    let sync = production_source(&root.join("src/output_repo/sync.rs"));

    assert!(
        sync.contains("pub(crate) fn sync_working_tree"),
        "output_repo/sync.rs should expose working-tree sync"
    );
    assert!(
        sync.contains("pub(crate) fn sync_candidate_to_output_area")
            && sync.contains("candidate: &CandidateTreePath")
            && sync.contains("output_path: &OutputRepoPath")
            && sync.contains("policy: &SyncPolicy")
            && sync.contains("Result<SyncSummary>"),
        "output_repo/sync.rs should expose narrow candidate-to-output sync"
    );
    for required in [
        "use crate::paths::{CandidateTreePath, OutputCandidateArea, OutputRepoPath};",
        "pub(crate) struct SyncPolicy",
        "pub(crate) struct SyncSummary",
        "OutputCandidateArea::from_output_repo",
        "files_copied: usize",
        "files_removed: usize",
        "directories_created: usize",
        "symlinks_copied: usize",
        "special_files_rejected: usize",
        "preserve_published_snapshot_metadata: bool",
        "SyncPolicy::replace_candidate_metadata",
        "metadata::candidate_metadata_dir",
        "metadata::published_metadata_dir",
        "metadata::COMMITTED_METADATA_DIR",
        "metadata::is_published_snapshot_metadata_file",
    ] {
        assert!(
            sync.contains(required),
            "output_repo/sync.rs should define narrow sync item {required}"
        );
    }
    assert!(
        sync.contains("output_path: &OutputRepoPath")
            && sync.contains("temp_tree_path: &CandidateTreePath"),
        "output_repo/sync.rs should require lifecycle-typed output and candidate paths"
    );
    assert!(
        sync.contains("fn sync_dir_contents"),
        "output_repo/sync.rs should own directory sync mechanics"
    );
    assert!(
        sync.contains("std::fs::copy"),
        "output_repo/sync.rs should own file copying"
    );
    assert!(
        !output_repo.contains("fn sync_dir_contents"),
        "output_repo.rs should not retain directory sync mechanics"
    );
    assert!(
        !output_repo.contains("fn sync_file"),
        "output_repo.rs should not retain file copy mechanics"
    );
    assert!(
        !output_repo.contains("fn regular_file_differs"),
        "output_repo.rs should not retain copy-diff mechanics"
    );
    assert!(
        !output_repo.contains("fn sync_symlink"),
        "output_repo.rs should not retain symlink sync mechanics"
    );

    let forbidden_schema_policy_or_reporting = [
        "serde::",
        "Deserialize",
        "Serialize",
        "toml::",
        "crate::config",
        "crate::generate",
        "crate::git",
        "crate::manifest",
        "crate::patches",
        "crate::reducer",
        "crate::selftest",
        "crate::upstream",
        "BaseMetadata",
        "GeneratedMetadata",
        "PublishedSnapshotMetadata",
        "AuthoritativePublishedState",
        "ReducerStats",
        "ClassifiedDiagnostic",
        "EditRecord",
        "publish_output_candidate",
        "validate_output_candidate",
        "write_base_metadata",
        "write_generated_metadata",
        "write_reducer_metadata",
        "load_committed",
        "RequestedConfigPath",
        "output_area: &OutputCandidateArea",
        "output_path: &str",
        "temp_tree_path: &str",
    ];

    for forbidden in forbidden_schema_policy_or_reporting {
        assert!(
            !sync.contains(forbidden),
            "output_repo/sync.rs must stay limited to copying/syncing files; found forbidden token {forbidden}"
        );
    }

    for forbidden_metadata_interpretation in [
        "CandidateMetadata",
        "PublishedMetadata",
        "schema_version",
        "metadata::validate_",
        "metadata::read_",
        "metadata::write_",
        "metadata::load_",
        "metadata::BASE_METADATA_FILE",
        "metadata::GENERATED_METADATA_FILE",
        "metadata::PATCH_METADATA_FILE",
        "metadata::PUBLISHED_SNAPSHOT_FILE",
        "metadata::REPORT_FILE",
        "published.toml",
        "toml::from",
        "toml::to",
        "serde_json",
    ] {
        assert!(
            !sync.contains(forbidden_metadata_interpretation),
            "output_repo/sync.rs must not interpret metadata; found forbidden token {forbidden_metadata_interpretation}"
        );
    }

    for forbidden_metadata_policy in ["OsStr::new(\".kslim\")", "preserve_top_level_kslim_dir"] {
        assert!(
            !sync.contains(forbidden_metadata_policy),
            "output_repo/sync.rs must preserve managed metadata through output_repo/metadata.rs constants, not hard-coded metadata policy {forbidden_metadata_policy}"
        );
    }
}
