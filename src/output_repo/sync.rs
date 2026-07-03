//! File copying and directory synchronization for output repositories.
//!
//! This module owns only filesystem sync mechanics. It must not validate
//! candidate metadata schemas, resolve upstream policy, publish refs, or render
//! reports.

use anyhow::Result;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs as unix_fs;

use crate::paths::{CandidateTreePath, OutputCandidateArea, OutputRepoPath};

use super::metadata;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct SyncPolicy {
    pub(crate) preserve_top_level_git_dir: bool,
    pub(crate) preserve_top_level_metadata_dir: bool,
    pub(crate) preserve_published_snapshot_metadata: bool,
    pub(crate) delete_absent_paths: bool,
}

#[allow(dead_code)]
impl SyncPolicy {
    pub(crate) fn replace_output_tree() -> Self {
        Self {
            preserve_top_level_git_dir: true,
            preserve_top_level_metadata_dir: true,
            preserve_published_snapshot_metadata: false,
            delete_absent_paths: true,
        }
    }

    pub(crate) fn replace_all() -> Self {
        Self {
            preserve_top_level_git_dir: false,
            preserve_top_level_metadata_dir: false,
            preserve_published_snapshot_metadata: false,
            delete_absent_paths: true,
        }
    }

    pub(crate) fn replace_candidate_metadata() -> Self {
        Self {
            preserve_top_level_git_dir: false,
            preserve_top_level_metadata_dir: false,
            preserve_published_snapshot_metadata: true,
            delete_absent_paths: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct SyncSummary {
    pub(crate) files_copied: usize,
    pub(crate) files_removed: usize,
    pub(crate) directories_created: usize,
    pub(crate) symlinks_copied: usize,
    pub(crate) special_files_rejected: usize,
}

#[derive(Debug, Clone)]
struct SyncBoundary {
    source_root: PathBuf,
    output_root: PathBuf,
    destination_root: PathBuf,
}

impl SyncBoundary {
    fn new(source_root: PathBuf, output_root: PathBuf, destination_root: PathBuf) -> Result<Self> {
        let source_root = normalize_sync_path("sync source root", &source_root)?;
        let output_root = normalize_sync_path("sync output root", &output_root)?;
        let destination_root = normalize_sync_path("sync destination root", &destination_root)?;
        ensure_path_inside_root("sync destination root", &output_root, &destination_root)?;
        ensure_roots_do_not_overlap(&source_root, &destination_root)?;
        Ok(Self {
            source_root,
            output_root,
            destination_root,
        })
    }

    fn ensure_source(&self, path: &Path) -> Result<()> {
        ensure_path_inside_root("sync source path", &self.source_root, path)
    }

    fn ensure_destination(&self, path: &Path) -> Result<()> {
        ensure_path_inside_root("sync destination path", &self.destination_root, path)?;
        ensure_path_inside_root("sync output path", &self.output_root, path)
    }
}

#[allow(dead_code)]
pub(crate) fn sync_candidate_to_output_area(
    candidate: &CandidateTreePath,
    output_path: &OutputRepoPath,
    policy: &SyncPolicy,
) -> Result<SyncSummary> {
    let output_area = OutputCandidateArea::from_output_repo(output_path)?;
    let candidate_root = normalize_existing_dir("candidate tree", candidate.as_path())?;
    let requested_output_root =
        normalize_sync_path("output candidate area", output_area.as_path())?;
    ensure_roots_do_not_overlap(&candidate_root, &requested_output_root)?;
    let mut summary = SyncSummary::default();
    ensure_sync_root_dir(output_area.as_path(), &mut summary)?;
    let output_root = normalize_existing_dir("output candidate area", output_area.as_path())?;
    let boundary = SyncBoundary::new(
        candidate_root.clone(),
        output_root.clone(),
        output_root.clone(),
    )?;

    sync_dir_contents(
        &boundary,
        &candidate_root,
        &output_root,
        true,
        policy,
        &mut summary,
    )?;
    Ok(summary)
}

/// Incrementally sync working tree contents in the output repo.
/// Keeps reserved top-level directories and only rewrites paths that differ.
pub(crate) fn sync_working_tree(
    output_path: &OutputRepoPath,
    temp_tree_path: &CandidateTreePath,
) -> Result<()> {
    let output_area = OutputCandidateArea::from_output_repo(output_path)?;
    let policy = SyncPolicy::replace_output_tree();

    let mut summary = sync_candidate_to_output_area(temp_tree_path, output_path, &policy)?;
    let output_root = normalize_existing_dir("output candidate area", output_area.as_path())?;
    let output_repo = OutputRepoPath::new(output_root.clone())?;
    let kslim_dir = metadata::published_metadata_dir(&output_repo)?
        .as_path()
        .to_path_buf();
    ensure_dir_counted_inside_root(&output_root, &kslim_dir, &mut summary)?;

    Ok(())
}

pub(crate) fn sync_candidate_metadata_dir(
    output_path: &OutputRepoPath,
    candidate_root: &CandidateTreePath,
) -> Result<()> {
    let candidate_metadata = metadata::candidate_metadata_dir(candidate_root)?
        .as_path()
        .to_path_buf();
    let candidate_metadata = normalize_existing_dir("candidate metadata dir", &candidate_metadata)?;
    let output_root = normalize_existing_dir("output metadata target root", output_path.as_path())?;
    let output_repo = OutputRepoPath::new(output_root.clone())?;
    let output_metadata = metadata::published_metadata_dir(&output_repo)?
        .as_path()
        .to_path_buf();
    let boundary = SyncBoundary::new(
        candidate_metadata.clone(),
        output_root,
        output_metadata.clone(),
    )?;
    let mut summary = SyncSummary::default();
    sync_dir_contents(
        &boundary,
        &candidate_metadata,
        &output_metadata,
        false,
        &SyncPolicy::replace_candidate_metadata(),
        &mut summary,
    )?;
    Ok(())
}

pub(crate) fn sync_candidate_committed_metadata_dir(
    output_path: &OutputRepoPath,
    candidate_root: &CandidateTreePath,
) -> Result<()> {
    let candidate_metadata = metadata::candidate_metadata_dir(candidate_root)?
        .as_path()
        .to_path_buf();
    let candidate_metadata = normalize_existing_dir("candidate metadata dir", &candidate_metadata)?;
    let output_root = normalize_existing_dir("output metadata target root", output_path.as_path())?;
    let output_metadata = output_root.join(metadata::COMMITTED_METADATA_DIR);
    let boundary = SyncBoundary::new(
        candidate_metadata.clone(),
        output_root,
        output_metadata.clone(),
    )?;
    let mut summary = SyncSummary::default();
    ensure_dir_counted_inside_root(&boundary.output_root, &output_metadata, &mut summary)?;

    let mut expected = BTreeSet::<OsString>::new();
    for file_name in metadata::committed_candidate_metadata_file_names() {
        let name = OsString::from(file_name);
        let source = candidate_metadata.join(&name);
        match std::fs::symlink_metadata(&source) {
            Ok(_) => {
                expected.insert(name.clone());
                sync_path(
                    &boundary,
                    &source,
                    &output_metadata.join(&name),
                    &SyncPolicy::replace_candidate_metadata(),
                    &mut summary,
                )?;
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }
    }

    for entry in std::fs::read_dir(&output_metadata)? {
        let entry = entry?;
        let name = normalize_entry_name("output committed metadata entry", &entry.file_name(), &output_metadata)?;
        if is_reserved_sync_entry(&name, false, &SyncPolicy::replace_candidate_metadata()) {
            continue;
        }
        if !expected.contains(&name) {
            remove_path_inside_root(&boundary, &entry.path(), &mut summary)?;
        }
    }
    Ok(())
}

fn sync_dir_contents(
    boundary: &SyncBoundary,
    src: &Path,
    dst: &Path,
    top_level: bool,
    policy: &SyncPolicy,
    summary: &mut SyncSummary,
) -> Result<()> {
    boundary.ensure_destination(dst)?;
    boundary.ensure_source(src)?;
    ensure_dir_counted_inside_root(&boundary.output_root, dst, summary)?;

    let mut expected = BTreeSet::<OsString>::new();
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = normalize_entry_name("candidate sync entry", &entry.file_name(), src)?;
        if is_reserved_sync_entry(&name, top_level, policy) {
            continue;
        }

        expected.insert(name.clone());
        sync_path(boundary, &entry.path(), &dst.join(&name), policy, summary)?;
    }

    if policy.delete_absent_paths {
        for entry in std::fs::read_dir(dst)? {
            let entry = entry?;
            let name = normalize_entry_name("output sync entry", &entry.file_name(), dst)?;
            if is_reserved_sync_entry(&name, top_level, policy) {
                continue;
            }
            if !expected.contains(&name) {
                remove_path_inside_root(boundary, &entry.path(), summary)?;
            }
        }
    }

    Ok(())
}

fn is_reserved_sync_entry(name: &OsStr, top_level: bool, policy: &SyncPolicy) -> bool {
    is_reserved_top_level_entry(name, top_level, policy)
        || (policy.preserve_published_snapshot_metadata
            && metadata::is_published_snapshot_metadata_file(name))
}

fn is_reserved_top_level_entry(name: &OsStr, top_level: bool, policy: &SyncPolicy) -> bool {
    top_level
        && ((policy.preserve_top_level_git_dir && name == OsStr::new(".git"))
            || (policy.preserve_top_level_metadata_dir
                && name == OsStr::new(metadata::COMMITTED_METADATA_DIR)))
}

fn sync_path(
    boundary: &SyncBoundary,
    src: &Path,
    dst: &Path,
    policy: &SyncPolicy,
    summary: &mut SyncSummary,
) -> Result<()> {
    boundary.ensure_source(src)?;
    boundary.ensure_destination(dst)?;
    let meta = std::fs::symlink_metadata(src)?;
    let file_type = meta.file_type();

    if file_type.is_dir() {
        if let Ok(dst_meta) = std::fs::symlink_metadata(dst) {
            if !dst_meta.file_type().is_dir() {
                remove_path_inside_root(boundary, dst, summary)?;
            }
        }
        ensure_dir_counted_inside_root(&boundary.output_root, dst, summary)?;
        sync_dir_contents(boundary, src, dst, false, policy, summary)?;
        return Ok(());
    }

    if file_type.is_file() {
        return sync_file(boundary, src, dst, summary);
    }

    if file_type.is_symlink() {
        return sync_symlink(boundary, src, dst, summary);
    }

    summary.special_files_rejected += 1;
    anyhow::bail!("unsupported file type in snapshot tree: {}", src.display())
}

fn sync_file(
    boundary: &SyncBoundary,
    src: &Path,
    dst: &Path,
    summary: &mut SyncSummary,
) -> Result<()> {
    if !regular_file_differs(src, dst)? {
        return Ok(());
    }

    if let Some(parent) = dst.parent() {
        boundary.ensure_destination(parent)?;
        ensure_dir_counted_inside_root(&boundary.output_root, parent, summary)?;
    }
    remove_path_inside_root(boundary, dst, summary)?;
    std::fs::copy(src, dst)?;
    summary.files_copied += 1;
    Ok(())
}

fn regular_file_differs(src: &Path, dst: &Path) -> Result<bool> {
    let src_meta = std::fs::metadata(src)?;
    let dst_meta = match std::fs::symlink_metadata(dst) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(err) => return Err(err.into()),
    };

    if !dst_meta.file_type().is_file() {
        return Ok(true);
    }
    if src_meta.len() != dst_meta.len() {
        return Ok(true);
    }

    Ok(!files_equal(src, dst)?)
}

fn files_equal(a: &Path, b: &Path) -> Result<bool> {
    let mut left = std::fs::File::open(a)?;
    let mut right = std::fs::File::open(b)?;
    let mut left_buf = [0_u8; 8192];
    let mut right_buf = [0_u8; 8192];

    loop {
        let left_n = left.read(&mut left_buf)?;
        let right_n = right.read(&mut right_buf)?;

        if left_n != right_n {
            return Ok(false);
        }
        if left_n == 0 {
            return Ok(true);
        }
        if left_buf[..left_n] != right_buf[..right_n] {
            return Ok(false);
        }
    }
}

#[cfg(unix)]
fn sync_symlink(
    boundary: &SyncBoundary,
    src: &Path,
    dst: &Path,
    summary: &mut SyncSummary,
) -> Result<()> {
    let target = std::fs::read_link(src)?;
    ensure_safe_relative_symlink_target(&boundary.source_root, src, &target)?;

    match std::fs::symlink_metadata(dst) {
        Ok(meta) if meta.file_type().is_symlink() => {
            if std::fs::read_link(dst)? == target {
                return Ok(());
            }
        }
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err.into()),
    }

    if let Some(parent) = dst.parent() {
        boundary.ensure_destination(parent)?;
        ensure_dir_counted_inside_root(&boundary.output_root, parent, summary)?;
    }
    remove_path_inside_root(boundary, dst, summary)?;
    unix_fs::symlink(&target, dst)?;
    summary.symlinks_copied += 1;
    Ok(())
}

#[cfg(not(unix))]
fn sync_symlink(
    _boundary: &SyncBoundary,
    _src: &Path,
    _dst: &Path,
    summary: &mut SyncSummary,
) -> Result<()> {
    summary.special_files_rejected += 1;
    anyhow::bail!("symlink snapshots are only supported on unix hosts")
}

#[cfg(unix)]
fn ensure_safe_relative_symlink_target(
    candidate_root: &Path,
    src: &Path,
    target: &Path,
) -> Result<()> {
    if target.as_os_str().is_empty() {
        anyhow::bail!("unsafe symlink target is empty: {}", src.display());
    }
    if target.is_absolute() {
        anyhow::bail!(
            "unsafe symlink target escapes candidate tree: {} -> {}",
            src.display(),
            target.display()
        );
    }

    let parent = src
        .parent()
        .ok_or_else(|| anyhow::anyhow!("symlink source path has no parent: {}", src.display()))?;
    let parent_relative = parent.strip_prefix(candidate_root).map_err(|_| {
        anyhow::anyhow!(
            "symlink source is outside candidate tree: {}",
            src.display()
        )
    })?;
    let mut depth = parent_relative
        .components()
        .filter(|component| matches!(component, Component::Normal(_)))
        .count();

    for component in target.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(_) => depth += 1,
            Component::ParentDir => {
                if depth == 0 {
                    anyhow::bail!(
                        "unsafe symlink target escapes candidate tree: {} -> {}",
                        src.display(),
                        target.display()
                    );
                }
                depth -= 1;
            }
            Component::RootDir | Component::Prefix(_) => {
                anyhow::bail!(
                    "unsafe symlink target escapes candidate tree: {} -> {}",
                    src.display(),
                    target.display()
                );
            }
        }
    }

    Ok(())
}

fn ensure_sync_root_dir(path: &Path, summary: &mut SyncSummary) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            let file_type = meta.file_type();
            if file_type.is_symlink() {
                anyhow::bail!(
                    "refusing to follow symlinked sync directory: {}",
                    path.display()
                );
            }
            if !file_type.is_dir() {
                anyhow::bail!("sync path is not a directory: {}", path.display());
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir_all(path)?;
            summary.directories_created += 1;
        }
        Err(err) => return Err(err.into()),
    }
    Ok(())
}

fn ensure_dir_counted_inside_root(
    output_root: &Path,
    path: &Path,
    summary: &mut SyncSummary,
) -> Result<()> {
    ensure_path_inside_root("sync directory", output_root, path)?;

    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            let file_type = meta.file_type();
            if file_type.is_symlink() {
                anyhow::bail!(
                    "refusing to follow symlinked sync directory: {}",
                    path.display()
                );
            }
            if !file_type.is_dir() {
                anyhow::bail!("sync path is not a directory: {}", path.display());
            }
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            create_dir_inside_root(output_root, path)?;
            summary.directories_created += 1;
            Ok(())
        }
        Err(err) => Err(err.into()),
    }
}

fn create_dir_inside_root(output_root: &Path, path: &Path) -> Result<()> {
    ensure_path_inside_root("sync directory", output_root, path)?;
    let output_root = normalize_sync_path("sync output root", output_root)?;
    let path = normalize_sync_path("sync directory", path)?;

    let root_meta = std::fs::symlink_metadata(&output_root)?;
    let root_file_type = root_meta.file_type();
    if root_file_type.is_symlink() {
        anyhow::bail!(
            "refusing to follow symlinked sync output root: {}",
            output_root.display()
        );
    }
    if !root_file_type.is_dir() {
        anyhow::bail!(
            "sync output root is not a directory: {}",
            output_root.display()
        );
    }

    let relative = path.strip_prefix(&output_root).map_err(|_| {
        anyhow::anyhow!(
            "sync root escape rejected: sync directory {} is outside {}",
            path.display(),
            output_root.display()
        )
    })?;
    let mut current = output_root;
    for component in relative.components() {
        let part = match component {
            Component::CurDir => continue,
            Component::Normal(part) => part,
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                anyhow::bail!(
                    "sync root escape rejected: sync directory contains invalid component: {}",
                    path.display()
                );
            }
        };
        current.push(part);

        match std::fs::symlink_metadata(&current) {
            Ok(meta) => {
                let file_type = meta.file_type();
                if file_type.is_symlink() {
                    anyhow::bail!(
                        "refusing to follow symlinked sync directory: {}",
                        current.display()
                    );
                }
                if !file_type.is_dir() {
                    anyhow::bail!("sync path is not a directory: {}", current.display());
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&current)?;
            }
            Err(err) => return Err(err.into()),
        }
    }

    Ok(())
}

fn remove_path_inside_root(
    boundary: &SyncBoundary,
    path: &Path,
    summary: &mut SyncSummary,
) -> Result<()> {
    boundary.ensure_destination(path)?;
    remove_path(path, summary)
}

fn remove_path(path: &Path, summary: &mut SyncSummary) -> Result<()> {
    let meta = match std::fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err.into()),
    };

    if meta.file_type().is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }

    summary.files_removed += 1;
    Ok(())
}

fn normalize_existing_dir(label: &str, path: &Path) -> Result<PathBuf> {
    let path = normalize_sync_path(label, path)?;
    let meta = match std::fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            anyhow::bail!(
                "{label} path is not an existing directory: {}",
                path.display()
            );
        }
        Err(err) => return Err(err.into()),
    };
    let file_type = meta.file_type();
    if file_type.is_symlink() {
        anyhow::bail!("{label} path must not be a symlink: {}", path.display());
    }
    if !file_type.is_dir() {
        anyhow::bail!(
            "{label} path is not an existing directory: {}",
            path.display()
        );
    }
    Ok(path.canonicalize()?)
}

fn normalize_sync_path(label: &str, path: &Path) -> Result<PathBuf> {
    if path.as_os_str().is_empty() {
        anyhow::bail!("{label} is empty");
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                normalized.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                anyhow::bail!(
                    "{label} must not contain parent components: {}",
                    path.display()
                );
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        Ok(PathBuf::from("."))
    } else {
        Ok(normalized)
    }
}

fn ensure_path_inside_root(label: &str, root: &Path, path: &Path) -> Result<()> {
    let root = normalize_sync_path("sync root", root)?;
    let path = normalize_sync_path(label, path)?;
    if path.starts_with(&root) {
        return Ok(());
    }

    anyhow::bail!(
        "sync root escape rejected: {label} {} is outside {}",
        path.display(),
        root.display()
    )
}

fn ensure_roots_do_not_overlap(source_root: &Path, destination_root: &Path) -> Result<()> {
    if source_root == destination_root
        || source_root.starts_with(destination_root)
        || destination_root.starts_with(source_root)
    {
        anyhow::bail!(
            "sync root escape rejected: source root {} and destination root {} must not overlap",
            source_root.display(),
            destination_root.display()
        );
    }
    Ok(())
}

fn normalize_entry_name(label: &str, name: &OsStr, parent: &Path) -> Result<OsString> {
    let path = Path::new(name);
    let mut components = path.components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(name.to_os_string()),
        _ => {
            anyhow::bail!(
                "{label} is not a normalized child path: {}",
                parent.join(path).display()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate_tree_path(path: &Path) -> CandidateTreePath {
        CandidateTreePath::new(path).unwrap()
    }

    #[test]
    fn sync_candidate_to_output_area_replaces_payload_and_reports_summary() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("candidate");
        let output = temp.path().join("output");
        std::fs::create_dir_all(candidate.join("drivers")).unwrap();
        std::fs::create_dir_all(output.join(".git")).unwrap();
        std::fs::create_dir_all(output.join(".kslim")).unwrap();
        std::fs::write(candidate.join("Makefile"), "new makefile\n").unwrap();
        std::fs::write(candidate.join("drivers/new.txt"), "new driver\n").unwrap();
        std::fs::write(output.join(".git/config"), "git config\n").unwrap();
        std::fs::write(output.join(".kslim/local.txt"), "local metadata\n").unwrap();
        std::fs::write(output.join("stale.txt"), "stale\n").unwrap();
        let output_area = OutputRepoPath::new(&output).unwrap();

        let summary = sync_candidate_to_output_area(
            &candidate_tree_path(&candidate),
            &output_area,
            &SyncPolicy::replace_output_tree(),
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(output.join("Makefile")).unwrap(),
            "new makefile\n"
        );
        assert_eq!(
            std::fs::read_to_string(output.join("drivers/new.txt")).unwrap(),
            "new driver\n"
        );
        assert_eq!(
            std::fs::read_to_string(output.join(".git/config")).unwrap(),
            "git config\n"
        );
        assert_eq!(
            std::fs::read_to_string(output.join(".kslim/local.txt")).unwrap(),
            "local metadata\n"
        );
        assert!(!output.join("stale.txt").exists());
        assert_eq!(summary.files_copied, 2);
        assert_eq!(summary.files_removed, 1);
        assert_eq!(summary.directories_created, 1);
        assert_eq!(summary.symlinks_copied, 0);
        assert_eq!(summary.special_files_rejected, 0);
    }

    #[test]
    fn sync_policy_preserves_top_level_git_directory() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("candidate");
        let output = temp.path().join("output");
        std::fs::create_dir_all(candidate.join(".git")).unwrap();
        std::fs::create_dir_all(output.join(".git")).unwrap();
        std::fs::write(candidate.join(".git/config"), "candidate git config\n").unwrap();
        std::fs::write(candidate.join("Makefile"), "new makefile\n").unwrap();
        std::fs::write(output.join(".git/config"), "output git config\n").unwrap();
        let output_area = OutputRepoPath::new(&output).unwrap();

        let summary = sync_candidate_to_output_area(
            &candidate_tree_path(&candidate),
            &output_area,
            &SyncPolicy::replace_output_tree(),
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(output.join(".git/config")).unwrap(),
            "output git config\n"
        );
        assert_eq!(
            std::fs::read_to_string(output.join("Makefile")).unwrap(),
            "new makefile\n"
        );
        assert_eq!(summary.files_copied, 1);
        assert_eq!(summary.files_removed, 0);
    }

    #[test]
    fn sync_policy_preserves_top_level_metadata_directory_for_metadata_module() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("candidate");
        let output = temp.path().join("output");
        std::fs::create_dir_all(candidate.join(metadata::COMMITTED_METADATA_DIR)).unwrap();
        std::fs::create_dir_all(output.join(metadata::COMMITTED_METADATA_DIR)).unwrap();
        std::fs::write(
            candidate
                .join(metadata::COMMITTED_METADATA_DIR)
                .join("candidate.txt"),
            "candidate metadata\n",
        )
        .unwrap();
        std::fs::write(
            output
                .join(metadata::COMMITTED_METADATA_DIR)
                .join("output.txt"),
            "output metadata\n",
        )
        .unwrap();
        std::fs::write(candidate.join("Makefile"), "new makefile\n").unwrap();
        let output_area = OutputRepoPath::new(&output).unwrap();

        let summary = sync_candidate_to_output_area(
            &candidate_tree_path(&candidate),
            &output_area,
            &SyncPolicy::replace_output_tree(),
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(
                output
                    .join(metadata::COMMITTED_METADATA_DIR)
                    .join("output.txt")
            )
            .unwrap(),
            "output metadata\n"
        );
        assert!(!output
            .join(metadata::COMMITTED_METADATA_DIR)
            .join("candidate.txt")
            .exists());
        assert_eq!(
            std::fs::read_to_string(output.join("Makefile")).unwrap(),
            "new makefile\n"
        );
        assert_eq!(summary.files_copied, 1);
        assert_eq!(summary.files_removed, 0);
    }

    #[test]
    fn sync_policy_deletes_paths_absent_from_replacement_candidate() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("candidate");
        let output = temp.path().join("output");
        std::fs::create_dir_all(candidate.join("drivers")).unwrap();
        std::fs::create_dir_all(output.join("drivers/stale-dir")).unwrap();
        std::fs::write(candidate.join("drivers/keep.txt"), "candidate\n").unwrap();
        std::fs::write(output.join("stale.txt"), "stale\n").unwrap();
        std::fs::write(output.join("drivers/stale-dir/old.txt"), "old\n").unwrap();
        let output_area = OutputRepoPath::new(&output).unwrap();

        let summary = sync_candidate_to_output_area(
            &candidate_tree_path(&candidate),
            &output_area,
            &SyncPolicy::replace_output_tree(),
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(output.join("drivers/keep.txt")).unwrap(),
            "candidate\n"
        );
        assert!(!output.join("stale.txt").exists());
        assert!(!output.join("drivers/stale-dir").exists());
        assert_eq!(summary.files_removed, 2);
    }

    #[test]
    fn sync_summary_counts_copied_files_only() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("candidate");
        let output = temp.path().join("output");
        std::fs::create_dir_all(&candidate).unwrap();
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(candidate.join("same.txt"), "same\n").unwrap();
        std::fs::write(candidate.join("changed.txt"), "new\n").unwrap();
        std::fs::write(candidate.join("new.txt"), "new file\n").unwrap();
        std::fs::write(output.join("same.txt"), "same\n").unwrap();
        std::fs::write(output.join("changed.txt"), "old\n").unwrap();
        let output_area = OutputRepoPath::new(&output).unwrap();

        let summary = sync_candidate_to_output_area(
            &candidate_tree_path(&candidate),
            &output_area,
            &SyncPolicy::replace_output_tree(),
        )
        .unwrap();

        assert_eq!(summary.files_copied, 2);
        assert_eq!(
            std::fs::read_to_string(output.join("same.txt")).unwrap(),
            "same\n"
        );
        assert_eq!(
            std::fs::read_to_string(output.join("changed.txt")).unwrap(),
            "new\n"
        );
        assert_eq!(
            std::fs::read_to_string(output.join("new.txt")).unwrap(),
            "new file\n"
        );
    }

    #[test]
    fn sync_summary_counts_removed_files() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("candidate");
        let output = temp.path().join("output");
        std::fs::create_dir_all(&candidate).unwrap();
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(candidate.join("keep.txt"), "keep\n").unwrap();
        std::fs::write(output.join("stale-one.txt"), "stale one\n").unwrap();
        std::fs::write(output.join("stale-two.txt"), "stale two\n").unwrap();
        let output_area = OutputRepoPath::new(&output).unwrap();

        let summary = sync_candidate_to_output_area(
            &candidate_tree_path(&candidate),
            &output_area,
            &SyncPolicy::replace_output_tree(),
        )
        .unwrap();

        assert_eq!(summary.files_removed, 2);
        assert!(!output.join("stale-one.txt").exists());
        assert!(!output.join("stale-two.txt").exists());
        assert_eq!(
            std::fs::read_to_string(output.join("keep.txt")).unwrap(),
            "keep\n"
        );
    }

    #[test]
    fn sync_summary_counts_created_directories() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("candidate");
        let output = temp.path().join("output");
        std::fs::create_dir_all(candidate.join("drivers/net")).unwrap();
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(candidate.join("drivers/net/new.txt"), "new\n").unwrap();
        let output_area = OutputRepoPath::new(&output).unwrap();

        let summary = sync_candidate_to_output_area(
            &candidate_tree_path(&candidate),
            &output_area,
            &SyncPolicy::replace_output_tree(),
        )
        .unwrap();

        assert_eq!(summary.directories_created, 2);
        assert!(output.join("drivers").is_dir());
        assert!(output.join("drivers/net").is_dir());
        assert_eq!(
            std::fs::read_to_string(output.join("drivers/net/new.txt")).unwrap(),
            "new\n"
        );
    }

    #[test]
    fn sync_policy_normalizes_root_paths_before_copying() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("candidate");
        let output = temp.path().join("output");
        std::fs::create_dir_all(candidate.join("drivers")).unwrap();
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(candidate.join("drivers/new.txt"), "new driver\n").unwrap();
        let candidate = CandidateTreePath::new(candidate.join(".")).unwrap();
        let output_area = OutputRepoPath::new(output.join(".")).unwrap();

        let summary = sync_candidate_to_output_area(
            &candidate,
            &output_area,
            &SyncPolicy::replace_output_tree(),
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(output.join("drivers/new.txt")).unwrap(),
            "new driver\n"
        );
        assert_eq!(summary.files_copied, 1);
    }

    #[test]
    fn sync_policy_rejects_output_area_inside_candidate_tree() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("candidate");
        let output = candidate.join("output");
        std::fs::create_dir_all(&candidate).unwrap();
        std::fs::write(candidate.join("Makefile"), "makefile\n").unwrap();
        let output_area = OutputRepoPath::new(&output).unwrap();

        let err = sync_candidate_to_output_area(
            &candidate_tree_path(&candidate),
            &output_area,
            &SyncPolicy::replace_output_tree(),
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("sync root escape rejected"));
        assert!(!output.exists());
    }

    #[test]
    fn sync_policy_rejects_candidate_tree_inside_output_area() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("output");
        let candidate = output.join("candidate");
        std::fs::create_dir_all(&candidate).unwrap();
        std::fs::write(candidate.join("Makefile"), "makefile\n").unwrap();
        std::fs::write(output.join("stale.txt"), "stale\n").unwrap();
        let output_area = OutputRepoPath::new(&output).unwrap();

        let err = sync_candidate_to_output_area(
            &candidate_tree_path(&candidate),
            &output_area,
            &SyncPolicy::replace_output_tree(),
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("sync root escape rejected"));
        assert_eq!(
            std::fs::read_to_string(candidate.join("Makefile")).unwrap(),
            "makefile\n"
        );
        assert_eq!(
            std::fs::read_to_string(output.join("stale.txt")).unwrap(),
            "stale\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn sync_policy_replaces_destination_directory_symlink_without_following_it() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("candidate");
        let output = temp.path().join("output");
        let outside = temp.path().join("outside");
        std::fs::create_dir_all(candidate.join("drivers")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(candidate.join("drivers/new.txt"), "new driver\n").unwrap();
        std::fs::write(outside.join("keep.txt"), "outside\n").unwrap();
        std::os::unix::fs::symlink(&outside, output.join("drivers")).unwrap();
        let output_area = OutputRepoPath::new(&output).unwrap();

        let summary = sync_candidate_to_output_area(
            &candidate_tree_path(&candidate),
            &output_area,
            &SyncPolicy::replace_output_tree(),
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(output.join("drivers/new.txt")).unwrap(),
            "new driver\n"
        );
        assert_eq!(
            std::fs::read_to_string(outside.join("keep.txt")).unwrap(),
            "outside\n"
        );
        assert!(!outside.join("new.txt").exists());
        assert_eq!(summary.files_removed, 1);
    }

    #[cfg(unix)]
    #[test]
    fn sync_policy_rejects_candidate_symlink_that_escapes_root() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("candidate");
        let output = temp.path().join("output");
        let outside = temp.path().join("outside.txt");
        std::fs::create_dir_all(&candidate).unwrap();
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(&outside, "outside\n").unwrap();
        std::os::unix::fs::symlink("../outside.txt", candidate.join("escape")).unwrap();
        let output_area = OutputRepoPath::new(&output).unwrap();

        let err = sync_candidate_to_output_area(
            &candidate_tree_path(&candidate),
            &output_area,
            &SyncPolicy::replace_output_tree(),
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("unsafe symlink target escapes candidate tree"));
        assert!(!output.join("escape").exists());
        assert_eq!(std::fs::read_to_string(outside).unwrap(), "outside\n");
    }

    #[cfg(unix)]
    #[test]
    fn sync_summary_counts_copied_symlinks_only() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("candidate");
        let output = temp.path().join("output");
        std::fs::create_dir_all(candidate.join("include")).unwrap();
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(candidate.join("include/same.h"), "same\n").unwrap();
        std::fs::write(candidate.join("include/changed.h"), "changed\n").unwrap();
        std::fs::write(candidate.join("include/new.h"), "new\n").unwrap();
        std::os::unix::fs::symlink("include/same.h", candidate.join("same-link.h")).unwrap();
        std::os::unix::fs::symlink("include/changed.h", candidate.join("changed-link.h")).unwrap();
        std::os::unix::fs::symlink("include/new.h", candidate.join("new-link.h")).unwrap();
        std::os::unix::fs::symlink("include/same.h", output.join("same-link.h")).unwrap();
        std::os::unix::fs::symlink("include/old.h", output.join("changed-link.h")).unwrap();
        let output_area = OutputRepoPath::new(&output).unwrap();

        let summary = sync_candidate_to_output_area(
            &candidate_tree_path(&candidate),
            &output_area,
            &SyncPolicy::replace_output_tree(),
        )
        .unwrap();

        assert_eq!(summary.symlinks_copied, 2);
        assert_eq!(
            std::fs::read_link(output.join("same-link.h")).unwrap(),
            PathBuf::from("include/same.h")
        );
        assert_eq!(
            std::fs::read_link(output.join("changed-link.h")).unwrap(),
            PathBuf::from("include/changed.h")
        );
        assert_eq!(
            std::fs::read_link(output.join("new-link.h")).unwrap(),
            PathBuf::from("include/new.h")
        );
    }

    #[cfg(unix)]
    #[test]
    fn sync_summary_counts_rejected_special_files() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("candidate");
        let output = temp.path().join("output");
        std::fs::create_dir_all(&candidate).unwrap();
        std::fs::create_dir_all(&output).unwrap();
        let fifo = candidate.join("unsupported-fifo");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap();
        assert!(
            status.success(),
            "failed to create fifo at {}",
            fifo.display()
        );
        let candidate_root = normalize_existing_dir("candidate tree", &candidate).unwrap();
        let output_root = normalize_existing_dir("output candidate area", &output).unwrap();
        let boundary = SyncBoundary::new(
            candidate_root.clone(),
            output_root.clone(),
            output_root.clone(),
        )
        .unwrap();
        let mut summary = SyncSummary::default();

        let err = sync_dir_contents(
            &boundary,
            &candidate_root,
            &output_root,
            true,
            &SyncPolicy::replace_output_tree(),
            &mut summary,
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("unsupported file type in snapshot tree"));
        assert_eq!(summary.special_files_rejected, 1);
        assert!(!output.join("unsupported-fifo").exists());
    }

    #[cfg(unix)]
    #[test]
    fn sync_policy_copies_safe_relative_symlink_as_link() {
        let temp = tempfile::tempdir().unwrap();
        let candidate = temp.path().join("candidate");
        let output = temp.path().join("output");
        std::fs::create_dir_all(candidate.join("include")).unwrap();
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(candidate.join("include/target.h"), "header\n").unwrap();
        std::os::unix::fs::symlink("include/target.h", candidate.join("target-link.h")).unwrap();
        let output_area = OutputRepoPath::new(&output).unwrap();

        let summary = sync_candidate_to_output_area(
            &candidate_tree_path(&candidate),
            &output_area,
            &SyncPolicy::replace_output_tree(),
        )
        .unwrap();

        assert_eq!(
            std::fs::read_link(output.join("target-link.h")).unwrap(),
            PathBuf::from("include/target.h")
        );
        assert_eq!(summary.symlinks_copied, 1);
    }

    #[test]
    fn output_sync_destination_rejects_parent_components() {
        let err = OutputRepoPath::new("../output").unwrap_err().to_string();

        assert!(err.contains("must not contain parent components"));
    }

    #[test]
    fn output_sync_destination_normalizes_current_dir_components() {
        let output_area = OutputRepoPath::new("./output/./tree").unwrap();

        assert_eq!(output_area.as_path(), Path::new("output/tree"));
    }
}
