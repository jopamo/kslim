use anyhow::{Context, Result};

use crate::process;

#[allow(dead_code)]
pub fn ensure_git() -> Result<()> {
    process::run("git", &["--version"])
        .map(|_| ())
        .context("git not found in PATH")
}

pub fn init_repo(path: &str) -> Result<()> {
    process::run("git", &["init", path])?;
    Ok(())
}

pub fn add_all(path: &str) -> Result<()> {
    process::run_in_dir(path, "git", &["add", "-A"])?;
    Ok(())
}

pub fn commit(path: &str, message: &str) -> Result<()> {
    process::run_in_dir(path, "git", &["commit", "-m", message])?;
    Ok(())
}

/// Returns true if a commit was created
pub fn commit_if_changed(path: &str, message: &str) -> Result<bool> {
    let status = process::run_in_dir(path, "git", &["status", "--porcelain"])?;
    if status.is_empty() {
        return Ok(false);
    }
    add_all(path)?;
    commit(path, message)?;
    Ok(true)
}

#[allow(dead_code)]
pub fn branch_exists(path: &str, branch: &str) -> Result<bool> {
    let result = process::run_in_dir(
        path,
        "git",
        &["rev-parse", "--verify", &format!("refs/heads/{}", branch)],
    );
    Ok(result.is_ok())
}

pub fn create_branch(path: &str, branch: &str) -> Result<()> {
    process::run_in_dir(path, "git", &["checkout", "-B", branch])?;
    Ok(())
}

pub fn create_tag(path: &str, tag: &str, message: &str) -> Result<()> {
    process::run_in_dir(path, "git", &["tag", "-a", tag, "-m", message])?;
    Ok(())
}

pub fn delete_tag(path: &str, tag: &str) -> Result<()> {
    let _ = process::run_in_dir(path, "git", &["tag", "-d", tag]);
    Ok(())
}

pub fn current_branch(path: &str) -> Result<String> {
    process::run_in_dir(path, "git", &["branch", "--show-current"])
}

pub fn is_dirty(path: &str) -> Result<bool> {
    let status = process::run_in_dir(path, "git", &["status", "--porcelain"])?;
    Ok(!status.is_empty())
}

/// Returns true if the repo is in detached HEAD state
pub fn is_detached_head(path: &str) -> Result<bool> {
    let branch = current_branch(path);
    Ok(branch.map(|b| b.is_empty()).unwrap_or(true))
}

pub fn head_commit(path: &str) -> Result<String> {
    process::run_in_dir(path, "git", &["rev-parse", "HEAD"])
}

pub fn remote_add(path: &str, name: &str, url: &str) -> Result<()> {
    let remotes = process::run_in_dir(path, "git", &["remote"])?;
    if remotes.lines().any(|l| l == name) {
        process::run_in_dir(path, "git", &["remote", "set-url", name, url])?;
    } else {
        process::run_in_dir(path, "git", &["remote", "add", name, url])?;
    }
    Ok(())
}

/// Get the URL of a named remote, or error if not found
pub fn remote_get_url(path: &str, name: &str) -> Result<String> {
    process::run_in_dir(path, "git", &["remote", "get-url", name])
}

pub fn push(path: &str, remote: &str, branch: &str) -> Result<()> {
    process::run_in_dir(path, "git", &["push", "-u", remote, branch])?;
    Ok(())
}

pub fn push_tag(path: &str, remote: &str, tag: &str) -> Result<()> {
    process::run_in_dir(path, "git", &["push", remote, tag])?;
    Ok(())
}

#[allow(dead_code)]
pub fn config_get(path: &str, key: &str) -> Result<String> {
    process::run_in_dir(path, "git", &["config", "--get", key])
}

pub fn config_set(path: &str, key: &str, value: &str) -> Result<()> {
    process::run_in_dir(path, "git", &["config", key, value])?;
    Ok(())
}

pub fn config_replace_all(path: &str, key: &str, value: &str) -> Result<()> {
    process::run_in_dir(path, "git", &["config", "--replace-all", key, value])?;
    Ok(())
}

#[allow(dead_code)]
pub fn init_bare(path: &str) -> Result<()> {
    process::run("git", &["init", "--bare", path])?;
    Ok(())
}

#[allow(dead_code)]
pub fn fetch(git_dir: &str, remote: &str) -> Result<()> {
    process::run_in_dir(git_dir, "git", &["fetch", "--tags", remote])?;
    Ok(())
}

#[allow(dead_code)]
pub fn remote_add_in_dir(git_dir: &str, name: &str, url: &str) -> Result<()> {
    let remotes = process::run_in_dir(git_dir, "git", &["remote"])?;
    if remotes.lines().any(|l| l == name) {
        process::run_in_dir(git_dir, "git", &["remote", "set-url", name, url])?;
    } else {
        process::run_in_dir(git_dir, "git", &["remote", "add", name, url])?;
    }
    Ok(())
}

pub fn rev_parse(git_dir: &str, refname: &str) -> Result<String> {
    process::run_in_dir(git_dir, "git", &["rev-parse", refname])
}

#[allow(dead_code)]
pub fn archive(git_dir: &str, refname: &str, outfile: &str) -> Result<()> {
    process::run_in_dir(
        git_dir,
        "git",
        &["archive", "--format=tar", refname, "-o", outfile],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn rev_list_count(git_dir: &str, from: &str, to: &str) -> Result<usize> {
    let out = process::run_in_dir(
        git_dir,
        "git",
        &["rev-list", "--count", &format!("{}..{}", from, to)],
    );
    match out {
        Ok(s) => Ok(s.parse().unwrap_or(0)),
        Err(_) => Ok(0),
    }
}
