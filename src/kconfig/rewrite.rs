use anyhow::Result;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use crate::edit_reason::{
    ensure_edit_records_for_mutation, sort_edit_records, write_verified_rewrite, EditProofSource,
    EditReason, EditRecord, LineRange,
};

use super::expression::{
    equivalent_kconfig_expr_simplification, first_removed_symbol, is_kconfig_symbol_char,
    parse_kconfig_expr, render_kconfig_expr, KconfigExpr, TristateLiteral,
};
use super::parser::{
    parse_kconfig_directive, parse_kconfig_source, split_kconfig_trailing_comment,
    strip_kconfig_keyword, KconfigDirective, KconfigEntryKind,
};
use super::report::{
    KconfigRelationRewriteStats, KconfigReportCounts, UnsupportedKconfigExpression,
};
use super::{
    indentation, is_kconfig_boundary, is_kconfig_help_directive,
    kconfig_dead_symbol_definition_kind_is_rewrite_supported,
    kconfig_menu_block_end_line_from_nodes, kconfig_node_line_range,
    kconfig_symbol_definition_kind_keyword, kconfig_files, kconfig_help_text_mask, join_lines,
    line_indentation_prefix, parse_config_symbol, parse_kconfig_document, relative_to_root_path,
    resolve_kconfig_source, KconfigDeadSymbolDefinitionProof, KconfigDocument,
    KconfigEmptyMenuRemovalProof, KconfigNode, KconfigRawLine, KconfigSource,
    KconfigSourceRemovalProof, KconfigSymbolDefinitionKind,
};


const KCONFIG_UNSUPPORTED_REMOVED_SYMBOL_EXPRESSION_REASON: &str =
    "expression syntax referencing removed symbols is not supported";
pub(super) const KCONFIG_UNKNOWN_REMOVED_TARGET_CONDITION_REASON: &str =
    "unknown condition expression blocks removed-target relation rewrite";

#[derive(Debug, Clone, PartialEq, Eq)]
enum RelationRewrite {
    Replace {
        line: String,
        reason: EditReason,
        report_kind: Option<KconfigReportKind>,
    },
    Delete {
        reason: EditReason,
        report_kind: Option<KconfigReportKind>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KconfigRelationSourceContext {
    symbol: String,
    indentation: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KconfigReportKind {
    DroppedSelect,
    DroppedImply,
    SimplifiedDepends,
    SimplifiedVisibleIf,
    SimplifiedDefault,
}

enum RelationLineAnalysis {
    Noop,
    Rewrite(RelationRewrite),
    Unsupported {
        directive: String,
        expression: String,
        reason: String,
    },
}

fn proof_source_for_relation_reason(reason: &EditReason) -> EditProofSource {
    match reason {
        EditReason::ManifestConfig { symbol } | EditReason::SimplifiedTristateExpr { symbol } => {
            EditProofSource::removal_manifest_config(symbol.clone())
        }
        other => unreachable!(
            "unsupported Kconfig relation edit reason: {}",
            other.json_key()
        ),
    }
}

pub(crate) fn prune_configs(root: &Path, configs: &[String]) -> Result<(usize, Vec<EditRecord>)> {
    let mut removed_blocks = 0usize;
    let config_set: HashSet<&str> = configs.iter().map(|s| s.as_str()).collect();
    let mut edits = Vec::new();

    for path in kconfig_files(root) {
        let content = std::fs::read_to_string(&path)?;
        let lines: Vec<&str> = content.lines().collect();
        let help_text = kconfig_help_text_mask(&lines);
        let mut out = String::with_capacity(content.len());
        let mut idx = 0usize;
        let mut modified = false;

        while idx < lines.len() {
            let line = lines[idx];
            if help_text[idx] {
                out.push_str(line);
                out.push('\n');
                idx += 1;
                continue;
            }
            let Some(KconfigDirective::Entry {
                symbol: config_name,
                ..
            }) = parse_kconfig_directive(line)
            else {
                out.push_str(line);
                out.push('\n');
                idx += 1;
                continue;
            };

            if config_set.contains(config_name.as_str()) {
                let base_indent = indentation(line);
                let (directive_text, comment_suffix) = split_kconfig_trailing_comment(line);
                let replacement = format!(
                    "{}# kslim: removed config {}{}\n",
                    line_indentation_prefix(directive_text),
                    config_name,
                    comment_suffix
                );
                out.push_str(&replacement);
                modified = true;
                removed_blocks += 1;
                let block_start = idx + 1;
                idx += 1;

                while idx < lines.len() {
                    let next = lines[idx];
                    if help_text[idx] {
                        idx += 1;
                        continue;
                    }
                    let next_trimmed = next.trim_start();
                    if next_trimmed.is_empty() {
                        idx += 1;
                        continue;
                    }
                    if indentation(next) <= base_indent && is_kconfig_boundary(next_trimmed) {
                        break;
                    }
                    idx += 1;
                }
                let block_end = idx;
                edits.push(EditRecord::new(
                    relative_to_root_path(root, &path),
                    Some(LineRange {
                        start: block_start,
                        end: block_end.max(block_start),
                    }),
                    join_lines(&lines[block_start - 1..block_end]),
                    replacement,
                    EditReason::ManifestConfig {
                        symbol: config_name.clone(),
                    },
                    EditProofSource::removal_manifest_config(config_name.clone()),
                    "prune.prune_configs",
                ));

                log::info!(
                    "prune: removed config '{}' from {}",
                    config_name,
                    path.display()
                );
                continue;
            }

            out.push_str(line);
            out.push('\n');
            idx += 1;
        }

        if modified {
            write_verified_rewrite(root, &path, &out, &edits, "prune.prune_configs")?;
        }
    }

    ensure_edit_records_for_mutation("prune.prune_configs", removed_blocks, &edits)?;
    sort_edit_records(&mut edits);

    Ok((removed_blocks, edits))
}

pub(crate) fn rewrite_kconfig_defaults(
    root: &Path,
    overrides: &BTreeMap<String, String>,
) -> Result<(usize, Vec<EditRecord>)> {
    let definitions = find_config_definitions(root, overrides)?;
    let mut edits = Vec::new();

    for symbol in overrides.keys() {
        let Some(matches) = definitions.get(symbol) else {
            anyhow::bail!(
                "slim.set_defaults symbol '{}' was not found in any Kconfig file",
                symbol
            );
        };
        if matches.len() > 1 {
            let locations = matches
                .iter()
                .map(|(path, line)| format!("{}:{}", path.display(), line))
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!(
                "slim.set_defaults symbol '{}' is defined more than once: {}",
                symbol,
                locations
            );
        }
    }

    let mut rewritten = 0usize;

    for path in kconfig_files(root) {
        let content = std::fs::read_to_string(&path)?;
        let lines: Vec<&str> = content.lines().collect();
        let help_text = kconfig_help_text_mask(&lines);
        let mut out = String::with_capacity(content.len());
        let mut idx = 0usize;
        let mut modified = false;

        while idx < lines.len() {
            let line = lines[idx];
            if help_text[idx] {
                out.push_str(line);
                out.push('\n');
                idx += 1;
                continue;
            }
            let Some(symbol) = parse_config_symbol(line) else {
                out.push_str(line);
                out.push('\n');
                idx += 1;
                continue;
            };

            let Some(value) = overrides.get(&symbol) else {
                out.push_str(line);
                out.push('\n');
                idx += 1;
                continue;
            };

            let base_indent = indentation(line);
            let mut end = idx + 1;
            while end < lines.len() {
                let next = lines[end];
                if help_text[end] {
                    end += 1;
                    continue;
                }
                let next_trimmed = next.trim_start();
                if next_trimmed.is_empty() {
                    end += 1;
                    continue;
                }
                if indentation(next) <= base_indent && is_kconfig_boundary(next_trimmed) {
                    break;
                }
                end += 1;
            }

            let original_block = join_lines(&lines[idx..end]);
            let rewritten_block = rewrite_config_default_block(&lines[idx..end], value);
            if rewritten_block != original_block {
                modified = true;
                rewritten += 1;

                edits.push(EditRecord::new(
                    relative_to_root_path(root, &path),
                    Some(LineRange {
                        start: idx + 1,
                        end,
                    }),
                    original_block.clone(),
                    rewritten_block.clone(),
                    EditReason::ManifestConfig {
                        symbol: symbol.clone(),
                    },
                    EditProofSource::removal_manifest_config(symbol.clone()),
                    "prune.rewrite_kconfig_defaults",
                ));
                log::info!(
                    "prune: rewrote default for config '{}' in {}",
                    symbol,
                    path.display()
                );
            }
            out.push_str(&rewritten_block);
            idx = end;
        }

        if modified {
            write_verified_rewrite(root, &path, &out, &edits, "prune.rewrite_kconfig_defaults")?;
        }
    }

    ensure_edit_records_for_mutation("prune.rewrite_kconfig_defaults", rewritten, &edits)?;
    sort_edit_records(&mut edits);

    Ok((rewritten, edits))
}

pub(crate) fn rewrite_kconfig_relations(
    root: &Path,
    removed_configs: &[String],
) -> Result<KconfigRelationRewriteStats> {
    if removed_configs.is_empty() {
        return Ok(KconfigRelationRewriteStats::default());
    }

    let removed: HashSet<&str> = removed_configs.iter().map(String::as_str).collect();
    let mut rewrites = 0usize;
    let mut edits = Vec::new();
    let mut unsupported = Vec::new();
    let mut report = KconfigReportCounts::default();

    for path in kconfig_files(root) {
        let content = std::fs::read_to_string(&path)?;
        let lines: Vec<&str> = content.lines().collect();
        let help_text = kconfig_help_text_mask(&lines);
        let relative = relative_to_root_path(root, &path);
        let mut modified = false;
        let mut out = String::with_capacity(content.len());
        let mut file_edits = Vec::new();

        let mut source_context: Option<KconfigRelationSourceContext> = None;

        for (idx, line) in lines.iter().copied().enumerate() {
            let analysis = if help_text[idx] {
                RelationLineAnalysis::Noop
            } else {
                clear_kconfig_relation_source_context_for_line(&mut source_context, line);
                analyze_kconfig_relation_line(
                    line,
                    &removed,
                    source_context.as_ref().map(|context| context.symbol.as_str()),
                )
            };
            match analysis {
                RelationLineAnalysis::Noop => {
                    out.push_str(line);
                    out.push('\n');
                }
                RelationLineAnalysis::Unsupported {
                    directive,
                    expression,
                    reason,
                } => {
                    out.push_str(line);
                    out.push('\n');
                    report.skipped_expressions += 1;
                    unsupported.push(UnsupportedKconfigExpression {
                        file: relative.clone(),
                        line: idx + 1,
                        directive,
                        expression,
                        reason,
                    });
                }
                RelationLineAnalysis::Rewrite(rewrite) => {
                    modified = true;
                    rewrites += 1;
                    let (after, reason) = match rewrite {
                        RelationRewrite::Replace {
                            line,
                            reason,
                            report_kind,
                        } => {
                            if let Some(kind) = report_kind {
                                increment_kconfig_report_count(&mut report, kind);
                            }
                            out.push_str(&line);
                            out.push('\n');
                            (format!("{line}\n"), reason)
                        }
                        RelationRewrite::Delete {
                            reason,
                            report_kind,
                        } => {
                            if let Some(kind) = report_kind {
                                increment_kconfig_report_count(&mut report, kind);
                            }
                            (String::new(), reason)
                        }
                    };

                    let proof_source = proof_source_for_relation_reason(&reason);
                    file_edits.push(EditRecord::new(
                        relative.clone(),
                        Some(LineRange {
                            start: idx + 1,
                            end: idx + 1,
                        }),
                        format!("{line}\n"),
                        after,
                        reason,
                        proof_source,
                        "kconfig.rewrite_relations",
                    ));
                }
            }

            if !help_text[idx] {
                if let Some(next_context) = kconfig_relation_source_context_from_line(line) {
                    source_context = Some(next_context);
                }
            }
        }

        if modified {
            write_verified_rewrite(root, &path, &out, &file_edits, "kconfig.rewrite_relations")?;
            edits.extend(file_edits);
        }
    }

    ensure_edit_records_for_mutation("kconfig.rewrite_relations", rewrites, &edits)?;
    sort_edit_records(&mut edits);
    unsupported.sort();

    Ok(KconfigRelationRewriteStats {
        rewrites,
        edits,
        unsupported,
        report,
    })
}

pub(crate) fn rewrite_kconfig_sources(
    root: &Path,
    source_removal_proofs: &[KconfigSourceRemovalProof],
) -> Result<(usize, Vec<EditRecord>)> {
    let mut removed_lines = 0usize;
    let mut edits = Vec::new();
    let proven_removed_sources = kconfig_source_removal_proofs_by_site(source_removal_proofs);

    for path in kconfig_files(root) {
        let content = std::fs::read_to_string(&path)?;
        let lines: Vec<&str> = content.lines().collect();
        let help_text = kconfig_help_text_mask(&lines);
        let current_dir = path.parent().unwrap_or(root);
        let relative = relative_to_root_path(root, &path);
        let mut modified = false;
        let mut out = String::with_capacity(content.len());
        let mut file_edits = Vec::new();

        for (idx, line) in lines.iter().copied().enumerate() {
            let mut keep = true;
            if !help_text[idx] {
                if let Some(source) = parse_kconfig_source(line) {
                    // `osource` and `orsource` are valid when their targets are
                    // absent; only required `source`/`rsource` directives can be
                    // considered dead.
                    if !source.optional
                        && !source.path.contains('$')
                        && resolve_kconfig_source(root, current_dir, &source).is_none()
                    {
                        let proof = kconfig_source_removal_proof_for_line(
                            &proven_removed_sources,
                            &relative,
                            idx + 1,
                            &source,
                        );
                        let (directive_text, comment_suffix) =
                            split_kconfig_trailing_comment(line);
                        if let Some(proof) = proof {
                            let replacement = format!(
                                "{}# kslim: removed {}{}\n",
                                line_indentation_prefix(directive_text),
                                directive_text.trim_start(),
                                comment_suffix
                            );
                            out.push_str(&replacement);
                            keep = false;
                            modified = true;
                            removed_lines += 1;
                            file_edits.push(EditRecord::new(
                                relative.clone(),
                                Some(LineRange {
                                    start: idx + 1,
                                    end: idx + 1,
                                }),
                                format!("{line}\n"),
                                replacement,
                                EditReason::RemovedKconfigSource,
                                EditProofSource::removal_manifest_kconfig_source(
                                    proof.removed_target.clone(),
                                ),
                                "prune.rewrite_kconfig_sources",
                            ));
                        }
                    }
                }
            }

            if keep {
                out.push_str(line);
                out.push('\n');
            }
        }

        if modified {
            write_verified_rewrite(
                root,
                &path,
                &out,
                &file_edits,
                "prune.rewrite_kconfig_sources",
            )?;
            edits.extend(file_edits);
        }
    }

    ensure_edit_records_for_mutation("prune.rewrite_kconfig_sources", removed_lines, &edits)?;
    sort_edit_records(&mut edits);

    Ok((removed_lines, edits))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct KconfigSourceRemovalSite {
    file: PathBuf,
    line: usize,
    source: String,
    optional: bool,
    relative: bool,
}

fn kconfig_source_removal_proofs_by_site(
    proofs: &[KconfigSourceRemovalProof],
) -> BTreeMap<KconfigSourceRemovalSite, &KconfigSourceRemovalProof> {
    let mut by_site = BTreeMap::new();
    for proof in proofs {
        if proof.optional || proof.source.contains('$') {
            continue;
        }
        by_site.insert(
            KconfigSourceRemovalSite {
                file: proof.file.clone(),
                line: proof.line,
                source: proof.source.clone(),
                optional: proof.optional,
                relative: proof.relative,
            },
            proof,
        );
    }
    by_site
}

fn kconfig_source_removal_proof_for_line<'a>(
    proofs: &'a BTreeMap<KconfigSourceRemovalSite, &'a KconfigSourceRemovalProof>,
    file: &Path,
    line: usize,
    source: &KconfigSource,
) -> Option<&'a KconfigSourceRemovalProof> {
    proofs
        .get(&KconfigSourceRemovalSite {
            file: file.to_path_buf(),
            line,
            source: source.path.clone(),
            optional: source.optional,
            relative: source.relative,
        })
        .copied()
}

#[allow(dead_code)]
pub(crate) fn rewrite_dead_kconfig_symbol_definitions(
    root: &Path,
    proofs: &[KconfigDeadSymbolDefinitionProof],
) -> Result<(usize, Vec<EditRecord>)> {
    let mut removed_blocks = 0usize;
    let mut edits = Vec::new();
    let proofs_by_file = dead_kconfig_symbol_definition_proofs_by_file(proofs);

    for path in kconfig_files(root) {
        let relative = relative_to_root_path(root, &path);
        let Some(file_proofs) = proofs_by_file.get(&relative) else {
            continue;
        };

        let content = std::fs::read_to_string(&path)?;
        let lines: Vec<&str> = content.lines().collect();
        let help_text = kconfig_help_text_mask(&lines);
        let mut out = String::with_capacity(content.len());
        let mut idx = 0usize;
        let mut modified = false;
        let mut file_edits = Vec::new();

        while idx < lines.len() {
            let line_number = idx + 1;
            let proof = file_proofs.get(&line_number).copied();
            if let Some(proof) = proof {
                if kconfig_dead_symbol_definition_proof_matches_line(
                    proof,
                    &lines,
                    &help_text,
                    idx,
                ) {
                    let line = lines[idx];
                    let (directive_text, comment_suffix) = split_kconfig_trailing_comment(line);
                    let replacement = format!(
                        "{}# kslim: removed unreachable {} {}{}\n",
                        line_indentation_prefix(directive_text),
                        kconfig_symbol_definition_kind_keyword(proof.definition_kind),
                        proof.symbol.as_str(),
                        comment_suffix
                    );
                    out.push_str(&replacement);
                    modified = true;
                    removed_blocks += 1;
                    file_edits.push(EditRecord::new(
                        relative.clone(),
                        Some(LineRange {
                            start: proof.start_line,
                            end: proof.end_line,
                        }),
                        join_lines(&lines[proof.start_line - 1..proof.end_line]),
                        replacement,
                        EditReason::RemovedDeadKconfigSymbolDefinition {
                            symbol: proof.symbol.clone(),
                        },
                        EditProofSource::kconfig_solver_unreachable_symbol_definition(
                            proof.symbol.clone(),
                            relative.clone(),
                            proof.start_line,
                        ),
                        "kconfig.rewrite_dead_symbol_definitions",
                    ));
                    idx = proof.end_line;
                    continue;
                }
            }

            out.push_str(lines[idx]);
            out.push('\n');
            idx += 1;
        }

        if modified {
            write_verified_rewrite(
                root,
                &path,
                &out,
                &file_edits,
                "kconfig.rewrite_dead_symbol_definitions",
            )?;
            edits.extend(file_edits);
        }
    }

    ensure_edit_records_for_mutation(
        "kconfig.rewrite_dead_symbol_definitions",
        removed_blocks,
        &edits,
    )?;
    sort_edit_records(&mut edits);

    Ok((removed_blocks, edits))
}

#[allow(dead_code)]
pub(crate) fn rewrite_empty_kconfig_menus(
    root: &Path,
    proofs: &[KconfigEmptyMenuRemovalProof],
) -> Result<(usize, Vec<EditRecord>)> {
    let mut removed_menus = 0usize;
    let mut edits = Vec::new();
    let proofs_by_file = empty_menu_removal_proofs_by_file(proofs);

    for path in kconfig_files(root) {
        let relative = relative_to_root_path(root, &path);
        let Some(file_proofs) = proofs_by_file.get(&relative) else {
            continue;
        };

        let content = std::fs::read_to_string(&path)?;
        let lines: Vec<&str> = content.lines().collect();
        let document = parse_kconfig_document(&content)?;
        let mut out = String::with_capacity(content.len());
        let mut idx = 0usize;
        let mut modified = false;
        let mut file_edits = Vec::new();

        while idx < lines.len() {
            let line_number = idx + 1;
            let proof = file_proofs.get(&line_number).copied();
            if let Some(proof) = proof {
                if kconfig_empty_menu_removal_proof_matches_document(proof, &document) {
                    let line = lines[idx];
                    let (directive_text, comment_suffix) = split_kconfig_trailing_comment(line);
                    let replacement = format!(
                        "{}# kslim: removed empty {}{}\n",
                        line_indentation_prefix(directive_text),
                        directive_text.trim_start(),
                        comment_suffix
                    );
                    out.push_str(&replacement);
                    modified = true;
                    removed_menus += 1;
                    file_edits.push(EditRecord::new(
                        relative.clone(),
                        Some(LineRange {
                            start: proof.start_line,
                            end: proof.end_line,
                        }),
                        join_lines(&lines[proof.start_line - 1..proof.end_line]),
                        replacement,
                        EditReason::RemovedEmptyKconfigMenu {
                            prompt: proof.prompt.clone(),
                        },
                        EditProofSource::kconfig_solver_empty_menu(
                            proof.prompt.clone(),
                            relative.clone(),
                            proof.start_line,
                        ),
                        "kconfig.rewrite_empty_menus",
                    ));
                    idx = proof.end_line;
                    continue;
                }
            }

            out.push_str(lines[idx]);
            out.push('\n');
            idx += 1;
        }

        if modified {
            write_verified_rewrite(
                root,
                &path,
                &out,
                &file_edits,
                "kconfig.rewrite_empty_menus",
            )?;
            edits.extend(file_edits);
        }
    }

    ensure_edit_records_for_mutation("kconfig.rewrite_empty_menus", removed_menus, &edits)?;
    sort_edit_records(&mut edits);

    Ok((removed_menus, edits))
}

#[allow(dead_code)]
fn dead_kconfig_symbol_definition_proofs_by_file(
    proofs: &[KconfigDeadSymbolDefinitionProof],
) -> BTreeMap<PathBuf, BTreeMap<usize, &KconfigDeadSymbolDefinitionProof>> {
    let mut by_file: BTreeMap<PathBuf, BTreeMap<usize, &KconfigDeadSymbolDefinitionProof>> =
        BTreeMap::new();
    for proof in proofs {
        if proof.start_line == 0
            || proof.end_line < proof.start_line
            || !kconfig_dead_symbol_definition_kind_is_rewrite_supported(proof.definition_kind)
        {
            continue;
        }
        by_file
            .entry(proof.file.clone())
            .or_default()
            .insert(proof.start_line, proof);
    }
    by_file
}

#[allow(dead_code)]
fn kconfig_dead_symbol_definition_proof_matches_line(
    proof: &KconfigDeadSymbolDefinitionProof,
    lines: &[&str],
    help_text: &[bool],
    idx: usize,
) -> bool {
    if proof.start_line != idx + 1 || proof.end_line > lines.len() || help_text[idx] {
        return false;
    }
    let Some(KconfigDirective::Entry { kind, symbol }) = parse_kconfig_directive(lines[idx]) else {
        return false;
    };
    if symbol != proof.symbol || kind != kconfig_entry_kind_for_definition(proof.definition_kind) {
        return false;
    }
    kconfig_definition_block_end(lines, help_text, idx) == proof.end_line
}

#[allow(dead_code)]
fn empty_menu_removal_proofs_by_file(
    proofs: &[KconfigEmptyMenuRemovalProof],
) -> BTreeMap<PathBuf, BTreeMap<usize, &KconfigEmptyMenuRemovalProof>> {
    let mut by_file: BTreeMap<PathBuf, BTreeMap<usize, &KconfigEmptyMenuRemovalProof>> =
        BTreeMap::new();
    for proof in proofs {
        if proof.start_line == 0 || proof.end_line < proof.start_line || proof.prompt.is_empty() {
            continue;
        }
        by_file
            .entry(proof.file.clone())
            .or_default()
            .insert(proof.start_line, proof);
    }
    by_file
}

#[allow(dead_code)]
fn kconfig_empty_menu_removal_proof_matches_document(
    proof: &KconfigEmptyMenuRemovalProof,
    document: &KconfigDocument,
) -> bool {
    if proof.start_line == 0 || proof.end_line < proof.start_line || proof.prompt.is_empty() {
        return false;
    }

    for (idx, node) in document.nodes().iter().enumerate() {
        let KconfigNode::Menu(menu) = node else {
            continue;
        };
        if menu.line() != proof.start_line || menu.prompt() != proof.prompt {
            continue;
        }
        if kconfig_menu_block_end_line_from_nodes(document.nodes(), idx) != Some(proof.end_line) {
            return false;
        }
        return kconfig_empty_menu_block_is_cleanup_only(document.nodes(), proof);
    }

    false
}

#[allow(dead_code)]
fn kconfig_empty_menu_block_is_cleanup_only(
    nodes: &[KconfigNode],
    proof: &KconfigEmptyMenuRemovalProof,
) -> bool {
    let mut saw_cleanup_comment = false;

    for node in nodes {
        let Some((start, end)) = kconfig_node_line_range(node) else {
            return false;
        };
        if end < proof.start_line || start > proof.end_line {
            continue;
        }

        match node {
            KconfigNode::Menu(menu) => {
                let Some(body_has_cleanup_comment) =
                    kconfig_wrapper_body_cleanup_status(menu.body())
                else {
                    return false;
                };
                saw_cleanup_comment |= body_has_cleanup_comment;
            }
            KconfigNode::If(if_entry) => {
                let Some(body_has_cleanup_comment) =
                    kconfig_wrapper_body_cleanup_status(if_entry.body())
                else {
                    return false;
                };
                saw_cleanup_comment |= body_has_cleanup_comment;
            }
            KconfigNode::Endmenu(_)
            | KconfigNode::Endif(_)
            | KconfigNode::BlankLine(_) => {}
            KconfigNode::LineComment(comment) => {
                saw_cleanup_comment |= comment.raw().text().contains("# kslim: removed ");
            }
            KconfigNode::Config(_)
            | KconfigNode::Menuconfig(_)
            | KconfigNode::Choice(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::Comment(_)
            | KconfigNode::SkippedSite(_) => return false,
        }
    }

    saw_cleanup_comment
}

#[allow(dead_code)]
fn kconfig_wrapper_body_is_cleanup_safe(body: &[KconfigRawLine]) -> bool {
    kconfig_wrapper_body_cleanup_status(body).is_some()
}

#[allow(dead_code)]
fn kconfig_wrapper_body_cleanup_status(body: &[KconfigRawLine]) -> Option<bool> {
    let mut saw_cleanup_comment = false;

    for line in body {
        let trimmed = line.text().trim_start();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') {
            saw_cleanup_comment |= trimmed.contains("# kslim: removed ");
            continue;
        }
        if !matches!(
            parse_kconfig_directive(line.text()),
            Some(KconfigDirective::DependsOn { .. } | KconfigDirective::VisibleIf { .. })
        ) {
            return None;
        }
    }

    Some(saw_cleanup_comment)
}

#[allow(dead_code)]
fn kconfig_definition_block_end(lines: &[&str], help_text: &[bool], idx: usize) -> usize {
    let base_indent = indentation(lines[idx]);
    let mut end = idx + 1;
    while end < lines.len() {
        let next = lines[end];
        if help_text[end] {
            end += 1;
            continue;
        }
        let next_trimmed = next.trim_start();
        if next_trimmed.is_empty() {
            end += 1;
            continue;
        }
        if indentation(next) <= base_indent && is_kconfig_boundary(next_trimmed) {
            break;
        }
        end += 1;
    }
    end
}

#[allow(dead_code)]
fn kconfig_entry_kind_for_definition(
    definition_kind: KconfigSymbolDefinitionKind,
) -> KconfigEntryKind {
    match definition_kind {
        KconfigSymbolDefinitionKind::Config => KconfigEntryKind::Config,
        KconfigSymbolDefinitionKind::Menuconfig => KconfigEntryKind::Menuconfig,
        KconfigSymbolDefinitionKind::Choice => KconfigEntryKind::Config,
    }
}

fn find_config_definitions(
    root: &Path,
    overrides: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, Vec<(PathBuf, usize)>>> {
    let wanted: HashSet<&str> = overrides.keys().map(String::as_str).collect();
    let mut definitions: BTreeMap<String, Vec<(PathBuf, usize)>> = BTreeMap::new();

    for path in kconfig_files(root) {
        let content = std::fs::read_to_string(&path)?;
        let lines: Vec<&str> = content.lines().collect();
        let help_text = kconfig_help_text_mask(&lines);
        for (idx, line) in lines.iter().copied().enumerate() {
            if help_text[idx] {
                continue;
            }
            let Some(symbol) = parse_config_symbol(line) else {
                continue;
            };
            if wanted.contains(symbol.as_str()) {
                definitions
                    .entry(symbol)
                    .or_default()
                    .push((path.clone(), idx + 1));
            }
        }
    }

    Ok(definitions)
}

fn analyze_kconfig_relation_line(
    line: &str,
    removed: &HashSet<&str>,
    source_symbol: Option<&str>,
) -> RelationLineAnalysis {
    let (directive_text, comment_suffix) = split_kconfig_trailing_comment(line);
    if kconfig_prompt_text_line(directive_text) {
        return RelationLineAnalysis::Noop;
    }
    let indent = line_indentation_prefix(directive_text);
    match parse_kconfig_directive(directive_text) {
        None => RelationLineAnalysis::Noop,
        Some(KconfigDirective::DependsOn { expr }) => {
            analyze_conditional_line(indent, "depends on ", &expr, comment_suffix, removed)
        }
        Some(KconfigDirective::VisibleIf { expr }) => {
            analyze_conditional_line(indent, "visible if ", &expr, comment_suffix, removed)
        }
        Some(KconfigDirective::Default { value, condition }) => analyze_default_line(
            indent,
            &value,
            condition.as_deref(),
            comment_suffix,
            removed,
        ),
        Some(KconfigDirective::Select { symbol, condition }) => analyze_target_line(
            indent,
            "select",
            &symbol,
            condition.as_deref(),
            comment_suffix,
            removed,
            source_symbol,
        ),
        Some(KconfigDirective::Imply { symbol, condition }) => analyze_target_line(
            indent,
            "imply",
            &symbol,
            condition.as_deref(),
            comment_suffix,
            removed,
            source_symbol,
        ),
        Some(KconfigDirective::If { expr }) => {
            analyze_conditional_line(indent, "if ", &expr, comment_suffix, removed)
        }
        Some(_) => RelationLineAnalysis::Noop,
    }
}

fn analyze_conditional_line(
    indent: &str,
    keyword: &str,
    expr_src: &str,
    comment_suffix: &str,
    removed: &HashSet<&str>,
) -> RelationLineAnalysis {
    let Some(expr) = parse_kconfig_expr(expr_src) else {
        return unsupported_if_removed_symbol(keyword.trim(), expr_src, removed);
    };
    let Some(removed_symbol) = first_removed_symbol(&expr, removed) else {
        return RelationLineAnalysis::Noop;
    };
    let Some(simplified) = equivalent_kconfig_expr_simplification(&expr, removed) else {
        return RelationLineAnalysis::Noop;
    };
    if simplified == expr {
        return RelationLineAnalysis::Noop;
    }

    RelationLineAnalysis::Rewrite(RelationRewrite::Replace {
        line: format!(
            "{indent}{keyword}{}{}",
            render_kconfig_expr(&simplified),
            comment_suffix
        ),
        reason: EditReason::SimplifiedTristateExpr {
            symbol: removed_symbol,
        },
        report_kind: match keyword.trim() {
            "depends on" => Some(KconfigReportKind::SimplifiedDepends),
            "visible if" => Some(KconfigReportKind::SimplifiedVisibleIf),
            "if" => None,
            _ => return RelationLineAnalysis::Noop,
        },
    })
}

fn analyze_default_line(
    indent: &str,
    value: &str,
    condition: Option<&str>,
    comment_suffix: &str,
    removed: &HashSet<&str>,
) -> RelationLineAnalysis {
    let Some(expr_src) = condition else {
        return RelationLineAnalysis::Noop;
    };
    let Some(expr) = parse_kconfig_expr(expr_src) else {
        return unsupported_if_removed_symbol("default", expr_src, removed);
    };
    let Some(removed_symbol) = first_removed_symbol(&expr, removed) else {
        return RelationLineAnalysis::Noop;
    };
    let Some(simplified) = equivalent_kconfig_expr_simplification(&expr, removed) else {
        return RelationLineAnalysis::Noop;
    };
    if simplified == expr {
        return RelationLineAnalysis::Noop;
    }

    match simplified {
        KconfigExpr::Literal(TristateLiteral::N) => {
            RelationLineAnalysis::Rewrite(RelationRewrite::Delete {
                reason: EditReason::SimplifiedTristateExpr {
                    symbol: removed_symbol,
                },
                report_kind: Some(KconfigReportKind::SimplifiedDefault),
            })
        }
        KconfigExpr::Literal(TristateLiteral::Y) => {
            RelationLineAnalysis::Rewrite(RelationRewrite::Replace {
                line: format!("{indent}default {}{}", value.trim(), comment_suffix),
                reason: EditReason::SimplifiedTristateExpr {
                    symbol: removed_symbol,
                },
                report_kind: Some(KconfigReportKind::SimplifiedDefault),
            })
        }
        _ => RelationLineAnalysis::Rewrite(RelationRewrite::Replace {
            line: format!(
                "{indent}default {} if {}{}",
                value.trim(),
                render_kconfig_expr(&simplified),
                comment_suffix
            ),
            reason: EditReason::SimplifiedTristateExpr {
                symbol: removed_symbol,
            },
            report_kind: Some(KconfigReportKind::SimplifiedDefault),
        }),
    }
}

fn analyze_target_line(
    indent: &str,
    keyword: &str,
    target: &str,
    condition: Option<&str>,
    comment_suffix: &str,
    removed: &HashSet<&str>,
    source_symbol: Option<&str>,
) -> RelationLineAnalysis {
    let target = target.trim();
    if target.is_empty() {
        return RelationLineAnalysis::Noop;
    }

    if removed.contains(target) {
        if matches!(keyword, "select" | "imply")
            && !kconfig_relation_source_remains_valid(source_symbol, removed)
        {
            return RelationLineAnalysis::Noop;
        }

        if let Some(condition) = condition {
            if parse_kconfig_expr(condition).is_none() {
                return unsupported_kconfig_expression(
                    keyword,
                    condition,
                    KCONFIG_UNKNOWN_REMOVED_TARGET_CONDITION_REASON,
                );
            }
        }

        return RelationLineAnalysis::Rewrite(RelationRewrite::Delete {
            reason: EditReason::ManifestConfig {
                symbol: target.to_string(),
            },
            report_kind: Some(match keyword {
                "select" => KconfigReportKind::DroppedSelect,
                "imply" => KconfigReportKind::DroppedImply,
                _ => return RelationLineAnalysis::Noop,
            }),
        });
    }

    let Some(condition) = condition else {
        return RelationLineAnalysis::Noop;
    };
    let Some(expr) = parse_kconfig_expr(condition) else {
        return unsupported_if_removed_symbol(keyword, condition, removed);
    };
    let Some(removed_symbol) = first_removed_symbol(&expr, removed) else {
        return RelationLineAnalysis::Noop;
    };
    let Some(simplified) = equivalent_kconfig_expr_simplification(&expr, removed) else {
        return RelationLineAnalysis::Noop;
    };
    if simplified == expr {
        return RelationLineAnalysis::Noop;
    }

    match simplified {
        KconfigExpr::Literal(TristateLiteral::N) => {
            RelationLineAnalysis::Rewrite(RelationRewrite::Delete {
                reason: EditReason::SimplifiedTristateExpr {
                    symbol: removed_symbol,
                },
                report_kind: Some(match keyword {
                    "select" => KconfigReportKind::DroppedSelect,
                    "imply" => KconfigReportKind::DroppedImply,
                    _ => return RelationLineAnalysis::Noop,
                }),
            })
        }
        KconfigExpr::Literal(TristateLiteral::Y) => {
            RelationLineAnalysis::Rewrite(RelationRewrite::Replace {
                line: format!("{indent}{keyword} {target}{comment_suffix}"),
                reason: EditReason::SimplifiedTristateExpr {
                    symbol: removed_symbol,
                },
                report_kind: None,
            })
        }
        _ => RelationLineAnalysis::Rewrite(RelationRewrite::Replace {
            line: format!(
                "{indent}{keyword} {target} if {}{}",
                render_kconfig_expr(&simplified),
                comment_suffix
            ),
            reason: EditReason::SimplifiedTristateExpr {
                symbol: removed_symbol,
            },
            report_kind: None,
        }),
    }
}

fn clear_kconfig_relation_source_context_for_line(
    source_context: &mut Option<KconfigRelationSourceContext>,
    line: &str,
) {
    let Some(context) = source_context else {
        return;
    };

    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return;
    }

    if indentation(line) <= context.indentation && is_kconfig_boundary(trimmed) {
        *source_context = None;
    }
}

fn kconfig_prompt_text_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if strip_kconfig_keyword(trimmed, "prompt").is_some() {
        return true;
    }

    ["bool", "tristate", "int", "hex", "string"]
        .iter()
        .any(|keyword| {
            strip_kconfig_keyword(trimmed, keyword)
                .is_some_and(|rest| rest.trim_start().starts_with('"'))
        })
}

fn kconfig_relation_source_context_from_line(
    line: &str,
) -> Option<KconfigRelationSourceContext> {
    let (directive_text, _) = split_kconfig_trailing_comment(line);
    let Some(KconfigDirective::Entry { symbol, .. }) = parse_kconfig_directive(directive_text)
    else {
        return None;
    };

    Some(KconfigRelationSourceContext {
        symbol,
        indentation: indentation(line),
    })
}

fn kconfig_relation_source_remains_valid(
    source_symbol: Option<&str>,
    removed: &HashSet<&str>,
) -> bool {
    source_symbol.is_some_and(|source_symbol| !removed.contains(source_symbol))
}

fn unsupported_if_removed_symbol(
    directive: &str,
    expr_src: &str,
    removed: &HashSet<&str>,
) -> RelationLineAnalysis {
    if expr_mentions_removed_symbol(expr_src, removed) {
        unsupported_kconfig_expression(
            directive,
            expr_src,
            KCONFIG_UNSUPPORTED_REMOVED_SYMBOL_EXPRESSION_REASON,
        )
    } else {
        RelationLineAnalysis::Noop
    }
}

fn unsupported_kconfig_expression(
    directive: &str,
    expr_src: &str,
    reason: &str,
) -> RelationLineAnalysis {
    RelationLineAnalysis::Unsupported {
        directive: directive.to_string(),
        expression: expr_src.trim().to_string(),
        reason: reason.to_string(),
    }
}

fn expr_mentions_removed_symbol(expr_src: &str, removed: &HashSet<&str>) -> bool {
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escape = false;
    for ch in expr_src.chars() {
        if escape {
            escape = false;
            continue;
        }
        match ch {
            '\\' if in_quotes => {
                escape = true;
                continue;
            }
            '"' => {
                if !current.is_empty() {
                    if removed.contains(current.as_str()) {
                        return true;
                    }
                    current.clear();
                }
                in_quotes = !in_quotes;
                continue;
            }
            _ if in_quotes => continue,
            _ => {}
        }

        if is_kconfig_symbol_char(ch) {
            current.push(ch);
            continue;
        }
        if !current.is_empty() {
            if removed.contains(current.as_str()) {
                return true;
            }
            current.clear();
        }
    }
    !current.is_empty() && removed.contains(current.as_str())
}

fn increment_kconfig_report_count(report: &mut KconfigReportCounts, kind: KconfigReportKind) {
    match kind {
        KconfigReportKind::DroppedSelect => report.dropped_selects += 1,
        KconfigReportKind::DroppedImply => report.dropped_implies += 1,
        KconfigReportKind::SimplifiedDepends => report.simplified_depends += 1,
        KconfigReportKind::SimplifiedVisibleIf => report.simplified_visible_if += 1,
        KconfigReportKind::SimplifiedDefault => report.simplified_defaults += 1,
    }
}

fn rewrite_config_default_block(block: &[&str], value: &str) -> String {
    if block.is_empty() {
        return String::new();
    }

    let default_value = value.trim();
    let body_indent = block_body_indent(&block[1..]);
    let mut body = Vec::new();
    let mut insert_pos = None;
    let mut idx = 0usize;
    let source = &block[1..];

    while idx < source.len() {
        let line = source[idx];
        let trimmed = line.trim_start();
        if is_kconfig_help_directive(trimmed) {
            let help_indent = indentation(line);
            body.push((*line).to_string());
            idx += 1;
            while idx < source.len() {
                let next = source[idx];
                if next.trim().is_empty() || indentation(next) > help_indent {
                    body.push(next.to_string());
                    idx += 1;
                } else {
                    break;
                }
            }
            continue;
        }
        if trimmed == "def_bool" || trimmed.starts_with("def_bool ") {
            let indent = &line[..line.len() - trimmed.len()];
            body.push(format!("{}bool", indent));
            insert_pos.get_or_insert(body.len());
            idx += 1;
            continue;
        }
        if trimmed == "def_tristate" || trimmed.starts_with("def_tristate ") {
            let indent = &line[..line.len() - trimmed.len()];
            body.push(format!("{}tristate", indent));
            insert_pos.get_or_insert(body.len());
            idx += 1;
            continue;
        }
        if trimmed.starts_with("default ") {
            insert_pos.get_or_insert(body.len());
            idx += 1;
            continue;
        }

        body.push((*line).to_string());
        idx += 1;
    }

    if insert_pos.is_none() {
        insert_pos = body
            .iter()
            .position(|line| is_kconfig_type_line(line.trim_start()))
            .map(|idx| idx + 1)
            .or(Some(0));
    }

    body.insert(
        insert_pos.unwrap_or(0),
        format!("{}default {}", body_indent, default_value),
    );

    let mut out = String::new();
    out.push_str(block[0]);
    out.push('\n');
    for line in body {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn block_body_indent(lines: &[&str]) -> String {
    lines
        .iter()
        .find_map(|line| {
            let trimmed = line.trim_start();
            if trimmed.is_empty() {
                None
            } else {
                Some(line[..line.len() - trimmed.len()].to_string())
            }
        })
        .unwrap_or_else(|| "\t".to_string())
}

fn is_kconfig_type_line(trimmed: &str) -> bool {
    ["bool", "tristate", "int", "hex", "string"]
        .iter()
        .any(|kind| trimmed == *kind || trimmed.starts_with(&format!("{kind} ")))
}
