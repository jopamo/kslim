//! Conservative runtime-registration removal proof.
//!
//! Runtime registration macros and calls publish entry points into kernel
//! runtime dispatch tables. Removing a provider while a live entry point remains
//! is treated as unsafe. This scanner proves the registered identifiers are not
//! referenced by live C/ASM sources, and fails closed for malformed recognized
//! registration invocations in removed providers.

use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::model::RuntimeRegistrationSurface;
use crate::path_policy::normalized_relative_path_covers;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RuntimeRegistrationRemovalProof {
    pub provider: PathBuf,
    pub registration_macro: String,
    pub entry_points: Vec<String>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RuntimeRegistration {
    provider: PathBuf,
    registration_macro: String,
    entry_points: Vec<String>,
    line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MalformedRegistration {
    file: PathBuf,
    line: usize,
    registration_macro: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LiveEntryPointReference {
    file: PathBuf,
    line: usize,
    entry_point: String,
}

pub(crate) fn prove_removed_runtime_registrations_have_no_live_entry_points(
    root: &Path,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> Result<BTreeSet<RuntimeRegistrationRemovalProof>> {
    let source_files = source_files(root)?;
    let mut removed_registrations = BTreeSet::new();
    let mut live_sources = Vec::new();

    for relative in &source_files {
        if path_is_removed(relative, removed_paths, removed_dirs, removed_files) {
            let scan = scan_runtime_registrations_in_file(root, relative)?;
            if let Some(malformed) = scan.malformed.into_iter().next() {
                anyhow::bail!(
                    "runtime registration removal requires parsable entry-point proof; unsupported {} invocation in {}:{}",
                    malformed.registration_macro,
                    malformed.file.display(),
                    malformed.line,
                );
            }
            removed_registrations.extend(scan.registrations);
        } else {
            live_sources.push(relative.clone());
        }
    }

    let mut proofs = BTreeSet::new();
    for registration in removed_registrations {
        let live_references =
            live_references_for_entry_points(root, &live_sources, &registration.entry_points)?;
        if !live_references.is_empty() {
            anyhow::bail!(
                "runtime registration removal requires proof that no live entry point remains for {} in {}:{}; live reference(s): {}",
                registration.registration_macro,
                registration.provider.display(),
                registration.line,
                render_live_references(&live_references),
            );
        }
        proofs.insert(RuntimeRegistrationRemovalProof {
            provider: registration.provider,
            registration_macro: registration.registration_macro,
            entry_points: registration.entry_points,
            line: registration.line,
        });
    }

    Ok(proofs)
}

#[derive(Debug, Default)]
struct RuntimeRegistrationScan {
    registrations: BTreeSet<RuntimeRegistration>,
    malformed: BTreeSet<MalformedRegistration>,
}

fn scan_runtime_registrations_in_file(
    root: &Path,
    relative: &Path,
) -> Result<RuntimeRegistrationScan> {
    let content = std::fs::read_to_string(root.join(relative)).with_context(|| {
        format!(
            "failed to read removed runtime-registration provider {}",
            relative.display()
        )
    })?;
    Ok(scan_runtime_registrations_in_content(relative, &content))
}

fn scan_runtime_registrations_in_content(
    relative: &Path,
    content: &str,
) -> RuntimeRegistrationScan {
    let source = mask_c_comments_and_literals(content);
    let mut scan = RuntimeRegistrationScan::default();
    let mut offset = 0usize;
    let mut line = 1usize;

    while let Some((start, token, token_line)) = next_identifier(&source, offset, line) {
        line = token_line;
        offset = start + token.len();
        if !RuntimeRegistrationSurface::is_known_registration_macro(token) {
            continue;
        }

        let Some((entry_points, end)) = parse_registration_entry_points(&source, offset) else {
            scan.malformed.insert(MalformedRegistration {
                file: relative.to_path_buf(),
                line: token_line,
                registration_macro: token.to_string(),
            });
            continue;
        };
        offset = end;
        scan.registrations.insert(RuntimeRegistration {
            provider: relative.to_path_buf(),
            registration_macro: token.to_string(),
            entry_points,
            line: token_line,
        });
    }

    scan
}

fn parse_registration_entry_points(source: &str, offset: usize) -> Option<(Vec<String>, usize)> {
    let mut cursor = skip_ascii_whitespace(source, offset);
    if !source[cursor..].starts_with('(') {
        return None;
    }
    cursor += 1;

    let mut entry_points = BTreeSet::new();
    loop {
        cursor = skip_ascii_whitespace(source, cursor);
        if source[cursor..].starts_with(')') {
            cursor += 1;
            break;
        }
        while source[cursor..].starts_with('&') || source[cursor..].starts_with('*') {
            cursor += 1;
            cursor = skip_ascii_whitespace(source, cursor);
        }
        let Some((identifier, end)) = parse_c_identifier(source, cursor) else {
            return None;
        };
        entry_points.insert(identifier.to_string());
        cursor = skip_ascii_whitespace(source, end);
        let ch = source[cursor..].chars().next()?;
        cursor += ch.len_utf8();
        match ch {
            ',' => continue,
            ')' => break,
            _ => return None,
        }
    }

    if entry_points.is_empty() {
        return None;
    }
    Some((entry_points.into_iter().collect(), cursor))
}

fn live_references_for_entry_points(
    root: &Path,
    live_sources: &[PathBuf],
    entry_points: &[String],
) -> Result<BTreeSet<LiveEntryPointReference>> {
    let mut references = BTreeSet::new();
    for relative in live_sources {
        let content = std::fs::read_to_string(root.join(relative)).with_context(|| {
            format!(
                "failed to read live source while proving no runtime entry point remains: {}",
                relative.display(),
            )
        })?;
        for entry_point in entry_points {
            for line in identifier_occurrence_lines(&content, entry_point) {
                references.insert(LiveEntryPointReference {
                    file: relative.clone(),
                    line,
                    entry_point: entry_point.clone(),
                });
            }
        }
    }
    Ok(references)
}

fn source_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry.path().strip_prefix(root).with_context(|| {
            format!(
                "failed to derive root-relative runtime-registration scan path for {}",
                entry.path().display()
            )
        })?;
        if is_c_or_asm_source_path(relative) {
            files.push(relative.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn is_c_or_asm_source_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("c" | "h" | "S" | "s" | "cc" | "cpp" | "cxx" | "hpp")
    )
}

fn path_is_removed(
    path: &Path,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> bool {
    removed_paths.contains(path)
        || removed_files.contains(path)
        || removed_dirs
            .iter()
            .any(|dir| normalized_relative_path_covers(dir, path))
}

fn identifier_occurrence_lines(content: &str, symbol: &str) -> BTreeSet<usize> {
    let source = mask_c_comments_and_literals(content);
    let mut lines = BTreeSet::new();
    let mut offset = 0usize;
    let mut line = 1usize;

    while let Some((start, token, token_line)) = next_identifier(&source, offset, line) {
        line = token_line;
        offset = start + token.len();
        if token == symbol {
            lines.insert(token_line);
        }
    }

    lines
}

fn next_identifier(
    source: &str,
    mut offset: usize,
    mut line: usize,
) -> Option<(usize, &str, usize)> {
    while offset < source.len() {
        let ch = source[offset..].chars().next()?;
        if ch == '\n' {
            line += 1;
            offset += 1;
            continue;
        }
        if is_c_identifier_start(ch) {
            let start = offset;
            offset += ch.len_utf8();
            while offset < source.len() {
                let ch = source[offset..].chars().next()?;
                if !is_c_identifier_continue(ch) {
                    break;
                }
                offset += ch.len_utf8();
            }
            return Some((start, &source[start..offset], line));
        }
        offset += ch.len_utf8();
    }
    None
}

fn parse_c_identifier(source: &str, offset: usize) -> Option<(&str, usize)> {
    let mut chars = source[offset..].char_indices();
    let (_, first) = chars.next()?;
    if !is_c_identifier_start(first) {
        return None;
    }

    let mut end = offset + first.len_utf8();
    for (idx, ch) in chars {
        if !is_c_identifier_continue(ch) {
            break;
        }
        end = offset + idx + ch.len_utf8();
    }
    Some((&source[offset..end], end))
}

fn skip_ascii_whitespace(source: &str, mut offset: usize) -> usize {
    while offset < source.len() {
        let byte = source.as_bytes()[offset];
        if !byte.is_ascii_whitespace() {
            break;
        }
        offset += 1;
    }
    offset
}

fn is_c_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_c_identifier_continue(ch: char) -> bool {
    is_c_identifier_start(ch) || ch.is_ascii_digit()
}

fn mask_c_comments_and_literals(content: &str) -> String {
    #[derive(Clone, Copy)]
    enum State {
        Normal,
        LineComment,
        BlockComment,
        StringLiteral,
        CharLiteral,
    }

    let mut out = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    let mut state = State::Normal;

    while let Some(ch) = chars.next() {
        match state {
            State::Normal if ch == '/' && chars.peek() == Some(&'/') => {
                out.push(' ');
                out.push(' ');
                chars.next();
                state = State::LineComment;
            }
            State::Normal if ch == '/' && chars.peek() == Some(&'*') => {
                out.push(' ');
                out.push(' ');
                chars.next();
                state = State::BlockComment;
            }
            State::Normal if ch == '"' => {
                out.push(' ');
                state = State::StringLiteral;
            }
            State::Normal if ch == '\'' => {
                out.push(' ');
                state = State::CharLiteral;
            }
            State::Normal => out.push(ch),
            State::LineComment if ch == '\n' => {
                out.push('\n');
                state = State::Normal;
            }
            State::LineComment => out.push(' '),
            State::BlockComment if ch == '*' && chars.peek() == Some(&'/') => {
                out.push(' ');
                out.push(' ');
                chars.next();
                state = State::Normal;
            }
            State::BlockComment if ch == '\n' => out.push('\n'),
            State::BlockComment => out.push(' '),
            State::StringLiteral if ch == '\\' => {
                out.push(' ');
                if let Some(escaped) = chars.next() {
                    out.push(if escaped == '\n' { '\n' } else { ' ' });
                }
            }
            State::StringLiteral if ch == '"' => {
                out.push(' ');
                state = State::Normal;
            }
            State::StringLiteral if ch == '\n' => {
                out.push('\n');
                state = State::Normal;
            }
            State::StringLiteral => out.push(' '),
            State::CharLiteral if ch == '\\' => {
                out.push(' ');
                if let Some(escaped) = chars.next() {
                    out.push(if escaped == '\n' { '\n' } else { ' ' });
                }
            }
            State::CharLiteral if ch == '\'' => {
                out.push(' ');
                state = State::Normal;
            }
            State::CharLiteral if ch == '\n' => {
                out.push('\n');
                state = State::Normal;
            }
            State::CharLiteral => out.push(' '),
        }
    }

    out
}

fn render_live_references(references: &BTreeSet<LiveEntryPointReference>) -> String {
    references
        .iter()
        .take(8)
        .map(|reference| {
            format!(
                "{}:{}:{}",
                reference.file.display(),
                reference.line,
                reference.entry_point,
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_runtime_registrations_ignores_comments_and_strings() {
        let scan = scan_runtime_registrations_in_content(
            Path::new("drivers/foo/provider.c"),
            concat!(
                "// module_init(commented_out)\n",
                "const char *s = \"module_init(in_string)\";\n",
                "static int real_init(void) { return 0; }\n",
                "module_init(real_init);\n",
            ),
        );

        assert!(scan.malformed.is_empty());
        assert_eq!(
            scan.registrations
                .iter()
                .map(|registration| {
                    (
                        registration.registration_macro.as_str(),
                        registration.entry_points.clone(),
                        registration.line,
                    )
                })
                .collect::<Vec<_>>(),
            vec![("module_init", vec![String::from("real_init")], 4)]
        );
    }

    #[test]
    fn test_scan_runtime_registrations_collects_multiple_entry_points() {
        let scan = scan_runtime_registrations_in_content(
            Path::new("drivers/foo/provider.c"),
            "module_platform_driver_probe(foo_driver, foo_probe);\nplatform_driver_register(&bar_driver);\n",
        );

        assert!(scan.malformed.is_empty());
        assert_eq!(
            scan.registrations
                .iter()
                .map(|registration| registration.entry_points.clone())
                .collect::<Vec<_>>(),
            vec![
                vec![String::from("foo_driver"), String::from("foo_probe")],
                vec![String::from("bar_driver")],
            ]
        );
    }

    #[test]
    fn test_prove_removed_runtime_registration_rejects_live_entry_point() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "drivers/foo/provider.c",
            "static int foo_init(void) { return 0; }\nmodule_init(foo_init);\n",
        );
        write(
            root,
            "drivers/live/user.c",
            "extern int foo_init(void);\nint call(void) { return foo_init(); }\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("drivers/foo/provider.c")]);
        let removed_files = removed_paths.clone();

        let err = format!(
            "{:#}",
            prove_removed_runtime_registrations_have_no_live_entry_points(
                root,
                &removed_paths,
                &BTreeSet::new(),
                &removed_files,
            )
            .unwrap_err()
        );

        assert!(err.contains("runtime registration removal requires proof"));
        assert!(err.contains("drivers/live/user.c"));
        assert!(err.contains("foo_init"));
    }

    #[test]
    fn test_prove_removed_runtime_registration_allows_only_removed_entry_points() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "drivers/foo/provider.c",
            "static int foo_init(void) { return 0; }\nmodule_init(foo_init);\n",
        );
        write(
            root,
            "drivers/foo/internal.c",
            "extern int foo_init(void);\nint call(void) { return foo_init(); }\n",
        );
        write(root, "drivers/live/user.c", "int live;\n");
        let removed_paths = BTreeSet::from([PathBuf::from("drivers/foo")]);
        let removed_dirs = removed_paths.clone();

        let proofs = prove_removed_runtime_registrations_have_no_live_entry_points(
            root,
            &removed_paths,
            &removed_dirs,
            &BTreeSet::new(),
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([RuntimeRegistrationRemovalProof {
                provider: PathBuf::from("drivers/foo/provider.c"),
                registration_macro: String::from("module_init"),
                entry_points: vec![String::from("foo_init")],
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_runtime_registration_rejects_malformed_entry_point() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "drivers/foo/provider.c", "module_init();\n");
        let removed_paths = BTreeSet::from([PathBuf::from("drivers/foo/provider.c")]);
        let removed_files = removed_paths.clone();

        let err = format!(
            "{:#}",
            prove_removed_runtime_registrations_have_no_live_entry_points(
                root,
                &removed_paths,
                &BTreeSet::new(),
                &removed_files,
            )
            .unwrap_err()
        );

        assert!(err.contains("parsable entry-point proof"));
        assert!(err.contains("drivers/foo/provider.c:1"));
    }

    #[test]
    fn test_prove_removed_runtime_registration_rejects_unsupported_expression() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "drivers/foo/provider.c",
            "module_init(foo_init + 1);\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("drivers/foo/provider.c")]);
        let removed_files = removed_paths.clone();

        let err = format!(
            "{:#}",
            prove_removed_runtime_registrations_have_no_live_entry_points(
                root,
                &removed_paths,
                &BTreeSet::new(),
                &removed_files,
            )
            .unwrap_err()
        );

        assert!(err.contains("parsable entry-point proof"));
        assert!(err.contains("drivers/foo/provider.c:1"));
    }

    fn write(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }
}
