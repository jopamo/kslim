//! Generated-artifact path discovery and classification.

use anyhow::Result;
use std::collections::BTreeSet;
use std::path::Path;

use crate::model::GeneratedArtifactPath;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct GeneratedArtifactDiscovery {
    artifacts: BTreeSet<GeneratedArtifactPath>,
}

#[allow(dead_code)]
impl GeneratedArtifactDiscovery {
    pub(crate) fn from_paths<I, P>(paths: I) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        Ok(Self {
            artifacts: discover_generated_artifacts(paths)?,
        })
    }

    pub(crate) fn artifacts(&self) -> &BTreeSet<GeneratedArtifactPath> {
        &self.artifacts
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }
}

#[allow(dead_code)]
pub(crate) fn discover_generated_artifacts<I, P>(paths: I) -> Result<BTreeSet<GeneratedArtifactPath>>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut artifacts = BTreeSet::new();
    for path in paths {
        let path = path.as_ref();
        if is_generated_artifact_like_path(path) {
            artifacts.insert(GeneratedArtifactPath::new(path.to_path_buf())?);
        }
    }
    Ok(artifacts)
}

pub(crate) fn is_generated_artifact_path(path: &Path) -> bool {
    GeneratedArtifactPath::matches_path(path)
}

pub(crate) fn is_generated_artifact_like_path(path: &Path) -> bool {
    is_generated_artifact_path(path) || raw_generated_artifact_path_parts_match(path)
}

pub(crate) fn raw_generated_artifact_path_parts_match(path: &Path) -> bool {
    let parts = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();

    match parts.as_slice() {
        ["include", "generated"] => true,
        ["include", "generated", child, ..] if *child != "uapi" => true,
        ["include", "config", ..] => true,
        ["arch", _, "include", "generated"] => true,
        ["arch", _, "include", "generated", child, ..] if *child != "uapi" => true,
        [artifact] => matches!(
            *artifact,
            ".config" | "Module.symvers" | "modules.order" | "System.map" | "vmlinux" | "vmlinux.o"
        ),
        _ => false,
    }
}
