//! Read-only query helpers for tree index facts.
//!
//! This module owns lookup/subject-normalization helpers over already-built
//! tree index facts. It does not scan, rebuild, mutate, or decide policy.

use std::path::{Path, PathBuf};

use super::{
    IncludeSite, KbuildDirectoryReference, KbuildObjectReference, KconfigSourceReference,
    TreeIndex,
};

impl TreeIndex {
    #[cfg(test)]
    pub fn contains_file(&self, path: &Path) -> bool {
        self.files.contains(path)
    }

    pub fn find_include_site(&self, file: &Path, target: &str) -> Option<&IncludeSite> {
        self.include_sites
            .iter()
            .find(|site| site.file == file && site.target == target)
    }

    pub fn has_include_site(&self, file: &Path, line: usize, target: &str) -> bool {
        self.include_sites.contains(&IncludeSite {
            file: file.to_path_buf(),
            line,
            target: target.to_string(),
        })
    }

    pub fn find_kconfig_source_ref(
        &self,
        file: &Path,
        line: usize,
        source: &str,
    ) -> Option<&KconfigSourceReference> {
        self.kconfig_sources.iter().find(|reference| {
            reference.file == file && reference.line == line && reference.source == source
        })
    }

    pub fn has_kconfig_source_ref(
        &self,
        file: &Path,
        line: usize,
        source: &str,
        optional: bool,
        relative: bool,
    ) -> bool {
        self.kconfig_sources.contains(&KconfigSourceReference {
            file: file.to_path_buf(),
            line,
            source: source.to_string(),
            optional,
            relative,
        })
    }

    pub fn find_kbuild_directory_refs(&self, path: &str) -> Vec<&KbuildDirectoryReference> {
        let Some(relative) = normalize_directory_subject(path) else {
            return Vec::new();
        };

        self.kbuild_dir_refs
            .iter()
            .filter(|reference| {
                reference
                    .resolved_paths
                    .iter()
                    .any(|candidate| candidate == &relative)
            })
            .collect()
    }

    pub fn has_kbuild_directory_ref(
        &self,
        file: &Path,
        line: usize,
        assignment_lhs: &str,
        directory: &str,
        resolved_path: &Path,
    ) -> bool {
        self.kbuild_dir_refs.iter().any(|reference| {
            reference.file == file
                && reference.line == line
                && reference.assignment_lhs == assignment_lhs
                && reference.directory == directory
                && reference
                    .resolved_paths
                    .iter()
                    .any(|candidate| candidate == resolved_path)
        })
    }

    pub fn find_kbuild_object_refs(&self, path: &str) -> Vec<&KbuildObjectReference> {
        let Some(relative) = normalize_object_subject(path) else {
            return Vec::new();
        };

        self.kbuild_object_refs
            .iter()
            .filter(|reference| reference.resolved_path == relative)
            .collect()
    }

    pub fn has_kbuild_object_ref(
        &self,
        file: &Path,
        line: usize,
        assignment_lhs: &str,
        object: &str,
        resolved_path: &Path,
    ) -> bool {
        self.kbuild_object_refs.iter().any(|reference| {
            reference.file == file
                && reference.line == line
                && reference.assignment_lhs == assignment_lhs
                && reference.object == object
                && reference.resolved_path == resolved_path
        })
    }
}

fn normalize_directory_subject(path: &str) -> Option<PathBuf> {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }

    Some(crate::kbuild::normalize_relative(Path::new(trimmed)))
}

fn normalize_object_subject(path: &str) -> Option<PathBuf> {
    let trimmed = path.trim();
    if trimmed.is_empty() || !trimmed.ends_with(".o") {
        return None;
    }

    Some(crate::kbuild::normalize_relative(Path::new(trimmed)))
}
