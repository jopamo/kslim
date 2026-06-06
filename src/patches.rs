use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;
use std::process::Command;

use crate::config::{PatchConfig, PatchSourceConfig};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PatchInfo {
    pub source: String,
    pub worktree_path: String,
    pub branch: String,
    pub head_commit: String,
    pub merge_base: String,
    pub base_remote: String,
    pub base_ref: String,
    pub patch_count: usize,
}

pub fn inspect_all(patches: &PatchConfig) -> Result<Vec<PatchInfo>> {
    patches
        .sources()
        .into_iter()
        .map(inspect_source)
        .collect::<Result<Vec<_>>>()
}

pub fn apply_all(tree_path: &str, patches: &PatchConfig) -> Result<Vec<PatchInfo>> {
    let mut infos = Vec::new();
    for source in patches.sources() {
        let info = apply_source(tree_path, source)?;
        infos.push(info);
    }
    Ok(infos)
}

pub fn apply_resolved_all(tree_path: &str, infos: &[PatchInfo]) -> Result<Vec<PatchInfo>> {
    for info in infos {
        apply_resolved_source(tree_path, info)?;
    }
    Ok(infos.to_vec())
}

pub fn total_patch_count(infos: &[PatchInfo]) -> usize {
    infos.iter().map(|info| info.patch_count).sum()
}

fn inspect_source(patches: &PatchSourceConfig) -> Result<PatchInfo> {
    if patches.source != "worktree" {
        anyhow::bail!(
            "patch source '{}' is not supported (expected 'worktree')",
            patches.source
        );
    }

    let worktree = Path::new(&patches.path);
    if !worktree.exists() {
        anyhow::bail!("patch worktree path does not exist: {}", worktree.display());
    }
    if !worktree.is_dir() {
        anyhow::bail!(
            "patch worktree path is not a directory: {}",
            worktree.display()
        );
    }
    if !worktree.join(".git").exists() {
        anyhow::bail!(
            "patch worktree path is not a git working tree: {}",
            worktree.display()
        );
    }

    let branch = crate::git::current_branch(&patches.path).with_context(|| {
        format!(
            "failed to read branch for patch worktree {}",
            worktree.display()
        )
    })?;
    if branch.trim().is_empty() {
        anyhow::bail!(
            "patch worktree {} is in detached HEAD state",
            worktree.display()
        );
    }

    if patches.require_clean && crate::git::is_dirty(&patches.path)? {
        anyhow::bail!(
            "patch worktree {} has uncommitted changes; commit/stash them or set patches.require_clean = false",
            worktree.display()
        );
    }

    let base_target = format!("{}/{}", patches.base_remote, patches.base_ref);
    let head_commit = crate::git::head_commit(&patches.path)?;
    let merge_base =
        crate::process::run_in_dir(&patches.path, "git", &["merge-base", "HEAD", &base_target])
            .with_context(|| {
                format!(
                    "failed to compute merge-base for patch worktree {} against {}",
                    worktree.display(),
                    base_target
                )
            })?;
    let patch_count = crate::git::rev_list_count(&patches.path, &merge_base, "HEAD")?;

    Ok(PatchInfo {
        source: patches.source.clone(),
        worktree_path: patches.path.clone(),
        branch,
        head_commit,
        merge_base,
        base_remote: patches.base_remote.clone(),
        base_ref: patches.base_ref.clone(),
        patch_count,
    })
}

fn apply_source(tree_path: &str, patches: &PatchSourceConfig) -> Result<PatchInfo> {
    let info = inspect_source(patches)?;

    apply_resolved_source(tree_path, &info)?;
    Ok(info)
}

fn apply_resolved_source(tree_path: &str, info: &PatchInfo) -> Result<()> {
    if info.source != "worktree" {
        anyhow::bail!(
            "patch source '{}' is not supported (expected 'worktree')",
            info.source
        );
    }

    if info.patch_count == 0 {
        return Ok(());
    }

    let range = format!("{}..{}", info.merge_base, info.head_commit);
    let diff = Command::new("git")
        .args(["diff", "--binary", &range])
        .current_dir(&info.worktree_path)
        .output()
        .with_context(|| {
            format!(
                "failed to export resolved patch diff {} from worktree {}",
                range, info.worktree_path
            )
        })?;

    if !diff.status.success() {
        let stderr = String::from_utf8_lossy(&diff.stderr);
        anyhow::bail!(
            "failed to export resolved patch diff {} from worktree {}: {}",
            range,
            info.worktree_path,
            stderr.trim_end()
        );
    }

    let mut patchfile = tempfile::NamedTempFile::new()
        .context("failed to create temporary file for exported patches")?;
    patchfile
        .write_all(&diff.stdout)
        .context("failed to write exported patches to temporary file")?;

    let output = Command::new("git")
        .args([
            "apply",
            "--allow-empty",
            "--binary",
            "--whitespace=nowarn",
            patchfile.path().to_string_lossy().as_ref(),
        ])
        .current_dir(tree_path)
        .output()
        .with_context(|| format!("failed to apply exported patches in {}", tree_path))?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "failed to apply patches from worktree {}\nstdout:\n{}\nstderr:\n{}",
            info.worktree_path,
            stdout.trim_end(),
            stderr.trim_end()
        );
    }

    Ok(())
}
