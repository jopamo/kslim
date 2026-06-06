use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use crate::paths::{CandidateTreePath, KernelSourceRoot, WorkspaceRoot};

use super::super::plan::GeneratePlan;

pub(super) struct WorkspacePaths {
    workspace_root: WorkspaceRoot,
    candidate_tree: CandidateTreePath,
    pub(super) temp_dir: Option<TempDir>,
    keep_temp: bool,
}

#[allow(dead_code)]
impl WorkspacePaths {
    pub(super) fn new(candidate_tree: impl Into<PathBuf>) -> Result<Self> {
        let candidate_tree = CandidateTreePath::new(candidate_tree)?;
        let workspace_root = WorkspaceRoot::new(candidate_tree.as_path().to_path_buf())?;
        Ok(Self {
            workspace_root,
            candidate_tree,
            temp_dir: None,
            keep_temp: false,
        })
    }

    pub(super) fn new_isolated_temp() -> Result<Self> {
        Self::new_isolated_temp_with_keep(false)
    }

    pub(super) fn new_isolated_temp_with_keep(keep_temp: bool) -> Result<Self> {
        let temp_dir = tempfile::Builder::new()
            .prefix("kslim-candidate-")
            .tempdir()
            .context("failed to create isolated candidate workspace")?;
        let workspace_root = WorkspaceRoot::new_temp_workspace(temp_dir.path())?;
        let candidate_tree = workspace_root.candidate_tree_path();
        Ok(Self {
            workspace_root,
            candidate_tree,
            temp_dir: Some(temp_dir),
            keep_temp,
        })
    }

    pub(super) fn workspace_root(&self) -> &Path {
        self.workspace_root.as_path()
    }

    pub(super) fn candidate_tree(&self) -> &Path {
        self.candidate_tree.as_path()
    }
}

impl Drop for WorkspacePaths {
    fn drop(&mut self) {
        if self.keep_temp {
            if let Some(temp_dir) = self.temp_dir.take() {
                std::mem::forget(temp_dir);
            }
        }
    }
}

pub(in crate::generate) struct MaterializedTree {
    pub(in crate::generate) temp_dir: TempDir,
    pub(in crate::generate) path: String,
    pub(in crate::generate) mutation_target: CandidateMutationTarget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::generate) struct CandidateMutationTarget {
    pub(super) tree_path: CandidateTreePath,
}

impl CandidateMutationTarget {
    pub(super) fn new(plan: &GeneratePlan, tree_path: impl Into<PathBuf>) -> Result<Self> {
        let tree_path = CandidateTreePath::new(tree_path)?;
        let output_path = plan.resolved.output_plan.output_path.as_path();
        if path_aliases_across_lifecycle(tree_path.as_path(), output_path)? {
            anyhow::bail!(
                "candidate mutation target aliases resolved output path: candidate={} output={}",
                tree_path.as_path().display(),
                output_path.display()
            );
        }
        Ok(Self { tree_path })
    }

    pub(super) fn as_path(&self) -> &Path {
        self.tree_path.as_path()
    }

    pub(super) fn kernel_source_root(&self) -> Result<KernelSourceRoot> {
        KernelSourceRoot::from_candidate_tree(&self.tree_path)
    }

    pub(super) fn path_str(&self) -> Result<&str> {
        self.tree_path
            .as_path()
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("candidate tree path is not valid UTF-8"))
    }
}

pub(super) fn project_root_for_requested_config(path: &Path) -> PathBuf {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

pub(super) fn ensure_candidate_mutation_target(
    plan: &GeneratePlan,
    tree_path: &Path,
) -> Result<CandidateMutationTarget> {
    CandidateMutationTarget::new(plan, tree_path)
}

pub(super) fn path_aliases_across_lifecycle(left: &Path, right: &Path) -> Result<bool> {
    let left = normalize_candidate_boundary_path(left)?;
    let right = normalize_candidate_boundary_path(right)?;
    if lifecycle_paths_overlap(&left, &right) {
        return Ok(true);
    }

    let left = resolve_candidate_boundary_path(&left)?;
    let right = resolve_candidate_boundary_path(&right)?;
    Ok(lifecycle_paths_overlap(&left, &right))
}

fn lifecycle_paths_overlap(left: &Path, right: &Path) -> bool {
    left == right || left.starts_with(right) || right.starts_with(left)
}

fn resolve_candidate_boundary_path(path: &Path) -> Result<PathBuf> {
    let normalized = normalize_candidate_boundary_path(path)?;
    if normalized.exists() {
        return normalized
            .canonicalize()
            .with_context(|| format!("failed to resolve lifecycle path {}", path.display()));
    }

    let mut missing_components = Vec::new();
    let mut ancestor = normalized.as_path();
    while !ancestor.exists() {
        let component = ancestor.file_name().ok_or_else(|| {
            anyhow::anyhow!(
                "failed to resolve lifecycle path without existing ancestor: {}",
                path.display()
            )
        })?;
        missing_components.push(component.to_os_string());
        ancestor = ancestor.parent().ok_or_else(|| {
            anyhow::anyhow!(
                "failed to resolve lifecycle path without parent: {}",
                path.display()
            )
        })?;
    }

    let mut resolved = ancestor
        .canonicalize()
        .with_context(|| format!("failed to resolve lifecycle path {}", ancestor.display()))?;
    for component in missing_components.iter().rev() {
        resolved.push(component);
    }
    Ok(resolved)
}

pub(super) fn normalize_candidate_boundary_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("failed to read current directory for candidate path normalization")?
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    Ok(normalized)
}

pub(super) fn ensure_path_inside_candidate_tree(
    candidate_tree: &Path,
    path: &Path,
    label: &str,
) -> Result<()> {
    let candidate_tree = normalize_candidate_boundary_path(candidate_tree)?;
    let path = normalize_candidate_boundary_path(path)?;
    if !path.starts_with(&candidate_tree) {
        anyhow::bail!(
            "{} is outside candidate tree: {} not under {}",
            label,
            path.display(),
            candidate_tree.display()
        );
    }
    Ok(())
}
