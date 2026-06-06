//! Metadata identity, fingerprint, ref, and version value models.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::validation::non_empty_model_value;

#[allow(dead_code)]
pub const CURRENT_METADATA_SCHEMA_VERSION: MetadataSchemaVersion = MetadataSchemaVersion(1);

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct MetadataSchemaVersion(u32);

#[allow(dead_code)]
impl MetadataSchemaVersion {
    pub const fn new(version: u32) -> Self {
        Self(version)
    }

    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct PlanFingerprint(String);

#[allow(dead_code)]
impl PlanFingerprint {
    pub fn new(fingerprint: impl Into<String>) -> Result<Self> {
        let fingerprint = non_empty_model_value("plan fingerprint", fingerprint)?;
        Ok(Self(fingerprint))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct TreeFingerprint(String);

#[allow(dead_code)]
impl TreeFingerprint {
    pub fn new(fingerprint: impl Into<String>) -> Result<Self> {
        let fingerprint = non_empty_model_value("tree fingerprint", fingerprint)?;
        Ok(Self(fingerprint))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct MetadataFingerprint(String);

#[allow(dead_code)]
impl MetadataFingerprint {
    pub fn new(fingerprint: impl Into<String>) -> Result<Self> {
        let fingerprint = non_empty_model_value("metadata fingerprint", fingerprint)?;
        Ok(Self(fingerprint))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct SnapshotId(String);

#[allow(dead_code)]
impl SnapshotId {
    pub fn new(id: impl Into<String>) -> Result<Self> {
        let id = non_empty_model_value("snapshot id", id)?;
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct GitCommitId(String);

#[allow(dead_code)]
impl GitCommitId {
    pub fn new(commit: impl Into<String>) -> Result<Self> {
        let commit = non_empty_model_value("git commit id", commit)?;
        Ok(Self(commit))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct OutputBranchName(String);

#[allow(dead_code)]
impl OutputBranchName {
    pub fn new(branch: impl Into<String>) -> Result<Self> {
        let branch = non_empty_model_value("output branch name", branch)?;
        Ok(Self(branch))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct ToolVersion(String);

#[allow(dead_code)]
impl ToolVersion {
    pub fn current() -> Result<Self> {
        Self::new(env!("CARGO_PKG_VERSION"))
    }

    pub fn new(version: impl Into<String>) -> Result<Self> {
        let version = non_empty_model_value("tool version", version)?;
        Ok(Self(version))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
