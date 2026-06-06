//! Typed lifecycle path wrappers and validating constructors.

use anyhow::Result;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::display::path_to_config_string;
use super::normalization::{
    canonicalize_existing_path, normalize_path_without_parent_components, reject_empty_path,
};
use super::traversal::{reject_absolute_like_relative_kernel_path, reject_parent_traversal};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RequestedConfigPath(PathBuf);

impl RequestedConfigPath {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let path = normalize_path_without_parent_components("requested config path", &path)?;
        Ok(Self(path))
    }

    #[allow(dead_code)]
    pub(crate) fn new_existing_file(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        reject_empty_path("requested config path", &path)?;
        if !path.is_file() {
            anyhow::bail!(
                "requested config path is not an existing file: {}",
                path.display()
            );
        }
        let path = canonicalize_existing_path("requested config path", &path)?;
        Ok(Self(path))
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.0.as_path()
    }
}

impl AsRef<Path> for RequestedConfigPath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct WorkspaceRoot(PathBuf);

impl WorkspaceRoot {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let path = normalize_path_without_parent_components("workspace root path", &path)?;
        Ok(Self(path))
    }

    #[allow(dead_code)]
    pub(crate) fn new_existing_dir(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        reject_empty_path("workspace root path", &path)?;
        if !path.is_dir() {
            anyhow::bail!(
                "workspace root path is not an existing directory: {}",
                path.display()
            );
        }
        let path = canonicalize_existing_path("workspace root path", &path)?;
        Ok(Self(path))
    }

    #[allow(dead_code)]
    pub(crate) fn new_temp_workspace(path: impl Into<PathBuf>) -> Result<Self> {
        let workspace = Self::new_existing_dir(path)?;
        let temp_root =
            canonicalize_existing_path("system temporary directory", &std::env::temp_dir())?;
        if !workspace.as_path().starts_with(&temp_root) {
            anyhow::bail!(
                "workspace root path is outside temporary directory: {}",
                workspace.as_path().display()
            );
        }
        Ok(workspace)
    }

    pub(crate) fn candidate_tree_path(&self) -> CandidateTreePath {
        CandidateTreePath(self.as_path().join("tree"))
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.0.as_path()
    }
}

impl AsRef<Path> for WorkspaceRoot {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CandidateTreePath(PathBuf);

impl CandidateTreePath {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let path = normalize_path_without_parent_components("candidate tree path", &path)?;
        Ok(Self(path))
    }

    #[allow(dead_code)]
    pub(crate) fn new_temp_tree(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        reject_empty_path("candidate tree path", &path)?;
        if !path.is_dir() {
            anyhow::bail!(
                "candidate tree path is not an existing directory: {}",
                path.display()
            );
        }
        let candidate = canonicalize_existing_path("candidate tree path", &path)?;
        let temp_root =
            canonicalize_existing_path("system temporary directory", &std::env::temp_dir())?;
        if !candidate.starts_with(&temp_root) {
            anyhow::bail!(
                "candidate tree path is outside temporary directory: {}",
                candidate.display()
            );
        }
        Ok(Self(candidate))
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.0.as_path()
    }
}

impl AsRef<Path> for CandidateTreePath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct KernelSourceRoot(PathBuf);

#[allow(dead_code)]
impl KernelSourceRoot {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let path = normalize_path_without_parent_components("kernel source root path", &path)?;
        Ok(Self(path))
    }

    pub(crate) fn new_existing_dir(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        reject_empty_path("kernel source root path", &path)?;
        if !path.is_dir() {
            anyhow::bail!(
                "kernel source root path is not an existing directory: {}",
                path.display()
            );
        }
        let path = canonicalize_existing_path("kernel source root path", &path)?;
        Ok(Self(path))
    }

    pub(crate) fn from_candidate_tree(candidate_tree: &CandidateTreePath) -> Result<Self> {
        Self::new(candidate_tree.as_path())
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.0.as_path()
    }
}

impl AsRef<Path> for KernelSourceRoot {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct KernelBuildDir(PathBuf);

#[allow(dead_code)]
impl KernelBuildDir {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let path = normalize_path_without_parent_components("kernel build dir path", &path)?;
        if path == Path::new(".") || path.parent().is_none() {
            anyhow::bail!(
                "kernel build dir path must not be the source root or filesystem root: {}",
                path.display()
            );
        }
        Ok(Self(path))
    }

    pub(crate) fn new_for_source_root(
        source_root: &KernelSourceRoot,
        path: impl Into<PathBuf>,
    ) -> Result<Self> {
        let path = Self::new(path)?;
        let path = if path.as_path().is_absolute() {
            path.as_path().to_path_buf()
        } else {
            source_root.as_path().join(path.as_path())
        };
        if path == source_root.as_path() {
            anyhow::bail!(
                "kernel build dir path must not alias kernel source root: {}",
                path.display()
            );
        }
        Self::new(path)
    }

    pub(crate) fn default_for_source_root(source_root: &KernelSourceRoot, index: usize) -> Self {
        Self(
            source_root
                .as_path()
                .join(".kslim-selftest")
                .join(format!("build-{}", index + 1)),
        )
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.0.as_path()
    }
}

impl AsRef<Path> for KernelBuildDir {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RelativeKernelPath(PathBuf);

#[allow(dead_code)]
impl RelativeKernelPath {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Result<Self> {
        Self::new_with_root_policy(path, false)
    }

    pub(crate) fn new_for_explicit_unsafe_root_removal(path: impl Into<PathBuf>) -> Result<Self> {
        Self::new_with_root_policy(path, true)
    }

    fn new_with_root_policy(
        path: impl Into<PathBuf>,
        unsafe_allow_tree_root: bool,
    ) -> Result<Self> {
        let path = path.into();
        reject_absolute_like_relative_kernel_path(&path)?;
        let path = normalize_path_without_parent_components("relative kernel path", &path)?;
        if path == Path::new(".") && !unsafe_allow_tree_root {
            anyhow::bail!(
                "relative kernel path must not resolve to the kernel tree root: {}",
                path.display()
            );
        }
        Ok(Self(path))
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    pub(crate) fn to_config_string(&self) -> String {
        path_to_config_string(self.as_path())
    }
}

impl AsRef<Path> for RelativeKernelPath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CandidateMetadataDir(PathBuf);

impl CandidateMetadataDir {
    #[allow(dead_code)]
    pub(crate) fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let path = normalize_path_without_parent_components("candidate metadata dir", &path)?;
        if path.file_name() != Some(OsStr::new(".kslim")) {
            anyhow::bail!(
                "candidate metadata dir must be the candidate .kslim dir: {}",
                path.display()
            );
        }
        Ok(Self(path))
    }

    pub(crate) fn new_in_candidate_tree(
        candidate_tree: &CandidateTreePath,
        path: impl Into<PathBuf>,
    ) -> Result<Self> {
        let metadata_dir = Self::new(path)?;
        let expected = candidate_tree.as_path().join(".kslim");
        if metadata_dir.as_path() != expected.as_path() {
            anyhow::bail!(
                "candidate metadata dir is not the candidate tree metadata dir: {} != {}",
                metadata_dir.as_path().display(),
                expected.display()
            );
        }
        Ok(metadata_dir)
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.0.as_path()
    }
}

impl AsRef<Path> for CandidateMetadataDir {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AttemptMetadataDir(PathBuf);

impl AttemptMetadataDir {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let path = normalize_path_without_parent_components("attempt metadata dir", &path)?;
        if path.file_name() != Some(OsStr::new("attempt"))
            || path.parent().and_then(Path::file_name) != Some(OsStr::new(".kslim"))
        {
            anyhow::bail!(
                "attempt metadata dir is not a non-authoritative attempt dir: {}",
                path.display()
            );
        }
        Ok(Self(path))
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.0.as_path()
    }
}

impl AsRef<Path> for AttemptMetadataDir {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct OutputRepoPath(PathBuf);

impl OutputRepoPath {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let path = normalize_path_without_parent_components("published output repo path", &path)?;
        Ok(Self(path))
    }

    #[allow(dead_code)]
    pub(crate) fn new_git_worktree(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        reject_empty_path("published output repo path", &path)?;
        if !path.is_dir() {
            anyhow::bail!(
                "published output repo path is not an existing directory: {}",
                path.display()
            );
        }
        let path = canonicalize_existing_path("published output repo path", &path)?;
        if !path.join(".git").exists() {
            anyhow::bail!(
                "published output repo path is not a git worktree: {}",
                path.display()
            );
        }
        Ok(Self(path))
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.0.as_path()
    }
}

impl AsRef<Path> for OutputRepoPath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct OutputCandidateArea(PathBuf);

impl OutputCandidateArea {
    pub(super) fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let path = normalize_path_without_parent_components("output candidate area path", &path)?;
        Ok(Self(path))
    }

    pub(crate) fn from_output_repo(output_repo: &OutputRepoPath) -> Result<Self> {
        Self::new(output_repo.as_path())
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.0.as_path()
    }
}

impl AsRef<Path> for OutputCandidateArea {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PublishedMetadataDir(PathBuf);

impl PublishedMetadataDir {
    #[allow(dead_code)]
    pub(super) fn new(path: impl Into<PathBuf>) -> Result<Self> {
        Self::new_committed_metadata_dir(path)
    }

    #[allow(dead_code)]
    pub(super) fn new_committed_metadata_dir(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let path = normalize_path_without_parent_components("published metadata dir", &path)?;
        let is_git_metadata_dir = path.file_name() == Some(OsStr::new("kslim"))
            && path.parent().and_then(Path::file_name) == Some(OsStr::new(".git"));
        let is_committed_tree_metadata_dir = path.file_name() == Some(OsStr::new(".kslim"));
        if !is_git_metadata_dir && !is_committed_tree_metadata_dir {
            anyhow::bail!(
                "published metadata dir is not a committed metadata dir: {}",
                path.display()
            );
        }
        Ok(Self(path))
    }

    pub(crate) fn new_in_output_repo(
        output_repo: &OutputRepoPath,
        path: impl Into<PathBuf>,
    ) -> Result<Self> {
        let path = path.into();
        let metadata_dir = Self::new_committed_metadata_dir(path)?;
        let expected = if output_repo.as_path().join(".git").exists() {
            output_repo.as_path().join(".git").join("kslim")
        } else {
            output_repo.as_path().join(".kslim")
        };
        if metadata_dir.as_path() != expected.as_path() {
            anyhow::bail!(
                "published metadata dir is not the output repo metadata dir: {} != {}",
                metadata_dir.as_path().display(),
                expected.display()
            );
        }
        Ok(metadata_dir)
    }

    pub(crate) fn new_committed_tree_in_output_repo(
        output_repo: &OutputRepoPath,
        path: impl Into<PathBuf>,
    ) -> Result<Self> {
        let path = path.into();
        let metadata_dir = Self::new_committed_metadata_dir(path)?;
        let expected = output_repo.as_path().join(".kslim");
        if metadata_dir.as_path() != expected.as_path() {
            anyhow::bail!(
                "published metadata dir is not the output repo committed tree metadata dir: {} != {}",
                metadata_dir.as_path().display(),
                expected.display()
            );
        }
        Ok(metadata_dir)
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.0.as_path()
    }
}

impl AsRef<Path> for PublishedMetadataDir {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LockfilePath(PathBuf);

#[allow(dead_code)]
impl LockfilePath {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let path = normalize_path_without_parent_components("authoritative lockfile path", &path)?;
        if path.file_name() != Some(OsStr::new("kslim.lock")) {
            anyhow::bail!(
                "authoritative lockfile path must end in kslim.lock: {}",
                path.display()
            );
        }
        Ok(Self(path))
    }

    pub(crate) fn new_in_project_root(project_root: impl AsRef<Path>) -> Result<Self> {
        let project_root = project_root.as_ref();
        reject_empty_path("authoritative lockfile project root", project_root)?;
        reject_parent_traversal("authoritative lockfile project root", project_root)?;
        Self::new(project_root.join("kslim.lock"))
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.0.as_path()
    }
}

impl AsRef<Path> for LockfilePath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}
