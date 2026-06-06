use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use std::path::Path;

pub fn check_access(url: &str) -> Result<Utf8PathBuf> {
    crate::network_policy::require_cli_no_network_endpoint("upstream.url", url)?;
    crate::network_policy::require_local_upstream_url(url)?;

    let path = Utf8PathBuf::from(url);
    let std_path = Path::new(url);

    if !std_path.exists() {
        anyhow::bail!(
            "upstream.url must point to an existing local git tree; '{}' does not exist",
            url
        );
    }

    crate::process::run_in_dir(
        url,
        "git",
        &["rev-parse", "--path-format=absolute", "--git-dir"],
    )
    .with_context(|| {
        format!(
            "upstream.url '{}' is not a readable local git repository or worktree",
            url
        )
    })?;

    Ok(path)
}

/// Direct read-only mode: verify the source repo is accessible.
pub fn sync(name: &str, url: &str) -> Result<()> {
    let path = check_access(url)?;
    log::info!(
        "upstream '{}' verified in direct read-only mode at {}",
        name,
        path
    );
    Ok(())
}

/// Resolve a ref to a full commit SHA. Uses `^{commit}` to dereference tags.
pub fn resolve_ref(git_dir: &str, refname: &str) -> Result<String> {
    let resolved = format!("{}^{{commit}}", refname);
    let sha = crate::git::rev_parse(git_dir, &resolved).with_context(|| {
        format!(
            "failed to resolve ref '{}'. Does the ref exist in the direct upstream repository? \
             Run `kslim upstream sync` to verify access.",
            refname
        )
    })?;
    Ok(sha)
}

/// Archive the tree at the given commit into a directory using git archive + tar
pub fn archive_tree(git_dir: &str, commit: &str, output_dir: &str) -> Result<()> {
    crate::network_policy::require_local_upstream_url(git_dir)?;
    crate::fsutil::ensure_dir(std::path::Path::new(output_dir))?;

    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "git --git-dir '{}' archive '{}' | tar -x -C '{}'",
            git_dir, commit, output_dir
        ))
        .output()
        .context("failed to archive upstream tree")?;

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        anyhow::bail!("archive failed: {}", stderr);
    }
    Ok(())
}

pub fn validate_tree(path: &str) -> Result<()> {
    let checks = [
        "Makefile", "Kconfig", "arch", "drivers", "fs", "include", "kernel", "mm", "net", "scripts",
    ];
    for check in &checks {
        if !std::path::Path::new(path).join(check).exists() {
            anyhow::bail!("generated tree missing essential path: {}", check);
        }
    }
    Ok(())
}

/// Get the ISO-8601 commit date for a ref
pub fn ref_timestamp(git_dir: &str, refname: &str) -> Result<String> {
    let resolved = format!("{}^{{commit}}", refname);
    crate::process::run_in_dir(git_dir, "git", &["log", "-1", "--format=%aI", &resolved])
}

#[allow(dead_code)]
pub fn ref_subject(git_dir: &str, refname: &str) -> Result<String> {
    crate::process::run_in_dir(git_dir, "git", &["log", "-1", "--format=%s", refname])
}
