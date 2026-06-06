//! Kbuild Makefile rewrite planning and rendering.
//!
//! This module owns proof-carrying stale kbuild reference removal from
//! Makefile/Kbuild assignments, including token-drop decisions, stale
//! composite pruning, multiline assignment rendering, edit creation, and
//! verified rewrite application.

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::edit_reason::{
    ensure_edit_records_for_mutation, sort_edit_records, write_verified_rewrite, EditProofSource,
    EditReason, EditRecord, LineRange,
};

use super::{
    has_direct_object_provider, include_path_candidates, logical_lines, make_dir_candidates,
    makefiles, object_provider_path, parse_kbuild_assignment, protected_make_logical_line_starts,
    relative_to_root, relative_to_root_path, CompositeKind, KbuildAssignment,
    KbuildAssignmentKind, KbuildRewriteReport, KbuildSkippedLine, LogicalLine, ObjListKind,
};

#[cfg(test)]
pub(crate) fn rewrite_makefiles(
    root: &Path,
    removed_files: &[PathBuf],
    removed_dirs: &[PathBuf],
) -> Result<(usize, Vec<EditRecord>)> {
    let report = rewrite_makefiles_report(root, removed_files, removed_dirs, &[])?;
    Ok((report.removed_refs, report.edits))
}

#[cfg(test)]
pub(crate) fn rewrite_makefiles_with_removed_configs(
    root: &Path,
    removed_files: &[PathBuf],
    removed_dirs: &[PathBuf],
    removed_configs: &[String],
) -> Result<(usize, Vec<EditRecord>)> {
    let report = rewrite_makefiles_report(root, removed_files, removed_dirs, removed_configs)?;
    Ok((report.removed_refs, report.edits))
}

pub(crate) fn rewrite_makefiles_report(
    root: &Path,
    removed_files: &[PathBuf],
    removed_dirs: &[PathBuf],
    removed_configs: &[String],
) -> Result<KbuildRewriteReport> {
    let removed = RemovedIndex::from_removed_inputs(removed_files, removed_dirs, removed_configs);
    let mut removed_refs = 0usize;
    let mut edits = Vec::new();
    let mut skipped_ambiguous_lines = Vec::new();

    for path in makefiles(root) {
        let content = std::fs::read_to_string(&path)?;
        let logical = logical_lines(&content);
        let protected_lines = protected_make_logical_line_starts(&logical);
        let current_dir = path.parent().unwrap_or(root);
        let composite = composite_objects_with_protected(&logical, &protected_lines);
        let stale_composites = stale_composite_objects(
            root,
            current_dir,
            &logical,
            &protected_lines,
            &composite,
            &removed,
        );
        let mut out = String::with_capacity(content.len());
        let mut modified = false;

        for entry in logical {
            if protected_lines.contains(&entry.start_line) {
                for raw in entry.original {
                    out.push_str(&raw);
                    out.push('\n');
                }
                continue;
            }

            let Some(assignment) = parse_kbuild_assignment(&entry.joined) else {
                for raw in entry.original {
                    out.push_str(&raw);
                    out.push('\n');
                }
                continue;
            };

            let mut kept = Vec::new();
            let mut dropped = Vec::new();
            let mut skip_reason = None;

            for token in assignment.rhs.split_whitespace() {
                match assignment_token_decision(
                    root,
                    current_dir,
                    token,
                    &assignment,
                    &composite,
                    &stale_composites,
                    &removed,
                ) {
                    RewriteTokenDecision::Drop => dropped.push(token.to_string()),
                    RewriteTokenDecision::Keep => kept.push(token.to_string()),
                    RewriteTokenDecision::SkipLine(reason) => {
                        skip_reason = Some(reason);
                        break;
                    }
                }
            }

            if let Some(reason) = skip_reason {
                skipped_ambiguous_lines.push(KbuildSkippedLine {
                    file: relative_to_root_path(root, &path),
                    line: entry.start_line,
                    assignment_lhs: assignment.lhs.to_string(),
                    reason,
                });
                for raw in entry.original {
                    out.push_str(&raw);
                    out.push('\n');
                }
                continue;
            }

            if dropped.is_empty() {
                for raw in entry.original {
                    out.push_str(&raw);
                    out.push('\n');
                }
                continue;
            }

            modified = true;
            removed_refs += dropped.len();
            let after = render_kbuild_assignment_rewrite(&entry, &assignment, &kept);
            let before = {
                let mut text = String::new();
                for raw in &entry.original {
                    text.push_str(raw);
                    text.push('\n');
                }
                text
            };

            for reference in &dropped {
                edits.push(EditRecord::new(
                    relative_to_root_path(root, &path),
                    Some(LineRange {
                        start: entry.start_line,
                        end: entry.start_line + entry.original.len().saturating_sub(1),
                    }),
                    before.clone(),
                    after.clone(),
                    EditReason::RemovedKbuildRef {
                        reference: reference.clone(),
                    },
                    EditProofSource::stale_kbuild_reference(reference.clone()),
                    "prune.rewrite_makefiles",
                ));
            }

            out.push_str(&after);
        }

        if modified {
            write_verified_rewrite(root, &path, &out, &edits, "prune.rewrite_makefiles")?;
        }
    }

    ensure_edit_records_for_mutation("prune.rewrite_makefiles", removed_refs, &edits)?;
    sort_edit_records(&mut edits);
    skipped_ambiguous_lines.sort();
    skipped_ambiguous_lines.dedup();

    Ok(KbuildRewriteReport {
        removed_refs,
        edits,
        skipped_ambiguous_lines,
    })
}

#[derive(Default)]
struct RemovedIndex {
    object_providers: HashSet<PathBuf>,
    removed_dirs: Vec<PathBuf>,
    removed_configs: HashSet<String>,
}

impl RemovedIndex {
    fn from_removed_inputs(
        removed_files: &[PathBuf],
        removed_dirs: &[PathBuf],
        removed_configs: &[String],
    ) -> Self {
        let mut object_providers = HashSet::new();
        for path in removed_files {
            if let Some(provider) = object_provider_path(path) {
                object_providers.insert(provider);
            }
        }

        Self {
            object_providers,
            removed_dirs: removed_dirs.to_vec(),
            removed_configs: removed_configs.iter().cloned().collect(),
        }
    }

    fn is_inside_removed_dir(&self, path: &Path) -> bool {
        self.removed_dirs.iter().any(|dir| path.starts_with(dir))
    }

    fn is_removed_object_provider(&self, path: &Path) -> bool {
        self.object_providers.contains(path) || self.is_inside_removed_dir(path)
    }

    fn matches_removed_dir(&self, path: &Path) -> bool {
        self.removed_dirs
            .iter()
            .any(|dir| path == dir || path.starts_with(dir))
    }

    fn matches_removed_config(&self, symbol: &str) -> bool {
        self.removed_configs.contains(symbol)
    }
}

enum RewriteTokenDecision {
    Keep,
    Drop,
    SkipLine(String),
}

fn assignment_token_decision(
    root: &Path,
    current_dir: &Path,
    token: &str,
    assignment: &KbuildAssignment<'_>,
    composite_targets: &HashSet<String>,
    stale_composites: &HashSet<String>,
    removed: &RemovedIndex,
) -> RewriteTokenDecision {
    if matches!(assignment.kind, KbuildAssignmentKind::CcFlags) {
        return include_path_flag_decision(root, current_dir, token, removed);
    }

    if should_drop_make_token(
        root,
        current_dir,
        token,
        assignment_is_gated_by_removed_config(assignment, removed),
        composite_targets,
        stale_composites,
        removed,
    ) {
        RewriteTokenDecision::Drop
    } else {
        RewriteTokenDecision::Keep
    }
}

fn should_drop_make_token(
    root: &Path,
    current_dir: &Path,
    token: &str,
    gated_by_removed_config: bool,
    composite_targets: &HashSet<String>,
    stale_composites: &HashSet<String>,
    removed: &RemovedIndex,
) -> bool {
    if token.is_empty()
        || token == "\\"
        || token.starts_with('-')
        || token.starts_with('/')
        || token.contains('$')
        || token.contains('%')
        || token.contains(':')
    {
        return false;
    }

    if token.ends_with('/') {
        if gated_by_removed_config {
            return true;
        }
        return make_dir_candidates(root, current_dir, token)
            .into_iter()
            .any(|rel| removed.matches_removed_dir(&rel));
    }

    if token.ends_with(".o") {
        if gated_by_removed_config {
            return true;
        }
        if stale_composites.contains(token) {
            return true;
        }
        if composite_targets.contains(token) {
            return false;
        }
        let rel_object = relative_to_root(root, current_dir, token);
        if removed.is_removed_object_provider(&rel_object) {
            return true;
        }
        return false;
    }

    false
}

fn include_path_flag_decision(
    root: &Path,
    current_dir: &Path,
    token: &str,
    removed: &RemovedIndex,
) -> RewriteTokenDecision {
    let Some(include_path) = token.strip_prefix("-I").filter(|path| !path.is_empty()) else {
        return RewriteTokenDecision::Keep;
    };
    if include_path.starts_with('/') || include_path.contains('$') {
        return RewriteTokenDecision::Keep;
    }

    let candidates = include_path_candidates(root, current_dir, include_path);
    let has_live = candidates.iter().any(|path| root.join(path).exists());
    let has_removed = candidates
        .iter()
        .any(|path| removed.matches_removed_dir(path));

    if has_live && has_removed {
        return RewriteTokenDecision::SkipLine(format!(
            "ambiguous include path flag '{}' resolves to both removed and live paths",
            token
        ));
    }

    if has_removed && !has_live {
        RewriteTokenDecision::Drop
    } else {
        RewriteTokenDecision::Keep
    }
}

fn assignment_is_gated_by_removed_config(
    assignment: &KbuildAssignment<'_>,
    removed: &RemovedIndex,
) -> bool {
    assignment_removed_config_symbol(assignment)
        .is_some_and(|symbol| removed.matches_removed_config(symbol))
}

fn assignment_removed_config_symbol<'a>(assignment: &'a KbuildAssignment<'a>) -> Option<&'a str> {
    match &assignment.kind {
        KbuildAssignmentKind::ObjList(ObjListKind::Config(symbol)) => Some(*symbol),
        KbuildAssignmentKind::CompositeMembers(CompositeKind::Config { symbol, .. }) => {
            Some(*symbol)
        }
        _ => None,
    }
}

pub(crate) fn composite_objects(lines: &[LogicalLine]) -> HashSet<String> {
    let protected_lines = protected_make_logical_line_starts(lines);
    composite_objects_with_protected(lines, &protected_lines)
}

fn composite_objects_with_protected(
    lines: &[LogicalLine],
    protected_lines: &HashSet<usize>,
) -> HashSet<String> {
    let mut out = HashSet::new();

    for logical in lines {
        if protected_lines.contains(&logical.start_line) {
            continue;
        }
        let Some(assignment) = parse_kbuild_assignment(&logical.joined) else {
            continue;
        };
        if let KbuildAssignmentKind::CompositeMembers(composite) = assignment.kind {
            out.insert(format!("{}.o", composite.target()));
        }
    }

    out
}

fn stale_composite_objects(
    root: &Path,
    current_dir: &Path,
    lines: &[LogicalLine],
    protected_lines: &HashSet<usize>,
    composite_targets: &HashSet<String>,
    removed: &RemovedIndex,
) -> HashSet<String> {
    let members = composite_object_member_tokens(lines, protected_lines, removed);
    let mut stale = HashSet::new();

    loop {
        let mut changed = false;

        for (target, tokens) in &members {
            if stale.contains(target) || has_direct_object_provider(current_dir, target) {
                continue;
            }
            if !tokens.is_empty()
                && tokens.iter().all(|member| {
                    should_drop_make_token(
                        root,
                        current_dir,
                        &member.token,
                        member.gated_by_removed_config,
                        composite_targets,
                        &stale,
                        removed,
                    )
                })
            {
                stale.insert(target.clone());
                changed = true;
            }
        }

        if !changed {
            return stale;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompositeMemberToken {
    token: String,
    gated_by_removed_config: bool,
}

fn composite_object_member_tokens(
    lines: &[LogicalLine],
    protected_lines: &HashSet<usize>,
    removed: &RemovedIndex,
) -> HashMap<String, Vec<CompositeMemberToken>> {
    let mut out = HashMap::new();

    for logical in lines {
        if protected_lines.contains(&logical.start_line) {
            continue;
        }
        let Some(assignment) = parse_kbuild_assignment(&logical.joined) else {
            continue;
        };
        let gated_by_removed_config = assignment_is_gated_by_removed_config(&assignment, removed);
        let KbuildAssignmentKind::CompositeMembers(ref kind) = assignment.kind else {
            continue;
        };

        out.entry(format!("{}.o", kind.target()))
            .or_insert_with(Vec::new)
            .extend(
                assignment
                    .rhs
                    .split_whitespace()
                    .map(|token| CompositeMemberToken {
                        token: token.to_string(),
                        gated_by_removed_config,
                    }),
            );
    }

    out
}

fn render_kbuild_assignment_rewrite(
    entry: &LogicalLine,
    assignment: &KbuildAssignment<'_>,
    kept: &[String],
) -> String {
    let comment_suffix = trailing_comment_suffix(&entry.original);
    if kept.is_empty() {
        return format!(
            "# kslim: removed stale make refs from {}{}\n",
            assignment.lhs, comment_suffix
        );
    }

    if entry.original.len() == 1 {
        return format!(
            "{} {} {}{}\n",
            assignment.lhs,
            assignment.op,
            kept.join(" "),
            comment_suffix
        );
    }

    render_multiline_kbuild_assignment_rewrite(entry, assignment, kept, &comment_suffix)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PhysicalRhsLine {
    prefix: String,
    tokens: Vec<String>,
    comment: String,
}

fn render_multiline_kbuild_assignment_rewrite(
    entry: &LogicalLine,
    assignment: &KbuildAssignment<'_>,
    kept: &[String],
    comment_suffix: &str,
) -> String {
    let physical = physical_rhs_lines(entry, assignment);
    if physical.is_empty() {
        return format!(
            "{} {} {}{}\n",
            assignment.lhs,
            assignment.op,
            kept.join(" "),
            comment_suffix
        );
    }

    let mut kept_counts = token_counts(kept);
    let mut rewritten = Vec::new();
    for (idx, line) in physical.iter().enumerate() {
        let kept_tokens = line
            .tokens
            .iter()
            .filter_map(|token| take_kept_token(&mut kept_counts, token).then_some(token.clone()))
            .collect::<Vec<_>>();
        if kept_tokens.is_empty() && idx != 0 {
            continue;
        }
        rewritten.push(PhysicalRhsLine {
            prefix: line.prefix.clone(),
            tokens: kept_tokens,
            comment: line.comment.clone(),
        });
    }

    if rewritten.is_empty() {
        return format!(
            "{} {} {}{}\n",
            assignment.lhs,
            assignment.op,
            kept.join(" "),
            comment_suffix
        );
    }

    let mut out = String::new();
    let last_idx = rewritten.len().saturating_sub(1);
    for (idx, line) in rewritten.iter().enumerate() {
        let mut text = line.prefix.clone();
        if !line.tokens.is_empty() {
            if !text.is_empty() && !text.ends_with([' ', '\t']) {
                text.push(' ');
            }
            text.push_str(&line.tokens.join(" "));
        }
        if idx < last_idx {
            out.push_str(text.trim_end());
            out.push_str(" \\\n");
        } else {
            out.push_str(text.trim_end());
            let comment = if line.comment.is_empty() {
                comment_suffix
            } else {
                line.comment.as_str()
            };
            out.push_str(comment);
            out.push('\n');
        }
    }
    out
}

fn token_counts(tokens: &[String]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for token in tokens {
        *counts.entry(token.clone()).or_insert(0) += 1;
    }
    counts
}

fn take_kept_token(counts: &mut HashMap<String, usize>, token: &str) -> bool {
    let Some(count) = counts.get_mut(token) else {
        return false;
    };
    if *count == 0 {
        return false;
    }
    *count -= 1;
    true
}

fn physical_rhs_lines(
    entry: &LogicalLine,
    assignment: &KbuildAssignment<'_>,
) -> Vec<PhysicalRhsLine> {
    entry
        .original
        .iter()
        .enumerate()
        .map(|(idx, raw)| physical_rhs_line(raw, idx == 0, assignment.op))
        .collect()
}

fn line_indentation_prefix(line: &str) -> &str {
    &line[..line.len() - line.trim_start().len()]
}

fn physical_rhs_line(raw: &str, first: bool, op: &str) -> PhysicalRhsLine {
    let (body, comment) = split_make_trailing_comment(raw);
    let body = strip_make_line_continuation(body);
    let (prefix, rhs) = if first {
        first_physical_rhs_prefix_and_body(body, op)
    } else {
        let indent = line_indentation_prefix(body).to_string();
        (indent, body.trim_start())
    };

    PhysicalRhsLine {
        prefix,
        tokens: rhs.split_whitespace().map(str::to_string).collect(),
        comment: comment.to_string(),
    }
}

fn first_physical_rhs_prefix_and_body<'a>(body: &'a str, op: &str) -> (String, &'a str) {
    let Some(op_start) = body.find(op) else {
        return (String::new(), body.trim_start());
    };
    let after_op = op_start + op.len();
    let whitespace = body[after_op..]
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .map(char::len_utf8)
        .sum::<usize>();
    let rhs_start = after_op + whitespace;
    let mut prefix = body[..rhs_start].to_string();
    if !prefix.ends_with([' ', '\t']) {
        prefix.push(' ');
    }
    (prefix, &body[rhs_start..])
}

fn split_make_trailing_comment(line: &str) -> (&str, &str) {
    let Some(hash_idx) = line.find('#') else {
        return (line, "");
    };
    let prefix_end = line[..hash_idx]
        .rfind(|c: char| !c.is_whitespace())
        .map_or(0, |idx| idx + 1);
    (&line[..prefix_end], &line[prefix_end..])
}

fn strip_make_line_continuation(line: &str) -> &str {
    let trimmed = line.trim_end();
    trimmed
        .strip_suffix('\\')
        .map(str::trim_end)
        .unwrap_or(trimmed)
}

fn trailing_comment_suffix(original: &[String]) -> String {
    let Some(last) = original.last() else {
        return String::new();
    };
    let Some(hash_idx) = last.find('#') else {
        return String::new();
    };
    let prefix_end = last[..hash_idx]
        .rfind(|c: char| !c.is_whitespace())
        .map_or(0, |idx| idx + 1);
    last[prefix_end..].to_string()
}
