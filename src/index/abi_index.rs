//! ABI-sensitive facts for the read-only tree index.
//!
//! This module owns read-only ABI path facts and source include references to
//! public-header or UAPI surfaces. It records facts only; ABI surface
//! classification and removal policy remain outside the tree index.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::abi::classify_abi_header_path;
pub use crate::abi::AbiSurfaceKind;

use super::file_index::is_host_absolute_path_like;
use super::source_index::IncludeSite;

pub type AbiPathIndex = BTreeSet<AbiPathFact>;
pub type AbiSourceReferenceIndex = BTreeSet<AbiSourceReference>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AbiPathFact {
    pub path: PathBuf,
    pub kind: AbiSurfaceKind,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AbiSourceReference {
    pub file: PathBuf,
    pub line: usize,
    pub target: PathBuf,
    pub kind: AbiSurfaceKind,
}

pub(in crate::index) fn abi_path_fact(path: &Path) -> Option<AbiPathFact> {
    classify_abi_header_path(path).map(|kind| AbiPathFact {
        path: path.to_path_buf(),
        kind,
    })
}

pub(in crate::index) fn abi_source_reference_from_include_site(
    site: &IncludeSite,
) -> Option<AbiSourceReference> {
    let target = include_target_to_abi_path(&site.target)?;
    let kind = classify_abi_header_path(&target)?;
    Some(AbiSourceReference {
        file: site.file.clone(),
        line: site.line,
        target,
        kind,
    })
}

fn include_target_to_abi_path(target: &str) -> Option<PathBuf> {
    if is_host_absolute_path_like(target) {
        return None;
    }

    let path = Path::new(target);
    if path.starts_with("include") || path.starts_with("arch") {
        return Some(path.to_path_buf());
    }
    if path.starts_with("uapi") || path.starts_with("linux") || path.starts_with("net") {
        return Some(Path::new("include").join(path));
    }

    None
}
