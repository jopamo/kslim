mod commit;
mod metadata;
mod naming;
mod publish;
mod report;
mod report_writer;
mod safety;
mod sync;
mod transaction;

#[allow(unused_imports)]
pub(crate) use crate::model::{
    GitCommitId as MetadataGitCommitId, MetadataFingerprint, MetadataSchemaVersion,
    OutputBranchName as MetadataOutputBranchName, PlanFingerprint, ReducerReportSummary,
    SelftestReportSummary, SnapshotId, ToolVersion as MetadataToolVersion, TreeFingerprint,
    CURRENT_METADATA_SCHEMA_VERSION,
};
pub(crate) use commit::stage_committed_metadata;
#[allow(unused_imports)]
pub(crate) use metadata::{candidate_metadata_dir, published_metadata_dir};
pub(crate) use metadata::{
    load_authoritative_published_state, load_committed_base_metadata,
    load_committed_generated_metadata, load_committed_published_snapshot_metadata,
    validate_committed_metadata_has_no_host_absolute_paths,
    validate_committed_metadata_has_no_raw_logs,
    validate_committed_metadata_has_no_temporary_paths,
    validate_committed_metadata_has_only_allowed_reproducible_timestamps,
    validate_reproducible_metadata_timestamp, write_verified_committed_published_snapshot_metadata,
    write_verified_published_snapshot_metadata,
};
#[allow(unused_imports)]
pub use metadata::{
    write_base_metadata, write_generated_metadata, write_patch_metadata, PublishedSnapshotMetadata,
    BASE_METADATA_FILE, COMMITTED_METADATA_DIR, GENERATED_METADATA_FILE, REPORT_FILE,
};
#[allow(unused_imports)]
pub use metadata::{
    AuthoritativePublishedState, BaseMetadata, GeneratedMetadata, PATCH_METADATA_FILE,
    PUBLISHED_SNAPSHOT_FILE,
};
#[allow(unused_imports)]
pub use naming::{
    branch_name, commit_message, snapshot_id, tag_name, CommitMessageDetails,
    COMMIT_MESSAGE_HOST_PATH_REDACTION, COMMIT_SECTION_BASE_COMMIT, COMMIT_SECTION_BASE_REF,
    COMMIT_SECTION_MODE, COMMIT_SECTION_PLAN_FINGERPRINT, COMMIT_SECTION_PROFILE,
    COMMIT_SECTION_REDUCER_SUMMARY, COMMIT_SECTION_SELFTEST_SUMMARY, COMMIT_SECTION_UPSTREAM,
    COMMIT_SUBJECT_IMPORT_PREFIX, STABLE_METADATA_FILE_NAMES,
};
pub(crate) use naming::sanitize_commit_message_value;
pub(crate) use publish::publish_output_candidate;
#[allow(unused_imports)]
pub use publish::validate_output_candidate;
#[allow(unused_imports)]
pub(crate) use report::{
    attempt_last_attempt_report_path, copy_candidate_reports_to_output_candidate_metadata,
    validate_last_attempt_json, CandidateReportCopySummary,
};
#[allow(unused_imports)]
pub use report::{
    GENERATE_REPORT_JSON, LAST_ATTEMPT_JSON, MATRIX_REPORT_JSON, NON_AUTHORITATIVE_ATTEMPT_SCOPE,
    REDUCER_DIAGNOSTICS_JSON, REDUCER_EDIT_SUMMARY_JSON, REDUCER_FAILURE_JSON,
    REDUCER_KCONFIG_REWRITE_REPORT_JSON, REDUCER_KCONFIG_SOLVER_REPORT_JSON,
    REDUCER_REMOVAL_MANIFEST, REDUCER_REPORT_JSON, REDUCER_REPORT_MD,
    REDUCER_SKIPPED_SITES_JSON,
};
#[allow(unused_imports)]
pub use report_writer::{
    reducer_artifact_path, write_failure_report, write_report, write_reducer_artifact,
    write_reducer_metadata, write_reducer_metadata_at_dir,
    write_reducer_metadata_at_dir_with_config, write_reducer_metadata_at_dir_with_context,
    write_reducer_metadata_with_config, write_reducer_metadata_with_context,
};
#[allow(unused_imports)]
pub(crate) use safety::{check_output_repo_safety, OutputRepoSafety};
#[allow(unused_imports)]
pub(crate) use sync::{
    sync_candidate_committed_metadata_dir, sync_candidate_metadata_dir,
    sync_candidate_to_output_area, sync_working_tree, SyncPolicy, SyncSummary,
};
#[allow(unused_imports)]
pub use transaction::{
    init_output_repo, is_kslim_managed, require_clean, require_managed, require_not_detached,
    sync_repo_git_config, write_managed_marker,
};

#[cfg(test)]
mod tests;
