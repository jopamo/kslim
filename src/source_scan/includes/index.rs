//! Include-site indexing for C-family source files.
//!
//! This module owns discovery and parsing of visible `#include` directives.
//! Cleanup and policy decisions consume the indexed sites instead of walking
//! source files themselves.

use anyhow::Result;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum IncludeKind {
    Quoted,
    Angle,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct IncludeSite {
    pub file: PathBuf,
    pub line: usize,
    pub header: String,
    pub kind: IncludeKind,
}

#[allow(dead_code)]
pub(crate) fn index_include_sites(root: &Path) -> Result<Vec<IncludeSite>> {
    let mut sites = Vec::new();

    for path in c_family_files(root) {
        let content = std::fs::read_to_string(&path)?;
        let lines = content.lines().collect::<Vec<_>>();
        let directive_visible = crate::source_scan::cpp::visible_cpp_directive_lines(&lines);
        for (idx, line) in lines.iter().enumerate() {
            if !directive_visible[idx] {
                continue;
            }
            let Some((kind, header)) = parse_include_site(line) else {
                continue;
            };
            sites.push(IncludeSite {
                file: relative_to_root_path(root, &path),
                line: idx + 1,
                header: header.to_string(),
                kind,
            });
        }
    }

    sites.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then(left.line.cmp(&right.line))
            .then(left.header.cmp(&right.header))
            .then(left.kind.cmp(&right.kind))
    });
    Ok(sites)
}

#[allow(dead_code)]
pub(in crate::source_scan::includes) fn parse_include_site(line: &str) -> Option<(IncludeKind, &str)> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix('#')?.trim_start();
    let rest = rest.strip_prefix("include")?;
    if !rest
        .chars()
        .next()
        .is_some_and(|ch| ch.is_whitespace() || ch == '<' || ch == '"')
    {
        return None;
    }

    let rest = rest.trim_start();
    if let Some(header) = parse_delimited_include(rest, '"', '"') {
        return Some((IncludeKind::Quoted, header));
    }
    if let Some(header) = parse_delimited_include(rest, '<', '>') {
        return Some((IncludeKind::Angle, header));
    }
    None
}

#[allow(dead_code)]
fn parse_delimited_include<'a>(line: &'a str, start: char, end: char) -> Option<&'a str> {
    let body = line.strip_prefix(start)?;
    let end_idx = body.find(end)?;
    let header = &body[..end_idx];
    if header.trim().is_empty() {
        return None;
    }

    let trailing = body[end_idx + end.len_utf8()..].trim_start();
    if trailing.is_empty() || trailing.starts_with("//") || trailing.starts_with("/*") {
        return Some(header);
    }
    None
}

#[allow(dead_code)]
pub(in crate::source_scan::includes) fn c_family_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        if entry
            .path()
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| matches!(ext, "c" | "h" | "S" | "s" | "cc" | "cpp" | "cxx"))
        {
            out.push(entry.into_path());
        }
    }
    out.sort();
    out
}

#[allow(dead_code)]
pub(in crate::source_scan::includes) fn relative_to_root_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
