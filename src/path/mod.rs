//! Typed path wrappers for lifecycle boundaries.
//!
//! These wrappers name the state phase or authority boundary a path belongs to.
//! Constructors are intentionally narrow; callers should not freely convert
//! arbitrary `PathBuf` values across requested, candidate, published, and
//! lockfile state.

mod display;
mod normalization;
mod traversal;
mod typed;
#[cfg(test)]
mod tests;

pub(crate) use typed::{
    AttemptMetadataDir, CandidateMetadataDir, CandidateTreePath, KernelBuildDir, KernelSourceRoot,
    LockfilePath, OutputCandidateArea, OutputRepoPath, PublishedMetadataDir, RelativeKernelPath,
    RequestedConfigPath, WorkspaceRoot,
};
