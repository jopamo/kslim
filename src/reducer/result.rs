use crate::cpp::{CppReportCounts, SkippedCppNestedEdgeCase, UnsupportedCppExpression};
use crate::diagnostics::ClassifiedDiagnostic;
use crate::edit_reason::{sort_edit_records, EditRecord};
use crate::fixups::{AppliedFixup, SkippedFixup};
use crate::includes::{IncludeReportCounts, ManualIncludeHandlingSite};
use crate::kbuild::KbuildSkippedLine;
use crate::kconfig::{
    KconfigReportCounts, KconfigSolverReport, UnsupportedKconfigExpression,
};
use crate::prune::{DeclaredPathPruneResult, RemovalAccounting};
use crate::removal_manifest::RemovalManifest;
use crate::tree_index::TreeIndex;
use serde::{Serialize, Serializer};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::diagnostics::RawDiagnosticExcerpt;

pub(crate) const REDUCER_RESULT_HOST_PATH_REDACTION: &str = "<host-path>";

#[derive(Debug, Default, Clone)]
pub struct ReducerStats {
    pub ran: bool,
    pub files_removed: usize,
    pub dirs_removed: usize,
    pub configs_disabled: usize,
    pub defaults_overridden: usize,
    pub kconfig_refs_removed: usize,
    pub makefile_refs_removed: usize,
    pub kconfig_report: KconfigReportCounts,
    pub kconfig_solver_report: KconfigSolverReport,
    pub cpp_report: CppReportCounts,
    pub include_report: IncludeReportCounts,
    pub unsupported_kconfig_expressions: Vec<UnsupportedKconfigExpression>,
    pub unsupported_cpp_expressions: Vec<UnsupportedCppExpression>,
    pub skipped_cpp_nested_edge_cases: Vec<SkippedCppNestedEdgeCase>,
    pub skipped_makefile_lines: Vec<KbuildSkippedLine>,
    pub removal: RemovalAccounting,
    pub edits: Vec<EditRecord>,
    pub applied_fixups: Vec<AppliedFixup>,
    pub skipped_fixups: Vec<SkippedFixup>,
    pub classified_diagnostics: Vec<ClassifiedDiagnostic>,
    pub raw_diagnostic_excerpts: Vec<RawDiagnosticExcerpt>,
    pub manual_include_sites: Vec<ManualIncludeHandlingSite>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ReducerStatus {
    Success,
    FailedUnknownDiagnostic,
    FailedUnsupportedSyntax,
    FailedNonConvergence,
    FailedBuildMatrix,
    FailedInternalInvariant,
}

impl Default for ReducerStatus {
    fn default() -> Self {
        Self::Success
    }
}

impl ReducerStatus {
    pub fn stable_name(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::FailedUnknownDiagnostic => "failed_unknown_diagnostic",
            Self::FailedUnsupportedSyntax => "failed_unsupported_syntax",
            Self::FailedNonConvergence => "failed_non_convergence",
            Self::FailedBuildMatrix => "failed_build_matrix",
            Self::FailedInternalInvariant => "failed_internal_invariant",
        }
    }

    fn from_stats(stats: &ReducerStats) -> Self {
        if stats_have_unknown_diagnostic(stats) {
            Self::FailedUnknownDiagnostic
        } else if stats_have_unsupported_syntax(stats) {
            Self::FailedUnsupportedSyntax
        } else {
            Self::Success
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct ReducerPassReport {
    pub name: String,
    pub changed: bool,
    #[serde(serialize_with = "serialize_committed_paths")]
    pub touched_files: Vec<PathBuf>,
    pub edit_count: usize,
    pub diagnostic_count: usize,
    pub skipped_site_count: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct EditSummary {
    pub total_edits: usize,
    pub files_removed: usize,
    pub dirs_removed: usize,
    pub configs_disabled: usize,
    pub defaults_overridden: usize,
    pub kconfig_refs_removed: usize,
    pub makefile_refs_removed: usize,
    pub cpp_branches_folded: usize,
    pub include_lines_removed: usize,
}

impl EditSummary {
    fn from_stats(stats: &ReducerStats) -> Self {
        Self {
            total_edits: stats.edits.len(),
            files_removed: stats.files_removed,
            dirs_removed: stats.dirs_removed,
            configs_disabled: stats.configs_disabled,
            defaults_overridden: stats.defaults_overridden,
            kconfig_refs_removed: stats.kconfig_refs_removed,
            makefile_refs_removed: stats.makefile_refs_removed,
            cpp_branches_folded: stats.cpp_report.branches_folded,
            include_lines_removed: stats.include_report.removed_include_lines,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticSummary {
    pub unsupported_kconfig_expressions: usize,
    pub unsupported_cpp_expressions: usize,
    pub skipped_cpp_nested_edge_cases: usize,
    pub skipped_makefile_lines: usize,
    pub skipped_fixups: usize,
    pub unknown_diagnostics: usize,
}

impl DiagnosticSummary {
    fn from_stats(stats: &ReducerStats) -> Self {
        Self {
            unsupported_kconfig_expressions: stats.unsupported_kconfig_expressions.len(),
            unsupported_cpp_expressions: stats.unsupported_cpp_expressions.len(),
            skipped_cpp_nested_edge_cases: stats.skipped_cpp_nested_edge_cases.len(),
            skipped_makefile_lines: stats.skipped_makefile_lines.len(),
            skipped_fixups: stats.skipped_fixups.len(),
            unknown_diagnostics: unknown_diagnostic_count(stats),
        }
    }

    fn total(&self) -> usize {
        self.unsupported_kconfig_expressions
            + self.unsupported_cpp_expressions
            + self.skipped_cpp_nested_edge_cases
            + self.skipped_makefile_lines
            + self.skipped_fixups
    }
}

fn stats_have_unknown_diagnostic(stats: &ReducerStats) -> bool {
    diagnostics_from_stats(stats)
        .into_iter()
        .any(ClassifiedDiagnostic::is_unknown_class)
}

fn stats_have_unsupported_syntax(stats: &ReducerStats) -> bool {
    !stats.unsupported_kconfig_expressions.is_empty()
        || !stats.unsupported_cpp_expressions.is_empty()
}

fn unknown_diagnostic_count(stats: &ReducerStats) -> usize {
    diagnostics_from_stats(stats)
        .into_iter()
        .filter(|diagnostic| diagnostic.is_unknown_class())
        .map(classified_diagnostic_key)
        .collect::<BTreeSet<_>>()
        .len()
}

fn diagnostics_from_stats(stats: &ReducerStats) -> Vec<&ClassifiedDiagnostic> {
    let mut diagnostics = Vec::new();
    diagnostics.extend(stats.classified_diagnostics.iter());
    diagnostics.extend(stats.applied_fixups.iter().map(|fixup| &fixup.diagnostic));
    diagnostics.extend(
        stats
            .skipped_fixups
            .iter()
            .map(|skipped| &skipped.diagnostic),
    );
    diagnostics
}

fn classified_diagnostic_key(
    diagnostic: &ClassifiedDiagnostic,
) -> (
    String,
    Option<PathBuf>,
    Option<usize>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    (
        diagnostic.class().stable_name().to_string(),
        diagnostic.file().map(committed_result_path),
        diagnostic.line(),
        diagnostic.subject().map(ToString::to_string),
        diagnostic.build_target().map(ToString::to_string),
        diagnostic.arch().map(ToString::to_string),
        diagnostic.config().map(ToString::to_string),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct SkippedSite {
    pub kind: String,
    #[serde(serialize_with = "serialize_committed_optional_path")]
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    #[serde(serialize_with = "serialize_committed_text")]
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct FixupApplication {
    pub fixer_name: String,
    pub diagnostic_class: String,
    pub edit_count: usize,
    pub proof_source_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum BuildMatrixStatus {
    NotRun,
    Passed,
    Failed,
}

impl Default for BuildMatrixStatus {
    fn default() -> Self {
        Self::NotRun
    }
}

impl BuildMatrixStatus {
    pub fn stable_name(self) -> &'static str {
        match self {
            Self::NotRun => "not_run",
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConvergenceStatus {
    Converged,
    NotConverged,
    NotEvaluated,
}

impl Default for ConvergenceStatus {
    fn default() -> Self {
        Self::Converged
    }
}

impl ConvergenceStatus {
    pub fn stable_name(self) -> &'static str {
        match self {
            Self::Converged => "converged",
            Self::NotConverged => "not_converged",
            Self::NotEvaluated => "not_evaluated",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct ReducerResult {
    pub status: ReducerStatus,
    pub publishable: bool,
    pub passes: Vec<ReducerPassReport>,
    pub edit_summary: EditSummary,
    pub diagnostic_summary: DiagnosticSummary,
    #[serde(serialize_with = "serialize_committed_paths")]
    pub touched_files: Vec<PathBuf>,
    pub skipped_sites: Vec<SkippedSite>,
    pub fixups_applied: Vec<FixupApplication>,
    pub final_build_status: BuildMatrixStatus,
    pub convergence: ConvergenceStatus,
    #[serde(skip_serializing)]
    pub manifest: Option<RemovalManifest>,
    #[serde(skip_serializing)]
    pub initial_index: Option<TreeIndex>,
    #[serde(skip_serializing)]
    pub declared_prune: Option<DeclaredPathPruneResult>,
    #[serde(skip_serializing)]
    pub post_prune_index: Option<TreeIndex>,
    #[serde(skip_serializing)]
    pub post_kconfig_index: Option<TreeIndex>,
    #[serde(skip_serializing)]
    pub post_kbuild_index: Option<TreeIndex>,
    #[serde(skip_serializing)]
    pub post_cpp_index: Option<TreeIndex>,
    #[serde(skip_serializing)]
    pub post_include_index: Option<TreeIndex>,
    #[serde(skip_serializing)]
    pub stats: ReducerStats,
}

impl Default for ReducerResult {
    fn default() -> Self {
        Self {
            status: ReducerStatus::Success,
            publishable: true,
            passes: Vec::new(),
            edit_summary: EditSummary::default(),
            diagnostic_summary: DiagnosticSummary::default(),
            touched_files: Vec::new(),
            skipped_sites: Vec::new(),
            fixups_applied: Vec::new(),
            final_build_status: BuildMatrixStatus::NotRun,
            convergence: ConvergenceStatus::Converged,
            manifest: None,
            initial_index: None,
            declared_prune: None,
            post_prune_index: None,
            post_kconfig_index: None,
            post_kbuild_index: None,
            post_cpp_index: None,
            post_include_index: None,
            stats: ReducerStats::default(),
        }
    }
}

impl ReducerResult {
    pub(crate) fn from_pipeline_artifacts(
        manifest: Option<RemovalManifest>,
        initial_index: Option<TreeIndex>,
        declared_prune: Option<DeclaredPathPruneResult>,
        post_prune_index: Option<TreeIndex>,
        post_kconfig_index: Option<TreeIndex>,
        post_kbuild_index: Option<TreeIndex>,
        post_cpp_index: Option<TreeIndex>,
        post_include_index: Option<TreeIndex>,
        mut stats: ReducerStats,
    ) -> Self {
        normalize_edit_records_in_stats(&mut stats);
        let status = ReducerStatus::from_stats(&stats);
        let publishable = matches!(status, ReducerStatus::Success);
        let edit_summary = EditSummary::from_stats(&stats);
        let diagnostic_summary = DiagnosticSummary::from_stats(&stats);
        let touched_files = touched_files_from_stats(&stats);
        let skipped_sites = skipped_sites_from_stats(&stats);
        let fixups_applied = fixup_applications_from_stats(&stats);
        let passes =
            pass_reports_from_stats(&stats, &touched_files, &diagnostic_summary, &skipped_sites);
        let convergence = match status {
            ReducerStatus::Success => ConvergenceStatus::Converged,
            ReducerStatus::FailedNonConvergence => ConvergenceStatus::NotConverged,
            _ => ConvergenceStatus::NotEvaluated,
        };

        Self {
            status,
            publishable,
            passes,
            edit_summary,
            diagnostic_summary,
            touched_files,
            skipped_sites,
            fixups_applied,
            final_build_status: BuildMatrixStatus::NotRun,
            convergence,
            manifest,
            initial_index,
            declared_prune,
            post_prune_index,
            post_kconfig_index,
            post_kbuild_index,
            post_cpp_index,
            post_include_index,
            stats,
        }
    }

    pub(crate) fn set_publication_state(
        &mut self,
        status: ReducerStatus,
        final_build_status: BuildMatrixStatus,
        convergence: ConvergenceStatus,
    ) {
        self.status = status;
        self.final_build_status = final_build_status;
        self.convergence = convergence;
        self.publishable = matches!(status, ReducerStatus::Success)
            && matches!(convergence, ConvergenceStatus::Converged);
    }

    pub(crate) fn apply_unsupported_syntax_policy(&mut self, fail_on_unsupported_syntax: bool) {
        if fail_on_unsupported_syntax || self.status != ReducerStatus::FailedUnsupportedSyntax {
            return;
        }
        if stats_have_unknown_diagnostic(&self.stats) {
            return;
        }
        self.set_publication_state(
            ReducerStatus::Success,
            BuildMatrixStatus::NotRun,
            ConvergenceStatus::Converged,
        );
    }

    pub(crate) fn apply_unknown_diagnostic_policy(&mut self, fail_on_unknown_diagnostics: bool) {
        if fail_on_unknown_diagnostics || self.status != ReducerStatus::FailedUnknownDiagnostic {
            return;
        }
        if stats_have_unsupported_syntax(&self.stats) {
            self.set_publication_state(
                ReducerStatus::FailedUnsupportedSyntax,
                BuildMatrixStatus::NotRun,
                ConvergenceStatus::NotEvaluated,
            );
            return;
        }
        self.set_publication_state(
            ReducerStatus::Success,
            BuildMatrixStatus::NotRun,
            ConvergenceStatus::Converged,
        );
    }
}

fn normalize_edit_records_in_stats(stats: &mut ReducerStats) {
    sort_edit_records(&mut stats.edits);
    for fixup in &mut stats.applied_fixups {
        sort_edit_records(&mut fixup.edits);
    }
}

fn touched_files_from_stats(stats: &ReducerStats) -> Vec<PathBuf> {
    let mut touched = BTreeSet::new();
    for edit in &stats.edits {
        touched.insert(committed_result_path(&edit.file));
    }
    for path in &stats.removal.removed_files {
        touched.insert(committed_result_path(path));
    }
    for path in &stats.removal.removed_dirs {
        touched.insert(committed_result_path(path));
    }
    touched.into_iter().collect()
}

fn skipped_sites_from_stats(stats: &ReducerStats) -> Vec<SkippedSite> {
    let mut sites = Vec::new();

    for site in &stats.unsupported_kconfig_expressions {
        sites.push(SkippedSite {
            kind: String::from("unsupported_kconfig_expression"),
            file: Some(committed_result_path(&site.file)),
            line: Some(site.line),
            reason: sanitize_committed_result_text(&format!("{}: {}", site.directive, site.reason)),
        });
    }
    for site in &stats.unsupported_cpp_expressions {
        sites.push(SkippedSite {
            kind: String::from("unsupported_cpp_expression"),
            file: Some(committed_result_path(&site.file)),
            line: Some(site.line),
            reason: sanitize_committed_result_text(&format!("{}: {}", site.directive, site.reason)),
        });
    }
    for site in &stats.skipped_cpp_nested_edge_cases {
        sites.push(SkippedSite {
            kind: String::from("skipped_cpp_nested_edge_case"),
            file: Some(committed_result_path(&site.file)),
            line: Some(site.line),
            reason: sanitize_committed_result_text(&site.reason),
        });
    }
    for site in &stats.skipped_makefile_lines {
        sites.push(SkippedSite {
            kind: String::from("skipped_makefile_line"),
            file: Some(committed_result_path(&site.file)),
            line: Some(site.line),
            reason: sanitize_committed_result_text(&format!(
                "{}: {}",
                site.assignment_lhs, site.reason
            )),
        });
    }
    for skipped in &stats.skipped_fixups {
        sites.push(SkippedSite {
            kind: String::from("skipped_fixup"),
            file: skipped.diagnostic.file().map(committed_result_path),
            line: skipped.diagnostic.line(),
            reason: sanitize_committed_result_text(&skipped.reason),
        });
    }

    sites.sort();
    sites
}

fn serialize_committed_paths<S>(paths: &Vec<PathBuf>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let sanitized = paths
        .iter()
        .map(|path| committed_result_path(path))
        .collect::<Vec<_>>();
    sanitized.serialize(serializer)
}

fn serialize_committed_optional_path<S>(
    path: &Option<PathBuf>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let sanitized = path.as_ref().map(|path| committed_result_path(path));
    sanitized.serialize(serializer)
}

fn serialize_committed_text<S>(value: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    sanitize_committed_result_text(value).serialize(serializer)
}

pub(crate) fn committed_result_path(path: &Path) -> PathBuf {
    let value = path.to_string_lossy();
    if contains_host_specific_absolute_path(&value) {
        PathBuf::from(REDUCER_RESULT_HOST_PATH_REDACTION)
    } else {
        path.to_path_buf()
    }
}

pub(crate) fn sanitize_committed_result_text(value: &str) -> String {
    if contains_host_specific_absolute_path(value) {
        REDUCER_RESULT_HOST_PATH_REDACTION.to_string()
    } else {
        value.to_string()
    }
}

fn contains_host_specific_absolute_path(value: &str) -> bool {
    crate::security::find_host_specific_absolute_path_marker(value).is_some()
}

fn fixup_applications_from_stats(stats: &ReducerStats) -> Vec<FixupApplication> {
    let mut fixups = stats
        .applied_fixups
        .iter()
        .map(|fixup| FixupApplication {
            fixer_name: fixup.fixer_name.to_string(),
            diagnostic_class: fixup.diagnostic.class().stable_name().to_string(),
            edit_count: fixup.edits.len(),
            proof_source_count: fixup.proof_sources.len(),
        })
        .collect::<Vec<_>>();
    fixups.sort();
    fixups
}

fn pass_reports_from_stats(
    stats: &ReducerStats,
    touched_files: &[PathBuf],
    diagnostic_summary: &DiagnosticSummary,
    skipped_sites: &[SkippedSite],
) -> Vec<ReducerPassReport> {
    if !stats.ran {
        return Vec::new();
    }

    vec![ReducerPassReport {
        name: String::from("reducer.pipeline"),
        changed: stats.files_removed > 0
            || stats.dirs_removed > 0
            || !stats.edits.is_empty()
            || !stats.applied_fixups.is_empty(),
        touched_files: touched_files.to_vec(),
        edit_count: stats.edits.len(),
        diagnostic_count: diagnostic_summary.total(),
        skipped_site_count: skipped_sites.len(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit_reason::{EditProofSource, EditReason};
    use crate::fixups::FixupProof;

    fn manifest_path_edit(path: &str) -> EditRecord {
        EditRecord::new(
            PathBuf::from(path),
            None,
            String::from("before\n"),
            String::new(),
            EditReason::ManifestPath {
                path: PathBuf::from(path),
            },
            EditProofSource::removal_manifest_path(PathBuf::from(path)),
            "test.reducer_result",
        )
    }

    fn missing_header_diagnostic(path: &str, line: usize, header: &str) -> ClassifiedDiagnostic {
        ClassifiedDiagnostic::MissingHeader {
            source_file: PathBuf::from(path),
            line,
            header: header.to_string(),
            build_target: None,
            arch: None,
            config: None,
        }
    }

    #[test]
    fn reducer_public_status_enum_names_are_stable() {
        let reducer_statuses = [
            (ReducerStatus::Success, "success"),
            (
                ReducerStatus::FailedUnknownDiagnostic,
                "failed_unknown_diagnostic",
            ),
            (
                ReducerStatus::FailedUnsupportedSyntax,
                "failed_unsupported_syntax",
            ),
            (
                ReducerStatus::FailedNonConvergence,
                "failed_non_convergence",
            ),
            (ReducerStatus::FailedBuildMatrix, "failed_build_matrix"),
            (
                ReducerStatus::FailedInternalInvariant,
                "failed_internal_invariant",
            ),
        ];
        for (status, stable_name) in reducer_statuses {
            assert_eq!(status.stable_name(), stable_name);
            assert_eq!(serde_json::to_value(status).unwrap(), stable_name);
        }

        let build_matrix_statuses = [
            (BuildMatrixStatus::NotRun, "not_run"),
            (BuildMatrixStatus::Passed, "passed"),
            (BuildMatrixStatus::Failed, "failed"),
        ];
        for (status, stable_name) in build_matrix_statuses {
            assert_eq!(status.stable_name(), stable_name);
            assert_eq!(serde_json::to_value(status).unwrap(), stable_name);
        }

        let convergence_statuses = [
            (ConvergenceStatus::Converged, "converged"),
            (ConvergenceStatus::NotConverged, "not_converged"),
            (ConvergenceStatus::NotEvaluated, "not_evaluated"),
        ];
        for (status, stable_name) in convergence_statuses {
            assert_eq!(status.stable_name(), stable_name);
            assert_eq!(serde_json::to_value(status).unwrap(), stable_name);
        }
    }

    #[test]
    fn reducer_result_normalizes_edit_records() {
        let edit_z = manifest_path_edit("z/removed.c");
        let edit_a = manifest_path_edit("a/removed.c");
        let result = ReducerResult::from_pipeline_artifacts(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            ReducerStats {
                ran: true,
                edits: vec![edit_z.clone(), edit_a.clone(), edit_z.clone()],
                applied_fixups: vec![AppliedFixup {
                    fixer_name: "test.fixup",
                    diagnostic: missing_header_diagnostic("drivers/live.c", 3, "missing.h"),
                    edits: vec![edit_z.clone(), edit_a.clone(), edit_z.clone()],
                    proof_sources: vec![FixupProof::ManifestPath {
                        path: PathBuf::from("z/removed.c"),
                    }],
                }],
                ..ReducerStats::default()
            },
        );

        assert_eq!(result.stats.edits, vec![edit_a.clone(), edit_z.clone()]);
        assert_eq!(result.stats.applied_fixups[0].edits, vec![edit_a, edit_z]);
        assert_eq!(result.edit_summary.total_edits, 2);
        assert_eq!(result.passes[0].edit_count, 2);
    }

    #[test]
    fn sanitize_committed_result_text_preserves_serialized_sed_backreference_patterns() {
        let value = "bad_syms=$$($(NM) $@ | sed -n 's/^.\\{8\\} [bc] \\(.*\\)/\\1/p')\n";
        assert_eq!(sanitize_committed_result_text(value), value);
    }

    #[test]
    fn sanitize_committed_result_text_preserves_serialized_shell_backreference_newline_patterns() {
        let value = "sed 's!x!.global \\2\\n.set \\2,0x\\1!'\n";
        assert_eq!(sanitize_committed_result_text(value), value);
    }

    #[test]
    fn sanitize_committed_result_text_preserves_prose_math_fragments() {
        let value = "cosh(X) = sign(X) * exp(|X|)/2.\n";
        assert_eq!(sanitize_committed_result_text(value), value);
    }
}
