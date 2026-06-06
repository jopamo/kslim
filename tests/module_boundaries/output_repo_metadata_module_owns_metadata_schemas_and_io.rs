use super::common::*;

#[test]
fn output_repo_metadata_module_owns_metadata_schemas_and_io() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output_repo = production_source(&root.join("src/output_repo.rs"));
    let metadata = production_source(&root.join("src/output_repo/metadata.rs"));

    for required in [
        "pub struct BaseMetadata",
        "pub struct GeneratedMetadata",
        "pub struct CandidateMetadata",
        "pub struct PublishedMetadata",
        "ensure_supported_metadata_schema_version",
        "pub enum CandidateMetadataMarker",
        "metadata_scope: CandidateMetadataMarker",
        "pub enum PublishedMetadataMarker",
        "metadata_scope: PublishedMetadataMarker",
        "use crate::model::{",
        "pub struct PublishedSnapshotMetadata",
        "pub struct AuthoritativePublishedState",
        "pub(crate) fn candidate_metadata_dir",
        "candidate_tree: &CandidateTreePath",
        "CandidateMetadataDir::new_in_candidate_tree",
        "pub(crate) fn published_metadata_dir",
        "output_repo: &OutputRepoPath",
        "PublishedMetadataDir::new_in_output_repo",
        "fn read_candidate_metadata",
        "metadata_dir: &CandidateMetadataDir",
        "fn write_candidate_metadata",
        "metadata: &CandidateMetadata",
        "fn serialize_candidate_metadata",
        "fn serialize_metadata_root",
        "fn read_published_metadata",
        "metadata_dir: &PublishedMetadataDir",
        "fn write_published_metadata",
        "metadata: &PublishedMetadata",
        "fn serialize_published_metadata",
        "pub fn write_base_metadata",
        "pub fn write_generated_metadata",
        "pub fn write_patch_metadata",
        "pub(crate) fn write_verified_published_snapshot_metadata",
        "output_repo: &OutputRepoPath",
        "metadata: &crate::generate::VerifiedPublishedSnapshotMetadata",
        "pub(crate) fn write_verified_committed_published_snapshot_metadata",
        "temporary_roots: &[&Path]",
        "fn write_verified_published_snapshot_metadata_to_dir",
        "published metadata write requires candidate verification proof",
        "pub(super) fn write_published_snapshot_metadata_unchecked",
        "pub(crate) fn load_committed_base_metadata",
        "pub(crate) fn load_committed_generated_metadata",
        "pub(crate) fn load_committed_published_snapshot_metadata",
        "pub(crate) fn load_authoritative_published_state",
        "lockfile_path: &LockfilePath",
        "output_repo: &OutputRepoPath",
        "pub(super) fn validate_candidate_metadata",
        "pub(super) fn validate_candidate_metadata_temporary_paths",
        "pub(crate) fn validate_committed_metadata_has_no_temporary_paths",
        "pub(super) fn validate_committed_metadata_named_files_have_no_temporary_paths",
        "pub(crate) fn validate_committed_metadata_has_no_raw_logs",
        "fn raw_log_marker",
        "pub(crate) fn validate_committed_metadata_has_only_allowed_reproducible_timestamps",
        "fn validate_committed_metadata_has_only_declared_reproducible_timestamps",
        "fn timestamp_markers",
        "pub(crate) fn validate_reproducible_metadata_timestamp",
        "fn temporary_path_markers",
        "crate::security::temporary_path_markers",
        "pub(super) fn is_published_snapshot_metadata_file",
    ] {
        assert!(
            metadata.contains(required),
            "output_repo/metadata.rs should own metadata schema/IO item {required}"
        );
    }

    for moved_item in [
        "pub struct BaseMetadata",
        "pub struct GeneratedMetadata",
        "pub struct PublishedSnapshotMetadata",
        "pub struct AuthoritativePublishedState",
        "pub fn write_base_metadata",
        "pub fn write_generated_metadata",
        "pub fn write_patch_metadata",
        "pub(crate) fn write_verified_published_snapshot_metadata",
        "pub(crate) fn write_verified_committed_published_snapshot_metadata",
        "fn write_verified_published_snapshot_metadata_to_dir",
        "pub(super) fn write_published_snapshot_metadata_unchecked",
        "pub(crate) fn load_committed_base_metadata",
        "pub(crate) fn load_committed_generated_metadata",
        "pub(crate) fn load_committed_published_snapshot_metadata",
        "pub(crate) fn load_authoritative_published_state",
        "fn read_committed_metadata_file",
        "fn committed_metadata_ref",
        "fn temporary_path_markers",
        "fn metadata_files",
        "fn collect_metadata_files",
    ] {
        assert!(
            !output_repo.contains(moved_item),
            "output_repo.rs should not retain metadata schema/IO implementation {moved_item}"
        );
    }

    assert!(
        output_repo.contains("write_verified_published_snapshot_metadata")
            && output_repo.contains("write_verified_committed_published_snapshot_metadata"),
        "output_repo.rs should re-export proof-guarded published snapshot metadata writes through the facade"
    );
    for forbidden_published_write_shape in [
        "write_verified_published_snapshot_metadata(\n    output_path: &str",
        "write_published_snapshot_metadata_unchecked(\n    output_path: &str",
    ] {
        assert!(
            !format!("{output_repo}\n{metadata}").contains(forbidden_published_write_shape),
            "published metadata writes must use PublishedMetadataDir/OutputRepoPath, not raw or candidate paths; found {forbidden_published_write_shape}"
        );
    }
    for forbidden_candidate_reader_api in [
        "read_candidate_metadata",
        "write_candidate_metadata",
        "serialize_candidate_metadata",
        "CandidateMetadata,",
        "CandidateMetadataMarker",
    ] {
        assert!(
            !output_repo.contains(forbidden_candidate_reader_api),
            "output_repo.rs must not expose candidate metadata reader/writer APIs to published-state callers; found {forbidden_candidate_reader_api}"
        );
    }
    for forbidden_raw_published_reader_api in [
        "read_published_metadata",
        "write_published_metadata",
        "serialize_published_metadata",
        "PublishedMetadata,",
        "PublishedMetadataMarker",
    ] {
        assert!(
            !output_repo.contains(forbidden_raw_published_reader_api),
            "output_repo.rs must not expose raw published metadata reader/writer APIs; callers should use OutputRepoPath-backed committed loaders, found {forbidden_raw_published_reader_api}"
        );
    }

    let authoritative_loader = metadata
        .split("pub(crate) fn load_authoritative_published_state")
        .nth(1)
        .and_then(|rest| {
            rest.split("fn ensure_no_committed_published_metadata_without_lockfile")
                .next()
        })
        .expect("metadata.rs should define authoritative published-state loader");
    for forbidden_candidate_dependency in [
        "CandidateMetadata",
        "CandidateMetadataDir",
        "candidate_metadata_dir",
        "read_candidate_metadata",
        "CANDIDATE_METADATA_FILE",
        "metadata_scope: CandidateMetadataMarker",
    ] {
        assert!(
            !authoritative_loader.contains(forbidden_candidate_dependency),
            "authoritative published-state loader must not read candidate metadata; found {forbidden_candidate_dependency}"
        );
    }

    let forbidden_reporting_or_sync = [
        "ReducerStats",
        "SelfTestResult",
        "ClassifiedDiagnostic",
        "EditRecord",
        "sync_working_tree",
        "sync_dir_contents",
        "publish_output_candidate",
        "branch_name(",
        "tag_name(",
        "crate::upstream",
    ];

    for forbidden in forbidden_reporting_or_sync {
        assert!(
            !metadata.contains(forbidden),
            "output_repo/metadata.rs must stay limited to metadata schemas and IO; found forbidden token {forbidden}"
        );
    }

    for forbidden_published_reader_shape in [
        "pub fn metadata_dir",
        "pub(crate) fn metadata_dir",
        "metadata_dir, write_base_metadata",
        "pub fn load_committed_base_metadata(output_path: &str",
        "pub fn load_committed_generated_metadata(\n    output_path: &str",
        "pub fn load_committed_published_snapshot_metadata(\n    output_path: &str",
        "pub(crate) fn load_authoritative_published_state(\n    project_root: &Path",
        "pub fn load_authoritative_published_state(\n    project_root: &Path,\n    output_path: &str",
        "pub(crate) fn read_published_metadata",
        "pub(crate) fn write_published_metadata",
        "pub(crate) fn serialize_published_metadata",
    ] {
        assert!(
            !metadata.contains(forbidden_published_reader_shape),
            "published metadata readers must require OutputRepoPath, not candidate/raw paths; found {forbidden_published_reader_shape}"
        );
    }
}
