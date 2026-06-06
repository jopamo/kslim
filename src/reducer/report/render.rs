use anyhow::Result;

use crate::config::ReducerConfig;
use crate::edit_reason::{
    validate_edit_records_with_policy, EditRecord, EditValidationPolicy,
};
use crate::removal_manifest::RemovalManifest;

use super::super::{ReducerResult, ReducerStats};
use super::model::{
    ReducerReportArtifactNames, RenderedReducerReportArtifacts, UnknownDiagnosticPolicy,
    UnsupportedSyntaxPolicy,
};
use super::{json, summary, text};

#[allow(dead_code)]
pub(crate) fn render_reducer_stats_report_artifacts(
    stats: &ReducerStats,
    reducer_config: Option<&ReducerConfig>,
    artifact_names: ReducerReportArtifactNames<'_>,
) -> Result<RenderedReducerReportArtifacts> {
    render_reducer_stats_report_artifacts_with_manifest(stats, reducer_config, None, artifact_names)
}

pub(crate) fn render_reducer_stats_report_artifacts_with_manifest(
    stats: &ReducerStats,
    reducer_config: Option<&ReducerConfig>,
    manifest: Option<&RemovalManifest>,
    artifact_names: ReducerReportArtifactNames<'_>,
) -> Result<RenderedReducerReportArtifacts> {
    let mut result = ReducerResult::from_pipeline_artifacts(
        manifest.cloned(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        stats.clone(),
    );
    if let Some(config) = reducer_config {
        result.apply_unknown_diagnostic_policy(config.fail_on_unknown_diagnostics);
        result.apply_unsupported_syntax_policy(config.report_unsupported_expressions);
    }
    render_reducer_result_report_artifacts(&result, reducer_config, artifact_names)
}

#[allow(dead_code)]
pub(crate) fn render_reducer_result_report_artifacts(
    result: &ReducerResult,
    reducer_config: Option<&ReducerConfig>,
    artifact_names: ReducerReportArtifactNames<'_>,
) -> Result<RenderedReducerReportArtifacts> {
    validate_report_edit_records(&result.stats.edits, reducer_config)?;
    let has_diagnostics = summary::has_reducer_diagnostics(&result.stats);

    Ok(RenderedReducerReportArtifacts {
        markdown: text::render_reducer_report_md(&result.stats, reducer_config, result),
        summary_json: json::render_reducer_report_json(result, reducer_config, artifact_names),
        diagnostics_json: json::render_reducer_diagnostics_json(&result.stats),
        edit_summary_json: json::render_reducer_edit_summary_json(&result.stats),
        kconfig_solver_report_json: json::render_kconfig_solver_report_json(&result.stats),
        kconfig_rewrite_report_json: json::render_kconfig_rewrite_report_json(&result.stats),
        skipped_sites_json: has_diagnostics
            .then(|| json::render_reducer_skipped_sites_json(&result.stats)),
    })
}

fn validate_report_edit_records(
    edits: &[EditRecord],
    reducer_config: Option<&ReducerConfig>,
) -> Result<()> {
    let policy = reducer_config.map_or_else(EditValidationPolicy::default, |config| {
        EditValidationPolicy {
            reject_unreasoned_edits: config.reject_unreasoned_edits,
            reject_speculative_fallout_edits: config.reject_speculative_fallout_edits,
        }
    });
    validate_edit_records_with_policy(edits, policy)
}

pub fn ensure_supported_fallout(
    stats: &ReducerStats,
    reducer_config: &crate::config::ReducerConfig,
) -> Result<()> {
    summary::validate_unknown_diagnostic_policy(
        stats,
        UnknownDiagnosticPolicy {
            reject_unknown_diagnostics: reducer_config.fail_on_unknown_diagnostics,
        },
    )?;
    summary::validate_unsupported_syntax_policy(
        stats,
        UnsupportedSyntaxPolicy {
            reject_unsupported_syntax: reducer_config.report_unsupported_expressions,
        },
    )
}
