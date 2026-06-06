use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

use crate::manifest;

use super::super::state::CandidateTreeState;
use crate::model::{MetadataFingerprint, TreeFingerprint};

pub(super) fn ensure_candidate_is_observable(candidate: &CandidateTreeState) -> Result<()> {
    if !candidate.materialized {
        anyhow::bail!("cannot verify candidate before materialization");
    }
    if !candidate.tree.as_path().is_dir() {
        anyhow::bail!(
            "candidate tree is not an existing directory: {}",
            candidate.tree.as_path().display()
        );
    }
    if !candidate.metadata_dir.as_path().is_dir() {
        anyhow::bail!(
            "candidate metadata directory is not an existing directory: {}",
            candidate.metadata_dir.as_path().display()
        );
    }
    ensure_path_inside_candidate_tree(
        candidate.tree.as_path(),
        candidate.metadata_dir.as_path(),
        "candidate metadata directory",
    )
}

pub(super) fn fingerprint_candidate_tree(tree_path: &Path) -> Result<TreeFingerprint> {
    let tree_path = tree_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("candidate tree path is not valid UTF-8"))?;
    let entries = manifest::generate_manifest(tree_path)?;
    TreeFingerprint::new(manifest::tree_fingerprint(&entries))
}

pub(super) fn fingerprint_candidate_metadata(metadata_dir: &Path) -> Result<MetadataFingerprint> {
    let mut files = Vec::new();
    for entry in WalkDir::new(metadata_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry.path().strip_prefix(metadata_dir).with_context(|| {
            format!(
                "failed to relativize candidate metadata path {} under {}",
                entry.path().display(),
                metadata_dir.display()
            )
        })?;
        files.push((
            normalize_relative_path(relative),
            hash_file(entry.path()).with_context(|| {
                format!(
                    "failed to hash candidate metadata file {}",
                    entry.path().display()
                )
            })?,
        ));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = Sha256::new();
    hash_field(
        &mut hasher,
        "format",
        "kslim-candidate-metadata-fingerprint-v1",
    );
    for (path, hash) in files {
        hash_field(&mut hasher, "path", &path);
        hash_field(&mut hasher, "sha256", &hash);
    }
    MetadataFingerprint::new(format!("metadata-{}", hex::encode(hasher.finalize())))
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn hash_field(hasher: &mut Sha256, name: &str, value: &str) {
    hasher.update(name.as_bytes());
    hasher.update(b"\0");
    hasher.update(value.len().to_string().as_bytes());
    hasher.update(b"\0");
    hasher.update(value.as_bytes());
    hasher.update(b"\0");
}

pub(super) fn normalize_relative_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn ensure_path_inside_candidate_tree(
    candidate_tree: &Path,
    path: &Path,
    label: &str,
) -> Result<()> {
    let candidate_tree = normalize_boundary_path(candidate_tree)?;
    let path = normalize_boundary_path(path)?;
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

fn normalize_boundary_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context(
                "failed to read current directory for candidate verification path normalization",
            )?
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    Ok(normalized)
}
