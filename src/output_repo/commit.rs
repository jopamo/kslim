//! Output repository commit staging helpers.
//!
//! This module owns git-index preparation required to make committed metadata
//! authoritative even when upstream kernel ignore rules hide dot-directories.

use anyhow::Result;

use crate::paths::OutputRepoPath;

use super::metadata::COMMITTED_METADATA_DIR;

pub(crate) fn stage_committed_metadata(output_repo: &OutputRepoPath) -> Result<()> {
    let metadata_dir = output_repo.as_path().join(COMMITTED_METADATA_DIR);
    if !metadata_dir.exists() {
        return Ok(());
    }
    let output_path = output_repo
        .as_path()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("published output repo path is not valid UTF-8"))?;
    crate::process::run_in_dir(
        output_path,
        "git",
        &["add", "-A", "--force", "--", COMMITTED_METADATA_DIR],
    )?;
    Ok(())
}
