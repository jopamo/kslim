//! Kconfig facts for the read-only tree index.
//!
//! This module owns Kconfig file detection, symbol definition indexing,
//! directive symbol-reference indexing, and Kconfig source-reference indexing.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::file_index::{is_host_absolute_path_like, FileIndex};

pub type KconfigFileIndex = BTreeSet<PathBuf>;
pub type KconfigDefinitionIndex = BTreeSet<KconfigDefinition>;
pub type KconfigReferenceIndex = BTreeSet<KconfigSymbolReference>;
pub type KconfigSourceIndex = BTreeSet<KconfigSourceReference>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct KconfigDefinition {
    pub file: PathBuf,
    pub line: usize,
    pub symbol: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct KconfigSymbolReference {
    pub file: PathBuf,
    pub line: usize,
    pub directive: String,
    pub symbol: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct KconfigSourceReference {
    pub file: PathBuf,
    pub line: usize,
    pub source: String,
    pub optional: bool,
    pub relative: bool,
}

#[derive(Debug, Default)]
pub(in crate::index) struct KconfigFileFacts {
    pub files: KconfigFileIndex,
    pub definitions: KconfigDefinitionIndex,
    pub references: KconfigReferenceIndex,
    pub sources: KconfigSourceIndex,
}

pub(in crate::index) fn kconfig_files_from_indexed_files(
    root: &Path,
    files: &FileIndex,
) -> Vec<PathBuf> {
    let mut out = files
        .iter()
        .filter(|path| is_kconfig_path(path))
        .map(|path| root.join(path))
        .collect::<Vec<_>>();
    out.sort();
    out
}

pub(in crate::index) fn scan_kconfig_file(relative: &Path, path: &Path) -> KconfigFileFacts {
    let mut facts = KconfigFileFacts::default();
    if !is_kconfig_path(relative) {
        return facts;
    }
    facts.files.insert(relative.to_path_buf());

    let Ok(content) = std::fs::read_to_string(path) else {
        return facts;
    };
    for (line_idx, line) in content.lines().enumerate() {
        let line_number = line_idx + 1;
        if let Some(symbol) = parse_kconfig_definition(line) {
            facts.definitions.insert(KconfigDefinition {
                file: relative.to_path_buf(),
                line: line_number,
                symbol,
            });
        }
        if let Some((directive, symbols)) = parse_kconfig_symbol_refs(line) {
            for symbol in symbols {
                facts.references.insert(KconfigSymbolReference {
                    file: relative.to_path_buf(),
                    line: line_number,
                    directive: directive.clone(),
                    symbol,
                });
            }
        }
        let Some(source) = crate::kconfig::parse_kconfig_source(line) else {
            continue;
        };
        if is_host_absolute_path_like(&source.path) {
            continue;
        }
        facts.sources.insert(KconfigSourceReference {
            file: relative.to_path_buf(),
            line: line_number,
            source: source.path,
            optional: source.optional,
            relative: source.relative,
        });
    }
    facts
}

fn is_kconfig_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "Kconfig" || name.starts_with("Kconfig."))
}

fn parse_kconfig_definition(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    for keyword in ["config", "menuconfig"] {
        if let Some(rest) = trimmed.strip_prefix(keyword) {
            if !rest.starts_with(char::is_whitespace) {
                continue;
            }
            let symbol = rest.split_whitespace().next()?;
            return is_symbol_token(symbol).then(|| symbol.to_string());
        }
    }
    None
}

fn parse_kconfig_symbol_refs(line: &str) -> Option<(String, BTreeSet<String>)> {
    let trimmed = line.trim_start();
    let (directive, expression) = if let Some(expr) = trimmed.strip_prefix("depends on ") {
        ("depends_on", expr)
    } else if let Some(expr) = trimmed.strip_prefix("select ") {
        ("select", expr)
    } else if let Some(expr) = trimmed.strip_prefix("imply ") {
        ("imply", expr)
    } else if let Some(expr) = trimmed.strip_prefix("visible if ") {
        ("visible_if", expr)
    } else if let Some(expr) = trimmed.strip_prefix("if ") {
        ("if", expr)
    } else if let Some(expr) = trimmed.strip_prefix("default ") {
        ("default", expr)
    } else {
        return None;
    };

    let symbols = collect_symbol_tokens(expression);
    (!symbols.is_empty()).then(|| (directive.to_string(), symbols))
}

pub(in crate::index) fn collect_symbol_tokens(input: &str) -> BTreeSet<String> {
    let mut symbols = BTreeSet::new();
    let mut token = String::new();

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            token.push(ch);
            continue;
        }
        insert_symbol_token(&mut symbols, &token);
        token.clear();
    }
    insert_symbol_token(&mut symbols, &token);

    symbols
}

fn insert_symbol_token(symbols: &mut BTreeSet<String>, token: &str) {
    let Some(symbol) = normalize_symbol_token(token) else {
        return;
    };
    symbols.insert(symbol);
}

fn normalize_symbol_token(token: &str) -> Option<String> {
    let token = token.trim();
    if !is_symbol_token(token) || is_non_symbol_keyword(token) {
        return None;
    }
    let symbol = token.strip_prefix("CONFIG_").unwrap_or(token);
    if is_non_symbol_keyword(symbol) {
        return None;
    }
    Some(symbol.to_string())
}

fn is_symbol_token(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_non_symbol_keyword(token: &str) -> bool {
    matches!(
        token,
        "if" | "on" | "y" | "m" | "n" | "defined" | "IS_ENABLED" | "IS_REACHABLE" | "CONFIG"
    )
}
