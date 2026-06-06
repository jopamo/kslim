//! C-family source facts for the read-only tree index.
//!
//! This module owns C-family include-site indexing and preprocessor gate
//! indexing. It records only relative source facts and filters host-absolute
//! include literals before they can enter the index.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::file_index::is_host_absolute_path_like;
use super::kconfig_index::collect_symbol_tokens;

pub type IncludeSiteIndex = BTreeSet<IncludeSite>;
pub type CppGateIndex = BTreeMap<String, BTreeSet<CppGate>>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IncludeSite {
    pub file: PathBuf,
    pub line: usize,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CppGate {
    pub file: PathBuf,
    pub line: usize,
    pub directive: String,
    pub expression: String,
}

#[derive(Debug, Default)]
pub(in crate::index) struct SourceFileFacts {
    pub include_sites: IncludeSiteIndex,
    pub cpp_gates_by_symbol: CppGateIndex,
}

pub(in crate::index) fn scan_c_family_file(relative: &Path, path: &Path) -> SourceFileFacts {
    let mut facts = SourceFileFacts::default();
    let Ok(content) = std::fs::read_to_string(path) else {
        return facts;
    };
    for (line_idx, line) in content.lines().enumerate() {
        let line_number = line_idx + 1;
        if let Some(target) = parse_include_target(line) {
            if !is_host_absolute_path_like(target) {
                facts.include_sites.insert(IncludeSite {
                    file: relative.to_path_buf(),
                    line: line_number,
                    target: target.to_string(),
                });
            }
        }
        if let Some(gate) = parse_cpp_gate(relative, line_number, line) {
            insert_cpp_gate(&mut facts.cpp_gates_by_symbol, gate);
        }
    }
    facts
}

pub(in crate::index) fn unique_cpp_gate_count(cpp_gates_by_symbol: &CppGateIndex) -> usize {
    let mut gates = BTreeSet::new();
    for by_symbol in cpp_gates_by_symbol.values() {
        gates.extend(by_symbol.iter());
    }
    gates.len()
}

fn insert_cpp_gate(cpp_gates_by_symbol: &mut CppGateIndex, gate: CppGate) {
    for symbol in collect_symbol_tokens(&gate.expression) {
        cpp_gates_by_symbol
            .entry(symbol)
            .or_default()
            .insert(gate.clone());
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(in crate::index) fn parse_include_target(line: &str) -> Option<&str> {
    let trimmed = line.trim();

    if let Some(quoted) = trimmed.strip_prefix("#include \"") {
        if let Some((target, "")) = quoted.split_once('"') {
            return (!target.trim().is_empty()).then_some(target);
        }
    }

    if let Some(angled) = trimmed.strip_prefix("#include <") {
        let (target, suffix) = angled.split_once('>')?;
        if !suffix.is_empty() || target.trim().is_empty() {
            return None;
        }
        return Some(target);
    }

    None
}

fn parse_cpp_gate(file: &Path, line: usize, input: &str) -> Option<CppGate> {
    let trimmed = input.trim_start();
    let rest = trimmed.strip_prefix('#')?.trim_start();
    let (directive, expression) = if let Some(expr) = rest.strip_prefix("ifdef") {
        ("ifdef", expr)
    } else if let Some(expr) = rest.strip_prefix("ifndef") {
        ("ifndef", expr)
    } else if let Some(expr) = rest.strip_prefix("elif") {
        ("elif", expr)
    } else if let Some(expr) = rest.strip_prefix("if") {
        ("if", expr)
    } else {
        return None;
    };
    if !expression
        .chars()
        .next()
        .is_some_and(|ch| ch.is_whitespace() || ch == '(')
    {
        return None;
    }

    let expression = expression.trim().to_string();
    (!collect_symbol_tokens(&expression).is_empty()).then(|| CppGate {
        file: file.to_path_buf(),
        line,
        directive: directive.to_string(),
        expression,
    })
}
