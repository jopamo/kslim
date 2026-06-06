//! Candidate and published metadata schemas plus metadata IO.
//!
//! This module owns committed metadata file names, schemas, path sanitization,
//! reproducible timestamp checks, candidate metadata validation, and loading
//! authoritative published-state metadata. It must not sync payload files,
//! render reducer reports, or decide publish ref policy.

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::config::{KslimConfig, ProfileConfig};
use crate::lockfile::{PublishedLockfile, ResolvedBase};
use crate::model::{
    GitCommitId, MetadataFingerprint, MetadataSchemaVersion, OutputBranchName, PlanFingerprint,
    ReducerReportSummary, SelftestReportSummary, SnapshotId, ToolVersion, TreeFingerprint,
    CURRENT_METADATA_SCHEMA_VERSION,
};
use crate::patches::PatchInfo;
use crate::paths::{
    CandidateMetadataDir, CandidateTreePath, LockfilePath, OutputRepoPath, PublishedMetadataDir,
};

pub const COMMITTED_METADATA_DIR: &str = ".kslim";
pub(super) const CANDIDATE_METADATA_FILE: &str = "candidate.toml";
pub const BASE_METADATA_FILE: &str = "base.toml";
pub const GENERATED_METADATA_FILE: &str = "generated.toml";
pub const PATCH_METADATA_FILE: &str = "patches.toml";
pub const REPORT_FILE: &str = "report.txt";
pub const PUBLISHED_METADATA_FILE: &str = "published.toml";
pub const PUBLISHED_SNAPSHOT_FILE: &str = PUBLISHED_METADATA_FILE;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CandidateMetadataMarker {
    Candidate,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CandidateMetadata {
    pub schema_version: MetadataSchemaVersion,
    pub metadata_scope: CandidateMetadataMarker,
    pub plan_fingerprint: PlanFingerprint,
    pub tree_fingerprint: TreeFingerprint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reducer: Option<ReducerReportSummary>,
    pub selftest: SelftestReportSummary,
    pub generated_by: ToolVersion,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PublishedMetadataMarker {
    Published,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PublishedMetadata {
    pub schema_version: MetadataSchemaVersion,
    pub metadata_scope: PublishedMetadataMarker,
    pub snapshot_id: SnapshotId,
    pub output_commit: GitCommitId,
    pub branch: OutputBranchName,
    pub plan_fingerprint: PlanFingerprint,
    pub tree_fingerprint: TreeFingerprint,
    pub metadata_fingerprint: MetadataFingerprint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reducer: Option<ReducerReportSummary>,
    pub selftest: SelftestReportSummary,
    pub generated_by: ToolVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaseMetadata {
    pub upstream_name: String,
    pub upstream_url: String,
    pub base_ref: String,
    pub base_commit: String,
    pub profile: String,
    pub mode: String,
    pub kslim_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneratedMetadata {
    pub generated_by: String,
    pub generated_at: String,
    pub kslim_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishedSnapshotMetadata {
    pub branch: String,
    pub tag: String,
    pub base_ref: String,
    pub base_commit: String,
    pub profile: String,
    pub mode: String,
    pub generated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_metadata_fingerprint: Option<String>,
    pub base_metadata_file: String,
    pub generated_metadata_file: String,
    pub manifest_file: String,
    pub report_file: String,
    pub kslim_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoritativePublishedState {
    pub lockfile: PublishedLockfile,
    pub base: BaseMetadata,
    pub generated: GeneratedMetadata,
    pub published: PublishedSnapshotMetadata,
}

pub(crate) fn candidate_metadata_dir(
    candidate_tree: &CandidateTreePath,
) -> Result<CandidateMetadataDir> {
    CandidateMetadataDir::new_in_candidate_tree(
        candidate_tree,
        candidate_tree.as_path().join(COMMITTED_METADATA_DIR),
    )
}

pub(crate) fn published_metadata_dir(output_repo: &OutputRepoPath) -> Result<PublishedMetadataDir> {
    PublishedMetadataDir::new_in_output_repo(output_repo, metadata_dir_path(output_repo.as_path()))
}

fn metadata_dir_path(output_path: &Path) -> PathBuf {
    if output_path.join(".git").exists() {
        output_path.join(".git").join("kslim")
    } else {
        output_path.join(".kslim")
    }
}

#[allow(dead_code)]
fn read_candidate_metadata(metadata_dir: &CandidateMetadataDir) -> Result<CandidateMetadata> {
    read_metadata_root_requiring_marker(
        metadata_dir.as_path(),
        CANDIDATE_METADATA_FILE,
        "candidate metadata",
        "candidate",
    )
}

#[allow(dead_code)]
fn write_candidate_metadata(
    metadata_dir: &CandidateMetadataDir,
    metadata: &CandidateMetadata,
) -> Result<()> {
    write_metadata_root_contents(
        metadata_dir.as_path(),
        CANDIDATE_METADATA_FILE,
        &serialize_candidate_metadata(metadata)?,
        "candidate metadata",
    )
}

#[allow(dead_code)]
fn serialize_candidate_metadata(metadata: &CandidateMetadata) -> Result<String> {
    ensure_supported_metadata_schema_version("candidate metadata", metadata.schema_version)?;
    serialize_metadata_root(metadata, "candidate metadata")
}

#[allow(dead_code)]
fn read_published_metadata(metadata_dir: &PublishedMetadataDir) -> Result<PublishedMetadata> {
    read_metadata_root_requiring_marker(
        metadata_dir.as_path(),
        PUBLISHED_METADATA_FILE,
        "published metadata",
        "published",
    )
}

#[allow(dead_code)]
fn write_published_metadata(
    metadata_dir: &PublishedMetadataDir,
    metadata: &PublishedMetadata,
) -> Result<()> {
    write_metadata_root_contents(
        metadata_dir.as_path(),
        PUBLISHED_METADATA_FILE,
        &serialize_published_metadata(metadata)?,
        "published metadata",
    )
}

#[allow(dead_code)]
fn serialize_published_metadata(metadata: &PublishedMetadata) -> Result<String> {
    ensure_supported_metadata_schema_version("published metadata", metadata.schema_version)?;
    serialize_metadata_root(metadata, "published metadata")
}

fn read_metadata_root_requiring_marker<T: DeserializeOwned>(
    metadata_dir: &Path,
    file_name: &str,
    label: &str,
    expected_scope: &str,
) -> Result<T> {
    let path = metadata_dir.join(file_name);
    let contents = read_metadata_root_contents(&path, label)?;
    let header: MetadataHeaderProbe = parse_metadata_root(&contents, &path, label)?;
    ensure_supported_metadata_schema_version(label, header.schema_version)?;
    if header.metadata_scope != expected_scope {
        anyhow::bail!(
            "{label} marker '{}' does not match expected '{}'",
            header.metadata_scope,
            expected_scope
        );
    }
    parse_metadata_root(&contents, &path, label)
}

fn ensure_supported_metadata_schema_version(
    label: &str,
    schema_version: MetadataSchemaVersion,
) -> Result<()> {
    if schema_version != CURRENT_METADATA_SCHEMA_VERSION {
        anyhow::bail!(
            "{label} schema version {} is not supported; expected {}",
            schema_version.as_u32(),
            CURRENT_METADATA_SCHEMA_VERSION.as_u32()
        );
    }
    Ok(())
}

fn read_metadata_root_contents(path: &Path, label: &str) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {label} {}", path.display()))
}

fn parse_metadata_root<T: DeserializeOwned>(contents: &str, path: &Path, label: &str) -> Result<T> {
    toml::from_str(contents).with_context(|| format!("failed to parse {label} {}", path.display()))
}

#[derive(Debug, Deserialize)]
struct MetadataHeaderProbe {
    schema_version: MetadataSchemaVersion,
    metadata_scope: String,
}

fn serialize_metadata_root<T: Serialize>(metadata: &T, label: &str) -> Result<String> {
    let mut contents =
        toml::to_string_pretty(metadata).with_context(|| format!("failed to serialize {label}"))?;
    normalize_serialized_metadata(&mut contents);
    Ok(contents)
}

fn normalize_serialized_metadata(contents: &mut String) {
    *contents = contents.replace("\r\n", "\n");
    while contents.ends_with("\n\n") {
        contents.pop();
    }
    if !contents.ends_with('\n') {
        contents.push('\n');
    }
}

fn write_metadata_root_contents(
    metadata_dir: &Path,
    file_name: &str,
    contents: &str,
    label: &str,
) -> Result<()> {
    crate::fsutil::ensure_dir(metadata_dir)?;
    let path = metadata_dir.join(file_name);
    std::fs::write(&path, contents)
        .with_context(|| format!("failed to write {label} {}", path.display()))
}

/// Write .kslim/base.toml. Uses the upstream commit date for stable output.
pub fn write_base_metadata(
    output_path: &str,
    config: &KslimConfig,
    profile: &ProfileConfig,
    resolved: &ResolvedBase,
    mode: &str,
) -> Result<()> {
    let kslim_dir = metadata_dir_path(Path::new(output_path));
    crate::fsutil::ensure_dir(&kslim_dir)?;

    let metadata = BaseMetadata {
        upstream_name: config.upstream.name.clone(),
        upstream_url: committed_upstream_label(config),
        base_ref: resolved.r#ref.clone(),
        base_commit: resolved.commit.clone(),
        profile: profile.profile.name.clone(),
        mode: mode.to_string(),
        kslim_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    std::fs::write(
        kslim_dir.join(BASE_METADATA_FILE),
        toml::to_string_pretty(&metadata)?,
    )?;
    Ok(())
}

/// Write .kslim/generated.toml. Takes an explicit `generated_at` timestamp
/// (should be the upstream commit date so repeated generation is idempotent).
pub fn write_generated_metadata(output_path: &str, generated_at: &str) -> Result<()> {
    validate_reproducible_timestamp("generated_at", generated_at)?;
    let kslim_dir = metadata_dir_path(Path::new(output_path));
    crate::fsutil::ensure_dir(&kslim_dir)?;

    let metadata = GeneratedMetadata {
        generated_by: String::from("kslim"),
        generated_at: generated_at.to_string(),
        kslim_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    std::fs::write(
        kslim_dir.join(GENERATED_METADATA_FILE),
        toml::to_string_pretty(&metadata)?,
    )?;
    Ok(())
}

/// Write .kslim/patches.toml when a patch source is configured.
pub fn write_patch_metadata(output_path: &str, patch_infos: Option<&[PatchInfo]>) -> Result<()> {
    let kslim_dir = metadata_dir_path(Path::new(output_path));
    crate::fsutil::ensure_dir(&kslim_dir)?;

    let path = kslim_dir.join(PATCH_METADATA_FILE);
    match patch_infos {
        Some(infos) if !infos.is_empty() => {
            let mut content = format!(
                "source_count = {}\ntotal_patch_count = {}\n",
                infos.len(),
                infos.iter().map(|info| info.patch_count).sum::<usize>()
            );
            for info in infos {
                content.push_str("\n[[sources]]\n");
                content.push_str(&format!(
                    r#"source = "{}"
worktree_path = "{}"
branch = "{}"
head_commit = "{}"
merge_base = "{}"
base_remote = "{}"
base_ref = "{}"
patch_count = {}
"#,
                    info.source,
                    patch_worktree_label(info, MetadataPathPolicy::Committed),
                    info.branch,
                    info.head_commit,
                    info.merge_base,
                    info.base_remote,
                    info.base_ref,
                    info.patch_count,
                ));
            }
            std::fs::write(path, content)?;
        }
        None => {
            if path.exists() {
                std::fs::remove_file(path)?;
            }
        }
        Some(_) => {
            if path.exists() {
                std::fs::remove_file(path)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn write_verified_published_snapshot_metadata(
    output_repo: &OutputRepoPath,
    metadata: &crate::generate::VerifiedPublishedSnapshotMetadata,
) -> Result<()> {
    let metadata_dir = published_metadata_dir(output_repo)?;
    write_verified_published_snapshot_metadata_to_dir(&metadata_dir, metadata)
}

pub(crate) fn write_verified_committed_published_snapshot_metadata(
    output_repo: &OutputRepoPath,
    metadata: &crate::generate::VerifiedPublishedSnapshotMetadata,
    temporary_roots: &[&Path],
) -> Result<()> {
    let metadata_dir = PublishedMetadataDir::new_committed_tree_in_output_repo(
        output_repo,
        output_repo.as_path().join(COMMITTED_METADATA_DIR),
    )?;
    validate_committed_metadata_text_has_no_temporary_paths(
        PUBLISHED_SNAPSHOT_FILE,
        &toml::to_string_pretty(metadata.metadata())?,
        temporary_roots,
    )
    .with_context(|| {
        "published committed metadata validation failed: temporary paths are forbidden"
    })?;
    if metadata_dir.as_path().exists() {
        validate_committed_metadata_has_no_temporary_paths(
            metadata_dir.as_path(),
            temporary_roots,
        )
        .with_context(|| {
            "published committed metadata validation failed: temporary paths are forbidden"
        })?;
        validate_committed_metadata_has_no_raw_logs(metadata_dir.as_path())
            .with_context(|| {
                "published committed metadata validation failed: raw logs are forbidden"
            })?;
    }
    write_verified_published_snapshot_metadata_to_dir(&metadata_dir, metadata).with_context(|| {
        "published committed metadata validation failed: host absolute paths are forbidden"
    })?;
    validate_committed_metadata_has_no_host_absolute_paths(metadata_dir.as_path())
        .with_context(|| {
            "published committed metadata validation failed: host absolute paths are forbidden"
        })?;
    validate_committed_metadata_has_no_temporary_paths(
        metadata_dir.as_path(),
        temporary_roots,
    )
    .with_context(|| {
        "published committed metadata validation failed: temporary paths are forbidden"
    })?;
    validate_committed_metadata_has_no_raw_logs(metadata_dir.as_path())
        .with_context(|| {
            "published committed metadata validation failed: raw logs are forbidden"
        })
}

fn write_verified_published_snapshot_metadata_to_dir(
    metadata_dir: &PublishedMetadataDir,
    metadata: &crate::generate::VerifiedPublishedSnapshotMetadata,
) -> Result<()> {
    if metadata.proof_summary().trim().is_empty() {
        anyhow::bail!("published metadata write requires candidate verification proof");
    }
    write_published_snapshot_metadata_unchecked(metadata_dir, metadata.metadata())
}

pub(super) fn write_published_snapshot_metadata_unchecked(
    metadata_dir: &PublishedMetadataDir,
    metadata: &PublishedSnapshotMetadata,
) -> Result<()> {
    validate_reproducible_timestamp("published.generated_at", &metadata.generated_at)?;
    let contents = toml::to_string_pretty(metadata)?;
    validate_committed_metadata_text_has_no_host_absolute_paths(
        PUBLISHED_SNAPSHOT_FILE,
        &contents,
    )?;
    validate_committed_metadata_text_has_no_raw_logs(PUBLISHED_SNAPSHOT_FILE, &contents)?;
    crate::fsutil::ensure_dir(metadata_dir.as_path())?;
    std::fs::write(metadata_dir.as_path().join(PUBLISHED_SNAPSHOT_FILE), contents)?;
    Ok(())
}

pub(crate) fn load_committed_base_metadata(
    output_repo: &OutputRepoPath,
    commit: &str,
) -> Result<BaseMetadata> {
    read_committed_metadata_file(output_repo, commit, BASE_METADATA_FILE)
}

pub(crate) fn load_committed_generated_metadata(
    output_repo: &OutputRepoPath,
    commit: &str,
) -> Result<GeneratedMetadata> {
    let metadata: GeneratedMetadata =
        read_committed_metadata_file(output_repo, commit, GENERATED_METADATA_FILE)?;
    validate_reproducible_timestamp("committed generated.generated_at", &metadata.generated_at)?;
    Ok(metadata)
}

pub(crate) fn load_committed_published_snapshot_metadata(
    output_repo: &OutputRepoPath,
    commit: &str,
) -> Result<PublishedSnapshotMetadata> {
    let metadata: PublishedSnapshotMetadata =
        read_committed_metadata_file(output_repo, commit, PUBLISHED_SNAPSHOT_FILE)?;
    validate_reproducible_timestamp("committed published.generated_at", &metadata.generated_at)?;
    Ok(metadata)
}

pub(crate) fn load_authoritative_published_state(
    lockfile_path: &LockfilePath,
    output_repo: &OutputRepoPath,
) -> Result<Option<AuthoritativePublishedState>> {
    let Some(lockfile) = crate::lockfile::load_lockfile(lockfile_path)? else {
        ensure_no_committed_published_metadata_without_lockfile(
            output_repo,
            "kslim.lock is missing",
        )?;
        return Ok(None);
    };
    let Some(published_lock) = lockfile.published else {
        ensure_no_committed_published_metadata_without_lockfile(
            output_repo,
            "kslim.lock has no published snapshot",
        )?;
        return Ok(None);
    };

    let output_path = output_repo_path_str(output_repo)?;
    let current_branch = crate::git::current_branch(output_path)?;
    if current_branch.trim().is_empty() {
        anyhow::bail!("published snapshot metadata is inconsistent: output repo is detached");
    }
    let current_commit = crate::git::head_commit(output_path)?;

    let base = load_committed_base_metadata(output_repo, &current_commit)?;
    let generated = load_committed_generated_metadata(output_repo, &current_commit)?;
    let published = load_committed_published_snapshot_metadata(output_repo, &current_commit)?;

    if published.branch.trim().is_empty() || published.tag.trim().is_empty() {
        anyhow::bail!(
            "authoritative published state is inconsistent: committed published metadata has empty branch/tag"
        );
    }
    verify_published_metadata_file(
        output_repo,
        &current_commit,
        &published.base_metadata_file,
        BASE_METADATA_FILE,
    )?;
    verify_published_metadata_file(
        output_repo,
        &current_commit,
        &published.generated_metadata_file,
        GENERATED_METADATA_FILE,
    )?;
    verify_published_metadata_file(
        output_repo,
        &current_commit,
        &published.manifest_file,
        crate::manifest::OUTPUT_MANIFEST_FILE_NAME,
    )?;
    verify_published_metadata_file(
        output_repo,
        &current_commit,
        &published.report_file,
        REPORT_FILE,
    )?;

    if published_lock.output_branch != current_branch {
        anyhow::bail!(
            "authoritative published state is inconsistent: lockfile branch '{}' does not match output branch '{}'",
            published_lock.output_branch,
            current_branch
        );
    }
    if published_lock.output_commit != current_commit {
        anyhow::bail!(
            "authoritative published state is inconsistent: lockfile output commit '{}' does not match output HEAD '{}'",
            published_lock.output_commit,
            current_commit
        );
    }
    if published_lock.tag != published.tag {
        anyhow::bail!(
            "authoritative published state is inconsistent: lockfile tag '{}' does not match committed published metadata '{}'",
            published_lock.tag,
            published.tag
        );
    }
    if published_lock.output_branch != published.branch {
        anyhow::bail!(
            "authoritative published state is inconsistent: lockfile branch '{}' does not match committed published metadata '{}'",
            published_lock.output_branch,
            published.branch
        );
    }
    if lockfile.resolved_base.r#ref != published_lock.base_ref
        || lockfile.resolved_base.commit != published_lock.base_commit
    {
        anyhow::bail!(
            "authoritative published state is inconsistent: lockfile resolved_base does not match lockfile published snapshot"
        );
    }
    if published_lock.base_ref != published.base_ref
        || published_lock.base_commit != published.base_commit
        || published_lock.profile != published.profile
        || published_lock.mode != published.mode
        || published_lock.generated_at != published.generated_at
    {
        anyhow::bail!(
            "authoritative published state is inconsistent: lockfile published snapshot does not match committed published metadata"
        );
    }
    if base.base_ref != published_lock.base_ref
        || base.base_commit != published_lock.base_commit
        || base.profile != published_lock.profile
        || base.mode != published_lock.mode
    {
        anyhow::bail!(
            "authoritative published state is inconsistent: committed base metadata does not match lockfile published snapshot"
        );
    }
    if generated.generated_at != published_lock.generated_at {
        anyhow::bail!(
            "authoritative published state is inconsistent: committed generated metadata does not match lockfile published snapshot"
        );
    }

    Ok(Some(AuthoritativePublishedState {
        lockfile: published_lock,
        base,
        generated,
        published,
    }))
}

fn ensure_no_committed_published_metadata_without_lockfile(
    output_repo: &OutputRepoPath,
    reason: &str,
) -> Result<()> {
    if let Some(commit) = committed_published_metadata_commit(output_repo)? {
        anyhow::bail!(
            "authoritative published state is incomplete: committed published metadata exists at output commit {} but {}; refusing to recover implicitly",
            commit,
            reason
        );
    }
    Ok(())
}

fn committed_published_metadata_commit(output_repo: &OutputRepoPath) -> Result<Option<String>> {
    if !output_repo.as_path().join(".git").exists() {
        return Ok(None);
    }
    let output_path = output_repo_path_str(output_repo)?;
    let commit = match crate::git::head_commit(output_path) {
        Ok(commit) if !commit.trim().is_empty() => commit,
        _ => return Ok(None),
    };
    match read_committed_metadata_blob(output_repo, &commit, PUBLISHED_SNAPSHOT_FILE) {
        Ok(_) => Ok(Some(commit)),
        Err(_) => Ok(None),
    }
}

fn verify_published_metadata_file(
    output_repo: &OutputRepoPath,
    commit: &str,
    actual: &str,
    expected: &str,
) -> Result<()> {
    if actual != expected {
        anyhow::bail!(
            "authoritative published state is inconsistent: expected published metadata file '{}' but found '{}'",
            expected,
            actual
        );
    }
    read_committed_metadata_blob(output_repo, commit, actual)?;
    Ok(())
}

fn read_committed_metadata_file<T: DeserializeOwned>(
    output_repo: &OutputRepoPath,
    commit: &str,
    file_name: &str,
) -> Result<T> {
    let contents = read_committed_metadata_blob(output_repo, commit, file_name)?;
    toml::from_str(&contents).with_context(|| {
        format!(
            "required committed published metadata is invalid: {}:{}",
            commit,
            committed_metadata_ref(file_name).unwrap_or_else(|_| file_name.to_string())
        )
    })
}

fn read_committed_metadata_blob(
    output_repo: &OutputRepoPath,
    commit: &str,
    file_name: &str,
) -> Result<String> {
    let metadata_ref = committed_metadata_ref(file_name)?;
    let output_path = output_repo_path_str(output_repo)?;
    crate::process::run_in_dir(
        output_path,
        "git",
        &["show", &format!("{}:{}", commit, metadata_ref)],
    )
    .with_context(|| {
        format!(
            "required committed published metadata missing: {}:{}",
            commit, metadata_ref
        )
    })
}

fn output_repo_path_str(output_repo: &OutputRepoPath) -> Result<&str> {
    output_repo
        .as_path()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("published output repo path is not valid UTF-8"))
}

fn committed_metadata_ref(file_name: &str) -> Result<String> {
    if file_name.trim().is_empty()
        || Path::new(file_name).is_absolute()
        || file_name.contains('/')
        || file_name.contains('\\')
    {
        anyhow::bail!(
            "committed published metadata file '{}' must be a single file name",
            file_name
        );
    }
    Ok(format!("{}/{}", COMMITTED_METADATA_DIR, file_name))
}

pub(super) fn validate_candidate_metadata(candidate_root: &Path) -> Result<()> {
    let candidate_metadata = metadata_dir_path(candidate_root);
    for required in [
        "managed.toml",
        BASE_METADATA_FILE,
        GENERATED_METADATA_FILE,
        crate::manifest::OUTPUT_MANIFEST_FILE_NAME,
        REPORT_FILE,
    ] {
        let path = candidate_metadata.join(required);
        if !path.exists() {
            anyhow::bail!(
                "output candidate validation failed: required candidate metadata missing: {}",
                path.display()
            );
        }
    }
    let published_metadata = candidate_metadata.join(PUBLISHED_SNAPSHOT_FILE);
    if published_metadata.exists() {
        anyhow::bail!(
            "output candidate validation failed: candidate metadata must not contain published snapshot metadata: {}; published metadata is written only by published-state APIs",
            published_metadata.display()
        );
    }
    validate_committed_metadata_has_no_temporary_paths(&candidate_metadata, &[candidate_root])?;
    validate_committed_metadata_has_no_host_absolute_paths(&candidate_metadata)?;
    validate_committed_metadata_has_no_raw_logs(&candidate_metadata)?;
    validate_committed_metadata_has_only_declared_reproducible_timestamps(&candidate_metadata)?;
    Ok(())
}

pub(super) fn is_published_snapshot_metadata_file(file_name: &OsStr) -> bool {
    file_name == OsStr::new(PUBLISHED_SNAPSHOT_FILE)
}

pub(super) fn validate_candidate_metadata_temporary_paths(
    candidate_root: &Path,
    temporary_roots: &[&Path],
) -> Result<()> {
    let candidate_metadata = metadata_dir_path(candidate_root);
    validate_committed_metadata_has_no_temporary_paths(&candidate_metadata, temporary_roots)
}

pub(crate) fn validate_committed_metadata_has_no_temporary_paths(
    metadata_dir: &Path,
    temporary_roots: &[&Path],
) -> Result<()> {
    let files = metadata_files(metadata_dir)?;
    validate_committed_metadata_files_have_no_temporary_paths(
        metadata_dir,
        &files,
        temporary_roots,
        "committed metadata",
    )
}

pub(super) fn validate_committed_metadata_named_files_have_no_temporary_paths(
    metadata_dir: &Path,
    file_names: &[&str],
    temporary_roots: &[&Path],
    artifact_label: &str,
) -> Result<()> {
    let files = file_names
        .iter()
        .map(|file_name| metadata_dir.join(file_name))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    validate_committed_metadata_files_have_no_temporary_paths(
        metadata_dir,
        &files,
        temporary_roots,
        artifact_label,
    )
}

fn validate_committed_metadata_files_have_no_temporary_paths(
    metadata_dir: &Path,
    files: &[PathBuf],
    temporary_roots: &[&Path],
    artifact_label: &str,
) -> Result<()> {
    let markers = temporary_path_markers(temporary_roots);
    if markers.is_empty() {
        return Ok(());
    }

    for file in files {
        let contents = std::fs::read_to_string(file).with_context(|| {
            format!(
                "failed to read committed metadata before temporary path validation: {}",
                file.display()
            )
        })?;
        validate_committed_metadata_text_has_no_temporary_paths(
            &format!(
                "{} {}",
                artifact_label,
                metadata_relative_path(metadata_dir, file)
            ),
            &contents,
            temporary_roots,
        )?;
    }

    Ok(())
}

pub(crate) fn validate_committed_metadata_text_has_no_temporary_paths(
    artifact_label: &str,
    contents: &str,
    temporary_roots: &[&Path],
) -> Result<()> {
    crate::security::validate_report_text_has_no_temporary_paths(
        artifact_label,
        contents,
        temporary_roots,
    )
}

fn temporary_path_markers(paths: &[&Path]) -> Vec<String> {
    crate::security::temporary_path_markers(paths)
}

fn metadata_files(metadata_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_metadata_files(metadata_dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_metadata_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries = std::fs::read_dir(dir)?.collect::<std::result::Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path)?;
        if meta.file_type().is_dir() {
            collect_metadata_files(&path, files)?;
        } else if meta.file_type().is_file() {
            files.push(path);
        }
    }

    Ok(())
}

pub(crate) fn validate_committed_metadata_has_no_host_absolute_paths(
    metadata_dir: &Path,
) -> Result<()> {
    for file in metadata_files(metadata_dir)? {
        let contents = std::fs::read_to_string(&file).with_context(|| {
            format!(
                "failed to read committed metadata before host path validation: {}",
                file.display()
            )
        })?;
        validate_committed_metadata_text_has_no_host_absolute_paths(
            &metadata_relative_path(metadata_dir, &file),
            &contents,
        )?;
    }
    Ok(())
}

pub(crate) fn validate_committed_metadata_text_has_no_host_absolute_paths(
    artifact_label: &str,
    contents: &str,
) -> Result<()> {
    if let Some(marker) = find_host_specific_absolute_path_marker(contents) {
        anyhow::bail!(
            "committed metadata {} contains host-only absolute path {:?}; host paths may appear only in non-authoritative attempt metadata",
            artifact_label,
            marker
        );
    }
    Ok(())
}

pub(crate) fn validate_committed_metadata_has_no_raw_logs(metadata_dir: &Path) -> Result<()> {
    for file in metadata_files(metadata_dir)? {
        if let Some(marker) = raw_log_file_marker(metadata_dir, &file) {
            anyhow::bail!(
                "committed metadata {} is a raw log artifact {}; raw logs may appear only in non-authoritative attempt metadata or CI artifacts; committed metadata must use normalized summaries",
                metadata_relative_path(metadata_dir, &file),
                marker
            );
        }
        let contents = std::fs::read_to_string(&file).with_context(|| {
            format!(
                "failed to read committed metadata before raw log validation: {}",
                file.display()
            )
        })?;
        validate_committed_metadata_text_has_no_raw_logs(
            &metadata_relative_path(metadata_dir, &file),
            &contents,
        )?;
    }
    Ok(())
}

pub(crate) fn validate_committed_metadata_text_has_no_raw_logs(
    artifact_label: &str,
    contents: &str,
) -> Result<()> {
    if let Some(marker) = raw_log_marker(contents) {
        anyhow::bail!(
            "committed metadata {} contains raw log marker {}; raw logs may appear only in non-authoritative attempt metadata or CI artifacts; committed metadata must use normalized summaries",
            artifact_label,
            marker
        );
    }
    Ok(())
}

fn raw_log_file_marker(metadata_dir: &Path, file: &Path) -> Option<String> {
    let relative = metadata_relative_path(metadata_dir, file);
    let file_name = file.file_name().and_then(OsStr::to_str).unwrap_or("");
    crate::security::raw_log_file_marker(&relative, file_name)
}

fn raw_log_marker(contents: &str) -> Option<String> {
    crate::security::raw_log_marker(contents)
}

pub(crate) fn validate_committed_metadata_has_only_allowed_reproducible_timestamps(
    metadata_dir: &Path,
    allowed_timestamps: &[&str],
) -> Result<()> {
    let allowed = allowed_reproducible_timestamps(allowed_timestamps)?;
    validate_committed_metadata_timestamp_markers(metadata_dir, &allowed)
}

fn validate_committed_metadata_has_only_declared_reproducible_timestamps(
    metadata_dir: &Path,
) -> Result<()> {
    let allowed = declared_committed_metadata_reproducible_timestamps(metadata_dir)?;
    validate_committed_metadata_timestamp_markers(metadata_dir, &allowed)
}

fn allowed_reproducible_timestamps(allowed_timestamps: &[&str]) -> Result<BTreeSet<String>> {
    let mut allowed = BTreeSet::new();
    for timestamp in allowed_timestamps {
        validate_reproducible_timestamp("committed metadata timestamp policy", timestamp)?;
        allowed.insert((*timestamp).to_string());
    }
    Ok(allowed)
}

fn declared_committed_metadata_reproducible_timestamps(
    metadata_dir: &Path,
) -> Result<BTreeSet<String>> {
    let mut declared = BTreeSet::new();
    for (file_name, field_name, label) in [
        (
            CANDIDATE_METADATA_FILE,
            "base_resolved_at",
            "candidate.base_resolved_at",
        ),
        (
            GENERATED_METADATA_FILE,
            "generated_at",
            "generated.generated_at",
        ),
        (
            PUBLISHED_SNAPSHOT_FILE,
            "generated_at",
            "published.generated_at",
        ),
    ] {
        let path = metadata_dir.join(file_name);
        let Some(timestamp) = read_optional_toml_string_field(&path, field_name)? else {
            continue;
        };
        validate_reproducible_timestamp(label, &timestamp)?;
        declared.insert(timestamp);
    }

    if declared.len() > 1 {
        anyhow::bail!(
            "committed metadata timestamp policy is inconsistent: committed metadata declares multiple resolved base timestamps: {}",
            declared.iter().cloned().collect::<Vec<_>>().join(", ")
        );
    }

    Ok(declared)
}

fn read_optional_toml_string_field(path: &Path, field_name: &str) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(path).with_context(|| {
        format!(
            "failed to read committed metadata timestamp policy source: {}",
            path.display()
        )
    })?;
    let value = toml::from_str::<toml::Value>(&contents).with_context(|| {
        format!(
            "failed to parse committed metadata timestamp policy source: {}",
            path.display()
        )
    })?;
    Ok(value
        .get(field_name)
        .and_then(toml::Value::as_str)
        .map(str::to_string))
}

fn validate_committed_metadata_timestamp_markers(
    metadata_dir: &Path,
    allowed: &BTreeSet<String>,
) -> Result<()> {
    for file in metadata_files(metadata_dir)? {
        let contents = std::fs::read_to_string(&file).with_context(|| {
            format!(
                "failed to read committed metadata before timestamp validation: {}",
                file.display()
            )
        })?;
        for marker in timestamp_markers(&contents) {
            if !is_reproducible_timestamp(&marker) {
                anyhow::bail!(
                    "committed metadata {} contains non-reproducible timestamp {:?}; committed metadata timestamps must be reproducible RFC3339 timestamps derived from the resolved base commit",
                    metadata_relative_path(metadata_dir, &file),
                    marker
                );
            }
            if allowed.is_empty() {
                anyhow::bail!(
                    "committed metadata {} contains timestamp {:?} but no reproducible timestamp policy source is declared",
                    metadata_relative_path(metadata_dir, &file),
                    marker
                );
            }
            if !allowed.contains(&marker) {
                anyhow::bail!(
                    "committed metadata {} contains timestamp {:?} outside reproducible timestamp policy; expected resolved base commit timestamp {}",
                    metadata_relative_path(metadata_dir, &file),
                    marker,
                    allowed.iter().cloned().collect::<Vec<_>>().join(", ")
                );
            }
        }
    }
    Ok(())
}

fn timestamp_markers(contents: &str) -> Vec<String> {
    crate::security::timestamp_markers(contents)
}

fn metadata_relative_path(metadata_dir: &Path, path: &Path) -> String {
    path.strip_prefix(metadata_dir)
        .map(normalized_metadata_path)
        .unwrap_or_else(|_| path.display().to_string())
}

fn normalized_metadata_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn find_host_specific_absolute_path_marker(contents: &str) -> Option<String> {
    crate::security::find_host_specific_absolute_path_marker(contents)
}

pub(crate) fn is_host_specific_absolute_path(value: &str) -> bool {
    crate::security::is_host_specific_absolute_path(value)
}

#[derive(Clone, Copy)]
pub(super) enum MetadataPathPolicy {
    Committed,
    Attempt,
}

pub(super) fn render_patch_section(
    patch_infos: Option<&[PatchInfo]>,
    path_policy: MetadataPathPolicy,
) -> String {
    match patch_infos {
        Some(infos) if !infos.is_empty() => {
            let mut section = format!(
                "\nPatches:\n  Sources: {}\n  Total count: {}\n",
                infos.len(),
                infos.iter().map(|info| info.patch_count).sum::<usize>()
            );
            for info in infos {
                section.push_str(&format!(
                    "  - {} | {} | {} commit(s) | head {}\n",
                    info.branch,
                    patch_worktree_label(info, path_policy),
                    info.patch_count,
                    info.head_commit
                ));
            }
            section
        }
        _ => "\nPatches:\n  None\n".to_string(),
    }
}

pub(super) fn committed_upstream_label(config: &KslimConfig) -> String {
    committed_metadata_label(
        &config.upstream.url,
        &format!("local-upstream:{}", config.upstream.name),
    )
}

fn patch_worktree_label(info: &PatchInfo, path_policy: MetadataPathPolicy) -> String {
    match path_policy {
        MetadataPathPolicy::Committed => committed_metadata_label(
            &info.worktree_path,
            &format!("local-worktree:{}", info.branch),
        ),
        MetadataPathPolicy::Attempt => info.worktree_path.clone(),
    }
}

fn committed_metadata_label(value: &str, replacement: &str) -> String {
    if is_host_specific_absolute_path(value) {
        replacement.to_string()
    } else {
        value.to_string()
    }
}

fn validate_reproducible_timestamp(label: &str, value: &str) -> Result<()> {
    crate::security::validate_reproducible_timestamp(label, value)
}

pub(crate) fn validate_reproducible_metadata_timestamp(label: &str, value: &str) -> Result<()> {
    validate_reproducible_timestamp(label, value)
}

pub(super) fn is_reproducible_timestamp(value: &str) -> bool {
    crate::security::is_reproducible_timestamp(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::{
        CandidateMetadataDir, CandidateTreePath, OutputRepoPath, PublishedMetadataDir,
    };
    use std::path::Path;

    fn candidate_metadata(reducer: Option<ReducerReportSummary>) -> CandidateMetadata {
        CandidateMetadata {
            schema_version: CURRENT_METADATA_SCHEMA_VERSION,
            metadata_scope: CandidateMetadataMarker::Candidate,
            plan_fingerprint: PlanFingerprint::new("fingerprint-plan").unwrap(),
            tree_fingerprint: TreeFingerprint::new("fingerprint-tree").unwrap(),
            reducer,
            selftest: SelftestReportSummary {
                enabled: true,
                built_in_checks: 2,
                kernel_builds_run: 1,
                commands_run: 3,
            },
            generated_by: ToolVersion::new("kslim-test").unwrap(),
        }
    }

    fn published_metadata(reducer: Option<ReducerReportSummary>) -> PublishedMetadata {
        PublishedMetadata {
            schema_version: CURRENT_METADATA_SCHEMA_VERSION,
            metadata_scope: PublishedMetadataMarker::Published,
            snapshot_id: SnapshotId::new("snapshot-main-1").unwrap(),
            output_commit: GitCommitId::new("abc123").unwrap(),
            branch: OutputBranchName::new("main").unwrap(),
            plan_fingerprint: PlanFingerprint::new("fingerprint-plan").unwrap(),
            tree_fingerprint: TreeFingerprint::new("fingerprint-tree").unwrap(),
            metadata_fingerprint: MetadataFingerprint::new("fingerprint-metadata").unwrap(),
            reducer,
            selftest: SelftestReportSummary {
                enabled: true,
                built_in_checks: 2,
                kernel_builds_run: 1,
                commands_run: 3,
            },
            generated_by: ToolVersion::new("kslim-test").unwrap(),
        }
    }

    fn published_metadata_dir_for_test(output: &Path) -> PublishedMetadataDir {
        std::fs::create_dir_all(output.join(".git")).unwrap();
        let output_repo = OutputRepoPath::new(output).unwrap();
        PublishedMetadataDir::new_in_output_repo(&output_repo, output.join(".git/kslim")).unwrap()
    }

    #[test]
    fn candidate_metadata_root_round_trips_with_scalar_identity_fields() {
        let metadata = candidate_metadata(None);

        let encoded = toml::to_string_pretty(&metadata).unwrap();

        assert!(encoded.contains("schema_version = 1"));
        assert!(encoded.contains("metadata_scope = \"candidate\""));
        assert!(encoded.contains("plan_fingerprint = \"fingerprint-plan\""));
        assert!(encoded.contains("tree_fingerprint = \"fingerprint-tree\""));
        assert!(encoded.contains("generated_by = \"kslim-test\""));
        assert!(encoded.contains("[selftest]"));
        assert!(encoded.contains("built_in_checks = 2"));
        assert!(
            !encoded.contains("[reducer]"),
            "absent reducer summaries should not create authoritative reducer metadata"
        );

        let decoded: CandidateMetadata = toml::from_str(&encoded).unwrap();
        assert_eq!(decoded, metadata);
    }

    #[test]
    fn published_metadata_root_round_trips_with_snapshot_identity_fields() {
        let metadata = published_metadata(None);

        let encoded = toml::to_string_pretty(&metadata).unwrap();

        assert!(encoded.contains("schema_version = 1"));
        assert!(encoded.contains("metadata_scope = \"published\""));
        assert!(encoded.contains("snapshot_id = \"snapshot-main-1\""));
        assert!(encoded.contains("output_commit = \"abc123\""));
        assert!(encoded.contains("branch = \"main\""));
        assert!(encoded.contains("plan_fingerprint = \"fingerprint-plan\""));
        assert!(encoded.contains("tree_fingerprint = \"fingerprint-tree\""));
        assert!(encoded.contains("metadata_fingerprint = \"fingerprint-metadata\""));
        assert!(encoded.contains("generated_by = \"kslim-test\""));
        assert!(encoded.contains("[selftest]"));
        assert!(
            !encoded.contains("[reducer]"),
            "absent reducer summaries should not create authoritative reducer metadata"
        );

        let decoded: PublishedMetadata = toml::from_str(&encoded).unwrap();
        assert_eq!(decoded, metadata);
    }

    #[test]
    fn published_metadata_root_includes_optional_reducer_summary() {
        let metadata = published_metadata(Some(ReducerReportSummary {
            files_removed: 4,
            dirs_removed: 2,
            edit_records: 9,
        }));

        let encoded = toml::to_string_pretty(&metadata).unwrap();

        assert!(encoded.contains("[reducer]"));
        assert!(encoded.contains("files_removed = 4"));
        assert!(encoded.contains("dirs_removed = 2"));
        assert!(encoded.contains("edit_records = 9"));

        let decoded: PublishedMetadata = toml::from_str(&encoded).unwrap();
        assert_eq!(decoded, metadata);
    }

    #[test]
    fn candidate_metadata_root_includes_optional_reducer_summary() {
        let metadata = candidate_metadata(Some(ReducerReportSummary {
            files_removed: 4,
            dirs_removed: 2,
            edit_records: 9,
        }));

        let encoded = toml::to_string_pretty(&metadata).unwrap();

        assert!(encoded.contains("[reducer]"));
        assert!(encoded.contains("files_removed = 4"));
        assert!(encoded.contains("dirs_removed = 2"));
        assert!(encoded.contains("edit_records = 9"));

        let decoded: CandidateMetadata = toml::from_str(&encoded).unwrap();
        assert_eq!(decoded, metadata);
    }

    #[test]
    fn deterministic_serialization_helpers_are_stable_and_normalized() {
        let candidate = candidate_metadata(Some(ReducerReportSummary {
            files_removed: 4,
            dirs_removed: 2,
            edit_records: 9,
        }));
        let published = published_metadata(Some(ReducerReportSummary {
            files_removed: 4,
            dirs_removed: 2,
            edit_records: 9,
        }));

        let candidate_first = serialize_candidate_metadata(&candidate).unwrap();
        let candidate_second = serialize_candidate_metadata(&candidate).unwrap();
        let published_first = serialize_published_metadata(&published).unwrap();
        let published_second = serialize_published_metadata(&published).unwrap();

        assert_eq!(candidate_first, candidate_second);
        assert_eq!(published_first, published_second);
        for serialized in [&candidate_first, &published_first] {
            assert!(serialized.ends_with('\n'));
            assert!(!serialized.ends_with("\n\n"));
            assert!(!serialized.contains("\r\n"));
            assert!(
                serialized.find("schema_version").unwrap()
                    < serialized.find("metadata_scope").unwrap()
            );
        }
        assert!(candidate_first.contains("metadata_scope = \"candidate\""));
        assert!(published_first.contains("metadata_scope = \"published\""));
    }

    #[test]
    fn explicit_candidate_metadata_reader_and_writer_use_candidate_dir() {
        let temp = tempfile::tempdir().unwrap();
        let metadata_dir = CandidateMetadataDir::new(temp.path().join(".kslim")).unwrap();
        let metadata = candidate_metadata(Some(ReducerReportSummary {
            files_removed: 1,
            dirs_removed: 0,
            edit_records: 2,
        }));

        write_candidate_metadata(&metadata_dir, &metadata).unwrap();

        let path = metadata_dir.as_path().join(CANDIDATE_METADATA_FILE);
        assert!(path.is_file());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            serialize_candidate_metadata(&metadata).unwrap()
        );
        assert!(!metadata_dir
            .as_path()
            .join(PUBLISHED_METADATA_FILE)
            .exists());
        assert_eq!(read_candidate_metadata(&metadata_dir).unwrap(), metadata);
    }

    #[test]
    fn explicit_published_metadata_reader_and_writer_use_published_dir() {
        let temp = tempfile::tempdir().unwrap();
        let metadata_dir = published_metadata_dir_for_test(&temp.path().join("output"));
        let metadata = published_metadata(Some(ReducerReportSummary {
            files_removed: 1,
            dirs_removed: 0,
            edit_records: 2,
        }));

        write_published_metadata(&metadata_dir, &metadata).unwrap();

        let path = metadata_dir.as_path().join(PUBLISHED_METADATA_FILE);
        assert!(path.is_file());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            serialize_published_metadata(&metadata).unwrap()
        );
        assert!(!metadata_dir
            .as_path()
            .join(CANDIDATE_METADATA_FILE)
            .exists());
        assert_eq!(read_published_metadata(&metadata_dir).unwrap(), metadata);
    }

    #[test]
    fn published_metadata_reader_rejects_candidate_marker() {
        let temp = tempfile::tempdir().unwrap();
        let metadata_dir = published_metadata_dir_for_test(&temp.path().join("output"));
        crate::fsutil::ensure_dir(metadata_dir.as_path()).unwrap();
        std::fs::write(
            metadata_dir.as_path().join(PUBLISHED_METADATA_FILE),
            toml::to_string_pretty(&candidate_metadata(None)).unwrap(),
        )
        .unwrap();

        let err = read_published_metadata(&metadata_dir)
            .unwrap_err()
            .to_string();

        assert!(
            err.contains(
                "published metadata marker 'candidate' does not match expected 'published'"
            ),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn candidate_metadata_reader_rejects_published_marker() {
        let temp = tempfile::tempdir().unwrap();
        let metadata_dir = CandidateMetadataDir::new(temp.path().join(".kslim")).unwrap();
        crate::fsutil::ensure_dir(metadata_dir.as_path()).unwrap();
        std::fs::write(
            metadata_dir.as_path().join(CANDIDATE_METADATA_FILE),
            toml::to_string_pretty(&published_metadata(None)).unwrap(),
        )
        .unwrap();

        let err = read_candidate_metadata(&metadata_dir)
            .unwrap_err()
            .to_string();

        assert!(
            err.contains(
                "candidate metadata marker 'published' does not match expected 'candidate'"
            ),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn metadata_readers_reject_unsupported_schema_version() {
        let temp = tempfile::tempdir().unwrap();
        let candidate_dir =
            CandidateMetadataDir::new(temp.path().join("candidate/.kslim")).unwrap();
        let published_dir = published_metadata_dir_for_test(&temp.path().join("output"));
        crate::fsutil::ensure_dir(candidate_dir.as_path()).unwrap();
        crate::fsutil::ensure_dir(published_dir.as_path()).unwrap();

        let mut candidate = candidate_metadata(None);
        candidate.schema_version = MetadataSchemaVersion::new(2);
        std::fs::write(
            candidate_dir.as_path().join(CANDIDATE_METADATA_FILE),
            toml::to_string_pretty(&candidate).unwrap(),
        )
        .unwrap();

        let mut published = published_metadata(None);
        published.schema_version = MetadataSchemaVersion::new(2);
        std::fs::write(
            published_dir.as_path().join(PUBLISHED_METADATA_FILE),
            toml::to_string_pretty(&published).unwrap(),
        )
        .unwrap();

        let candidate_err = read_candidate_metadata(&candidate_dir)
            .unwrap_err()
            .to_string();
        let published_err = read_published_metadata(&published_dir)
            .unwrap_err()
            .to_string();

        assert!(
            candidate_err
                .contains("candidate metadata schema version 2 is not supported; expected 1"),
            "unexpected error: {candidate_err}"
        );
        assert!(
            published_err
                .contains("published metadata schema version 2 is not supported; expected 1"),
            "unexpected error: {published_err}"
        );
    }

    #[test]
    fn metadata_writers_reject_unsupported_schema_version() {
        let temp = tempfile::tempdir().unwrap();
        let candidate_dir =
            CandidateMetadataDir::new(temp.path().join("candidate/.kslim")).unwrap();
        let published_dir = published_metadata_dir_for_test(&temp.path().join("output"));

        let mut candidate = candidate_metadata(None);
        candidate.schema_version = MetadataSchemaVersion::new(2);
        let mut published = published_metadata(None);
        published.schema_version = MetadataSchemaVersion::new(2);

        let candidate_err = write_candidate_metadata(&candidate_dir, &candidate)
            .unwrap_err()
            .to_string();
        let published_err = write_published_metadata(&published_dir, &published)
            .unwrap_err()
            .to_string();

        assert!(
            candidate_err
                .contains("candidate metadata schema version 2 is not supported; expected 1"),
            "unexpected error: {candidate_err}"
        );
        assert!(
            published_err
                .contains("published metadata schema version 2 is not supported; expected 1"),
            "unexpected error: {published_err}"
        );
    }

    #[test]
    fn metadata_dir_helpers_are_phase_typed() {
        let temp = tempfile::tempdir().unwrap();
        let candidate_tree = CandidateTreePath::new(temp.path().join("candidate")).unwrap();
        std::fs::create_dir_all(candidate_tree.as_path().join(".git")).unwrap();
        let output_repo_path = temp.path().join("output");
        std::fs::create_dir_all(output_repo_path.join(".git")).unwrap();
        let output_repo = OutputRepoPath::new(output_repo_path.clone()).unwrap();
        let tree_output_path = temp.path().join("tree-output");
        let tree_output_repo = OutputRepoPath::new(tree_output_path.clone()).unwrap();

        let candidate_dir = candidate_metadata_dir(&candidate_tree).unwrap();
        let published_dir = published_metadata_dir(&output_repo).unwrap();
        let tree_published_dir = published_metadata_dir(&tree_output_repo).unwrap();

        assert_eq!(
            candidate_dir.as_path(),
            temp.path().join("candidate/.kslim")
        );
        assert_eq!(published_dir.as_path(), output_repo_path.join(".git/kslim"));
        assert_eq!(
            tree_published_dir.as_path(),
            tree_output_path.join(".kslim")
        );
    }

    #[test]
    fn candidate_metadata_root_requires_candidate_marker() {
        let missing_marker = r#"
schema_version = 1
plan_fingerprint = "fingerprint-plan"
tree_fingerprint = "fingerprint-tree"
generated_by = "kslim-test"

[selftest]
enabled = true
built_in_checks = 0
kernel_builds_run = 0
commands_run = 0
"#;
        let wrong_marker = missing_marker.replace(
            "schema_version = 1",
            "schema_version = 1\nmetadata_scope = \"published\"",
        );

        assert!(toml::from_str::<CandidateMetadata>(missing_marker).is_err());
        assert!(toml::from_str::<CandidateMetadata>(&wrong_marker).is_err());
    }

    #[test]
    fn published_metadata_root_requires_published_marker() {
        let missing_marker = r#"
schema_version = 1
snapshot_id = "snapshot-main-1"
output_commit = "abc123"
branch = "main"
plan_fingerprint = "fingerprint-plan"
tree_fingerprint = "fingerprint-tree"
metadata_fingerprint = "fingerprint-metadata"
generated_by = "kslim-test"

[selftest]
enabled = true
built_in_checks = 0
kernel_builds_run = 0
commands_run = 0
"#;
        let wrong_marker = missing_marker.replace(
            "schema_version = 1",
            "schema_version = 1\nmetadata_scope = \"candidate\"",
        );

        assert!(toml::from_str::<PublishedMetadata>(missing_marker).is_err());
        assert!(toml::from_str::<PublishedMetadata>(&wrong_marker).is_err());
    }

    #[test]
    fn metadata_identity_values_reject_empty_strings() {
        assert!(PlanFingerprint::new(" ").is_err());
        assert!(TreeFingerprint::new("").is_err());
        assert!(MetadataFingerprint::new("").is_err());
        assert!(SnapshotId::new("").is_err());
        assert!(GitCommitId::new(" ").is_err());
        assert!(OutputBranchName::new("\t").is_err());
        assert!(ToolVersion::new("\t").is_err());
    }

    #[test]
    fn committed_metadata_rejects_raw_log_content() {
        let temp = tempfile::tempdir().unwrap();
        let metadata_dir = temp.path().join(".kslim");
        crate::fsutil::ensure_dir(&metadata_dir).unwrap();
        std::fs::write(
            metadata_dir.join("diagnostics.json"),
            r#"{"raw_excerpts":["stderr:\nprivate compiler output\n"]}"#,
        )
        .unwrap();

        let err = validate_committed_metadata_has_no_raw_logs(&metadata_dir)
            .unwrap_err()
            .to_string();

        assert!(err.contains("raw log"));
        assert!(err.contains("normalized summaries"));
        assert!(err.contains("diagnostics.json"));
    }

    #[test]
    fn committed_metadata_rejects_raw_log_artifact_files() {
        let temp = tempfile::tempdir().unwrap();
        let metadata_dir = temp.path().join(".kslim");
        crate::fsutil::ensure_dir(&metadata_dir).unwrap();
        std::fs::write(metadata_dir.join("build.log"), "raw compiler output\n").unwrap();

        let err = validate_committed_metadata_has_no_raw_logs(&metadata_dir)
            .unwrap_err()
            .to_string();

        assert!(err.contains("raw log artifact"));
        assert!(err.contains("build.log"));
    }

    #[test]
    fn committed_metadata_allows_normalized_log_summaries() {
        let temp = tempfile::tempdir().unwrap();
        let metadata_dir = temp.path().join(".kslim");
        crate::fsutil::ensure_dir(&metadata_dir).unwrap();
        std::fs::write(
            metadata_dir.join("diagnostics.json"),
            concat!(
                "{\n",
                "  \"diagnostic_log_summaries_by_command\": [\n",
                "    {\"command_context\":\"make modules\",\"log_excerpt_count\":2}\n",
                "  ]\n",
                "}\n"
            ),
        )
        .unwrap();

        validate_committed_metadata_has_no_raw_logs(&metadata_dir).unwrap();
    }
}
