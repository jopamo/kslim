use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::output_repo;
use crate::paths::{LockfilePath, OutputRepoPath};

/// Options for the publish command
pub struct PublishOptions {
    pub dry_run: bool,
    pub force: bool,
    pub no_network: bool,
}

/// Minimal publish-time request.
///
/// This intentionally excludes upstream configuration, selected profiles, and
/// candidate metadata paths. Publish state is loaded from `kslim.lock` plus the
/// committed metadata in the output repo.
#[derive(Debug, Clone)]
pub struct PublishRequest {
    pub project_root: PathBuf,
    pub output_path: String,
    pub remote_name: String,
    pub remote: String,
}

#[derive(Debug, Clone)]
struct CommittedPublishState {
    branch: String,
    output_commit: String,
    tag: String,
}

impl CommittedPublishState {
    fn load(request: &PublishRequest) -> Result<Self> {
        let output_repo_path = OutputRepoPath::new(request.output_path.as_str())?;
        let lockfile_path = LockfilePath::new_in_project_root(request.project_root.as_path())?;
        let state =
            output_repo::load_authoritative_published_state(&lockfile_path, &output_repo_path)?
                .ok_or_else(|| {
                    anyhow::anyhow!(
                "authoritative published state is missing: generate a published snapshot first"
            )
                })?;

        Ok(Self {
            branch: state.lockfile.output_branch,
            output_commit: state.lockfile.output_commit,
            tag: state.lockfile.tag,
        })
    }
}

#[derive(Debug, Deserialize)]
struct PublishOnlyConfig {
    output: PublishOnlyOutputConfig,
    #[serde(default)]
    git: PublishOnlyGitConfig,
    publish: Option<PublishOnlyRemoteConfig>,
}

#[derive(Debug, Deserialize)]
struct PublishOnlyOutputConfig {
    path: String,
}

#[derive(Debug, Deserialize)]
struct PublishOnlyGitConfig {
    #[serde(default = "default_publish_remote_name")]
    remote_name: String,
}

impl Default for PublishOnlyGitConfig {
    fn default() -> Self {
        Self {
            remote_name: default_publish_remote_name(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct PublishOnlyRemoteConfig {
    remote: String,
}

fn default_publish_remote_name() -> String {
    "origin".to_string()
}

pub fn load_publish_request(project_root: &Path) -> Result<PublishRequest> {
    let path = project_root.join("kslim.toml");
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let config: PublishOnlyConfig =
        toml::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))?;
    let publish_config = config
        .publish
        .context("no publish.remote configured in kslim.toml")?;

    if config.output.path.trim().is_empty() {
        anyhow::bail!("output.path must not be empty");
    }
    if config.git.remote_name.trim().is_empty() {
        anyhow::bail!("git.remote_name must not be empty");
    }
    if publish_config.remote.trim().is_empty() {
        anyhow::bail!("publish.remote must not be empty");
    }

    Ok(PublishRequest {
        project_root: project_root.to_path_buf(),
        output_path: config.output.path,
        remote_name: config.git.remote_name,
        remote: publish_config.remote,
    })
}

pub fn publish(request: &PublishRequest, opts: &PublishOptions) -> Result<()> {
    let output_path = &request.output_path;
    let remote = &request.remote;
    let remote_name = &request.remote_name;
    if opts.no_network {
        crate::network_policy::require_cli_no_network_endpoint("publish.remote", remote)?;
    }

    let out = std::path::Path::new(output_path);
    if !out.exists() {
        anyhow::bail!(
            "output repo at {} does not exist. Run `kslim generate` first.",
            output_path
        );
    }
    if !out.join(".git").exists() {
        anyhow::bail!("output path {} is not a git repository", output_path);
    }

    output_repo::require_managed(output_path)?;
    output_repo::require_clean(output_path, opts.force)?;
    output_repo::require_not_detached(output_path, opts.force)?;

    let published = CommittedPublishState::load(request)?;

    let existing_url = crate::git::remote_get_url(output_path, remote_name);
    match existing_url {
        Ok(url) if url.trim() == remote.trim() => {}
        Ok(url) => {
            if opts.force {
                log::warn!(
                    "remote '{}' URL mismatch: existing='{}' configured='{}'. \
                     Updating due to --force.",
                    remote_name,
                    url.trim(),
                    remote
                );
                crate::git::remote_add(output_path, remote_name, remote)?;
            } else {
                anyhow::bail!(
                    "remote '{}' URL mismatch: existing='{}' configured='{}'. \
                     Use --force to override.",
                    remote_name,
                    url.trim(),
                    remote
                );
            }
        }
        Err(_) => {
            crate::git::remote_add(output_path, remote_name, remote)?;
        }
    }

    if opts.dry_run {
        println!("[dry-run] would push branch: {}", published.branch);
        println!("[dry-run] output commit:      {}", published.output_commit);
        println!("[dry-run] would push tag:     {}", published.tag);
        println!("[dry-run] remote:             {}", remote);
        return Ok(());
    }

    log::info!("pushing branch '{}' to {}", published.branch, remote);
    crate::git::push(output_path, remote_name, &published.branch)?;

    let _ = crate::git::delete_tag(output_path, &published.tag);
    let tag_msg = format!("kslim generated tag for {}", published.branch);
    crate::git::create_tag(output_path, &published.tag, &tag_msg)?;
    crate::git::push_tag(output_path, remote_name, &published.tag)?;

    log::info!(
        "published branch '{}' and tag '{}'",
        published.branch,
        published.tag
    );

    Ok(())
}
