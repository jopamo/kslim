//! Kbuild object graph indexing and path-resolution helpers.
//!
//! This module owns read-only Kbuild graph facts: source/object providers,
//! object references, composite members, directory references, config-gated
//! references, and include-path flags. Rewrite and selftest code consume these
//! facts and path helpers instead of rebuilding ad hoc graph state.

use anyhow::Result;
use std::collections::{BTreeSet, HashSet};
use std::path::{Component, Path, PathBuf};

use super::{
    logical_lines, parse_kbuild_assignment, parse_kbuild_assignment_kind,
    protected_make_logical_line_starts, CompositeKind, KbuildAssignment, KbuildAssignmentKind,
    ObjListKind,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct KbuildIndex {
    pub object_providers: BTreeSet<PathBuf>,
    pub object_references: Vec<KbuildObjectReference>,
    pub composite_object_members: Vec<KbuildCompositeObjectMember>,
    pub directory_references: Vec<KbuildDirectoryReference>,
    pub config_gated_references: Vec<KbuildConfigGatedReference>,
    pub include_path_flags: Vec<KbuildIncludePathFlag>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KbuildObjectReference {
    pub file: PathBuf,
    pub line: usize,
    pub assignment_lhs: String,
    pub object: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KbuildCompositeObjectMember {
    pub file: PathBuf,
    pub line: usize,
    pub target: PathBuf,
    pub member: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KbuildDirectoryReference {
    pub file: PathBuf,
    pub line: usize,
    pub assignment_lhs: String,
    pub directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KbuildConfigGatedReference {
    pub file: PathBuf,
    pub line: usize,
    pub assignment_lhs: String,
    pub symbol: String,
    pub reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KbuildIncludePathFlag {
    pub file: PathBuf,
    pub line: usize,
    pub flag: String,
    pub include_path: String,
}


pub(in crate::kbuild) fn relative_to_root(
    root: &Path,
    current_dir: &Path,
    token: &str,
) -> PathBuf {
    let absolute = normalize_relative(&current_dir.join(token));
    absolute
        .strip_prefix(root)
        .unwrap_or(&absolute)
        .to_path_buf()
}

pub(in crate::kbuild) fn relative_to_root_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn is_source_like(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext,
                "c" | "S" | "s" | "rs" | "cc" | "cpp" | "cxx" | "m" | "mm"
            )
        })
}

pub(crate) fn build_kbuild_index(root: &Path) -> Result<KbuildIndex> {
    let mut index = KbuildIndex::default();
    index_source_object_providers(root, &mut index.object_providers)?;

    for path in makefiles(root) {
        let content = std::fs::read_to_string(&path)?;
        let logical = logical_lines(&content);
        let protected_lines = protected_make_logical_line_starts(&logical);
        let current_dir = path.parent().unwrap_or(root);
        let relative_file = relative_to_root_path(root, &path);

        for line in &logical {
            if protected_lines.contains(&line.start_line) {
                continue;
            }
            let Some(assignment) = parse_kbuild_assignment(&line.joined) else {
                continue;
            };
            index_assignment(
                root,
                current_dir,
                &relative_file,
                line.start_line,
                &assignment,
                &mut index,
            );
        }
    }

    Ok(index)
}

fn index_source_object_providers(root: &Path, providers: &mut BTreeSet<PathBuf>) -> Result<()> {
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if let Some(provider) = source_object_provider(root, entry.path()) {
            providers.insert(provider);
        }
    }
    Ok(())
}

fn source_object_provider(root: &Path, path: &Path) -> Option<PathBuf> {
    let relative = relative_to_root_path(root, path);
    object_provider_path(&relative)
}

pub(in crate::kbuild) fn object_provider_path(path: &Path) -> Option<PathBuf> {
    if is_source_like(path) {
        return Some(path.with_extension("o"));
    }
    let text = path.to_string_lossy();
    let stem = text.strip_suffix(".o_shipped")?;
    Some(PathBuf::from(format!("{stem}.o")))
}

fn index_assignment(
    root: &Path,
    current_dir: &Path,
    relative_file: &Path,
    start_line: usize,
    assignment: &KbuildAssignment<'_>,
    index: &mut KbuildIndex,
) {
    match &assignment.kind {
        KbuildAssignmentKind::ObjList(kind) => {
            index_tokens(
                root,
                current_dir,
                relative_file,
                start_line,
                assignment,
                None,
                config_symbol_from_obj_list(kind),
                index,
            );
        }
        KbuildAssignmentKind::CompositeMembers(kind) => {
            index.object_providers.insert(relative_to_root(
                root,
                current_dir,
                &format!("{}.o", kind.target()),
            ));
            index_tokens(
                root,
                current_dir,
                relative_file,
                start_line,
                assignment,
                Some(kind.target()),
                config_symbol_from_composite(kind),
                index,
            );
        }
        KbuildAssignmentKind::SubdirList => {
            index_tokens(
                root,
                current_dir,
                relative_file,
                start_line,
                assignment,
                None,
                None,
                index,
            );
        }
        KbuildAssignmentKind::CcFlags => {
            for token in assignment.rhs.split_whitespace() {
                if let Some(include_path) = token.strip_prefix("-I").filter(|path| !path.is_empty())
                {
                    index.include_path_flags.push(KbuildIncludePathFlag {
                        file: relative_file.to_path_buf(),
                        line: start_line,
                        flag: token.to_string(),
                        include_path: include_path.to_string(),
                    });
                }
            }
        }
    }
}

fn index_tokens(
    root: &Path,
    current_dir: &Path,
    relative_file: &Path,
    start_line: usize,
    assignment: &KbuildAssignment<'_>,
    composite_target: Option<&str>,
    config_symbol: Option<&str>,
    index: &mut KbuildIndex,
) {
    for token in assignment.rhs.split_whitespace() {
        if token.ends_with(".o") {
            index.object_references.push(KbuildObjectReference {
                file: relative_file.to_path_buf(),
                line: start_line,
                assignment_lhs: assignment.lhs.to_string(),
                object: token.to_string(),
            });
            if let Some(target) = composite_target {
                index
                    .composite_object_members
                    .push(KbuildCompositeObjectMember {
                        file: relative_file.to_path_buf(),
                        line: start_line,
                        target: relative_to_root(root, current_dir, &format!("{}.o", target)),
                        member: token.to_string(),
                    });
            }
            if let Some(symbol) = config_symbol {
                index
                    .config_gated_references
                    .push(KbuildConfigGatedReference {
                        file: relative_file.to_path_buf(),
                        line: start_line,
                        assignment_lhs: assignment.lhs.to_string(),
                        symbol: symbol.to_string(),
                        reference: token.to_string(),
                    });
            }
        } else if token.ends_with('/') {
            index.directory_references.push(KbuildDirectoryReference {
                file: relative_file.to_path_buf(),
                line: start_line,
                assignment_lhs: assignment.lhs.to_string(),
                directory: token.to_string(),
            });
            if let Some(symbol) = config_symbol {
                index
                    .config_gated_references
                    .push(KbuildConfigGatedReference {
                        file: relative_file.to_path_buf(),
                        line: start_line,
                        assignment_lhs: assignment.lhs.to_string(),
                        symbol: symbol.to_string(),
                        reference: token.to_string(),
                    });
            }
        }
    }
}

fn config_symbol_from_obj_list<'a>(kind: &'a ObjListKind<'a>) -> Option<&'a str> {
    match kind {
        ObjListKind::Config(symbol) => Some(*symbol),
        _ => None,
    }
}

fn config_symbol_from_composite<'a>(kind: &'a CompositeKind<'a>) -> Option<&'a str> {
    match kind {
        CompositeKind::Config { symbol, .. } => Some(*symbol),
        _ => None,
    }
}

pub(crate) fn is_build_graph_assignment(lhs: &str) -> bool {
    parse_kbuild_assignment_kind(lhs).is_some()
}

pub(crate) fn has_object_provider(
    current_dir: &Path,
    token: &str,
    composite_objects: &HashSet<String>,
) -> bool {
    if composite_objects.contains(token) {
        return true;
    }

    has_direct_object_provider(current_dir, token)
}

pub(in crate::kbuild) fn has_direct_object_provider(current_dir: &Path, token: &str) -> bool {
    let Some(stem) = token.strip_suffix(".o") else {
        return false;
    };

    if current_dir.join(token).exists() || current_dir.join(format!("{}.o_shipped", stem)).exists()
    {
        return true;
    }

    for ext in ["c", "S", "s", "rs", "cc", "cpp", "cxx", "m", "mm"] {
        if current_dir.join(format!("{}.{}", stem, ext)).exists() {
            return true;
        }
    }

    false
}

pub(crate) fn make_dir_candidates(root: &Path, current_dir: &Path, token: &str) -> Vec<PathBuf> {
    let Some(token) = token.strip_suffix('/') else {
        return Vec::new();
    };
    if token.is_empty() || token.starts_with('/') {
        return Vec::new();
    }

    let mut out = Vec::new();

    for candidate in [current_dir.join(token), root.join(token)] {
        let normalized = normalize_relative(&candidate);
        let Ok(relative) = normalized.strip_prefix(root) else {
            continue;
        };
        let relative = relative.to_path_buf();
        if !out.contains(&relative) {
            out.push(relative);
        }
    }

    out.sort();
    out.dedup();
    out
}

pub(in crate::kbuild) fn include_path_candidates(
    root: &Path,
    current_dir: &Path,
    include_path: &str,
) -> Vec<PathBuf> {
    if include_path.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();

    for candidate in [current_dir.join(include_path), root.join(include_path)] {
        let normalized = normalize_relative(&candidate);
        let Ok(relative) = normalized.strip_prefix(root) else {
            continue;
        };
        let relative = relative.to_path_buf();
        if !out.contains(&relative) {
            out.push(relative);
        }
    }

    out.sort();
    out.dedup();
    out
}

pub(crate) fn makefiles(root: &Path) -> Vec<PathBuf> {
    walk_named(root, &["Makefile", "Kbuild"])
}

pub(crate) fn normalize_relative(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(seg) => out.push(seg),
            Component::RootDir | Component::Prefix(_) => {
                out = PathBuf::from(component.as_os_str());
            }
        }
    }

    out
}

fn walk_named(root: &Path, names: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if entry
            .file_name()
            .to_str()
            .is_some_and(|name| names.iter().any(|candidate| candidate == &name))
        {
            out.push(entry.into_path());
        }
    }
    out.sort();
    out
}
