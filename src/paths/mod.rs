//! Compatibility facade for typed path wrappers.
//!
//! New path ownership lives in `crate::path`; this module preserves existing
//! `crate::paths::*` call sites while migration proceeds.

pub(crate) use crate::path::{
    AttemptMetadataDir, CandidateMetadataDir, CandidateTreePath, KernelBuildDir, KernelSourceRoot,
    LockfilePath, OutputCandidateArea, OutputRepoPath, PublishedMetadataDir, RelativeKernelPath,
    RequestedConfigPath, WorkspaceRoot,
};
