//! Generated artifact discovery, policy, and clean-build verification models.
//!
//! This module owns generated-artifact classification, generated include-root
//! policy normalization, and clean-build verification state. It does not own
//! candidate tree mutation, publication metadata, or reducer report rendering.

mod artifact;
mod clean_build;
mod policy;

#[allow(unused_imports)]
pub(crate) use artifact::{
    discover_generated_artifacts, is_generated_artifact_like_path, is_generated_artifact_path,
    raw_generated_artifact_path_parts_match, GeneratedArtifactDiscovery,
};
#[allow(unused_imports)]
pub(crate) use clean_build::{CleanBuildVerification, CleanBuildVerificationStatus};
pub(crate) use policy::{is_generated_include_header_path, normalize_generated_include_roots};
