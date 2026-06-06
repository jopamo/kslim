use super::{
    commit_message, load_authoritative_published_state, publish_output_candidate,
    reducer_artifact_path, sync_candidate_metadata_dir, sync_working_tree, validate_output_candidate,
    write_base_metadata, write_failure_report, write_generated_metadata, write_patch_metadata,
    write_reducer_artifact, write_reducer_metadata, write_reducer_metadata_at_dir, write_report,
    BASE_METADATA_FILE, COMMITTED_METADATA_DIR, CommitMessageDetails, GENERATED_METADATA_FILE,
    LAST_ATTEMPT_JSON,
    PATCH_METADATA_FILE, PUBLISHED_SNAPSHOT_FILE, REDUCER_DIAGNOSTICS_JSON,
    REDUCER_EDIT_SUMMARY_JSON, REDUCER_KCONFIG_REWRITE_REPORT_JSON,
    REDUCER_KCONFIG_SOLVER_REPORT_JSON, REDUCER_REMOVAL_MANIFEST, REDUCER_REPORT_JSON,
    REDUCER_REPORT_MD, REDUCER_SKIPPED_SITES_JSON, REPORT_FILE,
};
use super::metadata::{is_host_specific_absolute_path, is_reproducible_timestamp};
use crate::diagnostics::ClassifiedDiagnostic;
use crate::edit_reason::{EditProofSource, EditReason, EditRecord, LineRange};
use crate::fixups::{AppliedFixup, FixupProof, SkippedFixup};
use crate::generate::GenerateStage;
use crate::kbuild::KbuildSkippedLine;
use crate::kconfig::{
    KconfigReportCounts, KconfigSolverDefaultReenabledSymbol, KconfigSolverReport,
    UnsupportedKconfigExpression,
};
use crate::patches::PatchInfo;
use crate::paths::{CandidateTreePath, LockfilePath, OutputRepoPath};
use crate::prune::RemovalAccounting;
use crate::reducer::ReducerStats;
use std::path::PathBuf;
use std::time::Duration;

fn output_repo_path(path: &std::path::Path) -> OutputRepoPath {
    OutputRepoPath::new(path).unwrap()
}

fn candidate_tree_path(path: &std::path::Path) -> CandidateTreePath {
    CandidateTreePath::new(path).unwrap()
}

fn create_valid_output_candidate(candidate: &std::path::Path) {
    for dir in &[
        "arch", "drivers", "fs", "include", "kernel", "mm", "net", "scripts", ".kslim",
    ] {
        std::fs::create_dir_all(candidate.join(dir)).unwrap();
    }
    std::fs::write(candidate.join("Makefile"), "# test\n").unwrap();
    std::fs::write(candidate.join("Kconfig"), "# test\n").unwrap();
    std::fs::write(
        candidate.join(".kslim/managed.toml"),
        "managed_by = \"kslim\"\n",
    )
    .unwrap();
    std::fs::write(
        candidate.join(format!(".kslim/{}", BASE_METADATA_FILE)),
        "base_ref = \"v1.0\"\n",
    )
    .unwrap();
    std::fs::write(
        candidate.join(format!(".kslim/{}", GENERATED_METADATA_FILE)),
        "generated_at = \"2026-01-01T00:00:00Z\"\n",
    )
    .unwrap();
    std::fs::write(candidate.join(".kslim/manifest.txt"), "hash  1  Makefile\n").unwrap();
    std::fs::write(
        candidate.join(format!(".kslim/{}", REPORT_FILE)),
        "report\n",
    )
    .unwrap();
}

#[path = "tests_metadata.rs"]
mod metadata;
#[path = "tests_sync.rs"]
mod sync;
#[path = "tests_report.rs"]
mod report;
#[path = "tests_publication.rs"]
mod publication;
#[path = "tests_safety.rs"]
mod safety;
