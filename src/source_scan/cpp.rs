//! Preprocessor-aware cleanup for removed-config-gated branches.
//!
//! The pass folds branches when removed symbols make the enclosing preprocessor
//! expression provably true or false under a conservative model: removed
//! symbols are false, everything else is unknown, and `!`, `&&`, and `||` are
//! evaluated over that three-valued truth space. Unsupported syntax is left
//! untouched and reported only when it references removed symbols.

use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use crate::edit_reason::{
    ensure_edit_records_for_mutation, sort_edit_records, write_verified_rewrite, EditProofSource,
    EditReason, EditRecord, LineRange,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct UnsupportedCppExpression {
    pub file: PathBuf,
    pub line: usize,
    pub directive: String,
    pub expression: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SkippedCppNestedEdgeCase {
    pub file: PathBuf,
    pub line: usize,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DeadCppBranchProof {
    pub line: usize,
    pub symbol: String,
}

#[derive(Debug, Default)]
pub(crate) struct CppFoldReport {
    pub counts: CppReportCounts,
    pub edits: Vec<EditRecord>,
    pub unsupported_expressions: Vec<UnsupportedCppExpression>,
    pub skipped_nested_edge_cases: Vec<SkippedCppNestedEdgeCase>,
    rewrites: Vec<PendingCppRewrite>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CppReportCounts {
    pub branches_folded: usize,
    pub files_touched: usize,
    pub skipped_nested_edge_cases: usize,
}

#[derive(Debug)]
struct PendingCppRewrite {
    path: PathBuf,
    content: String,
    edits: Vec<EditRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CppDirective {
    If(CppCondition),
    Elif(CppCondition),
    Else,
    Endif,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CppCondition {
    Symbol(CppSymbol),
    Literal(bool),
    Not(Box<CppCondition>),
    And(Box<CppCondition>, Box<CppCondition>),
    Or(Box<CppCondition>, Box<CppCondition>),
    Unsupported(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CppSymbol {
    Plain(String),
    Defined(String),
    IsEnabled(String),
    IsReachable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CppToken {
    Ident(String),
    Not,
    And,
    Or,
    LParen,
    RParen,
}

impl CppCondition {
    fn first_removed_symbol(&self, removed: &HashSet<&str>) -> Option<String> {
        let mut symbols = BTreeSet::new();
        self.collect_removed_symbols(removed, &mut symbols);
        symbols.into_iter().next()
    }

    fn collect_removed_symbols(&self, removed: &HashSet<&str>, symbols: &mut BTreeSet<String>) {
        match self {
            Self::Symbol(symbol) => {
                let normalized = symbol.normalized_name();
                if removed.contains(normalized) {
                    symbols.insert(normalized.to_string());
                }
            }
            Self::Literal(_) => {}
            Self::Not(inner) => inner.collect_removed_symbols(removed, symbols),
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => {
                lhs.collect_removed_symbols(removed, symbols);
                rhs.collect_removed_symbols(removed, symbols);
            }
            Self::Unsupported(_) => {}
        }
    }
}

impl CppSymbol {
    fn normalized_name(&self) -> &str {
        match self {
            Self::Plain(symbol)
            | Self::Defined(symbol)
            | Self::IsEnabled(symbol)
            | Self::IsReachable(symbol) => normalize_cpp_symbol(symbol),
        }
    }

    fn render(&self) -> String {
        match self {
            Self::Plain(symbol) => symbol.clone(),
            Self::Defined(symbol) => format!("defined({symbol})"),
            Self::IsEnabled(symbol) => format!("IS_ENABLED({symbol})"),
            Self::IsReachable(symbol) => format!("IS_REACHABLE({symbol})"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TruthValue {
    True,
    False,
    Unknown,
}

#[derive(Debug, Clone, Default)]
struct ManifestConfigTruth<'a> {
    removed: HashSet<&'a str>,
}

impl<'a> ManifestConfigTruth<'a> {
    fn from_removed_configs(removed_configs: &'a [String]) -> Self {
        Self {
            removed: removed_configs.iter().map(String::as_str).collect(),
        }
    }

    fn condition_truth(&self, condition: &CppCondition) -> TruthValue {
        match condition {
            CppCondition::Symbol(symbol) => self.symbol_truth(symbol.normalized_name()),
            CppCondition::Literal(true) => TruthValue::True,
            CppCondition::Literal(false) => TruthValue::False,
            CppCondition::Not(inner) => match self.condition_truth(inner) {
                TruthValue::True => TruthValue::False,
                TruthValue::False => TruthValue::True,
                TruthValue::Unknown => TruthValue::Unknown,
            },
            CppCondition::And(lhs, rhs) => {
                let lhs = self.condition_truth(lhs);
                let rhs = self.condition_truth(rhs);
                match (lhs, rhs) {
                    (TruthValue::False, _) | (_, TruthValue::False) => TruthValue::False,
                    (TruthValue::True, TruthValue::True) => TruthValue::True,
                    _ => TruthValue::Unknown,
                }
            }
            CppCondition::Or(lhs, rhs) => {
                let lhs = self.condition_truth(lhs);
                let rhs = self.condition_truth(rhs);
                match (lhs, rhs) {
                    (TruthValue::True, _) | (_, TruthValue::True) => TruthValue::True,
                    (TruthValue::False, TruthValue::False) => TruthValue::False,
                    _ => TruthValue::Unknown,
                }
            }
            CppCondition::Unsupported(_) => TruthValue::Unknown,
        }
    }

    fn simplify_condition(&self, condition: &CppCondition) -> CppCondition {
        match condition {
            CppCondition::Symbol(symbol) => {
                if self.removed.contains(symbol.normalized_name()) {
                    CppCondition::Literal(false)
                } else {
                    CppCondition::Symbol(symbol.clone())
                }
            }
            CppCondition::Literal(value) => CppCondition::Literal(*value),
            CppCondition::Not(inner) => match self.simplify_condition(inner) {
                CppCondition::Literal(value) => CppCondition::Literal(!value),
                simplified => CppCondition::Not(Box::new(simplified)),
            },
            CppCondition::And(lhs, rhs) => {
                let lhs = self.simplify_condition(lhs);
                let rhs = self.simplify_condition(rhs);
                match (&lhs, &rhs) {
                    (CppCondition::Literal(false), _) | (_, CppCondition::Literal(false)) => {
                        CppCondition::Literal(false)
                    }
                    (CppCondition::Literal(true), _) => rhs,
                    (_, CppCondition::Literal(true)) => lhs,
                    _ if lhs == rhs => lhs,
                    _ => CppCondition::And(Box::new(lhs), Box::new(rhs)),
                }
            }
            CppCondition::Or(lhs, rhs) => {
                let lhs = self.simplify_condition(lhs);
                let rhs = self.simplify_condition(rhs);
                match (&lhs, &rhs) {
                    (CppCondition::Literal(true), _) | (_, CppCondition::Literal(true)) => {
                        CppCondition::Literal(true)
                    }
                    (CppCondition::Literal(false), _) => rhs,
                    (_, CppCondition::Literal(false)) => lhs,
                    _ if lhs == rhs => lhs,
                    _ => CppCondition::Or(Box::new(lhs), Box::new(rhs)),
                }
            }
            CppCondition::Unsupported(expression) => {
                CppCondition::Unsupported(expression.clone())
            }
        }
    }

    fn symbol_truth(&self, symbol: &str) -> TruthValue {
        if self.removed.contains(symbol) {
            TruthValue::False
        } else {
            TruthValue::Unknown
        }
    }

    fn expression_mentions_removed_symbol(&self, expression: &str) -> bool {
        let mut token = String::new();

        for ch in expression.chars() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                token.push(ch);
                continue;
            }

            if cpp_token_mentions_removed_symbol(&token, &self.removed) {
                return true;
            }
            token.clear();
        }

        cpp_token_mentions_removed_symbol(&token, &self.removed)
    }
}

fn cpp_token_mentions_removed_symbol(token: &str, removed: &HashSet<&str>) -> bool {
    if token.is_empty() {
        return false;
    }

    removed.contains(token)
        || token
            .strip_prefix("CONFIG_")
            .is_some_and(|symbol| removed.contains(symbol))
}

fn render_cpp_condition(condition: &CppCondition) -> String {
    render_cpp_condition_with_precedence(condition, 0)
}

fn render_cpp_if_line(condition: &CppCondition) -> String {
    format!("#if {}", render_cpp_condition(condition))
}

fn render_cpp_condition_with_precedence(condition: &CppCondition, parent_precedence: u8) -> String {
    let precedence = cpp_condition_precedence(condition);
    let rendered = match condition {
        CppCondition::Symbol(symbol) => symbol.render(),
        CppCondition::Literal(true) => String::from("1"),
        CppCondition::Literal(false) => String::from("0"),
        CppCondition::Not(inner) => {
            format!("!{}", render_cpp_condition_with_precedence(inner, precedence))
        }
        CppCondition::And(lhs, rhs) => format!(
            "{} && {}",
            render_cpp_condition_with_precedence(lhs, precedence),
            render_cpp_condition_with_precedence(rhs, precedence)
        ),
        CppCondition::Or(lhs, rhs) => format!(
            "{} || {}",
            render_cpp_condition_with_precedence(lhs, precedence),
            render_cpp_condition_with_precedence(rhs, precedence)
        ),
        CppCondition::Unsupported(expression) => expression.clone(),
    };

    if precedence < parent_precedence {
        format!("({rendered})")
    } else {
        rendered
    }
}

fn cpp_condition_precedence(condition: &CppCondition) -> u8 {
    match condition {
        CppCondition::Or(_, _) => 1,
        CppCondition::And(_, _) => 2,
        CppCondition::Not(_) => 3,
        CppCondition::Symbol(_) | CppCondition::Literal(_) | CppCondition::Unsupported(_) => 4,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DirectiveChain {
    elif_indices: Vec<usize>,
    else_idx: Option<usize>,
    endif_idx: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CppLineContext {
    directive_visible: bool,
}

#[cfg(test)]
pub fn fold_removed_config_branches(
    root: &Path,
    removed_configs: &[String],
) -> Result<Vec<EditRecord>> {
    let report = fold_removed_config_branches_report(root, removed_configs)?;
    apply_fold_report(root, &report)?;
    Ok(report.edits)
}

pub(crate) fn fold_removed_config_branches_report(
    root: &Path,
    removed_configs: &[String],
) -> Result<CppFoldReport> {
    if removed_configs.is_empty() {
        return Ok(CppFoldReport::default());
    }

    let manifest_truth = ManifestConfigTruth::from_removed_configs(removed_configs);
    let mut report = CppFoldReport::default();

    for path in c_family_files(root) {
        let content = std::fs::read_to_string(&path)?;
        let lines: Vec<&str> = content.lines().collect();
        let (rewritten, file_edits, unsupported, skipped_nested, modified) =
            fold_lines(root, &path, &lines, &manifest_truth)?;
        report.unsupported_expressions.extend(unsupported);
        report.skipped_nested_edge_cases.extend(skipped_nested);
        if modified {
            report.edits.extend(file_edits.clone());
            report.rewrites.push(PendingCppRewrite {
                path,
                content: render_lines(&rewritten),
                edits: file_edits,
            });
        }
    }

    report.counts.branches_folded = report
        .edits
        .iter()
        .filter(|edit| edit.line_range.is_some_and(|range| !range.is_single_line()))
        .count();
    report.counts.files_touched = report.rewrites.len();
    report.counts.skipped_nested_edge_cases = report.skipped_nested_edge_cases.len();
    ensure_edit_records_for_mutation(
        "cpp.fold_removed_config_branches",
        report
            .counts
            .branches_folded
            .max(report.counts.files_touched),
        &report.edits,
    )?;
    canonicalize_cpp_fold_report(&mut report);

    Ok(report)
}

fn canonicalize_cpp_fold_report(report: &mut CppFoldReport) {
    sort_edit_records(&mut report.edits);
    report.unsupported_expressions.sort();
    report.skipped_nested_edge_cases.sort();
    for rewrite in &mut report.rewrites {
        sort_edit_records(&mut rewrite.edits);
    }
    report.rewrites.sort_by(|left, right| left.path.cmp(&right.path));
}

pub(crate) fn apply_fold_report(root: &Path, report: &CppFoldReport) -> Result<()> {
    ensure_edit_records_for_mutation(
        "cpp.fold_removed_config_branches",
        report
            .counts
            .branches_folded
            .max(report.counts.files_touched),
        &report.edits,
    )?;

    for rewrite in &report.rewrites {
        write_verified_rewrite(
            root,
            &rewrite.path,
            &rewrite.content,
            &rewrite.edits,
            "cpp.fold_removed_config_branches",
        )?;
    }

    Ok(())
}

pub(crate) fn proven_dead_cpp_branch_lines(
    lines: &[&str],
    removed_configs: &[String],
) -> BTreeMap<usize, DeadCppBranchProof> {
    if removed_configs.is_empty() {
        return BTreeMap::new();
    }

    let manifest_truth = ManifestConfigTruth::from_removed_configs(removed_configs);
    let contexts = cpp_line_contexts(lines);
    let mut dead_lines = BTreeMap::new();
    collect_dead_cpp_branch_lines(lines, &contexts, &manifest_truth, 0, &mut dead_lines);
    dead_lines
}

pub(crate) fn visible_cpp_directive_lines(lines: &[&str]) -> Vec<bool> {
    cpp_line_contexts(lines)
        .into_iter()
        .map(|context| context.directive_visible)
        .collect()
}

fn fold_lines(
    root: &Path,
    path: &Path,
    lines: &[&str],
    manifest_truth: &ManifestConfigTruth<'_>,
) -> Result<(
    Vec<String>,
    Vec<EditRecord>,
    Vec<UnsupportedCppExpression>,
    Vec<SkippedCppNestedEdgeCase>,
    bool,
)> {
    fold_lines_with_offset(root, path, lines, manifest_truth, 0)
}

fn fold_lines_with_offset(
    root: &Path,
    path: &Path,
    lines: &[&str],
    manifest_truth: &ManifestConfigTruth<'_>,
    line_offset: usize,
) -> Result<(
    Vec<String>,
    Vec<EditRecord>,
    Vec<UnsupportedCppExpression>,
    Vec<SkippedCppNestedEdgeCase>,
    bool,
)> {
    let mut out = Vec::new();
    let mut edits = Vec::new();
    let mut unsupported = Vec::new();
    let mut skipped_nested = Vec::new();
    let contexts = cpp_line_contexts(lines);
    let mut idx = 0usize;
    let mut modified = false;

    while idx < lines.len() {
        let line = lines[idx];
        let Some(CppDirective::If(condition)) =
            parse_contextual_cpp_directive(line, &contexts[idx])
        else {
            out.push(line.to_string());
            idx += 1;
            continue;
        };

        let Some(chain) = find_matching_directive(lines, &contexts, idx) else {
            out.push(line.to_string());
            idx += 1;
            continue;
        };
        let chain_lines = &lines[idx..=chain.endif_idx];
        let chain_contexts = &contexts[idx..=chain.endif_idx];
        if !directive_structure_is_fully_understood_with_context(chain_lines, chain_contexts) {
            copy_lines(&mut out, chain_lines);
            if has_nested_directives(chain_lines, chain_contexts) {
                skipped_nested.push(SkippedCppNestedEdgeCase {
                    file: relative_to_root_path(root, path),
                    line: line_offset + idx + 1,
                    reason: String::from("nested directive structure is not fully understood"),
                });
            }
            unsupported.extend(scan_unsupported_expressions(
                root,
                path,
                chain_lines,
                chain_contexts,
                manifest_truth,
                line_offset + idx,
            ));
            idx = chain.endif_idx + 1;
            continue;
        }
        let truth = manifest_truth.condition_truth(&condition);
        let simplified_condition = manifest_truth.simplify_condition(&condition);
        if truth == TruthValue::Unknown {
            let mut line_rewritten = false;
            if simplified_condition != condition {
                let Some(symbol) = condition.first_removed_symbol(&manifest_truth.removed) else {
                    copy_lines(&mut out, chain_lines);
                    unsupported.extend(scan_unsupported_expressions(
                        root,
                        path,
                        chain_lines,
                        chain_contexts,
                        manifest_truth,
                        line_offset + idx,
                    ));
                    idx = chain.endif_idx + 1;
                    continue;
                };

                let rewritten_line = render_cpp_if_line(&simplified_condition);
                out.push(rewritten_line.clone());
                copy_lines(&mut out, &chain_lines[1..]);
                edits.push(EditRecord::new(
                    relative_to_root_path(root, path),
                    Some(LineRange {
                        start: line_offset + idx + 1,
                        end: line_offset + idx + 1,
                    }),
                    format!("{line}\n"),
                    format!("{rewritten_line}\n"),
                    EditReason::ManifestConfig {
                        symbol: symbol.clone(),
                    },
                    EditProofSource::removal_manifest_config(symbol),
                    "cpp.fold_removed_config_branches",
                ));
                modified = true;
                line_rewritten = true;
            } else {
                copy_lines(&mut out, chain_lines);
            }
            if has_nested_directives(chain_lines, chain_contexts) {
                skipped_nested.push(SkippedCppNestedEdgeCase {
                    file: relative_to_root_path(root, path),
                    line: line_offset + idx + 1,
                    reason: String::from(
                        "unknown enclosing condition prevents folding nested branches",
                    ),
                });
            }
            if !line_rewritten {
                if let Some(site) = unresolved_removed_condition_site(
                    root,
                    path,
                    line,
                    &condition,
                    manifest_truth,
                    line_offset + idx + 1,
                ) {
                    unsupported.push(site);
                }
            }
            unsupported.extend(scan_unsupported_expressions(
                root,
                path,
                chain_lines,
                chain_contexts,
                manifest_truth,
                line_offset + idx,
            ));
            idx = chain.endif_idx + 1;
            continue;
        }
        let Some(symbol) = condition.first_removed_symbol(&manifest_truth.removed) else {
            copy_lines(&mut out, chain_lines);
            unsupported.extend(scan_unsupported_expressions(
                root,
                path,
                chain_lines,
                chain_contexts,
                manifest_truth,
                line_offset + idx,
            ));
            idx = chain.endif_idx + 1;
            continue;
        };
        if !chain.elif_indices.is_empty() {
            if simplified_condition != condition {
                let rewritten_line = render_cpp_if_line(&simplified_condition);
                out.push(rewritten_line.clone());
                copy_lines(&mut out, &chain_lines[1..]);
                edits.push(EditRecord::new(
                    relative_to_root_path(root, path),
                    Some(LineRange {
                        start: line_offset + idx + 1,
                        end: line_offset + idx + 1,
                    }),
                    format!("{line}\n"),
                    format!("{rewritten_line}\n"),
                    EditReason::ManifestConfig {
                        symbol: symbol.clone(),
                    },
                    EditProofSource::removal_manifest_config(symbol.clone()),
                    "cpp.fold_removed_config_branches",
                ));
                modified = true;
            } else {
                copy_lines(&mut out, chain_lines);
            }
            unsupported.extend(scan_unsupported_expressions(
                root,
                path,
                chain_lines,
                chain_contexts,
                manifest_truth,
                line_offset + idx,
            ));
            idx = chain.endif_idx + 1;
            continue;
        }

        let (chosen_start, chosen_end) = if truth == TruthValue::True {
            (idx + 1, chain.else_idx.unwrap_or(chain.endif_idx))
        } else if let Some(else_idx) = chain.else_idx {
            (else_idx + 1, chain.endif_idx)
        } else {
            (chain.endif_idx, chain.endif_idx)
        };
        let chosen = &lines[chosen_start..chosen_end];
        let (
            rewritten_branch,
            branch_edits,
            branch_unsupported,
            branch_skipped_nested,
            _branch_modified,
        ) = fold_lines_with_offset(
            root,
            path,
            chosen,
            manifest_truth,
            line_offset + chosen_start,
        )?;

        let before = join_lines(chain_lines);
        let after = render_lines(&rewritten_branch);

        edits.push(EditRecord::new(
            relative_to_root_path(root, path),
            Some(LineRange {
                start: line_offset + idx + 1,
                end: line_offset + chain.endif_idx + 1,
            }),
            before,
            after.clone(),
            EditReason::ManifestConfig {
                symbol: symbol.to_string(),
            },
            EditProofSource::removal_manifest_config(symbol.to_string()),
            "cpp.fold_removed_config_branches",
        ));

        edits.extend(branch_edits);
        unsupported.extend(branch_unsupported);
        skipped_nested.extend(branch_skipped_nested);
        out.extend(rewritten_branch);
        idx = chain.endif_idx + 1;
        modified = true;
    }

    Ok((out, edits, unsupported, skipped_nested, modified))
}

fn collect_dead_cpp_branch_lines(
    lines: &[&str],
    contexts: &[CppLineContext],
    manifest_truth: &ManifestConfigTruth<'_>,
    line_offset: usize,
    dead_lines: &mut BTreeMap<usize, DeadCppBranchProof>,
) {
    let mut idx = 0usize;

    while idx < lines.len() {
        let Some(CppDirective::If(condition)) =
            parse_contextual_cpp_directive(lines[idx], &contexts[idx])
        else {
            idx += 1;
            continue;
        };

        let Some(chain) = find_matching_directive(lines, contexts, idx) else {
            idx += 1;
            continue;
        };
        let chain_lines = &lines[idx..=chain.endif_idx];
        let chain_contexts = &contexts[idx..=chain.endif_idx];
        if !directive_structure_is_fully_understood_with_context(chain_lines, chain_contexts)
            || !chain.elif_indices.is_empty()
        {
            idx = chain.endif_idx + 1;
            continue;
        }

        let true_start = idx + 1;
        let true_end = chain.else_idx.unwrap_or(chain.endif_idx);
        let else_range = chain
            .else_idx
            .map(|else_idx| (else_idx + 1, chain.endif_idx));
        let truth = manifest_truth.condition_truth(&condition);
        let proof_symbol = condition.first_removed_symbol(&manifest_truth.removed);

        match (truth, proof_symbol) {
            (TruthValue::False, Some(symbol)) => {
                mark_dead_cpp_branch_range(dead_lines, line_offset, true_start, true_end, &symbol);
                if let Some((else_start, else_end)) = else_range {
                    collect_dead_cpp_branch_range(
                        lines,
                        contexts,
                        manifest_truth,
                        line_offset,
                        else_start,
                        else_end,
                        dead_lines,
                    );
                }
            }
            (TruthValue::True, Some(symbol)) => {
                collect_dead_cpp_branch_range(
                    lines,
                    contexts,
                    manifest_truth,
                    line_offset,
                    true_start,
                    true_end,
                    dead_lines,
                );
                if let Some((else_start, else_end)) = else_range {
                    mark_dead_cpp_branch_range(
                        dead_lines,
                        line_offset,
                        else_start,
                        else_end,
                        &symbol,
                    );
                }
            }
            _ => {
                collect_dead_cpp_branch_range(
                    lines,
                    contexts,
                    manifest_truth,
                    line_offset,
                    true_start,
                    true_end,
                    dead_lines,
                );
                if let Some((else_start, else_end)) = else_range {
                    collect_dead_cpp_branch_range(
                        lines,
                        contexts,
                        manifest_truth,
                        line_offset,
                        else_start,
                        else_end,
                        dead_lines,
                    );
                }
            }
        }

        idx = chain.endif_idx + 1;
    }
}

fn collect_dead_cpp_branch_range(
    lines: &[&str],
    contexts: &[CppLineContext],
    manifest_truth: &ManifestConfigTruth<'_>,
    line_offset: usize,
    start: usize,
    end: usize,
    dead_lines: &mut BTreeMap<usize, DeadCppBranchProof>,
) {
    if start >= end {
        return;
    }

    collect_dead_cpp_branch_lines(
        &lines[start..end],
        &contexts[start..end],
        manifest_truth,
        line_offset + start,
        dead_lines,
    );
}

fn mark_dead_cpp_branch_range(
    dead_lines: &mut BTreeMap<usize, DeadCppBranchProof>,
    line_offset: usize,
    start: usize,
    end: usize,
    symbol: &str,
) {
    for line_idx in start..end {
        let line = line_offset + line_idx + 1;
        dead_lines
            .entry(line)
            .or_insert_with(|| DeadCppBranchProof {
                line,
                symbol: symbol.to_string(),
            });
    }
}

fn unresolved_removed_condition_site(
    root: &Path,
    path: &Path,
    line: &str,
    condition: &CppCondition,
    manifest_truth: &ManifestConfigTruth<'_>,
    line_number: usize,
) -> Option<UnsupportedCppExpression> {
    if matches!(condition, CppCondition::Unsupported(_))
        || condition
            .first_removed_symbol(&manifest_truth.removed)
            .is_none()
    {
        return None;
    }

    Some(UnsupportedCppExpression {
        file: relative_to_root_path(root, path),
        line: line_number,
        directive: String::from("if"),
        expression: cpp_directive_expression(line, "if")?.to_string(),
        reason: String::from(
            "preprocessor expression referencing removed symbols could not be resolved to a deterministic truth value",
        ),
    })
}

fn cpp_directive_expression<'a>(line: &'a str, directive: &str) -> Option<&'a str> {
    let (keyword, expression) = split_cpp_directive(line)?;
    if keyword != directive {
        return None;
    }
    (!expression.is_empty()).then_some(expression)
}

fn parse_cpp_directive(line: &str) -> Option<CppDirective> {
    let (directive, body) = split_cpp_directive(line)?;

    match directive {
        "ifdef" => Some(CppDirective::If(
            parse_cpp_symbol(body)
                .map(CppCondition::Symbol)
                .unwrap_or_else(|| CppCondition::Unsupported(body.to_string())),
        )),
        "ifndef" => Some(CppDirective::If(
            parse_cpp_symbol(body)
                .map(|symbol| CppCondition::Not(Box::new(CppCondition::Symbol(symbol))))
                .unwrap_or_else(|| CppCondition::Unsupported(body.to_string())),
        )),
        "if" => Some(CppDirective::If(
            parse_cpp_condition(body)
                .unwrap_or_else(|| CppCondition::Unsupported(body.to_string())),
        )),
        "elif" => Some(CppDirective::Elif(
            parse_cpp_condition(body)
                .unwrap_or_else(|| CppCondition::Unsupported(body.to_string())),
        )),
        "else" => Some(CppDirective::Else),
        "endif" => Some(CppDirective::Endif),
        _ => None,
    }
}

fn parse_contextual_cpp_directive(line: &str, context: &CppLineContext) -> Option<CppDirective> {
    context
        .directive_visible
        .then(|| parse_cpp_directive(line))
        .flatten()
}

fn split_cpp_directive(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix('#')?.trim_start();
    if rest.is_empty() {
        return None;
    }

    let mut end = 0usize;
    for (idx, ch) in rest.char_indices() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            end = idx + ch.len_utf8();
            continue;
        }
        break;
    }
    if end == 0 {
        return None;
    }

    Some((&rest[..end], rest[end..].trim()))
}

fn parse_cpp_condition(expr: &str) -> Option<CppCondition> {
    let expr = expr.trim();
    if expr.is_empty() {
        return None;
    }

    let Some(tokens) = tokenize_cpp_expr(expr) else {
        return Some(CppCondition::Unsupported(expr.to_string()));
    };

    let mut idx = 0usize;
    let Some(condition) = parse_cpp_or_expr(&tokens, &mut idx) else {
        return Some(CppCondition::Unsupported(expr.to_string()));
    };
    if idx != tokens.len() {
        return Some(CppCondition::Unsupported(expr.to_string()));
    }

    Some(condition)
}

fn tokenize_cpp_expr(input: &str) -> Option<Vec<CppToken>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut idx = 0usize;

    while idx < chars.len() {
        match chars[idx] {
            ' ' | '\t' => idx += 1,
            '!' => {
                tokens.push(CppToken::Not);
                idx += 1;
            }
            '&' => {
                if chars.get(idx + 1) != Some(&'&') {
                    return None;
                }
                tokens.push(CppToken::And);
                idx += 2;
            }
            '|' => {
                if chars.get(idx + 1) != Some(&'|') {
                    return None;
                }
                tokens.push(CppToken::Or);
                idx += 2;
            }
            '(' => {
                tokens.push(CppToken::LParen);
                idx += 1;
            }
            ')' => {
                tokens.push(CppToken::RParen);
                idx += 1;
            }
            ch if ch.is_ascii_alphanumeric() || ch == '_' => {
                let start = idx;
                idx += 1;
                while idx < chars.len() && (chars[idx].is_ascii_alphanumeric() || chars[idx] == '_')
                {
                    idx += 1;
                }
                tokens.push(CppToken::Ident(chars[start..idx].iter().collect()));
            }
            _ => return None,
        }
    }

    (!tokens.is_empty()).then_some(tokens)
}

fn parse_cpp_or_expr(tokens: &[CppToken], idx: &mut usize) -> Option<CppCondition> {
    let mut expr = parse_cpp_and_expr(tokens, idx)?;
    while matches!(tokens.get(*idx), Some(CppToken::Or)) {
        *idx += 1;
        let rhs = parse_cpp_and_expr(tokens, idx)?;
        expr = CppCondition::Or(Box::new(expr), Box::new(rhs));
    }
    Some(expr)
}

fn parse_cpp_and_expr(tokens: &[CppToken], idx: &mut usize) -> Option<CppCondition> {
    let mut expr = parse_cpp_unary_expr(tokens, idx)?;
    while matches!(tokens.get(*idx), Some(CppToken::And)) {
        *idx += 1;
        let rhs = parse_cpp_unary_expr(tokens, idx)?;
        expr = CppCondition::And(Box::new(expr), Box::new(rhs));
    }
    Some(expr)
}

fn parse_cpp_unary_expr(tokens: &[CppToken], idx: &mut usize) -> Option<CppCondition> {
    if matches!(tokens.get(*idx), Some(CppToken::Not)) {
        *idx += 1;
        return Some(CppCondition::Not(Box::new(parse_cpp_unary_expr(
            tokens, idx,
        )?)));
    }
    parse_cpp_primary_expr(tokens, idx)
}

fn parse_cpp_primary_expr(tokens: &[CppToken], idx: &mut usize) -> Option<CppCondition> {
    match tokens.get(*idx)? {
        CppToken::LParen => {
            *idx += 1;
            let expr = parse_cpp_or_expr(tokens, idx)?;
            match tokens.get(*idx) {
                Some(CppToken::RParen) => {
                    *idx += 1;
                    Some(expr)
                }
                _ => None,
            }
        }
        CppToken::Ident(ident) if ident == "1" => {
            *idx += 1;
            Some(CppCondition::Literal(true))
        }
        CppToken::Ident(ident) if ident == "0" => {
            *idx += 1;
            Some(CppCondition::Literal(false))
        }
        CppToken::Ident(ident) if ident == "defined" => parse_defined_cpp_expr(tokens, idx),
        CppToken::Ident(ident) if ident == "IS_ENABLED" => parse_cpp_macro_expr(tokens, idx),
        CppToken::Ident(ident) if ident == "IS_REACHABLE" => parse_cpp_macro_expr(tokens, idx),
        CppToken::Ident(ident) => {
            *idx += 1;
            Some(CppCondition::Symbol(CppSymbol::Plain(ident.clone())))
        }
        _ => None,
    }
}

fn parse_defined_cpp_expr(tokens: &[CppToken], idx: &mut usize) -> Option<CppCondition> {
    *idx += 1;
    let symbol = if matches!(tokens.get(*idx), Some(CppToken::LParen)) {
        *idx += 1;
        let symbol = parse_cpp_ident(tokens, idx)?;
        if !matches!(tokens.get(*idx), Some(CppToken::RParen)) {
            return None;
        }
        *idx += 1;
        symbol
    } else {
        parse_cpp_ident(tokens, idx)?
    };

    Some(CppCondition::Symbol(CppSymbol::Defined(symbol)))
}

fn parse_cpp_macro_expr(tokens: &[CppToken], idx: &mut usize) -> Option<CppCondition> {
    let symbol_kind = match tokens.get(*idx)? {
        CppToken::Ident(ident) if ident == "IS_ENABLED" => CppSymbol::IsEnabled(String::new()),
        CppToken::Ident(ident) if ident == "IS_REACHABLE" => {
            CppSymbol::IsReachable(String::new())
        }
        _ => return None,
    };
    *idx += 1;
    if !matches!(tokens.get(*idx), Some(CppToken::LParen)) {
        return None;
    }
    *idx += 1;
    let symbol = parse_cpp_ident(tokens, idx)?;
    if !matches!(tokens.get(*idx), Some(CppToken::RParen)) {
        return None;
    }
    *idx += 1;

    let symbol = match symbol_kind {
        CppSymbol::IsEnabled(_) => CppSymbol::IsEnabled(symbol),
        CppSymbol::IsReachable(_) => CppSymbol::IsReachable(symbol),
        _ => unreachable!(),
    };

    Some(CppCondition::Symbol(symbol))
}

fn parse_cpp_ident(tokens: &[CppToken], idx: &mut usize) -> Option<String> {
    match tokens.get(*idx)? {
        CppToken::Ident(ident) if is_cpp_identifier(ident) => {
            *idx += 1;
            Some(ident.clone())
        }
        _ => None,
    }
}

fn parse_cpp_symbol(token: &str) -> Option<CppSymbol> {
    let symbol = token.trim();
    if symbol.is_empty() || symbol.contains(char::is_whitespace) || !is_cpp_identifier(symbol) {
        return None;
    }
    Some(CppSymbol::Defined(symbol.to_string()))
}

fn is_cpp_identifier(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn normalize_cpp_symbol(token: &str) -> &str {
    token.strip_prefix("CONFIG_").unwrap_or(token)
}

fn find_matching_directive(
    lines: &[&str],
    contexts: &[CppLineContext],
    start: usize,
) -> Option<DirectiveChain> {
    let mut depth = 0usize;
    let mut elif_indices = Vec::new();
    let mut else_idx = None;

    for (idx, line) in lines.iter().enumerate().skip(start + 1) {
        match parse_contextual_cpp_directive(line, &contexts[idx]) {
            Some(CppDirective::If(_)) => depth += 1,
            Some(CppDirective::Endif) => {
                if depth == 0 {
                    return Some(DirectiveChain {
                        elif_indices,
                        else_idx,
                        endif_idx: idx,
                    });
                }
                depth -= 1;
            }
            Some(CppDirective::Elif(_)) if depth == 0 => {
                if else_idx.is_some() {
                    return None;
                }
                elif_indices.push(idx);
            }
            Some(CppDirective::Else) if depth == 0 => {
                if else_idx.is_some() {
                    return None;
                }
                else_idx = Some(idx);
            }
            _ => {}
        }
    }

    None
}

#[cfg(test)]
fn directive_structure_is_fully_understood(lines: &[&str]) -> bool {
    let contexts = cpp_line_contexts(lines);
    directive_structure_is_fully_understood_with_context(lines, &contexts)
}

fn directive_structure_is_fully_understood_with_context(
    lines: &[&str],
    contexts: &[CppLineContext],
) -> bool {
    let mut stack = Vec::new();

    for (line, context) in lines.iter().zip(contexts) {
        match parse_contextual_cpp_directive(line, context) {
            Some(CppDirective::If(_)) => stack.push(false),
            Some(CppDirective::Elif(_)) => {
                let Some(saw_else) = stack.last() else {
                    return false;
                };
                if *saw_else {
                    return false;
                }
            }
            Some(CppDirective::Else) => {
                let Some(saw_else) = stack.last_mut() else {
                    return false;
                };
                if *saw_else {
                    return false;
                }
                *saw_else = true;
            }
            Some(CppDirective::Endif) => {
                if stack.pop().is_none() {
                    return false;
                }
            }
            None if context.directive_visible && is_branch_directive_like(line) => {
                return false;
            }
            None => {}
        }
    }

    stack.is_empty()
}

fn cpp_line_contexts(lines: &[&str]) -> Vec<CppLineContext> {
    let mut out = Vec::with_capacity(lines.len());
    let mut in_block_comment = false;
    let mut previous_line_continues = false;

    for line in lines {
        out.push(CppLineContext {
            directive_visible: !previous_line_continues
                && line_starts_with_visible_cpp_hash(line, in_block_comment),
        });
        in_block_comment = cpp_block_comment_state_after_line(line, in_block_comment);
        previous_line_continues = cpp_line_continues(line);
    }

    out
}

fn line_starts_with_visible_cpp_hash(line: &str, mut in_block_comment: bool) -> bool {
    let mut rest = line;

    loop {
        if in_block_comment {
            let Some(end) = rest.find("*/") else {
                return false;
            };
            rest = &rest[end + 2..];
            in_block_comment = false;
            continue;
        }

        let trimmed = rest.trim_start_matches(|ch| matches!(ch, ' ' | '\t' | '\r' | '\x0c'));
        if trimmed.is_empty() {
            return false;
        }
        if let Some(after_comment_start) = trimmed.strip_prefix("/*") {
            rest = after_comment_start;
            in_block_comment = true;
            continue;
        }
        return trimmed.starts_with('#');
    }
}

fn cpp_block_comment_state_after_line(line: &str, mut in_block_comment: bool) -> bool {
    let mut rest = line;

    loop {
        if in_block_comment {
            let Some(end) = rest.find("*/") else {
                return true;
            };
            rest = &rest[end + 2..];
            in_block_comment = false;
            continue;
        }

        let Some(start) = rest.find("/*") else {
            return false;
        };
        rest = &rest[start + 2..];
        in_block_comment = true;
    }
}

fn cpp_line_continues(line: &str) -> bool {
    line.trim_end().ends_with('\\')
}

fn is_branch_directive_like(line: &str) -> bool {
    split_cpp_directive(line).is_some_and(|(keyword, _)| {
        matches!(
            keyword,
            "if" | "ifdef" | "ifndef" | "elif" | "else" | "endif"
        )
    })
}

fn c_family_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
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

fn has_nested_directives(lines: &[&str], contexts: &[CppLineContext]) -> bool {
    lines.iter().zip(contexts).skip(1).any(|(line, context)| {
        matches!(
            parse_contextual_cpp_directive(line, context),
            Some(CppDirective::If(_))
        )
    })
}

fn scan_unsupported_expressions(
    root: &Path,
    path: &Path,
    lines: &[&str],
    contexts: &[CppLineContext],
    manifest_truth: &ManifestConfigTruth<'_>,
    base_line: usize,
) -> Vec<UnsupportedCppExpression> {
    lines
        .iter()
        .zip(contexts)
        .enumerate()
        .filter_map(|(idx, (line, context))| match parse_contextual_cpp_directive(line, context) {
            Some(CppDirective::If(CppCondition::Unsupported(expression)))
                if manifest_truth.expression_mentions_removed_symbol(&expression) =>
            {
                Some(UnsupportedCppExpression {
                    file: relative_to_root_path(root, path),
                    line: base_line + idx + 1,
                    directive: String::from("if"),
                    expression: expression.to_string(),
                    reason: String::from(
                        "preprocessor expression syntax referencing removed symbols is not supported",
                    ),
                })
            }
            Some(CppDirective::Elif(CppCondition::Unsupported(expression)))
                if manifest_truth.expression_mentions_removed_symbol(&expression) =>
            {
                Some(UnsupportedCppExpression {
                    file: relative_to_root_path(root, path),
                    line: base_line + idx + 1,
                    directive: String::from("elif"),
                    expression: expression.to_string(),
                    reason: String::from(
                        "preprocessor expression syntax referencing removed symbols is not supported",
                    ),
                })
            }
            _ => None,
        })
        .collect()
}

fn join_lines(lines: &[&str]) -> String {
    let mut out = String::new();
    for line in lines {
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn copy_lines(out: &mut Vec<String>, lines: &[&str]) {
    out.extend(lines.iter().map(|line| (*line).to_string()));
}

fn render_lines(lines: &[String]) -> String {
    let mut out = String::new();
    for line in lines {
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn relative_to_root_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(symbol: &str) -> CppCondition {
        CppCondition::Symbol(CppSymbol::Plain(symbol.to_string()))
    }

    fn defined_sym(symbol: &str) -> CppCondition {
        CppCondition::Symbol(CppSymbol::Defined(symbol.to_string()))
    }

    fn is_enabled_sym(symbol: &str) -> CppCondition {
        CppCondition::Symbol(CppSymbol::IsEnabled(symbol.to_string()))
    }

    fn is_reachable_sym(symbol: &str) -> CppCondition {
        CppCondition::Symbol(CppSymbol::IsReachable(symbol.to_string()))
    }

    #[test]
    fn test_manifest_config_truth_marks_removed_symbols_false_and_others_unknown() {
        let removed = [String::from("FOO")];
        let truth = ManifestConfigTruth::from_removed_configs(&removed);

        assert_eq!(truth.symbol_truth("FOO"), TruthValue::False);
        assert_eq!(truth.symbol_truth("BAR"), TruthValue::Unknown);
    }

    #[test]
    fn test_manifest_config_truth_evaluates_supported_condition_forms() {
        let removed = [String::from("FOO")];
        let truth = ManifestConfigTruth::from_removed_configs(&removed);

        assert_eq!(
            truth.condition_truth(&sym("FOO")),
            TruthValue::False
        );
        assert_eq!(
            truth.condition_truth(&CppCondition::Not(Box::new(sym("FOO")))),
            TruthValue::True
        );
        assert_eq!(
            truth.condition_truth(&CppCondition::And(
                Box::new(sym("BAR")),
                Box::new(sym("FOO")),
            )),
            TruthValue::False
        );
        assert_eq!(
            truth.condition_truth(&CppCondition::Or(
                Box::new(sym("BAR")),
                Box::new(sym("FOO")),
            )),
            TruthValue::Unknown
        );
        assert_eq!(
            truth.condition_truth(&CppCondition::And(
                Box::new(CppCondition::Or(
                    Box::new(sym("BAR")),
                    Box::new(sym("BAZ")),
                )),
                Box::new(sym("FOO")),
            )),
            TruthValue::False
        );
        assert_eq!(
            truth.condition_truth(&sym("BAR")),
            TruthValue::Unknown
        );
    }

    #[test]
    fn test_cpp_condition_uses_sorted_removed_symbol_for_proof() {
        let removed = HashSet::from(["ZED", "ALPHA"]);
        let condition = CppCondition::Or(
            Box::new(sym("ZED")),
            Box::new(sym("ALPHA")),
        );

        assert_eq!(
            condition.first_removed_symbol(&removed),
            Some(String::from("ALPHA"))
        );
    }

    #[test]
    fn test_parse_cpp_directive_supports_if_forms() {
        assert_eq!(
            parse_cpp_directive("#ifdef CONFIG_FOO"),
            Some(CppDirective::If(defined_sym("CONFIG_FOO")))
        );
        assert_eq!(
            parse_cpp_directive("#ifndef CONFIG_FOO"),
            Some(CppDirective::If(CppCondition::Not(Box::new(defined_sym(
                "CONFIG_FOO",
            )))))
        );
        assert_eq!(
            parse_cpp_directive("#if defined(CONFIG_FOO)"),
            Some(CppDirective::If(defined_sym("CONFIG_FOO")))
        );
        assert_eq!(
            parse_cpp_directive("#if !defined(CONFIG_FOO)"),
            Some(CppDirective::If(CppCondition::Not(Box::new(defined_sym(
                "CONFIG_FOO",
            )))))
        );
        assert_eq!(
            parse_cpp_directive("#if IS_ENABLED(CONFIG_FOO)"),
            Some(CppDirective::If(is_enabled_sym("CONFIG_FOO")))
        );
        assert_eq!(
            parse_cpp_directive("#if IS_REACHABLE(CONFIG_FOO)"),
            Some(CppDirective::If(is_reachable_sym("CONFIG_FOO")))
        );
        assert_eq!(
            parse_cpp_directive(
                "#if (defined(DEBUG) || defined(XFS_WARN)) && defined(CONFIG_LOCKDEP)"
            ),
            Some(CppDirective::If(CppCondition::And(
                Box::new(CppCondition::Or(
                    Box::new(defined_sym("DEBUG")),
                    Box::new(defined_sym("XFS_WARN")),
                )),
                Box::new(defined_sym("CONFIG_LOCKDEP")),
            )))
        );
        assert_eq!(
            parse_cpp_directive("#if defined CONFIG_PROVE_RCU || defined CONFIG_LOCKDEP"),
            Some(CppDirective::If(CppCondition::Or(
                Box::new(defined_sym("CONFIG_PROVE_RCU")),
                Box::new(defined_sym("CONFIG_LOCKDEP")),
            )))
        );
        assert_eq!(
            parse_cpp_directive("#if(defined(CONFIG_FOO))"),
            Some(CppDirective::If(defined_sym("CONFIG_FOO")))
        );
        assert_eq!(
            parse_cpp_directive("#ifdef(CONFIG_FOO)"),
            Some(CppDirective::If(CppCondition::Unsupported(String::from(
                "(CONFIG_FOO)"
            ))))
        );
    }

    #[test]
    fn test_parse_cpp_directive_supports_branch_chain_directives() {
        assert_eq!(
            parse_cpp_directive("#elif defined(CONFIG_BAR)"),
            Some(CppDirective::Elif(defined_sym("CONFIG_BAR")))
        );
        assert_eq!(
            parse_cpp_directive("#elif(defined(CONFIG_BAR))"),
            Some(CppDirective::Elif(defined_sym("CONFIG_BAR")))
        );
        assert_eq!(parse_cpp_directive("#else"), Some(CppDirective::Else));
        assert_eq!(parse_cpp_directive("#endif"), Some(CppDirective::Endif));
    }

    #[test]
    fn test_directive_structure_is_fully_understood_accepts_balanced_nested_chains() {
        let lines = [
            "#ifdef CONFIG_FOO",
            "#if defined(CONFIG_BAR)",
            "int nested;",
            "#else",
            "int nested_else;",
            "#endif",
            "#else",
            "int kept;",
            "#endif",
        ];

        assert!(directive_structure_is_fully_understood(&lines));
    }

    #[test]
    fn test_directive_structure_is_fully_understood_rejects_malformed_nested_chain() {
        let lines = [
            "#ifdef CONFIG_FOO",
            "#if defined(CONFIG_BAR)",
            "int nested;",
            "#else",
            "int nested_else;",
            "#else",
            "int broken;",
            "#endif",
            "#else",
            "int kept;",
            "#endif",
        ];

        assert!(!directive_structure_is_fully_understood(&lines));
    }

    #[test]
    fn test_cpp_line_contexts_hide_directives_inside_block_comments() {
        let lines = [
            "/*",
            "#ifdef CONFIG_FOO",
            "#endif",
            "*/",
            "#ifdef CONFIG_FOO",
        ];
        let contexts = cpp_line_contexts(&lines);

        assert_eq!(
            contexts
                .iter()
                .map(|context| context.directive_visible)
                .collect::<Vec<_>>(),
            vec![false, false, false, false, true]
        );
    }

    #[test]
    fn test_cpp_line_contexts_hide_directives_inside_continued_lines() {
        let lines = ["#define TEXT \\", "#ifdef CONFIG_FOO", "#ifdef CONFIG_BAR"];
        let contexts = cpp_line_contexts(&lines);

        assert_eq!(
            contexts
                .iter()
                .map(|context| context.directive_visible)
                .collect::<Vec<_>>(),
            vec![true, false, true]
        );
    }

    #[test]
    fn test_proven_dead_cpp_branch_lines_marks_nested_removed_config_branch() {
        let lines = [
            "#if defined(CONFIG_LIVE)",
            "#ifdef CONFIG_REMOVED",
            "#include <linux/dead.h>",
            "#endif",
            "#endif",
        ];

        let proofs = proven_dead_cpp_branch_lines(&lines, &[String::from("REMOVED")]);

        assert_eq!(
            proofs.get(&3),
            Some(&DeadCppBranchProof {
                line: 3,
                symbol: String::from("REMOVED"),
            })
        );
    }

    #[test]
    fn test_proven_dead_cpp_branch_lines_skips_elif_chains() {
        let lines = [
            "#ifdef CONFIG_REMOVED",
            "#include <linux/dead.h>",
            "#elif defined(CONFIG_OTHER)",
            "#include <linux/maybe.h>",
            "#endif",
        ];

        let proofs = proven_dead_cpp_branch_lines(&lines, &[String::from("REMOVED")]);

        assert!(proofs.is_empty());
    }

    #[test]
    fn test_fold_removed_config_branches_keeps_else_branch_for_removed_ifdef() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "#ifdef CONFIG_FOO\nint removed;\n#else\nint kept;\n#endif\n",
        )
        .unwrap();

        let edits = fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "int kept;\n");
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_fold_removed_config_branches_ignores_directives_inside_block_comments() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = concat!(
            "/*\n",
            "#ifdef CONFIG_FOO\n",
            "int commented_removed;\n",
            "#else\n",
            "int commented_kept;\n",
            "#endif\n",
            "*/\n",
            "#ifdef CONFIG_FOO\n",
            "int removed;\n",
            "#else\n",
            "int kept;\n",
            "#endif\n",
        );
        std::fs::write(&path, original).unwrap();

        let edits = fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            concat!(
                "/*\n",
                "#ifdef CONFIG_FOO\n",
                "int commented_removed;\n",
                "#else\n",
                "int commented_kept;\n",
                "#endif\n",
                "*/\n",
                "int kept;\n",
            )
        );
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_fold_removed_config_branches_ignores_directives_inside_continued_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = concat!(
            "#define TEXT \\\n",
            "#ifdef CONFIG_FOO\n",
            "int literal_text;\n",
            "#define END_TEXT \\\n",
            "#endif\n",
            "#ifdef CONFIG_FOO\n",
            "int removed;\n",
            "#else\n",
            "int kept;\n",
            "#endif\n",
        );
        std::fs::write(&path, original).unwrap();

        let edits = fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            concat!(
                "#define TEXT \\\n",
                "#ifdef CONFIG_FOO\n",
                "int literal_text;\n",
                "#define END_TEXT \\\n",
                "#endif\n",
                "int kept;\n",
            )
        );
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_fold_removed_config_branches_keeps_else_branch_for_removed_if_defined() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "#if defined(CONFIG_FOO)\nint removed;\n#else\nint kept;\n#endif\n",
        )
        .unwrap();

        let edits = fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "int kept;\n");
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_fold_removed_config_branches_keeps_true_branch_for_removed_ifndef() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "#ifndef CONFIG_FOO\nint kept;\n#else\nint removed;\n#endif\n",
        )
        .unwrap();

        fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "int kept;\n");
    }

    #[test]
    fn test_fold_removed_config_branches_keeps_true_branch_for_removed_if_not_defined() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "#if !defined(CONFIG_FOO)\nint kept;\n#else\nint removed;\n#endif\n",
        )
        .unwrap();

        fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "int kept;\n");
    }

    #[test]
    fn test_fold_removed_config_branches_keeps_else_branch_for_removed_is_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "#if IS_ENABLED(CONFIG_FOO)\nint removed;\n#else\nint kept;\n#endif\n",
        )
        .unwrap();

        fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "int kept;\n");
    }

    #[test]
    fn test_fold_removed_config_branches_keeps_else_branch_for_removed_is_reachable() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "#if IS_REACHABLE(CONFIG_FOO)\nint removed;\n#else\nint kept;\n#endif\n",
        )
        .unwrap();

        fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "int kept;\n");
    }

    #[test]
    fn test_fold_removed_config_branches_keeps_selected_else_branch_contents_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            concat!(
                "#ifdef CONFIG_FOO\n",
                "int removed;\n",
                "#else\n",
                "\tint kept; /* keep exact spacing */\n",
                "\n",
                "#if defined(CONFIG_BAR)\n",
                "\tint nested;\n",
                "#endif\n",
                "#endif\n",
            ),
        )
        .unwrap();

        fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            concat!(
                "\tint kept; /* keep exact spacing */\n",
                "\n",
                "#if defined(CONFIG_BAR)\n",
                "\tint nested;\n",
                "#endif\n",
            )
        );
    }

    #[test]
    fn test_fold_removed_config_branches_keeps_selected_true_branch_contents_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            concat!(
                "#if !defined(CONFIG_FOO)\n",
                "/* keep this comment */\n",
                "\tint kept;\n",
                "#else\n",
                "int removed;\n",
                "#endif\n",
            ),
        )
        .unwrap();

        fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "/* keep this comment */\n\tint kept;\n"
        );
    }

    #[test]
    fn test_fold_removed_config_branches_supports_boolean_expression_for_removed_symbol() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = concat!(
            "#if (defined(DEBUG) || defined(XFS_WARN)) && defined(CONFIG_FOO)\n",
            "int removed;\n",
            "#else\n",
            "int kept;\n",
            "#endif\n",
        );
        std::fs::write(&path, original).unwrap();

        let report = fold_removed_config_branches_report(root, &[String::from("FOO")]).unwrap();

        apply_fold_report(root, &report).unwrap();
        assert!(report.unsupported_expressions.is_empty());
        assert_eq!(report.edits.len(), 1);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "int kept;\n");
    }

    #[test]
    fn test_fold_removed_config_branches_reports_unsupported_arithmetic_expression_for_removed_symbol(
    ) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = "#if defined(CONFIG_FOO) + defined(CONFIG_BAR)\nint maybe_kept;\n#endif\n";
        std::fs::write(&path, original).unwrap();

        let report = fold_removed_config_branches_report(root, &[String::from("FOO")]).unwrap();

        assert!(report.edits.is_empty());
        assert_eq!(report.unsupported_expressions.len(), 1);
        assert_eq!(
            report.unsupported_expressions[0],
            UnsupportedCppExpression {
                file: PathBuf::from("drivers/test.c"),
                line: 1,
                directive: String::from("if"),
                expression: String::from("defined(CONFIG_FOO) + defined(CONFIG_BAR)"),
                reason: String::from(
                    "preprocessor expression syntax referencing removed symbols is not supported",
                ),
            }
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn test_fold_removed_config_branches_simplifies_mixed_removed_boolean_expression() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = "#if defined(CONFIG_FOO) || defined(CONFIG_BAR)\nint maybe_kept;\n#endif\n";
        std::fs::write(&path, original).unwrap();

        let report = fold_removed_config_branches_report(root, &[String::from("FOO")]).unwrap();
        apply_fold_report(root, &report).unwrap();

        assert_eq!(report.counts.branches_folded, 0);
        assert_eq!(report.edits.len(), 1);
        assert!(report.unsupported_expressions.is_empty());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "#if defined(CONFIG_BAR)\nint maybe_kept;\n#endif\n"
        );
    }

    #[test]
    fn test_fold_removed_config_branches_simplifies_mixed_removed_expression_preserving_macro_style()
    {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "#if defined(CONFIG_FOO) || IS_ENABLED(CONFIG_BAR)\nint maybe_kept;\n#endif\n",
        )
        .unwrap();

        let report = fold_removed_config_branches_report(root, &[String::from("FOO")]).unwrap();
        apply_fold_report(root, &report).unwrap();

        assert_eq!(report.counts.branches_folded, 0);
        assert_eq!(report.edits.len(), 1);
        assert!(report.unsupported_expressions.is_empty());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "#if IS_ENABLED(CONFIG_BAR)\nint maybe_kept;\n#endif\n"
        );
    }

    #[test]
    fn test_fold_removed_config_branches_does_not_report_unrelated_unsupported_expression() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = "#if defined(CONFIG_BAR) || defined(CONFIG_BAZ)\nint maybe_kept;\n#endif\n";
        std::fs::write(&path, original).unwrap();

        let report = fold_removed_config_branches_report(root, &[String::from("FOO")]).unwrap();

        assert!(report.edits.is_empty());
        assert!(report.unsupported_expressions.is_empty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn test_fold_removed_config_branches_leaves_unsupported_expression_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = "#if defined(CONFIG_FOO) + defined(CONFIG_BAR)\nint maybe_kept;\n#endif\n";
        std::fs::write(&path, original).unwrap();

        let edits = fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert!(edits.is_empty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn test_fold_removed_config_branches_removes_dead_branch_with_balanced_nested_live_branch() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            concat!(
                "#ifdef CONFIG_FOO\n",
                "int removed;\n",
                "#else\n",
                "#if defined(CONFIG_BAR)\n",
                "int nested_live;\n",
                "#else\n",
                "int nested_else;\n",
                "#endif\n",
                "#endif\n",
            ),
        )
        .unwrap();

        let edits = fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            concat!(
                "#if defined(CONFIG_BAR)\n",
                "int nested_live;\n",
                "#else\n",
                "int nested_else;\n",
                "#endif\n",
            )
        );
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_fold_removed_config_branches_handles_nested_if_without_directive_whitespace() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            concat!(
                "#ifdef CONFIG_FOO\n",
                "#if(defined(CONFIG_BAR))\n",
                "int nested_live;\n",
                "#endif\n",
                "#else\n",
                "int kept;\n",
                "#endif\n",
            ),
        )
        .unwrap();

        let edits = fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "int kept;\n");
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_fold_removed_config_branches_folds_nested_supported_branches_in_kept_region() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            concat!(
                "#ifdef CONFIG_FOO\n",
                "int removed_outer;\n",
                "#else\n",
                "#ifdef CONFIG_BAR\n",
                "int removed_inner;\n",
                "#else\n",
                "int kept_inner;\n",
                "#endif\n",
                "#endif\n",
            ),
        )
        .unwrap();

        let edits = fold_removed_config_branches(root, &[String::from("FOO"), String::from("BAR")])
            .unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "int kept_inner;\n");
        assert_eq!(edits.len(), 2);
        assert!(edits
            .iter()
            .all(|edit| matches!(edit.reason, EditReason::ManifestConfig { .. })));
    }

    #[test]
    fn test_fold_removed_config_branches_report_counts_folded_branches() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            concat!(
                "#ifdef CONFIG_FOO\n",
                "int removed_outer;\n",
                "#else\n",
                "#ifdef CONFIG_BAR\n",
                "int removed_inner;\n",
                "#else\n",
                "int kept_inner;\n",
                "#endif\n",
                "#endif\n",
            ),
        )
        .unwrap();

        let report =
            fold_removed_config_branches_report(root, &[String::from("FOO"), String::from("BAR")])
                .unwrap();

        assert_eq!(report.counts.branches_folded, 2);
        assert_eq!(report.counts.files_touched, 1);
        assert_eq!(report.edits.len(), 2);
    }

    #[test]
    fn test_fold_removed_config_branches_does_not_report_unsupported_in_dead_nested_branch() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            concat!(
                "#ifdef CONFIG_FOO\n",
                "#if defined(CONFIG_BAR) || defined(CONFIG_BAZ)\n",
                "int dead_unsupported;\n",
                "#endif\n",
                "#else\n",
                "int kept;\n",
                "#endif\n",
            ),
        )
        .unwrap();

        let report =
            fold_removed_config_branches_report(root, &[String::from("FOO"), String::from("BAR")])
                .unwrap();
        apply_fold_report(root, &report).unwrap();

        assert!(report.unsupported_expressions.is_empty());
        assert!(report.skipped_nested_edge_cases.is_empty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "int kept;\n");
    }

    #[test]
    fn test_fold_removed_config_branches_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            concat!(
                "#ifdef CONFIG_FOO\n",
                "int removed_outer;\n",
                "#else\n",
                "#ifdef CONFIG_BAR\n",
                "int removed_inner;\n",
                "#else\n",
                "int kept_inner;\n",
                "#endif\n",
                "#endif\n",
            ),
        )
        .unwrap();

        let first = fold_removed_config_branches(root, &[String::from("FOO"), String::from("BAR")])
            .unwrap();
        let after_first = std::fs::read_to_string(&path).unwrap();
        let second =
            fold_removed_config_branches(root, &[String::from("FOO"), String::from("BAR")])
                .unwrap();

        assert_eq!(after_first, "int kept_inner;\n");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), after_first);
        assert_eq!(first.len(), 2);
        assert!(second.is_empty());
    }

    #[test]
    fn test_fold_removed_config_branches_skips_malformed_nested_chain() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = concat!(
            "#ifdef CONFIG_FOO\n",
            "#if defined(CONFIG_BAR)\n",
            "int removed;\n",
            "#else\n",
            "int also_removed;\n",
            "#else\n",
            "int broken;\n",
            "#endif\n",
            "#else\n",
            "int kept;\n",
            "#endif\n",
        );
        std::fs::write(&path, original).unwrap();

        let edits = fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert!(edits.is_empty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn test_fold_removed_config_branches_preserves_unknown_supported_chain() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = concat!(
            "#if defined(CONFIG_BAR)\n",
            "int maybe_kept;\n",
            "#else\n",
            "int maybe_removed;\n",
            "#endif\n",
        );
        std::fs::write(&path, original).unwrap();

        let edits = fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert!(edits.is_empty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn test_fold_removed_config_branches_preserves_unknown_outer_chain_with_nested_removed_branch()
    {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = concat!(
            "#if defined(CONFIG_BAR)\n",
            "#ifdef CONFIG_FOO\n",
            "int removed;\n",
            "#else\n",
            "int kept;\n",
            "#endif\n",
            "#endif\n",
        );
        std::fs::write(&path, original).unwrap();

        let edits = fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert!(edits.is_empty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn test_fold_removed_config_branches_reports_skipped_unknown_nested_edge_case() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = concat!(
            "#if defined(CONFIG_BAR)\n",
            "#ifdef CONFIG_FOO\n",
            "int removed;\n",
            "#else\n",
            "int kept;\n",
            "#endif\n",
            "#endif\n",
        );
        std::fs::write(&path, original).unwrap();

        let report = fold_removed_config_branches_report(root, &[String::from("FOO")]).unwrap();

        assert_eq!(report.counts.skipped_nested_edge_cases, 1);
        assert_eq!(
            report.skipped_nested_edge_cases[0],
            SkippedCppNestedEdgeCase {
                file: PathBuf::from("drivers/test.c"),
                line: 1,
                reason: String::from(
                    "unknown enclosing condition prevents folding nested branches",
                ),
            }
        );
        assert!(report.edits.is_empty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn test_fold_removed_config_branches_simplifies_removed_if_condition_inside_elif_chain() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = concat!(
            "#ifdef CONFIG_FOO\n",
            "int removed;\n",
            "#elif defined(CONFIG_BAR)\n",
            "int maybe_kept;\n",
            "#else\n",
            "int kept;\n",
            "#endif\n",
        );
        std::fs::write(&path, original).unwrap();

        let edits = fold_removed_config_branches(root, &[String::from("FOO")]).unwrap();

        assert_eq!(edits.len(), 1);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            concat!(
                "#if 0\n",
                "int removed;\n",
                "#elif defined(CONFIG_BAR)\n",
                "int maybe_kept;\n",
                "#else\n",
                "int kept;\n",
                "#endif\n",
            )
        );
    }

    #[test]
    fn test_fold_removed_config_branches_reports_skipped_malformed_nested_edge_case() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("drivers/test.c");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = concat!(
            "#ifdef CONFIG_FOO\n",
            "#if defined(CONFIG_BAR)\n",
            "int removed;\n",
            "#else\n",
            "int also_removed;\n",
            "#else\n",
            "int broken;\n",
            "#endif\n",
            "#else\n",
            "int kept;\n",
            "#endif\n",
        );
        std::fs::write(&path, original).unwrap();

        let report = fold_removed_config_branches_report(root, &[String::from("FOO")]).unwrap();

        assert_eq!(report.counts.skipped_nested_edge_cases, 1);
        assert_eq!(
            report.skipped_nested_edge_cases[0],
            SkippedCppNestedEdgeCase {
                file: PathBuf::from("drivers/test.c"),
                line: 1,
                reason: String::from("nested directive structure is not fully understood"),
            }
        );
        assert!(report.edits.is_empty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }
}
