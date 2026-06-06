use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::paths::LockfilePath;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    pub resolved_base: ResolvedBase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published: Option<PublishedLockfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedBase {
    pub upstream: String,
    pub url: String,
    pub r#ref: String,
    pub commit: String,
    pub resolved_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishedLockfile {
    pub output_branch: String,
    pub output_commit: String,
    pub tag: String,
    pub base_ref: String,
    pub base_commit: String,
    pub profile: String,
    pub mode: String,
    pub generated_at: String,
}

/// A lockfile update produced by resolving requested base state.
///
/// This update can refresh `resolved_base`, but it deliberately carries no
/// published snapshot. Candidate-tree state must not construct or write
/// authoritative lockfile contents directly.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedBaseLockfileUpdate {
    lockfile: Lockfile,
}

impl ResolvedBaseLockfileUpdate {
    pub(crate) fn new(resolved_base: ResolvedBase) -> Self {
        Self {
            lockfile: Lockfile {
                resolved_base,
                published: None,
            },
        }
    }

    fn as_lockfile(&self) -> &Lockfile {
        &self.lockfile
    }
}

/// A lockfile update produced from committed published snapshot state.
///
/// The only authoritative writer for successful generation should build this
/// after output metadata has been committed and verified. Candidate state has
/// no API here.
#[derive(Debug, Clone)]
pub(crate) struct PublishedLockfileUpdate {
    lockfile: Lockfile,
}

impl PublishedLockfileUpdate {
    pub(crate) fn new(resolved_base: ResolvedBase, published: PublishedLockfile) -> Result<Self> {
        validate_published_lockfile(&published)?;
        Ok(Self {
            lockfile: Lockfile {
                resolved_base,
                published: Some(published),
            },
        })
    }

    fn as_lockfile(&self) -> &Lockfile {
        &self.lockfile
    }
}

fn validate_published_lockfile(published: &PublishedLockfile) -> Result<()> {
    if published.output_branch.trim().is_empty() {
        anyhow::bail!("published lockfile output branch is empty");
    }
    if published.output_commit.trim().is_empty() {
        anyhow::bail!("published lockfile output commit is empty");
    }
    if published.tag.trim().is_empty() {
        anyhow::bail!("published lockfile tag is empty");
    }
    if published.base_ref.trim().is_empty() {
        anyhow::bail!("published lockfile base ref is empty");
    }
    if published.base_commit.trim().is_empty() {
        anyhow::bail!("published lockfile base commit is empty");
    }
    if published.profile.trim().is_empty() {
        anyhow::bail!("published lockfile profile is empty");
    }
    if published.mode.trim().is_empty() {
        anyhow::bail!("published lockfile mode is empty");
    }
    if published.generated_at.trim().is_empty() {
        anyhow::bail!("published lockfile generated timestamp is empty");
    }
    Ok(())
}

pub(crate) fn load_lockfile(path: &LockfilePath) -> Result<Option<Lockfile>> {
    let path = path.as_path();
    if !path.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(path)?;
    let lock: Lockfile = toml::from_str(&contents)?;
    Ok(Some(lock))
}

pub(crate) fn load_resolved_base_for_request(
    path: &LockfilePath,
    upstream: &str,
    url: &str,
    ref_name: &str,
) -> Result<ResolvedBase> {
    let lock = load_lockfile(path)?.ok_or_else(|| {
        anyhow::anyhow!(
            "--offline requires {} with resolved_base; run `kslim base resolve` before going offline",
            path.as_path().display()
        )
    })?;
    let resolved = lock.resolved_base;
    if resolved.upstream != upstream || resolved.url != url || resolved.r#ref != ref_name {
        anyhow::bail!(
            "--offline lockfile resolved_base does not match requested upstream/ref; expected {} {} {}, got {} {} {}",
            upstream,
            url,
            ref_name,
            resolved.upstream,
            resolved.url,
            resolved.r#ref
        );
    }
    Ok(resolved)
}

pub(crate) fn write_resolved_base_lockfile(
    path: &LockfilePath,
    update: &ResolvedBaseLockfileUpdate,
) -> Result<()> {
    write_lockfile_contents(path, update.as_lockfile())
}

pub(crate) fn write_published_lockfile(
    path: &LockfilePath,
    update: &PublishedLockfileUpdate,
) -> Result<()> {
    write_lockfile_contents(path, update.as_lockfile())
}

#[derive(Debug, Clone)]
pub(crate) struct LockfileFailureAtomicState {
    path: LockfilePath,
    original: LockfilePathSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LockfilePathSnapshot {
    Missing,
    File(Vec<u8>),
    Directory,
    Symlink(PathBuf),
}

pub(crate) fn capture_lockfile_failure_atomic_state(
    path: &LockfilePath,
) -> Result<LockfileFailureAtomicState> {
    let original = match std::fs::symlink_metadata(path.as_path()) {
        Ok(metadata) if metadata.file_type().is_file() => {
            LockfilePathSnapshot::File(std::fs::read(path.as_path())?)
        }
        Ok(metadata) if metadata.file_type().is_dir() => LockfilePathSnapshot::Directory,
        Ok(metadata) if metadata.file_type().is_symlink() => {
            LockfilePathSnapshot::Symlink(std::fs::read_link(path.as_path())?)
        }
        Ok(_) => anyhow::bail!(
            "kslim.lock path has unsupported file type before lockfile update: {}",
            path.as_path().display()
        ),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => LockfilePathSnapshot::Missing,
        Err(err) => return Err(err.into()),
    };
    Ok(LockfileFailureAtomicState {
        path: path.clone(),
        original,
    })
}

pub(crate) fn rollback_lockfile_failure_atomic_state(
    state: &LockfileFailureAtomicState,
) -> Result<()> {
    if ensure_failed_run_lockfile_unmodified(state).is_ok() {
        return Ok(());
    }

    match &state.original {
        LockfilePathSnapshot::Missing => remove_lockfile_path_if_exists(state.path.as_path())?,
        LockfilePathSnapshot::File(contents) => {
            remove_lockfile_path_if_non_file(state.path.as_path())?;
            std::fs::write(state.path.as_path(), contents)?;
        }
        LockfilePathSnapshot::Directory => {
            if !state.path.as_path().is_dir() {
                remove_lockfile_path_if_exists(state.path.as_path())?;
                std::fs::create_dir(state.path.as_path())?;
            }
        }
        LockfilePathSnapshot::Symlink(target) => {
            remove_lockfile_path_if_exists(state.path.as_path())?;
            restore_lockfile_symlink(target, state.path.as_path())?;
        }
    }

    ensure_failed_run_lockfile_unmodified(state)
}

pub(crate) fn ensure_failed_run_lockfile_unmodified(
    state: &LockfileFailureAtomicState,
) -> Result<()> {
    match &state.original {
        LockfilePathSnapshot::Missing => match std::fs::symlink_metadata(state.path.as_path()) {
            Ok(_) => anyhow::bail!(
                "failed run updated kslim.lock: lockfile path now exists at {}",
                state.path.as_path().display()
            ),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.into()),
        },
        LockfilePathSnapshot::File(expected) => {
            let metadata = std::fs::symlink_metadata(state.path.as_path())?;
            if !metadata.file_type().is_file() {
                anyhow::bail!(
                    "failed run updated kslim.lock: lockfile type changed at {}",
                    state.path.as_path().display()
                );
            }
            let actual = std::fs::read(state.path.as_path())?;
            if &actual != expected {
                anyhow::bail!(
                    "failed run updated kslim.lock: contents changed at {}",
                    state.path.as_path().display()
                );
            }
            Ok(())
        }
        LockfilePathSnapshot::Directory => {
            let metadata = std::fs::symlink_metadata(state.path.as_path())?;
            if !metadata.file_type().is_dir() {
                anyhow::bail!(
                    "failed run updated kslim.lock: directory path changed at {}",
                    state.path.as_path().display()
                );
            }
            Ok(())
        }
        LockfilePathSnapshot::Symlink(expected) => {
            let metadata = std::fs::symlink_metadata(state.path.as_path())?;
            if !metadata.file_type().is_symlink() {
                anyhow::bail!(
                    "failed run updated kslim.lock: symlink path changed at {}",
                    state.path.as_path().display()
                );
            }
            let actual = std::fs::read_link(state.path.as_path())?;
            if &actual != expected {
                anyhow::bail!(
                    "failed run updated kslim.lock: symlink target changed at {}",
                    state.path.as_path().display()
                );
            }
            Ok(())
        }
    }
}

fn write_lockfile_contents(path: &LockfilePath, lock: &Lockfile) -> Result<()> {
    let rollback = capture_lockfile_failure_atomic_state(path)?;
    let contents = toml::to_string_pretty(lock)?;
    match std::fs::write(path.as_path(), contents) {
        Ok(()) => Ok(()),
        Err(err) => {
            if let Err(rollback_err) = rollback_lockfile_failure_atomic_state(&rollback) {
                anyhow::bail!(
                    "lockfile write failed: {:#}; rollback also failed: {:#}",
                    err,
                    rollback_err
                );
            }
            Err(err.into())
        }
    }
}

fn remove_lockfile_path_if_exists(path: &Path) -> Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err.into()),
    };
    if metadata.file_type().is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn remove_lockfile_path_if_non_file(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(()),
        Ok(_) => remove_lockfile_path_if_exists(path),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

#[cfg(unix)]
fn restore_lockfile_symlink(target: &Path, path: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, path)?;
    Ok(())
}

#[cfg(not(unix))]
fn restore_lockfile_symlink(_target: &Path, path: &Path) -> Result<()> {
    anyhow::bail!(
        "failed run updated kslim.lock: symlink restore is unsupported at {}",
        path.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_resolved_base() -> ResolvedBase {
        ResolvedBase {
            upstream: String::from("linux"),
            url: String::from("/tmp/linux.git"),
            r#ref: String::from("v1.0"),
            commit: String::from("deadbeef"),
            resolved_at: String::from("2026-01-01T00:00:00Z"),
        }
    }

    fn test_published_lockfile() -> PublishedLockfile {
        PublishedLockfile {
            output_branch: String::from("kslim/v1.0/default"),
            output_commit: String::from("cafebabe"),
            tag: String::from("kslim-v1.0-default-r1"),
            base_ref: String::from("v1.0"),
            base_commit: String::from("deadbeef"),
            profile: String::from("default"),
            mode: String::from("unmodified-upstream"),
            generated_at: String::from("2026-01-01T00:00:00Z"),
        }
    }

    #[test]
    fn test_resolved_base_lockfile_update_cannot_publish_candidate_state() {
        let tmp = tempfile::tempdir().unwrap();
        let path = LockfilePath::new_in_project_root(tmp.path()).unwrap();
        let update = ResolvedBaseLockfileUpdate::new(test_resolved_base());

        write_resolved_base_lockfile(&path, &update).unwrap();

        let lock = load_lockfile(&path).unwrap().unwrap();
        assert_eq!(lock.resolved_base.commit, "deadbeef");
        assert!(lock.published.is_none());
    }

    #[test]
    fn test_published_lockfile_update_writes_authoritative_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let path = LockfilePath::new_in_project_root(tmp.path()).unwrap();
        let update =
            PublishedLockfileUpdate::new(test_resolved_base(), test_published_lockfile()).unwrap();

        write_published_lockfile(&path, &update).unwrap();

        let lock = load_lockfile(&path).unwrap().unwrap();
        let published = lock.published.unwrap();
        assert_eq!(published.output_commit, "cafebabe");
        assert_eq!(published.base_commit, lock.resolved_base.commit);
    }

    #[test]
    fn test_published_lockfile_update_rejects_incomplete_snapshot() {
        let mut published = test_published_lockfile();
        published.output_commit = String::from(" ");

        let err = PublishedLockfileUpdate::new(test_resolved_base(), published)
            .unwrap_err()
            .to_string();

        assert!(err.contains("published lockfile output commit is empty"));
    }

    #[test]
    fn test_failed_run_lockfile_rollback_restores_existing_contents() {
        let tmp = tempfile::tempdir().unwrap();
        let lockfile = tmp.path().join("kslim.lock");
        std::fs::write(&lockfile, "authoritative lockfile\n").unwrap();
        let path = LockfilePath::new_in_project_root(tmp.path()).unwrap();
        let rollback = capture_lockfile_failure_atomic_state(&path).unwrap();

        std::fs::write(&lockfile, "failed candidate lockfile\n").unwrap();
        let err = ensure_failed_run_lockfile_unmodified(&rollback)
            .unwrap_err()
            .to_string();

        assert!(err.contains("failed run updated kslim.lock"));
        rollback_lockfile_failure_atomic_state(&rollback).unwrap();
        assert_eq!(
            std::fs::read_to_string(&lockfile).unwrap(),
            "authoritative lockfile\n"
        );
    }

    #[test]
    fn test_failed_run_lockfile_rollback_removes_created_lockfile() {
        let tmp = tempfile::tempdir().unwrap();
        let lockfile = tmp.path().join("kslim.lock");
        let path = LockfilePath::new_in_project_root(tmp.path()).unwrap();
        let rollback = capture_lockfile_failure_atomic_state(&path).unwrap();

        std::fs::write(&lockfile, "failed candidate lockfile\n").unwrap();

        rollback_lockfile_failure_atomic_state(&rollback).unwrap();
        assert!(!lockfile.exists());
    }
}
