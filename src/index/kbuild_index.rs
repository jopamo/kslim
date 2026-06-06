//! Kbuild facts for the read-only tree index.
//!
//! This module owns Kbuild Makefile discovery, object provider indexing,
//! object reference indexing, and directory reference indexing.

use anyhow::Result;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::file_index::{
    is_host_absolute_path_like, is_relative_index_path, normalize_relative_to_root,
    relative_path_under_root,
};

pub type KbuildFileIndex = BTreeSet<PathBuf>;
pub type KbuildObjectProviderIndex = BTreeSet<PathBuf>;
pub type KbuildObjectReferenceIndex = BTreeSet<KbuildObjectReference>;
pub type KbuildDirectoryReferenceIndex = BTreeSet<KbuildDirectoryReference>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct KbuildDirectoryReference {
    pub file: PathBuf,
    pub line: usize,
    pub assignment_lhs: String,
    pub directory: String,
    pub resolved_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct KbuildObjectReference {
    pub file: PathBuf,
    pub line: usize,
    pub assignment_lhs: String,
    pub object: String,
    pub resolved_path: PathBuf,
}

#[derive(Debug, Default)]
pub(in crate::index) struct KbuildDomainFacts {
    pub files: KbuildFileIndex,
    pub object_providers: KbuildObjectProviderIndex,
    pub object_refs: KbuildObjectReferenceIndex,
    pub directory_refs: KbuildDirectoryReferenceIndex,
}

pub(in crate::index) fn build_kbuild_domain(root: &Path) -> Result<KbuildDomainFacts> {
    let mut facts = KbuildDomainFacts::default();

    for path in crate::kbuild::makefiles(root) {
        facts.files.insert(relative_path_under_root(root, &path)?);
    }

    let kbuild_index = crate::kbuild::build_kbuild_index(root)?;
    facts.object_providers = kbuild_index
        .object_providers
        .into_iter()
        .filter(|path| is_relative_index_path(path))
        .collect();
    for reference in kbuild_index.object_references {
        if is_host_absolute_path_like(&reference.object) {
            continue;
        }
        let current_dir = root.join(reference.file.parent().unwrap_or(Path::new("")));
        let Some(resolved_path) =
            normalize_relative_to_root(root, &current_dir.join(&reference.object))
        else {
            continue;
        };
        facts.object_refs.insert(KbuildObjectReference {
            file: reference.file,
            line: reference.line,
            assignment_lhs: reference.assignment_lhs,
            object: reference.object.clone(),
            resolved_path,
        });
    }
    for reference in kbuild_index.directory_references {
        if is_host_absolute_path_like(&reference.directory) {
            continue;
        }
        let current_dir = root.join(reference.file.parent().unwrap_or(Path::new("")));
        let resolved_paths =
            crate::kbuild::make_dir_candidates(root, &current_dir, &reference.directory);
        if resolved_paths.is_empty() {
            continue;
        }
        facts.directory_refs.insert(KbuildDirectoryReference {
            file: reference.file,
            line: reference.line,
            assignment_lhs: reference.assignment_lhs,
            directory: reference.directory.clone(),
            resolved_paths,
        });
    }

    Ok(facts)
}
