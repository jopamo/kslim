//! Prepared candidate publication into the managed output repository.
//!
//! This module consumes an already materialized and verified candidate path,
//! checks that required committed metadata is present and safe, then delegates
//! payload and metadata copying to the sync module.

use anyhow::Result;
use std::path::Path;

use crate::paths::{CandidateTreePath, OutputRepoPath};

use super::{metadata, report, sync};

pub(crate) fn publish_output_candidate(
    output_path: &OutputRepoPath,
    candidate_root: &CandidateTreePath,
    verified_tree_path: &CandidateTreePath,
) -> Result<()> {
    let candidate_metadata = metadata::candidate_metadata_dir(candidate_root)?;
    report::validate_candidate_committed_reports_temporary_paths(
        &candidate_metadata,
        &[candidate_root.as_path(), verified_tree_path.as_path()],
    )?;
    metadata::validate_candidate_metadata_temporary_paths(
        candidate_root.as_path(),
        &[candidate_root.as_path(), verified_tree_path.as_path()],
    )?;
    validate_output_candidate(candidate_root.as_path())?;

    sync::sync_working_tree(output_path, candidate_root)?;
    sync::sync_candidate_metadata_dir(output_path, candidate_root)?;
    sync::sync_candidate_committed_metadata_dir(output_path, candidate_root)?;
    Ok(())
}

pub fn validate_output_candidate(candidate_root: &Path) -> Result<()> {
    validate_candidate_tree_shape(candidate_root)?;
    let candidate_root_path = CandidateTreePath::new(candidate_root)?;
    let candidate_metadata = metadata::candidate_metadata_dir(&candidate_root_path)?;
    report::validate_candidate_committed_reports_temporary_paths(
        &candidate_metadata,
        &[candidate_root],
    )?;
    metadata::validate_candidate_metadata(candidate_root)?;
    Ok(())
}

fn validate_candidate_tree_shape(candidate_root: &Path) -> Result<()> {
    let checks = [
        "Makefile", "Kconfig", "arch", "drivers", "fs", "include", "kernel", "mm", "net", "scripts",
    ];
    for check in &checks {
        if !candidate_root.join(check).exists() {
            anyhow::bail!("generated tree missing essential path: {}", check);
        }
    }
    Ok(())
}
