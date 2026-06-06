use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::cpp::UnsupportedCppExpression;
use crate::diagnostics::ClassifiedDiagnostic;
use crate::edit_reason::{EditProofSourceKind, EditReason, EditRecord};
use crate::fixups::{AppliedFixup, FixupProof, SkippedFixup};
use crate::kbuild::{logical_lines, parse_kbuild_assignment, CompositeKind, KbuildAssignmentKind};
use crate::kconfig::UnsupportedKconfigExpression;

use super::super::result::{committed_result_path, sanitize_committed_result_text};
use super::super::ReducerStats;
use super::model::{KbuildEditReport, UnknownDiagnosticPolicy, UnsupportedSyntaxPolicy};

pub(super) fn has_reducer_diagnostics(stats: &ReducerStats) -> bool {
    !stats.unsupported_kconfig_expressions.is_empty()
        || !stats.unsupported_cpp_expressions.is_empty()
        || !stats.skipped_cpp_nested_edge_cases.is_empty()
        || !stats.skipped_makefile_lines.is_empty()
        || !stats.skipped_fixups.is_empty()
        || !stats.manual_include_sites.is_empty()
        || stats.include_report.live_missing_includes > 0
        || stats.include_report.ambiguous_includes > 0
}

pub(super) fn count_edit_proof_sources(
    edits: &[EditRecord],
) -> BTreeMap<EditProofSourceKind, usize> {
    let mut counts = BTreeMap::new();
    for edit in edits {
        *counts.entry(edit.proof_source.kind()).or_default() += 1;
    }
    counts
}

pub(super) fn render_kbuild_edit_report(edits: &[EditRecord]) -> KbuildEditReport {
    let mut report = KbuildEditReport::default();
    let composite_targets = cleaned_composite_targets(edits);

    for edit in edits {
        if let EditReason::RemovedKbuildRef { reference } = &edit.reason {
            if reference.starts_with("-I") {
                report.removed_stale_include_paths += 1;
            } else if reference.ends_with('/') {
                report.removed_directory_refs += 1;
            } else if reference.ends_with(".o") {
                if composite_targets.contains(&(edit.file.clone(), reference.clone())) {
                    report.cleaned_composite_objects += 1;
                } else {
                    report.removed_object_refs += 1;
                }
            }
        }
    }

    report
}

fn cleaned_composite_targets(edits: &[EditRecord]) -> BTreeSet<(PathBuf, String)> {
    let mut targets = BTreeSet::new();

    for edit in edits {
        let EditReason::RemovedKbuildRef { .. } = &edit.reason else {
            continue;
        };

        for logical in logical_lines(&edit.before) {
            let Some(assignment) = parse_kbuild_assignment(&logical.joined) else {
                continue;
            };
            let KbuildAssignmentKind::CompositeMembers(kind) = assignment.kind else {
                continue;
            };

            targets.insert((
                edit.file.clone(),
                format!("{}.o", composite_target_name(&kind)),
            ));
        }
    }

    targets
}

fn composite_target_name<'a>(kind: &'a CompositeKind<'a>) -> &'a str {
    match kind {
        CompositeKind::BuiltIn { target }
        | CompositeKind::Module { target }
        | CompositeKind::Config { target, .. }
        | CompositeKind::Objs { target } => target,
    }
}

pub(super) fn sorted_applied_fixups(fixups: &[AppliedFixup]) -> Vec<&AppliedFixup> {
    let mut refs = fixups.iter().collect::<Vec<_>>();
    refs.sort_by(|left, right| {
        left.fixer_name
            .cmp(right.fixer_name)
            .then(
                classified_diagnostic_sort_key(&left.diagnostic)
                    .cmp(&classified_diagnostic_sort_key(&right.diagnostic)),
            )
            .then(left.edits.len().cmp(&right.edits.len()))
            .then(left.proof_sources.len().cmp(&right.proof_sources.len()))
    });
    refs
}

pub(super) fn sorted_skipped_fixups(skipped_fixups: &[SkippedFixup]) -> Vec<&SkippedFixup> {
    let mut refs = skipped_fixups.iter().collect::<Vec<_>>();
    refs.sort_by(|left, right| {
        classified_diagnostic_sort_key(&left.diagnostic)
            .cmp(&classified_diagnostic_sort_key(&right.diagnostic))
            .then(
                left.fixer_name
                    .unwrap_or("")
                    .cmp(right.fixer_name.unwrap_or("")),
            )
            .then(left.reason.cmp(&right.reason))
    });
    refs
}

fn classified_diagnostic_sort_key(
    diagnostic: &ClassifiedDiagnostic,
) -> (u8, String, Option<usize>, String, String, String, String) {
    (
        classified_diagnostic_rank(diagnostic),
        diagnostic
            .file()
            .map(committed_path_string)
            .unwrap_or_default(),
        diagnostic.line(),
        diagnostic.subject().unwrap_or("").to_string(),
        diagnostic.build_target().unwrap_or("").to_string(),
        diagnostic.arch().unwrap_or("").to_string(),
        diagnostic.config().unwrap_or("").to_string(),
    )
}

fn classified_diagnostic_rank(diagnostic: &ClassifiedDiagnostic) -> u8 {
    match diagnostic {
        ClassifiedDiagnostic::MissingHeader { .. } => 0,
        ClassifiedDiagnostic::MissingKconfigSource { .. } => 1,
        ClassifiedDiagnostic::MissingMakeDirectory { .. } => 2,
        ClassifiedDiagnostic::MissingMakeTarget { .. } => 3,
        ClassifiedDiagnostic::UndeclaredIdentifier { .. } => 4,
        ClassifiedDiagnostic::ImplicitDeclaration { .. } => 5,
        ClassifiedDiagnostic::UndefinedReference { .. } => 6,
        ClassifiedDiagnostic::Unknown => 7,
    }
}

pub(super) fn classified_diagnostics_from_stats(
    stats: &ReducerStats,
) -> Vec<&ClassifiedDiagnostic> {
    let mut diagnostics = Vec::new();
    diagnostics.extend(stats.classified_diagnostics.iter());
    diagnostics.extend(stats.applied_fixups.iter().map(|fixup| &fixup.diagnostic));
    diagnostics.extend(
        stats
            .skipped_fixups
            .iter()
            .map(|skipped| &skipped.diagnostic),
    );
    diagnostics.sort_by(|left, right| {
        classified_diagnostic_sort_key(left).cmp(&classified_diagnostic_sort_key(right))
    });
    diagnostics.dedup_by(|left, right| {
        classified_diagnostic_sort_key(left) == classified_diagnostic_sort_key(right)
    });
    diagnostics
}

pub(super) fn render_classified_diagnostic_md(diagnostic: &ClassifiedDiagnostic) -> String {
    crate::diagnostics::render_classified_diagnostic_md(
        diagnostic,
        committed_path_string,
        sanitize_committed_result_text,
    )
}

pub(super) fn render_fixup_proof_md(proof: &FixupProof) -> String {
    match proof {
        FixupProof::ManifestPath { path } => {
            format!("manifest path {}", committed_path_string(path))
        }
        FixupProof::TreeIndexIncludeSite { file, line, target } => {
            format!(
                "tree index include {}:{} -> {}",
                committed_path_string(file),
                line,
                sanitize_committed_result_text(target)
            )
        }
        FixupProof::TreeIndexKbuildDirectoryRef {
            file,
            line,
            assignment_lhs,
            directory,
            resolved_path,
        } => format!(
            "tree index kbuild directory {}:{} {} += {} -> {}",
            committed_path_string(file),
            line,
            sanitize_committed_result_text(assignment_lhs),
            sanitize_committed_result_text(directory),
            committed_path_string(resolved_path)
        ),
        FixupProof::TreeIndexKbuildObjectRef {
            file,
            line,
            assignment_lhs,
            object,
            resolved_path,
        } => format!(
            "tree index kbuild object {}:{} {} += {} -> {}",
            committed_path_string(file),
            line,
            sanitize_committed_result_text(assignment_lhs),
            sanitize_committed_result_text(object),
            committed_path_string(resolved_path)
        ),
        FixupProof::TreeIndexKconfigSourceRef {
            file,
            line,
            source,
            optional,
            relative,
        } => format!(
            "tree index Kconfig source {}:{} source={} optional={} relative={}",
            committed_path_string(file),
            line,
            sanitize_committed_result_text(source),
            optional,
            relative
        ),
        FixupProof::ClassifiedDiagnostic {
            class,
            file,
            line,
            subject,
        } => format!(
            "classified diagnostic class={} file={} line={} subject={}",
            class.stable_name(),
            file.as_deref()
                .map(committed_path_string)
                .unwrap_or_else(|| String::from("<none>")),
            line.map(|line| line.to_string())
                .unwrap_or_else(|| String::from("<none>")),
            subject
                .as_deref()
                .map(sanitize_committed_result_text)
                .unwrap_or_else(|| String::from("<none>"))
        ),
    }
}

pub(super) fn committed_paths_as_strings(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| committed_path_string(path))
        .collect()
}

pub(super) fn committed_path_string(path: &Path) -> String {
    committed_result_path(path).to_string_lossy().to_string()
}

pub(super) fn sorted_refs<T: Ord>(items: &[T]) -> Vec<&T> {
    let mut refs = items.iter().collect::<Vec<_>>();
    refs.sort();
    refs
}

pub(crate) fn render_unsupported_expression_report(
    unsupported_kconfig: &[UnsupportedKconfigExpression],
    unsupported_cpp: &[UnsupportedCppExpression],
) -> String {
    let mut out = String::new();

    if !unsupported_kconfig.is_empty() {
        out.push_str("unsupported Kconfig expressions referencing removed symbols were found:\n");
        for site in sorted_refs(unsupported_kconfig) {
            out.push_str(&format!(
                "- {}:{} | {} | {} | {}\n",
                site.file.display(),
                site.line,
                site.directive,
                site.expression,
                site.reason
            ));
        }
    }

    if !unsupported_cpp.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(
            "unsupported preprocessor expressions referencing removed symbols were found:\n",
        );
        for site in sorted_refs(unsupported_cpp) {
            out.push_str(&format!(
                "- {}:{} | {} | {} | {}\n",
                site.file.display(),
                site.line,
                site.directive,
                site.expression,
                site.reason
            ));
        }
    }
    out
}

pub(super) fn has_unsupported_syntax(stats: &ReducerStats) -> bool {
    !stats.unsupported_kconfig_expressions.is_empty()
        || !stats.unsupported_cpp_expressions.is_empty()
}

pub(super) fn has_unknown_diagnostics(stats: &ReducerStats) -> bool {
    stats
        .classified_diagnostics
        .iter()
        .any(ClassifiedDiagnostic::is_unknown_class)
        || stats
            .applied_fixups
            .iter()
            .any(|fixup| fixup.diagnostic.is_unknown_class())
        || stats.skipped_fixups.iter().any(|skipped| {
            skipped.diagnostic.is_unknown_class() || skipped.reason == "unknown diagnostic"
        })
}

pub(crate) fn validate_unknown_diagnostic_policy(
    stats: &ReducerStats,
    policy: UnknownDiagnosticPolicy,
) -> anyhow::Result<()> {
    if policy.reject_unknown_diagnostics && has_unknown_diagnostics(stats) {
        anyhow::bail!("{}", render_unknown_diagnostic_report(stats));
    }
    Ok(())
}

pub(crate) fn validate_unsupported_syntax_policy(
    stats: &ReducerStats,
    policy: UnsupportedSyntaxPolicy,
) -> anyhow::Result<()> {
    if policy.reject_unsupported_syntax && has_unsupported_syntax(stats) {
        anyhow::bail!("{}", render_unsupported_syntax_report(stats));
    }
    Ok(())
}

pub(super) fn render_unsupported_syntax_report(stats: &ReducerStats) -> String {
    render_unsupported_expression_report(
        &stats.unsupported_kconfig_expressions,
        &stats.unsupported_cpp_expressions,
    )
}

fn render_unknown_diagnostic_report(stats: &ReducerStats) -> String {
    let sites = unknown_diagnostic_sites(stats);
    let mut out = format!(
        "unknown diagnostic in strict mode: reducer reported {} unknown diagnostic(s)",
        sites.len()
    );
    for site in sites {
        out.push_str("\n- ");
        out.push_str(&site);
    }
    out
}

fn unknown_diagnostic_sites(stats: &ReducerStats) -> Vec<String> {
    let mut sites = BTreeSet::new();

    for diagnostic in &stats.classified_diagnostics {
        if diagnostic.is_unknown_class() {
            sites.insert(format!(
                "classified diagnostic: {}",
                render_classified_diagnostic_md(diagnostic)
            ));
        }
    }

    for fixup in &stats.applied_fixups {
        if fixup.diagnostic.is_unknown_class() {
            sites.insert(format!(
                "consumed diagnostic by {}: {}",
                sanitize_committed_result_text(&fixup.fixer_name),
                render_classified_diagnostic_md(&fixup.diagnostic)
            ));
        }
    }

    for skipped in &stats.skipped_fixups {
        if skipped.diagnostic.is_unknown_class() || skipped.reason == "unknown diagnostic" {
            let fixer_suffix = skipped
                .fixer_name
                .map(|fixer| format!(" by {}", sanitize_committed_result_text(fixer)))
                .unwrap_or_default();
            sites.insert(format!(
                "skipped diagnostic{}: reason={} {}",
                fixer_suffix,
                sanitize_committed_result_text(&skipped.reason),
                render_classified_diagnostic_md(&skipped.diagnostic)
            ));
        }
    }

    sites.into_iter().collect()
}
