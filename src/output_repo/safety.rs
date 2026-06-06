//! Read-only output repository safety preflight.
//!
//! This module owns the boundary between a resolved output plan and the output
//! repository that may later be mutated. It must not publish metadata, sync
//! candidate files, update lockfiles, or infer upstream truth.

use anyhow::{Context, Result};
use std::path::{Component, Path, PathBuf};

use crate::generate::OutputPlan;
use crate::model::{GitCommitId, OutputBranchName};
use crate::paths::{LockfilePath, OutputRepoPath, PublishedMetadataDir};

use super::metadata;

/// Read-only preflight state for an output repository target.
///
/// This value records the resolved output target and coarse observed facts. It
/// is not an authoritative published snapshot and must not be used as proof
/// that candidate or attempt metadata has been published.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct OutputRepoSafety {
    pub(crate) repo: OutputRepoPath,
    pub(crate) metadata_dir: PublishedMetadataDir,
    pub(crate) branch: OutputBranchName,
    pub(crate) head: Option<GitCommitId>,
    pub(crate) clean: bool,
    pub(crate) metadata_consistent: bool,
    pub(crate) lockfile_consistent: bool,
    pub(crate) expected_branch: String,
    pub(crate) expected_mode: String,
    pub(crate) repo_exists: bool,
    pub(crate) git_dir_exists: bool,
    pub(crate) is_git_worktree: bool,
    pub(crate) current_branch: String,
    pub(crate) current_head: Option<String>,
    pub(crate) branch_matches_expected: bool,
    pub(crate) tracked_tree_clean: bool,
    pub(crate) untracked_paths: Vec<PathBuf>,
    pub(crate) metadata_dir_sane: bool,
    pub(crate) published_metadata_present: bool,
    pub(crate) published_metadata_commit: Option<String>,
    pub(crate) published_metadata_consistent: bool,
    pub(crate) lockfile_path: Option<LockfilePath>,
    pub(crate) lockfile_present: bool,
    pub(crate) lockfile_published_snapshot_present: bool,
}

/// Establish the read-only safety check boundary for an output repository.
#[allow(dead_code)]
pub(crate) fn check_output_repo_safety(
    repo: &OutputRepoPath,
    expected: &OutputPlan,
) -> Result<OutputRepoSafety> {
    if expected.branch.trim().is_empty() {
        anyhow::bail!("output repo safety check expected branch is empty");
    }
    if expected.mode.trim().is_empty() {
        anyhow::bail!("output repo safety check expected mode is empty");
    }

    let repo_path = normalize_output_repo_path(repo.as_path()).with_context(|| {
        format!(
            "failed to normalize output repo {}",
            repo.as_path().display()
        )
    })?;
    let expected_path =
        normalize_output_repo_path(expected.output_path.as_path()).with_context(|| {
            format!(
                "failed to normalize expected output repo {}",
                expected.output_path.as_path().display()
            )
        })?;
    if repo_path != expected_path {
        anyhow::bail!(
            "output repo safety check target '{}' does not match expected output '{}'",
            repo.as_path().display(),
            expected.output_path.as_path().display()
        );
    }
    if !repo.as_path().exists() {
        anyhow::bail!(
            "output repo safety check failed: repo does not exist: {}",
            repo.as_path().display()
        );
    }
    let is_git_worktree = is_git_worktree(repo.as_path());
    if !is_git_worktree {
        anyhow::bail!(
            "output repo safety check failed: repo is not a Git worktree: {}",
            repo.as_path().display()
        );
    }
    let current_branch = current_output_branch(repo.as_path())?;
    if current_branch.trim().is_empty() {
        anyhow::bail!(
            "output repo safety check failed: repo is detached; expected branch '{}'",
            expected.branch
        );
    }
    let branch_matches_expected = current_branch == expected.branch;
    if !branch_matches_expected {
        anyhow::bail!(
            "output repo safety check failed: current branch '{}' does not match expected branch '{}'",
            current_branch,
            expected.branch
        );
    }
    let branch = OutputBranchName::new(current_branch.clone())?;
    let current_head = current_output_head(repo.as_path())?;
    let head = current_head.clone().map(GitCommitId::new).transpose()?;
    let tracked_tree_clean = output_tracked_tree_clean(repo.as_path())?;
    if !tracked_tree_clean {
        anyhow::bail!(
            "output repo safety check failed: tree is not clean before mutation: {} has tracked or staged changes",
            repo.as_path().display()
        );
    }
    let untracked_paths = output_untracked_paths(repo.as_path())?;
    if !untracked_paths.is_empty() {
        anyhow::bail!(
            "output repo safety check failed: untracked files would be clobbered before mutation: {}",
            format_untracked_paths(&untracked_paths)
        );
    }
    let clean = tracked_tree_clean && untracked_paths.is_empty();
    let metadata_dir = metadata::published_metadata_dir(repo)?;
    ensure_metadata_dirs_are_sane(repo.as_path(), metadata_dir.as_path())?;
    let published_metadata = load_current_published_metadata(repo, current_head.as_deref())?;
    if let Some(published) = published_metadata.as_ref() {
        if published.branch != current_branch {
            anyhow::bail!(
                "output repo safety check failed: committed published metadata branch '{}' does not match current output branch '{}'",
                published.branch,
                current_branch
            );
        }
    }
    let lockfile = load_expected_lockfile(expected)?;
    if let Some(published) = published_metadata.as_ref() {
        if let Some(lockfile) = lockfile.as_ref() {
            let current_commit = current_head.as_deref().ok_or_else(|| {
                anyhow::anyhow!(
                    "output repo safety check failed: committed published metadata exists without output HEAD"
                )
            })?;
            ensure_lockfile_matches_published_metadata(
                lockfile,
                published,
                &current_branch,
                current_commit,
            )?;
        }
    }
    let published_metadata_present = published_metadata.is_some();
    let published_metadata_commit = published_metadata_present
        .then(|| current_head.clone())
        .flatten();
    let lockfile_present = lockfile.is_some();
    let lockfile_published_snapshot_present = lockfile
        .as_ref()
        .and_then(|lockfile| lockfile.published.as_ref())
        .is_some();
    let metadata_consistent = true;
    let lockfile_consistent = true;

    Ok(OutputRepoSafety {
        repo: repo.clone(),
        metadata_dir,
        branch,
        head,
        clean,
        metadata_consistent,
        lockfile_consistent,
        expected_branch: expected.branch.clone(),
        expected_mode: expected.mode.clone(),
        repo_exists: repo.as_path().exists(),
        git_dir_exists: repo.as_path().join(".git").exists(),
        is_git_worktree,
        current_branch,
        current_head,
        branch_matches_expected,
        tracked_tree_clean,
        untracked_paths,
        metadata_dir_sane: true,
        published_metadata_present,
        published_metadata_commit,
        published_metadata_consistent: true,
        lockfile_path: expected.lockfile_path.clone(),
        lockfile_present,
        lockfile_published_snapshot_present,
    })
}

#[allow(dead_code)]
fn load_expected_lockfile(expected: &OutputPlan) -> Result<Option<crate::lockfile::Lockfile>> {
    let Some(path) = expected.lockfile_path.as_ref() else {
        return Ok(None);
    };
    if !path.as_path().exists() {
        return Ok(None);
    }
    if !path.as_path().is_file() {
        anyhow::bail!(
            "output repo safety check failed: expected lockfile path is not a file: {}",
            path.as_path().display()
        );
    }
    crate::lockfile::load_lockfile(path).with_context(|| {
        format!(
            "failed to load expected lockfile {}",
            path.as_path().display()
        )
    })
}

#[allow(dead_code)]
fn ensure_lockfile_matches_published_metadata(
    lockfile: &crate::lockfile::Lockfile,
    published: &metadata::PublishedSnapshotMetadata,
    current_branch: &str,
    current_commit: &str,
) -> Result<()> {
    let Some(published_lock) = lockfile.published.as_ref() else {
        anyhow::bail!(
            "output repo safety check failed: lockfile has no published snapshot to match committed published metadata"
        );
    };
    if published_lock.output_branch != current_branch {
        anyhow::bail!(
            "output repo safety check failed: lockfile output branch '{}' does not match current output branch '{}'",
            published_lock.output_branch,
            current_branch
        );
    }
    if published_lock.output_commit != current_commit {
        anyhow::bail!(
            "output repo safety check failed: lockfile output commit '{}' does not match output HEAD '{}'",
            published_lock.output_commit,
            current_commit
        );
    }
    if published_lock.output_branch != published.branch {
        anyhow::bail!(
            "output repo safety check failed: lockfile output branch '{}' does not match committed published metadata branch '{}'",
            published_lock.output_branch,
            published.branch
        );
    }
    if published_lock.tag != published.tag {
        anyhow::bail!(
            "output repo safety check failed: lockfile tag '{}' does not match committed published metadata tag '{}'",
            published_lock.tag,
            published.tag
        );
    }
    if lockfile.resolved_base.r#ref != published_lock.base_ref
        || lockfile.resolved_base.commit != published_lock.base_commit
    {
        anyhow::bail!(
            "output repo safety check failed: lockfile resolved_base does not match lockfile published snapshot"
        );
    }
    if published_lock.base_ref != published.base_ref
        || published_lock.base_commit != published.base_commit
        || published_lock.profile != published.profile
        || published_lock.mode != published.mode
        || published_lock.generated_at != published.generated_at
    {
        anyhow::bail!(
            "output repo safety check failed: lockfile published snapshot does not match committed published metadata"
        );
    }
    Ok(())
}

#[allow(dead_code)]
fn load_current_published_metadata(
    repo: &OutputRepoPath,
    current_head: Option<&str>,
) -> Result<Option<metadata::PublishedSnapshotMetadata>> {
    let Some(commit) = current_head else {
        return Ok(None);
    };
    if !committed_published_metadata_exists(repo.as_path(), commit) {
        return Ok(None);
    }
    metadata::load_committed_published_snapshot_metadata(repo, commit)
        .map(Some)
        .with_context(|| {
            format!(
                "failed to load current committed published metadata at output HEAD {}",
                commit
            )
        })
}

#[allow(dead_code)]
fn committed_published_metadata_exists(repo: &Path, commit: &str) -> bool {
    let repo = repo.to_string_lossy();
    let metadata_ref = format!("{}:{}", commit, committed_published_metadata_ref());
    crate::process::run_in_dir(&repo, "git", &["cat-file", "-e", &metadata_ref]).is_ok()
}

#[allow(dead_code)]
fn committed_published_metadata_ref() -> String {
    format!(
        "{}/{}",
        metadata::COMMITTED_METADATA_DIR,
        metadata::PUBLISHED_SNAPSHOT_FILE
    )
}

#[allow(dead_code)]
fn ensure_metadata_dirs_are_sane(repo: &Path, private_metadata_dir: &Path) -> Result<()> {
    ensure_optional_metadata_dir_shape("output repo private metadata dir", private_metadata_dir)?;

    let committed_metadata_dir = repo.join(metadata::COMMITTED_METADATA_DIR);
    if committed_metadata_dir != private_metadata_dir {
        ensure_optional_metadata_dir_shape(
            "output repo committed metadata dir",
            &committed_metadata_dir,
        )?;
    }

    Ok(())
}

#[allow(dead_code)]
fn ensure_optional_metadata_dir_shape(label: &str, path: &Path) -> Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(err)
                .with_context(|| format!("failed to inspect {label}: {}", path.display()));
        }
    };
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        anyhow::bail!(
            "output repo safety check failed: {label} is a symlink: {}",
            path.display()
        );
    }
    if !file_type.is_dir() {
        anyhow::bail!(
            "output repo safety check failed: {label} is not a directory: {}",
            path.display()
        );
    }
    Ok(())
}

#[allow(dead_code)]
fn output_untracked_paths(repo: &Path) -> Result<Vec<PathBuf>> {
    let repo = repo.to_string_lossy();
    let status = crate::process::run_in_dir(
        &repo,
        "git",
        &["status", "--porcelain", "--untracked-files=all"],
    )
    .context("failed to inspect output repo untracked paths")?;

    let mut paths = status
        .lines()
        .filter_map(|line| line.strip_prefix("?? "))
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

#[allow(dead_code)]
fn format_untracked_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

#[allow(dead_code)]
fn output_tracked_tree_clean(repo: &Path) -> Result<bool> {
    let repo = repo.to_string_lossy();
    let status = crate::process::run_in_dir(
        &repo,
        "git",
        &["status", "--porcelain", "--untracked-files=no"],
    )
    .context("failed to inspect output repo tracked working tree")?;
    Ok(status.trim().is_empty())
}

#[allow(dead_code)]
fn current_output_branch(repo: &Path) -> Result<String> {
    let repo = repo.to_string_lossy();
    crate::process::run_in_dir(&repo, "git", &["branch", "--show-current"])
        .context("failed to read output repo current branch")
}

#[allow(dead_code)]
fn current_output_head(repo: &Path) -> Result<Option<String>> {
    let repo = repo.to_string_lossy();
    match crate::process::run_in_dir(&repo, "git", &["rev-parse", "--verify", "HEAD"]) {
        Ok(commit) if !commit.trim().is_empty() => Ok(Some(commit)),
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

#[allow(dead_code)]
fn is_git_worktree(repo: &Path) -> bool {
    if !repo.is_dir() {
        return false;
    }
    let repo = repo.to_string_lossy();
    matches!(
        crate::process::run_in_dir(&repo, "git", &["rev-parse", "--is-inside-work-tree"]),
        Ok(value) if value == "true"
    )
}

#[allow(dead_code)]
fn normalize_output_repo_path(path: &Path) -> Result<PathBuf> {
    if path.as_os_str().is_empty() {
        anyhow::bail!("output repo path is empty");
    }

    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::generate::OutputNamingPlan;

    fn init_repo_on_branch(repo_path: &Path, branch: &str) {
        crate::git::init_repo(repo_path.to_str().unwrap()).unwrap();
        crate::git::create_branch(repo_path.to_str().unwrap(), branch).unwrap();
    }

    fn configure_repo_identity(repo_path: &Path) {
        crate::process::run_in_dir(
            repo_path.to_str().unwrap(),
            "git",
            &["config", "user.email", "test@kslim.local"],
        )
        .unwrap();
        crate::process::run_in_dir(
            repo_path.to_str().unwrap(),
            "git",
            &["config", "user.name", "kslim test"],
        )
        .unwrap();
    }

    fn commit_published_metadata(repo_path: &Path, branch: &str) -> String {
        configure_repo_identity(repo_path);
        std::fs::create_dir_all(repo_path.join(".kslim")).unwrap();
        let metadata = metadata::PublishedSnapshotMetadata {
            branch: branch.to_string(),
            tag: "kslim-v6.1-tiny-r1".to_string(),
            base_ref: "v6.1".to_string(),
            base_commit: "abc123".to_string(),
            profile: "tiny".to_string(),
            mode: "generate".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            candidate_metadata_fingerprint: None,
            base_metadata_file: metadata::BASE_METADATA_FILE.to_string(),
            generated_metadata_file: metadata::GENERATED_METADATA_FILE.to_string(),
            manifest_file: crate::manifest::OUTPUT_MANIFEST_FILE_NAME.to_string(),
            report_file: metadata::REPORT_FILE.to_string(),
            kslim_version: env!("CARGO_PKG_VERSION").to_string(),
        };
        std::fs::write(
            repo_path
                .join(".kslim")
                .join(metadata::PUBLISHED_SNAPSHOT_FILE),
            toml::to_string_pretty(&metadata).unwrap(),
        )
        .unwrap();
        crate::git::add_all(repo_path.to_str().unwrap()).unwrap();
        crate::git::commit(repo_path.to_str().unwrap(), "commit published metadata").unwrap();
        crate::git::head_commit(repo_path.to_str().unwrap()).unwrap()
    }

    fn test_resolved_base() -> crate::lockfile::ResolvedBase {
        crate::lockfile::ResolvedBase {
            upstream: "linux".to_string(),
            url: "/tmp/linux.git".to_string(),
            r#ref: "v6.1".to_string(),
            commit: "abc123".to_string(),
            resolved_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn test_published_lockfile(
        output_commit: &str,
        branch: &str,
    ) -> crate::lockfile::PublishedLockfile {
        crate::lockfile::PublishedLockfile {
            output_branch: branch.to_string(),
            output_commit: output_commit.to_string(),
            tag: "kslim-v6.1-tiny-r1".to_string(),
            base_ref: "v6.1".to_string(),
            base_commit: "abc123".to_string(),
            profile: "tiny".to_string(),
            mode: "generate".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn write_published_lockfile(project_root: &Path, output_commit: &str, branch: &str) {
        std::fs::create_dir_all(project_root).unwrap();
        let lockfile_path = LockfilePath::new_in_project_root(project_root).unwrap();
        let update = crate::lockfile::PublishedLockfileUpdate::new(
            test_resolved_base(),
            test_published_lockfile(output_commit, branch),
        )
        .unwrap();
        crate::lockfile::write_published_lockfile(&lockfile_path, &update).unwrap();
    }

    fn write_resolved_only_lockfile(project_root: &Path) {
        std::fs::create_dir_all(project_root).unwrap();
        let lockfile_path = LockfilePath::new_in_project_root(project_root).unwrap();
        let update = crate::lockfile::ResolvedBaseLockfileUpdate::new(test_resolved_base());
        crate::lockfile::write_resolved_base_lockfile(&lockfile_path, &update).unwrap();
    }

    fn output_plan(path: &Path, branch: &str, mode: &str) -> OutputPlan {
        output_plan_with_lockfile(path, branch, mode, None)
    }

    fn output_plan_with_lockfile(
        path: &Path,
        branch: &str,
        mode: &str,
        lockfile_path: Option<LockfilePath>,
    ) -> OutputPlan {
        OutputPlan {
            output_path: OutputRepoPath::new(path).unwrap(),
            branch: branch.to_string(),
            mode: mode.to_string(),
            lockfile_path,
            naming: OutputNamingPlan {
                project_name: "demo".to_string(),
                profile_name: "tiny".to_string(),
                branch_prefix: "kslim".to_string(),
                explicit_branch: Some(branch.to_string()),
                base_ref: "v6.1".to_string(),
                base_commit: "abc123".to_string(),
            },
        }
    }

    #[test]
    fn check_output_repo_safety_reports_target_plan_boundary() {
        let temp = tempfile::tempdir().unwrap();
        let repo_path = temp.path().join("out");
        init_repo_on_branch(&repo_path, "kslim-main");
        let repo = OutputRepoPath::new(&repo_path).unwrap();
        let expected = output_plan(&repo_path, "kslim-main", "generate");

        let safety = check_output_repo_safety(&repo, &expected).unwrap();

        assert_eq!(safety.repo, repo);
        assert_eq!(safety.branch.as_str(), "kslim-main");
        assert_eq!(safety.head, None);
        assert!(safety.clean);
        assert!(safety.metadata_consistent);
        assert!(safety.lockfile_consistent);
        assert_eq!(safety.expected_branch, "kslim-main");
        assert_eq!(safety.expected_mode, "generate");
        assert!(safety.repo_exists);
        assert!(safety.git_dir_exists);
        assert!(safety.is_git_worktree);
        assert_eq!(safety.current_branch, "kslim-main");
        assert_eq!(safety.current_head, None);
        assert!(safety.branch_matches_expected);
        assert!(safety.tracked_tree_clean);
        assert!(safety.untracked_paths.is_empty());
        assert!(safety.metadata_dir_sane);
        assert!(!safety.published_metadata_present);
        assert_eq!(safety.published_metadata_commit, None);
        assert!(safety.published_metadata_consistent);
        assert_eq!(safety.lockfile_path, None);
        assert!(!safety.lockfile_present);
        assert!(!safety.lockfile_published_snapshot_present);
        assert!(safety.lockfile_consistent);
        assert_eq!(safety.metadata_dir.as_path(), repo_path.join(".git/kslim"));
    }

    #[test]
    fn check_output_repo_safety_accepts_current_published_metadata_for_current_branch_and_head() {
        let temp = tempfile::tempdir().unwrap();
        let repo_path = temp.path().join("out");
        init_repo_on_branch(&repo_path, "kslim-main");
        let commit = commit_published_metadata(&repo_path, "kslim-main");
        let repo = OutputRepoPath::new(&repo_path).unwrap();
        let expected = output_plan(&repo_path, "kslim-main", "generate");

        let safety = check_output_repo_safety(&repo, &expected).unwrap();

        assert_eq!(safety.current_head.as_deref(), Some(commit.as_str()));
        assert_eq!(
            safety.head.as_ref().map(|head| head.as_str()),
            Some(commit.as_str())
        );
        assert!(safety.published_metadata_present);
        assert_eq!(
            safety.published_metadata_commit.as_deref(),
            Some(commit.as_str())
        );
        assert!(safety.metadata_consistent);
        assert!(safety.published_metadata_consistent);
        assert!(!safety.lockfile_present);
        assert!(!safety.lockfile_published_snapshot_present);
        assert!(safety.lockfile_consistent);
    }

    #[test]
    fn check_output_repo_safety_accepts_lockfile_matching_current_published_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let project_root = temp.path().join("project");
        let repo_path = temp.path().join("out");
        init_repo_on_branch(&repo_path, "kslim-main");
        let commit = commit_published_metadata(&repo_path, "kslim-main");
        write_published_lockfile(&project_root, &commit, "kslim-main");
        let repo = OutputRepoPath::new(&repo_path).unwrap();
        let expected = output_plan_with_lockfile(
            &repo_path,
            "kslim-main",
            "generate",
            Some(LockfilePath::new_in_project_root(&project_root).unwrap()),
        );

        let safety = check_output_repo_safety(&repo, &expected).unwrap();

        assert_eq!(
            safety.lockfile_path.as_ref().map(|path| path.as_path()),
            Some(project_root.join("kslim.lock").as_path())
        );
        assert!(safety.lockfile_present);
        assert!(safety.lockfile_published_snapshot_present);
        assert!(safety.clean);
        assert!(safety.metadata_consistent);
        assert!(safety.lockfile_consistent);
    }

    #[test]
    fn check_output_repo_safety_rejects_mismatched_expected_output() {
        let temp = tempfile::tempdir().unwrap();
        let repo_path = temp.path().join("out");
        let other_path = temp.path().join("other");
        std::fs::create_dir_all(&repo_path).unwrap();
        let repo = OutputRepoPath::new(&repo_path).unwrap();
        let expected = output_plan(&other_path, "kslim-main", "generate");

        let err = check_output_repo_safety(&repo, &expected).unwrap_err();

        assert!(
            err.to_string().contains("does not match expected output"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn check_output_repo_safety_rejects_missing_repo_before_published_truth() {
        let temp = tempfile::tempdir().unwrap();
        let repo_path = temp.path().join("missing");
        let repo = OutputRepoPath::new(&repo_path).unwrap();
        let expected = output_plan(&repo_path, "kslim-main", "generate");

        let err = check_output_repo_safety(&repo, &expected).unwrap_err();

        assert!(
            err.to_string().contains("repo does not exist"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn check_output_repo_safety_rejects_existing_non_git_directory() {
        let temp = tempfile::tempdir().unwrap();
        let repo_path = temp.path().join("out");
        std::fs::create_dir_all(&repo_path).unwrap();
        let repo = OutputRepoPath::new(&repo_path).unwrap();
        let expected = output_plan(&repo_path, "kslim-main", "generate");

        let err = check_output_repo_safety(&repo, &expected).unwrap_err();

        assert!(
            err.to_string().contains("not a Git worktree"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn check_output_repo_safety_rejects_unexpected_branch() {
        let temp = tempfile::tempdir().unwrap();
        let repo_path = temp.path().join("out");
        init_repo_on_branch(&repo_path, "other-branch");
        let repo = OutputRepoPath::new(&repo_path).unwrap();
        let expected = output_plan(&repo_path, "kslim-main", "generate");

        let err = check_output_repo_safety(&repo, &expected).unwrap_err();

        assert!(
            err.to_string().contains(
                "current branch 'other-branch' does not match expected branch 'kslim-main'"
            ),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn check_output_repo_safety_rejects_tracked_tree_changes_before_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let repo_path = temp.path().join("out");
        init_repo_on_branch(&repo_path, "kslim-main");
        std::fs::write(repo_path.join("tracked.txt"), "staged\n").unwrap();
        crate::process::run_in_dir(repo_path.to_str().unwrap(), "git", &["add", "tracked.txt"])
            .unwrap();
        let repo = OutputRepoPath::new(&repo_path).unwrap();
        let expected = output_plan(&repo_path, "kslim-main", "generate");

        let err = check_output_repo_safety(&repo, &expected).unwrap_err();

        assert!(
            err.to_string()
                .contains("tree is not clean before mutation"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn check_output_repo_safety_rejects_unstaged_tracked_changes_by_default() {
        let temp = tempfile::tempdir().unwrap();
        let repo_path = temp.path().join("out");
        init_repo_on_branch(&repo_path, "kslim-main");
        configure_repo_identity(&repo_path);
        std::fs::write(repo_path.join("tracked.txt"), "clean\n").unwrap();
        crate::git::add_all(repo_path.to_str().unwrap()).unwrap();
        crate::git::commit(repo_path.to_str().unwrap(), "commit clean baseline").unwrap();
        std::fs::write(repo_path.join("tracked.txt"), "dirty\n").unwrap();
        let repo = OutputRepoPath::new(&repo_path).unwrap();
        let expected = output_plan(&repo_path, "kslim-main", "generate");

        let err = check_output_repo_safety(&repo, &expected).unwrap_err();

        assert!(
            err.to_string()
                .contains("tree is not clean before mutation"),
            "unexpected error: {err:#}"
        );
        assert!(crate::git::is_dirty(repo_path.to_str().unwrap()).unwrap());
    }

    #[test]
    fn check_output_repo_safety_rejects_untracked_files_before_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let repo_path = temp.path().join("out");
        init_repo_on_branch(&repo_path, "kslim-main");
        std::fs::write(repo_path.join("loose.txt"), "untracked\n").unwrap();
        let repo = OutputRepoPath::new(&repo_path).unwrap();
        let expected = output_plan(&repo_path, "kslim-main", "generate");

        let err = check_output_repo_safety(&repo, &expected).unwrap_err();

        assert!(
            err.to_string()
                .contains("untracked files would be clobbered"),
            "unexpected error: {err:#}"
        );
        assert!(
            err.to_string().contains("loose.txt"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn check_output_repo_safety_rejects_private_metadata_file() {
        let temp = tempfile::tempdir().unwrap();
        let repo_path = temp.path().join("out");
        init_repo_on_branch(&repo_path, "kslim-main");
        std::fs::write(repo_path.join(".git/kslim"), "not a metadata dir\n").unwrap();
        let repo = OutputRepoPath::new(&repo_path).unwrap();
        let expected = output_plan(&repo_path, "kslim-main", "generate");

        let err = check_output_repo_safety(&repo, &expected).unwrap_err();

        assert!(
            err.to_string()
                .contains("private metadata dir is not a directory"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn check_output_repo_safety_rejects_committed_metadata_file() {
        let temp = tempfile::tempdir().unwrap();
        let repo_path = temp.path().join("out");
        init_repo_on_branch(&repo_path, "kslim-main");
        configure_repo_identity(&repo_path);
        std::fs::write(repo_path.join(".kslim"), "not a metadata dir\n").unwrap();
        crate::git::add_all(repo_path.to_str().unwrap()).unwrap();
        crate::git::commit(repo_path.to_str().unwrap(), "commit malformed metadata").unwrap();
        let repo = OutputRepoPath::new(&repo_path).unwrap();
        let expected = output_plan(&repo_path, "kslim-main", "generate");

        let err = check_output_repo_safety(&repo, &expected).unwrap_err();

        assert!(
            err.to_string()
                .contains("committed metadata dir is not a directory"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn check_output_repo_safety_rejects_published_metadata_for_other_branch() {
        let temp = tempfile::tempdir().unwrap();
        let repo_path = temp.path().join("out");
        init_repo_on_branch(&repo_path, "kslim-main");
        commit_published_metadata(&repo_path, "other-branch");
        let repo = OutputRepoPath::new(&repo_path).unwrap();
        let expected = output_plan(&repo_path, "kslim-main", "generate");

        let err = check_output_repo_safety(&repo, &expected).unwrap_err();

        assert!(
            err.to_string().contains(
                "committed published metadata branch 'other-branch' does not match current output branch 'kslim-main'"
            ),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn check_output_repo_safety_rejects_lockfile_output_commit_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let project_root = temp.path().join("project");
        let repo_path = temp.path().join("out");
        init_repo_on_branch(&repo_path, "kslim-main");
        let commit = commit_published_metadata(&repo_path, "kslim-main");
        write_published_lockfile(&project_root, "differentcommit", "kslim-main");
        let repo = OutputRepoPath::new(&repo_path).unwrap();
        let expected = output_plan_with_lockfile(
            &repo_path,
            "kslim-main",
            "generate",
            Some(LockfilePath::new_in_project_root(&project_root).unwrap()),
        );

        let err = check_output_repo_safety(&repo, &expected).unwrap_err();

        assert!(
            err.to_string()
                .contains("lockfile output commit 'differentcommit' does not match output HEAD"),
            "unexpected error: {err:#}"
        );
        assert!(
            err.to_string().contains(&commit),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn check_output_repo_safety_rejects_resolved_only_lockfile_with_published_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let project_root = temp.path().join("project");
        let repo_path = temp.path().join("out");
        init_repo_on_branch(&repo_path, "kslim-main");
        commit_published_metadata(&repo_path, "kslim-main");
        write_resolved_only_lockfile(&project_root);
        let repo = OutputRepoPath::new(&repo_path).unwrap();
        let expected = output_plan_with_lockfile(
            &repo_path,
            "kslim-main",
            "generate",
            Some(LockfilePath::new_in_project_root(&project_root).unwrap()),
        );

        let err = check_output_repo_safety(&repo, &expected).unwrap_err();

        assert!(
            err.to_string().contains(
                "lockfile has no published snapshot to match committed published metadata"
            ),
            "unexpected error: {err:#}"
        );
    }
}
