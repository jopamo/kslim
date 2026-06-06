//! Clean-build verification state for generated artifacts.

use crate::model::GeneratedArtifactPath;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CleanBuildVerificationStatus {
    NotRequested,
    Required,
    Verified,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CleanBuildVerification {
    status: CleanBuildVerificationStatus,
    verified_artifacts: Vec<GeneratedArtifactPath>,
}

#[allow(dead_code)]
impl CleanBuildVerification {
    pub(crate) fn not_requested() -> Self {
        Self {
            status: CleanBuildVerificationStatus::NotRequested,
            verified_artifacts: Vec::new(),
        }
    }

    pub(crate) fn required() -> Self {
        Self {
            status: CleanBuildVerificationStatus::Required,
            verified_artifacts: Vec::new(),
        }
    }

    pub(crate) fn verified(artifacts: impl IntoIterator<Item = GeneratedArtifactPath>) -> Self {
        let mut verified_artifacts = artifacts.into_iter().collect::<Vec<_>>();
        verified_artifacts.sort();
        verified_artifacts.dedup();
        Self {
            status: CleanBuildVerificationStatus::Verified,
            verified_artifacts,
        }
    }

    pub(crate) fn status(&self) -> CleanBuildVerificationStatus {
        self.status
    }

    pub(crate) fn verified_artifacts(&self) -> &[GeneratedArtifactPath] {
        &self.verified_artifacts
    }
}
