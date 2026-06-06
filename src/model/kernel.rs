//! Kernel config, build, ABI, source, header, and generated-artifact models.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::path::{Component, Path, PathBuf};

use crate::path_policy::{contains_parent_traversal, path_is_absolute_like};

use super::validation::{
    generated_artifact_path_parts_match, non_empty_model_value,
    normalized_relative_model_path_parts, source_file_path_matches, uapi_path_parts_match,
};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct ArchName(String);

#[allow(dead_code)]
impl ArchName {
    pub fn new(arch: impl Into<String>) -> Result<Self> {
        let arch = non_empty_model_value("kernel architecture name", arch)?;
        if !arch
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        {
            anyhow::bail!(
                "kernel architecture name contains invalid characters: {}",
                arch
            );
        }
        Ok(Self(arch))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct KconfigSymbol(String);

#[allow(dead_code)]
impl KconfigSymbol {
    pub fn new(symbol: impl Into<String>) -> Result<Self> {
        let symbol = non_empty_model_value("Kconfig symbol", symbol)?;
        if !symbol
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            anyhow::bail!("Kconfig symbol contains invalid characters: {}", symbol);
        }
        Ok(Self(symbol))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for KconfigSymbol {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct SourceFilePath(String);

#[allow(dead_code)]
impl SourceFilePath {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let parts = normalized_relative_model_path_parts("source file path", &path)?;
        let normalized = parts.join("/");
        if normalized.chars().any(|ch| matches!(ch, '$' | '%' | ':')) {
            anyhow::bail!(
                "source file path contains unsupported syntax: {}",
                path.display()
            );
        }
        if !source_file_path_matches(Path::new(&normalized)) {
            anyhow::bail!(
                "source file path must end with .c, .S, or .rs: {}",
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

impl Borrow<str> for SourceFilePath {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct HeaderPath(String);

#[allow(dead_code)]
impl HeaderPath {
    pub fn new(path: impl Into<String>) -> Result<Self> {
        let path = non_empty_model_value("header path", path)?;
        if path.chars().any(char::is_whitespace) {
            anyhow::bail!("header path contains whitespace: {}", path);
        }
        if contains_parent_traversal(&path) {
            anyhow::bail!("header path must not contain '..': {}", path);
        }
        if path.contains('\\') {
            anyhow::bail!("header path contains invalid separator: {}", path);
        }

        let path_ref = Path::new(&path);
        if path_is_absolute_like(path_ref) {
            anyhow::bail!("header path must be relative to the kernel tree: {}", path);
        }

        let mut parts = Vec::new();
        for component in path_ref.components() {
            match component {
                Component::CurDir => {}
                Component::Normal(part) => {
                    let Some(part) = part.to_str() else {
                        anyhow::bail!("header path contains non-UTF-8 component");
                    };
                    parts.push(part);
                }
                Component::ParentDir => {
                    anyhow::bail!("header path must not contain '..': {}", path);
                }
                Component::RootDir | Component::Prefix(_) => {
                    anyhow::bail!("header path must be relative to the kernel tree: {}", path);
                }
            }
        }

        if parts.is_empty() {
            anyhow::bail!(
                "header path must not resolve to the kernel tree root: {}",
                path
            );
        }

        let normalized = parts.join("/");
        if !normalized.ends_with(".h") {
            anyhow::bail!("header path must end with .h: {}", path);
        }

        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl Borrow<str> for HeaderPath {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct UapiPath(String);

#[allow(dead_code)]
impl UapiPath {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let parts = normalized_relative_model_path_parts("UAPI path", &path)?;
        if !uapi_path_parts_match(&parts) {
            anyhow::bail!(
                "UAPI path must be under include/uapi, include/generated/uapi, arch/<arch>/include/uapi, or arch/<arch>/include/generated/uapi: {}",
                path.display()
            );
        }
        Ok(Self(parts.join("/")))
    }

    pub fn matches_path(path: &Path) -> bool {
        normalized_relative_model_path_parts("UAPI path", path)
            .map(|parts| uapi_path_parts_match(&parts))
            .unwrap_or(false)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl Borrow<str> for UapiPath {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct GeneratedArtifactPath(String);

#[allow(dead_code)]
impl GeneratedArtifactPath {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let parts = normalized_relative_model_path_parts("generated artifact path", &path)?;
        let normalized = parts.join("/");
        if normalized.chars().any(|ch| matches!(ch, '$' | '%' | ':')) {
            anyhow::bail!(
                "generated artifact path contains unsupported syntax: {}",
                path.display()
            );
        }
        if !generated_artifact_path_parts_match(&parts) {
            anyhow::bail!(
                "generated artifact path must be under include/generated, include/config, arch/<arch>/include/generated, or be a known top-level generated build artifact: {}",
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

impl Borrow<str> for GeneratedArtifactPath {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct KbuildObject(String);

#[allow(dead_code)]
impl KbuildObject {
    pub fn new(object: impl Into<String>) -> Result<Self> {
        let object = non_empty_model_value("kbuild object", object)?;
        if object.chars().any(char::is_whitespace) {
            anyhow::bail!("kbuild object contains whitespace: {}", object);
        }
        if contains_parent_traversal(&object) {
            anyhow::bail!("kbuild object must not contain '..': {}", object);
        }

        let directory_ref = object.ends_with('/');
        let trimmed = object.trim_end_matches('/');
        if trimmed.is_empty() {
            anyhow::bail!(
                "kbuild object must not resolve to the kernel tree root: {}",
                object
            );
        }

        let path = Path::new(trimmed);
        if path_is_absolute_like(path) {
            anyhow::bail!(
                "kbuild object must be relative to the kernel tree: {}",
                object
            );
        }
        if object
            .chars()
            .any(|ch| matches!(ch, '$' | '%' | ':' | '\\'))
        {
            anyhow::bail!("kbuild object contains unsupported make syntax: {}", object);
        }

        let mut parts = Vec::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::Normal(part) => {
                    let Some(part) = part.to_str() else {
                        anyhow::bail!("kbuild object contains non-UTF-8 component");
                    };
                    parts.push(part);
                }
                Component::ParentDir => {
                    anyhow::bail!("kbuild object must not contain '..': {}", object);
                }
                Component::RootDir | Component::Prefix(_) => {
                    anyhow::bail!(
                        "kbuild object must be relative to the kernel tree: {}",
                        object
                    );
                }
            }
        }

        if parts.is_empty() {
            anyhow::bail!(
                "kbuild object must not resolve to the kernel tree root: {}",
                object
            );
        }

        let mut normalized = parts.join("/");
        if directory_ref {
            normalized.push('/');
        } else if !normalized.ends_with(".o") {
            anyhow::bail!("kbuild object must end with .o or /: {}", object);
        }

        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_directory_ref(&self) -> bool {
        self.0.ends_with('/')
    }
}

impl Borrow<str> for KbuildObject {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
