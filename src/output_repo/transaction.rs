//! Output repository initialization and command-facing transaction preflight.
//!
//! This module owns managed-output markers, existing output preflight gates
//! used by generate/publish commands, and deterministic git configuration
//! setup for managed output repositories.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::config::{KslimConfig, ProfileConfig};
use crate::error::KslimError;
use crate::paths::OutputRepoPath;

use super::{metadata, naming::initial_branch};

const MANAGED_FILE: &str = ".kslim/managed.toml";
const GIT_MANAGED_FILE: &str = ".git/kslim/managed.toml";

// ── Safety markers ────────────────────────────────────────────────────────────

pub fn is_kslim_managed(output_path: &str) -> bool {
    let output = Path::new(output_path);
    output.join(GIT_MANAGED_FILE).exists() || output.join(MANAGED_FILE).exists()
}

pub fn require_managed(output_path: &str) -> Result<()> {
    if !is_kslim_managed(output_path) {
        return Err(KslimError::NotManaged(output_path.to_string()).into());
    }
    Ok(())
}

pub fn require_clean(output_path: &str, force: bool) -> Result<()> {
    if crate::git::is_dirty(output_path)? {
        if force {
            log::warn!("output repo is dirty but --force specified, proceeding");
        } else {
            anyhow::bail!(
                "output repo at {} has uncommitted changes. \
                 Commit or stash them, or use --force to overwrite.",
                output_path
            );
        }
    }
    Ok(())
}

pub fn require_not_detached(output_path: &str, force: bool) -> Result<()> {
    if crate::git::is_detached_head(output_path)? {
        if force {
            log::warn!("output repo is in detached HEAD state but --force specified, proceeding");
        } else {
            anyhow::bail!(
                "output repo at {} is in detached HEAD state. \
                 Switch to a branch first, or use --force.",
                output_path
            );
        }
    }
    Ok(())
}

// ── Output repo init ──────────────────────────────────────────────────────────

pub fn init_output_repo(config: &KslimConfig, _profile: &ProfileConfig) -> Result<()> {
    let output_path = &config.output.path;
    let out = Path::new(output_path);
    let initial_branch = initial_branch(config);

    if !out.exists() {
        crate::fsutil::ensure_dir(out)?;
        crate::git::init_repo(output_path)?;
        // Set up git identity for commits (needed for git commit to work)
        let _ = crate::process::run_in_dir(
            output_path,
            "git",
            &["config", "user.email", "kslim@localhost"],
        );
        let _ = crate::process::run_in_dir(output_path, "git", &["config", "user.name", "kslim"]);
        crate::git::create_branch(output_path, &initial_branch)?;
        write_managed_marker(output_path, &config.project.name)?;
        crate::process::run_in_dir(
            output_path,
            "git",
            &[
                "commit",
                "--allow-empty",
                "-m",
                "kslim: initialize managed output repo",
            ],
        )?;
    } else {
        let git_dir = out.join(".git");
        if !git_dir.exists() {
            anyhow::bail!("output path exists but is not a git repository");
        }
        if !is_kslim_managed(output_path) {
            return Err(KslimError::NotManaged(output_path.to_string()).into());
        }
    }

    sync_repo_git_config(config, None)?;

    Ok(())
}

pub fn sync_repo_git_config(config: &KslimConfig, branch: Option<&str>) -> Result<()> {
    let output_path = &config.output.path;

    crate::git::config_set(output_path, "user.email", &config.git.user_email)?;
    crate::git::config_set(output_path, "user.name", &config.git.user_name)?;

    if let Some(publish) = &config.publish {
        let remote_name = &config.git.remote_name;
        crate::git::remote_add(output_path, remote_name, &publish.remote)?;
        crate::git::config_replace_all(
            output_path,
            &format!("remote.{}.fetch", remote_name),
            &format!("+refs/heads/*:refs/remotes/{}/*", remote_name),
        )?;

        if let Some(branch) = branch {
            crate::git::config_set(
                output_path,
                &format!("branch.{}.remote", branch),
                remote_name,
            )?;
            crate::git::config_set(
                output_path,
                &format!("branch.{}.merge", branch),
                &format!("refs/heads/{}", branch),
            )?;
        }
    }

    Ok(())
}

pub fn write_managed_marker(output_path: &str, project_name: &str) -> Result<()> {
    let kslim_dir = published_metadata_dir_path(Path::new(output_path))?;
    crate::fsutil::ensure_dir(&kslim_dir)?;

    let content = format!("managed_by = \"kslim\"\nproject = \"{}\"\n", project_name);
    std::fs::write(kslim_dir.join("managed.toml"), &content)?;
    Ok(())
}

fn published_metadata_dir_path(output_path: &Path) -> Result<PathBuf> {
    let output_repo = OutputRepoPath::new(output_path)?;
    Ok(metadata::published_metadata_dir(&output_repo)?
        .as_path()
        .to_path_buf())
}
