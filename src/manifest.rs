//! Output file-hash manifest generation for emitted trees.
//!
//! This module owns the deterministic `.kslim/manifest.txt` snapshot of the
//! generated output payload. It intentionally tracks emitted payload files and
//! hashes, not kslim metadata or reducer removal intent. Reducer
//! removal-manifest logic lives in `src/removal_manifest.rs` and should not be
//! added here.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const OUTPUT_MANIFEST_FILE_NAME: &str = "manifest.txt";

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

pub fn generate_manifest(root: &str) -> Result<Vec<FileEntry>> {
    let mut entries = collect_file_entries(Path::new(root))?;
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

pub fn write_manifest(entries: &[FileEntry], output_path: &str) -> Result<()> {
    let manifest_path = output_manifest_path(std::path::Path::new(output_path));
    crate::fsutil::ensure_dir(manifest_path.parent().unwrap())?;
    std::fs::write(&manifest_path, render_manifest(entries))?;
    Ok(())
}

pub fn tree_fingerprint(entries: &[FileEntry]) -> String {
    let mut entries = entries.iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.path.cmp(&right.path));

    let mut hasher = Sha256::new();
    hash_fingerprint_field(&mut hasher, "format", "kslim-candidate-tree-fingerprint-v1");
    for entry in entries {
        hash_fingerprint_field(&mut hasher, "path", &entry.path);
        hash_fingerprint_field(&mut hasher, "size", &entry.size.to_string());
        hash_fingerprint_field(&mut hasher, "sha256", &entry.sha256);
    }
    format!("tree-{}", hex::encode(hasher.finalize()))
}

pub fn output_manifest_path(output_path: &Path) -> PathBuf {
    output_manifest_metadata_dir(output_path).join(OUTPUT_MANIFEST_FILE_NAME)
}

fn output_manifest_metadata_dir(output_path: &Path) -> PathBuf {
    if output_path.join(".git").exists() {
        output_path.join(".git").join("kslim")
    } else {
        output_path.join(".kslim")
    }
}

fn collect_file_entries(root: &Path) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let rel = entry
            .path()
            .strip_prefix(root)
            .with_context(|| format!("path prefix error: {}", entry.path().display()))?;
        let rel_str = rel.to_string_lossy().to_string();

        // Skip VCS and kslim metadata directories. The published manifest
        // should describe payload truth, not private or transport metadata.
        if rel_str.starts_with(".git") || rel_str.starts_with(".kslim") {
            continue;
        }

        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

        entries.push(FileEntry {
            path: rel_str,
            size,
            sha256: hash_file(entry.path())?,
        });
    }

    Ok(entries)
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn hash_fingerprint_field(hasher: &mut Sha256, name: &str, value: &str) {
    hasher.update(name.as_bytes());
    hasher.update(b"\0");
    hasher.update(value.len().to_string().as_bytes());
    hasher.update(b"\0");
    hasher.update(value.as_bytes());
    hasher.update(b"\0");
}

fn render_manifest(entries: &[FileEntry]) -> String {
    let mut content = String::new();
    for entry in entries {
        content.push_str(&format!(
            "{}  {}  {}\n",
            entry.sha256, entry.size, entry.path
        ));
    }
    content
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_manifest_is_sorted_and_skips_metadata_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        std::fs::create_dir_all(tmp.path().join(".kslim")).unwrap();
        std::fs::create_dir_all(tmp.path().join("b")).unwrap();
        std::fs::create_dir_all(tmp.path().join("a")).unwrap();
        std::fs::write(tmp.path().join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(tmp.path().join(".kslim/base.toml"), "base = true\n").unwrap();
        std::fs::write(tmp.path().join("b/file.txt"), "bbb\n").unwrap();
        std::fs::write(tmp.path().join("a/file.txt"), "aaa\n").unwrap();

        let entries = generate_manifest(tmp.path().to_str().unwrap()).unwrap();

        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.path.as_str())
                .collect::<Vec<_>>(),
            vec!["a/file.txt", "b/file.txt",]
        );
        assert!(entries.iter().all(|entry| !entry.path.starts_with(".git")));
        assert!(entries
            .iter()
            .all(|entry| !entry.path.starts_with(".kslim")));
    }

    #[test]
    fn test_write_manifest_writes_output_hash_manifest_to_metadata_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");
        std::fs::create_dir_all(output.join(".git")).unwrap();

        write_manifest(
            &[FileEntry {
                path: "Makefile".to_string(),
                size: 7,
                sha256: "abc123".to_string(),
            }],
            output.to_str().unwrap(),
        )
        .unwrap();

        let manifest = std::fs::read_to_string(output.join(".git/kslim/manifest.txt")).unwrap();
        assert_eq!(manifest, "abc123  7  Makefile\n");
    }

    #[test]
    fn test_tree_fingerprint_is_stable_for_manifest_entries() {
        let entries = vec![
            FileEntry {
                path: "b.txt".to_string(),
                size: 2,
                sha256: "bbb".to_string(),
            },
            FileEntry {
                path: "a.txt".to_string(),
                size: 1,
                sha256: "aaa".to_string(),
            },
        ];
        let mut reversed = entries.clone();
        reversed.reverse();

        let fingerprint = tree_fingerprint(&entries);

        assert!(fingerprint.starts_with("tree-"));
        assert_eq!(fingerprint, tree_fingerprint(&reversed));
    }

    #[test]
    fn test_output_manifest_path_is_distinct_from_reducer_removal_manifest_name() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("output");

        let manifest_path = output_manifest_path(&output);

        assert!(manifest_path.ends_with("manifest.txt"));
        assert!(!manifest_path.ends_with("removal-manifest.toml"));
    }
}
