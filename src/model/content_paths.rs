//! Documentation, tool, and sample path value models.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::path::{Path, PathBuf};

use super::validation::{
    documentation_path_parts_match, normalized_relative_model_path_parts, sample_path_parts_match,
    tool_path_parts_match,
};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct DocumentationPath(String);

#[allow(dead_code)]
impl DocumentationPath {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let parts = normalized_relative_model_path_parts("documentation path", &path)?;
        let normalized = parts.join("/");
        if normalized.chars().any(|ch| matches!(ch, '$' | '%' | ':')) {
            anyhow::bail!(
                "documentation path contains unsupported syntax: {}",
                path.display()
            );
        }
        if !documentation_path_parts_match(&parts) {
            anyhow::bail!(
                "documentation path must be under Documentation: {}",
                path.display()
            );
        }
        Ok(Self(normalized))
    }

    pub fn matches_path(path: &Path) -> bool {
        Self::new(path.to_path_buf()).is_ok()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl Borrow<str> for DocumentationPath {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct ToolPath(String);

#[allow(dead_code)]
impl ToolPath {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let parts = normalized_relative_model_path_parts("tool path", &path)?;
        let normalized = parts.join("/");
        if normalized.chars().any(|ch| matches!(ch, '$' | '%' | ':')) {
            anyhow::bail!("tool path contains unsupported syntax: {}", path.display());
        }
        if !tool_path_parts_match(&parts) {
            anyhow::bail!("tool path must be under tools: {}", path.display());
        }
        Ok(Self(normalized))
    }

    pub fn matches_path(path: &Path) -> bool {
        Self::new(path.to_path_buf()).is_ok()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl Borrow<str> for ToolPath {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct SamplePath(String);

#[allow(dead_code)]
impl SamplePath {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let parts = normalized_relative_model_path_parts("sample path", &path)?;
        let normalized = parts.join("/");
        if normalized.chars().any(|ch| matches!(ch, '$' | '%' | ':')) {
            anyhow::bail!(
                "sample path contains unsupported syntax: {}",
                path.display()
            );
        }
        if !sample_path_parts_match(&parts) {
            anyhow::bail!("sample path must be under samples: {}", path.display());
        }
        Ok(Self(normalized))
    }

    pub fn matches_path(path: &Path) -> bool {
        Self::new(path.to_path_buf()).is_ok()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl Borrow<str> for SamplePath {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
