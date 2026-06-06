//! Conservative exported-symbol removal proof.
//!
//! Removing a file that provides an `EXPORT_SYMBOL*()` entry can break live
//! consumers outside the removed subtree. This scanner is intentionally simple:
//! it proves absence of live textual C/ASM consumers, and fails closed when a
//! removed provider uses an unsupported export form.

use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::model::ExportedSymbol;
use crate::path_policy::normalized_relative_path_covers;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ExportedSymbolRemovalProof {
    pub symbol: ExportedSymbol,
    pub provider: PathBuf,
    pub export_macro: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ExportedSymbolDefinition {
    symbol: ExportedSymbol,
    provider: PathBuf,
    export_macro: String,
    line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MalformedExport {
    file: PathBuf,
    line: usize,
    export_macro: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LiveSymbolConsumer {
    file: PathBuf,
    line: usize,
    symbol: ExportedSymbol,
}

pub(crate) fn prove_removed_exports_have_no_live_consumers(
    root: &Path,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> Result<BTreeSet<ExportedSymbolRemovalProof>> {
    let source_files = source_files(root)?;
    let mut removed_exports = BTreeSet::new();
    let mut live_sources = Vec::new();

    for relative in &source_files {
        if path_is_removed(relative, removed_paths, removed_dirs, removed_files) {
            let scan = scan_exported_symbols_in_file(root, relative)?;
            if let Some(malformed) = scan.malformed.into_iter().next() {
                anyhow::bail!(
                    "exported symbol provider removal requires parsable EXPORT_SYMBOL proof; unsupported {} invocation in {}:{}",
                    malformed.export_macro,
                    malformed.file.display(),
                    malformed.line,
                );
            }
            removed_exports.extend(scan.definitions);
        } else {
            live_sources.push(relative.clone());
        }
    }

    let mut proofs = BTreeSet::new();
    for export in removed_exports {
        let consumers = live_consumers_for_symbol(root, &live_sources, &export.symbol)?;
        if !consumers.is_empty() {
            anyhow::bail!(
                "exported symbol provider removal requires proof that no live consumer remains for '{}' exported by {}:{}; live consumer(s): {}",
                export.symbol.as_str(),
                export.provider.display(),
                export.line,
                render_consumers(&consumers),
            );
        }
        proofs.insert(ExportedSymbolRemovalProof {
            symbol: export.symbol,
            provider: export.provider,
            export_macro: export.export_macro,
            line: export.line,
        });
    }

    Ok(proofs)
}

#[derive(Debug, Default)]
struct ExportScan {
    definitions: BTreeSet<ExportedSymbolDefinition>,
    malformed: BTreeSet<MalformedExport>,
}

fn scan_exported_symbols_in_file(root: &Path, relative: &Path) -> Result<ExportScan> {
    let content = std::fs::read_to_string(root.join(relative)).with_context(|| {
        format!(
            "failed to read exported-symbol provider {}",
            relative.display()
        )
    })?;
    Ok(scan_exported_symbols_in_content(relative, &content))
}

fn scan_exported_symbols_in_content(relative: &Path, content: &str) -> ExportScan {
    let source = mask_c_comments_and_literals(content);
    let mut scan = ExportScan::default();
    let mut offset = 0usize;
    let mut line = 1usize;

    while let Some((start, token, token_line)) = next_identifier(&source, offset, line) {
        line = token_line;
        offset = start + token.len();
        if !is_export_macro(token) {
            continue;
        }

        let after_token = skip_ascii_whitespace(&source, offset);
        let Some(after_open) = source[after_token..]
            .strip_prefix('(')
            .map(|_| after_token + 1)
        else {
            scan.malformed.insert(MalformedExport {
                file: relative.to_path_buf(),
                line: token_line,
                export_macro: token.to_string(),
            });
            continue;
        };
        let first_arg = skip_ascii_whitespace(&source, after_open);
        let Some((symbol, _end)) = parse_c_identifier(&source, first_arg) else {
            scan.malformed.insert(MalformedExport {
                file: relative.to_path_buf(),
                line: token_line,
                export_macro: token.to_string(),
            });
            continue;
        };
        scan.definitions.insert(ExportedSymbolDefinition {
            symbol: ExportedSymbol::new(symbol)
                .expect("parse_c_identifier should return valid exported symbol"),
            provider: relative.to_path_buf(),
            export_macro: token.to_string(),
            line: token_line,
        });
    }

    scan
}

fn live_consumers_for_symbol(
    root: &Path,
    live_sources: &[PathBuf],
    symbol: &ExportedSymbol,
) -> Result<BTreeSet<LiveSymbolConsumer>> {
    let mut consumers = BTreeSet::new();
    for relative in live_sources {
        let content = std::fs::read_to_string(root.join(relative)).with_context(|| {
            format!(
                "failed to read live source while proving no consumers for exported symbol '{}': {}",
                symbol.as_str(),
                relative.display(),
            )
        })?;
        for line in identifier_occurrence_lines(&content, symbol.as_str()) {
            consumers.insert(LiveSymbolConsumer {
                file: relative.clone(),
                line,
                symbol: symbol.clone(),
            });
        }
    }
    Ok(consumers)
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
                "failed to derive root-relative exported-symbol scan path for {}",
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

fn is_export_macro(token: &str) -> bool {
    matches!(
        token,
        "EXPORT_SYMBOL"
            | "EXPORT_SYMBOL_GPL"
            | "EXPORT_SYMBOL_GPL_FUTURE"
            | "EXPORT_SYMBOL_NS"
            | "EXPORT_SYMBOL_NS_GPL"
    )
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

fn render_consumers(consumers: &BTreeSet<LiveSymbolConsumer>) -> String {
    consumers
        .iter()
        .take(8)
        .map(|consumer| format!("{}:{}", consumer.file.display(), consumer.line,))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_exported_symbols_ignores_comments_and_strings() {
        let scan = scan_exported_symbols_in_content(
            Path::new("drivers/foo/provider.c"),
            concat!(
                "// EXPORT_SYMBOL(commented_out)\n",
                "const char *s = \"EXPORT_SYMBOL(in_string)\";\n",
                "void real(void) {}\n",
                "EXPORT_SYMBOL_GPL(real);\n",
            ),
        );

        assert!(scan.malformed.is_empty());
        assert_eq!(
            scan.definitions
                .iter()
                .map(|definition| (definition.symbol.as_str(), definition.line))
                .collect::<Vec<_>>(),
            vec![("real", 4)]
        );
    }

    #[test]
    fn test_prove_removed_exports_rejects_live_consumer() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "drivers/foo/provider.c",
            "void foo_api(void) {}\nEXPORT_SYMBOL(foo_api);\n",
        );
        write(
            root,
            "drivers/live/user.c",
            "extern void foo_api(void);\nvoid user(void) { foo_api(); }\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("drivers/foo/provider.c")]);
        let removed_files = removed_paths.clone();

        let err = format!(
            "{:#}",
            prove_removed_exports_have_no_live_consumers(
                root,
                &removed_paths,
                &BTreeSet::new(),
                &removed_files,
            )
            .unwrap_err()
        );

        assert!(err.contains("exported symbol provider removal requires proof"));
        assert!(err.contains("foo_api"));
        assert!(err.contains("drivers/live/user.c"));
    }

    #[test]
    fn test_prove_removed_exports_allows_only_removed_consumers() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "drivers/foo/provider.c",
            "void foo_api(void) {}\nEXPORT_SYMBOL_NS(foo_api, NS);\n",
        );
        write(
            root,
            "drivers/foo/user.c",
            "extern void foo_api(void);\nvoid user(void) { foo_api(); }\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("drivers/foo")]);
        let removed_dirs = removed_paths.clone();

        let proofs = prove_removed_exports_have_no_live_consumers(
            root,
            &removed_paths,
            &removed_dirs,
            &BTreeSet::new(),
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([ExportedSymbolRemovalProof {
                symbol: ExportedSymbol::new("foo_api").unwrap(),
                provider: PathBuf::from("drivers/foo/provider.c"),
                export_macro: String::from("EXPORT_SYMBOL_NS"),
                line: 2,
            }])
        );
    }

    fn write(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }
}
