use std::collections::BTreeMap;

use crate::config::ReducerConfig;
use crate::edit_reason::EditProofSourceKind;

use super::super::result::sanitize_committed_result_text;
use super::super::{ReducerResult, ReducerStats};
use super::model::REDUCER_REPORT_SCHEMA_VERSION;
use super::summary::{
    count_edit_proof_sources, render_classified_diagnostic_md, render_fixup_proof_md,
    render_kbuild_edit_report, sorted_applied_fixups, sorted_skipped_fixups,
};

pub(super) fn render_reducer_report_md(
    stats: &ReducerStats,
    reducer_config: Option<&ReducerConfig>,
    result: &ReducerResult,
) -> String {
    let mut by_pass = BTreeMap::<&str, usize>::new();
    for edit in &stats.edits {
        *by_pass.entry(edit.pass_name).or_default() += 1;
    }
    let kbuild_report = render_kbuild_edit_report(&stats.edits);
    let proof_counts = count_edit_proof_sources(&stats.edits);

    let mut out = String::new();
    out.push_str("# kslim reducer report\n\n");
    out.push_str(&format!(
        "Schema version: {}\n\n",
        REDUCER_REPORT_SCHEMA_VERSION
    ));
    out.push_str(&format!("Reducer ran: {}\n", stats.ran));
    out.push_str(&format!("Final status: {}\n", result.status.stable_name()));
    out.push_str(&format!("Publishable: {}\n", result.publishable));
    out.push_str(&format!(
        "Build matrix status: {}\n",
        result.final_build_status.stable_name()
    ));
    out.push_str(&format!(
        "Convergence status: {}\n\n",
        result.convergence.stable_name()
    ));

    out.push_str("## Reducer config\n\n");
    match reducer_config {
        Some(config) => {
            out.push_str(&format!(
                "- Max fixup passes: {}\n",
                config.max_fixup_passes
            ));
            out.push_str(&format!(
                "- Report unsupported expressions: {}\n",
                config.report_unsupported_expressions
            ));
            out.push_str(&format!(
                "- Fail on unknown diagnostics: {}\n",
                config.fail_on_unknown_diagnostics
            ));
            out.push_str(&format!(
                "- Reject unproven fixups: {}\n",
                config.reject_unproven_fixups
            ));
            out.push_str(&format!(
                "- Reject unreasoned edits: {}\n",
                config.reject_unreasoned_edits
            ));
            out.push_str(&format!(
                "- Reject speculative fallout edits: {}\n",
                config.reject_speculative_fallout_edits
            ));
            out.push_str(&format!(
                "- Fail on missing prune paths: {}\n",
                config.fail_on_missing_prune_paths
            ));
            out.push_str(&format!(
                "- Ignore unsupported special removals: {}\n",
                config.ignore_unsupported_special_removals
            ));
        }
        None => out.push_str("- Not recorded with this reducer artifact writer\n"),
    }

    out.push_str("\n## Normalized removal manifest\n\n");
    if let Some(manifest) = &result.manifest {
        out.push_str(&format!(
            "- Declared paths: {}\n",
            manifest.removed_paths().len()
        ));
        out.push_str(&format!(
            "- Preserved paths: {}\n",
            manifest.preserved_paths().len()
        ));
        out.push_str(&format!(
            "- Removed headers: {}\n",
            manifest.removed_headers.len()
        ));
        out.push_str(&format!(
            "- Removed public headers: {}\n",
            manifest.removed_public_headers.len()
        ));
        out.push_str(&format!(
            "- Removed Kconfig sources: {}\n",
            manifest.removed_kconfig_sources.len()
        ));
        out.push_str(&format!(
            "- Removed kbuild objects: {}\n",
            manifest.removed_kbuild_objects.len()
        ));
        out.push_str(&format!(
            "- Removed device bindings with no live DTS/DTSI/schema references: {}\n",
            manifest.removed_device_bindings_vec().len()
        ));
        out.push_str(&format!(
            "- Removed runtime registrations with no live entry points: {}\n",
            manifest.removed_runtime_registrations_vec().len()
        ));
        out.push_str(&format!(
            "- Removed exported symbols with no live consumers: {}\n",
            manifest.removed_exported_symbols_vec().len()
        ));
        out.push_str(&format!(
            "- Removed config symbols: {}\n",
            manifest.removed_config_symbols().len()
        ));
        out.push_str(&format!(
            "- Preserved config symbols: {}\n",
            manifest.preserved_config_symbols().len()
        ));
        out.push_str(&format!(
            "- Default overrides: {}\n",
            manifest.default_overrides().len()
        ));
    } else {
        out.push_str("- Declared manifest unavailable in this report context\n");
    }
    out.push_str(&format!(
        "- Removed files: {}\n",
        stats.removal.removed_files.len()
    ));
    out.push_str(&format!(
        "- Removed directories: {}\n",
        stats.removal.removed_dirs.len()
    ));
    out.push_str(&format!(
        "- Missing declared paths: {}\n",
        stats.removal.missing_paths.len()
    ));

    out.push_str("\n## Summary\n\n");
    out.push_str(&format!("- Files removed: {}\n", stats.files_removed));
    out.push_str(&format!("- Directories removed: {}\n", stats.dirs_removed));
    out.push_str(&format!(
        "- Empty parents cleaned: {}\n",
        stats.removal.empty_parents_cleaned.len()
    ));
    out.push_str(&format!(
        "- Missing declared paths: {}\n",
        stats.removal.missing_paths.len()
    ));
    out.push_str(&format!("- Configs disabled: {}\n", stats.configs_disabled));
    out.push_str(&format!(
        "- Defaults overridden: {}\n",
        stats.defaults_overridden
    ));
    out.push_str(&format!(
        "- Kconfig refs removed: {}\n",
        stats.kconfig_refs_removed
    ));
    out.push_str(&format!(
        "- Makefile refs removed: {}\n",
        stats.makefile_refs_removed
    ));
    out.push_str(&format!(
        "- Unsupported Kconfig expressions: {}\n",
        stats.unsupported_kconfig_expressions.len()
    ));
    out.push_str(&format!("- Edit records: {}\n", stats.edits.len()));

    out.push_str("\n## Edit truth sources\n\n");
    for kind in [
        EditProofSourceKind::RemovalManifestEntry,
        EditProofSourceKind::TreeIndexEntry,
        EditProofSourceKind::KconfigSolverProof,
        EditProofSourceKind::StaleReference,
        EditProofSourceKind::ClassifiedDiagnostic,
    ] {
        out.push_str(&format!(
            "- {}: {}\n",
            kind.report_label(),
            proof_counts.get(&kind).copied().unwrap_or(0)
        ));
    }

    out.push_str("\n## Kconfig reducer report\n\n");
    out.push_str(&format!(
        "- Dropped selects: {}\n",
        stats.kconfig_report.dropped_selects
    ));
    out.push_str(&format!(
        "- Dropped implies: {}\n",
        stats.kconfig_report.dropped_implies
    ));
    out.push_str(&format!(
        "- Simplified depends: {}\n",
        stats.kconfig_report.simplified_depends
    ));
    out.push_str(&format!(
        "- Simplified visible if: {}\n",
        stats.kconfig_report.simplified_visible_if
    ));
    out.push_str(&format!(
        "- Simplified defaults: {}\n",
        stats.kconfig_report.simplified_defaults
    ));
    out.push_str(&format!(
        "- Removed sources: {}\n",
        stats.kconfig_report.removed_sources
    ));
    out.push_str(&format!(
        "- Removed empty menus: {}\n",
        stats.kconfig_report.removed_empty_menus
    ));
    out.push_str(&format!(
        "- Skipped expressions: {}\n",
        stats.kconfig_report.skipped_expressions
    ));

    out.push_str("\n## Kbuild reducer report\n\n");
    out.push_str(&format!(
        "- Removed directory refs: {}\n",
        kbuild_report.removed_directory_refs
    ));
    out.push_str(&format!(
        "- Removed object refs: {}\n",
        kbuild_report.removed_object_refs
    ));
    out.push_str(&format!(
        "- Cleaned composite objects: {}\n",
        kbuild_report.cleaned_composite_objects
    ));
    out.push_str(&format!(
        "- Removed stale include paths: {}\n",
        kbuild_report.removed_stale_include_paths
    ));
    out.push_str(&format!(
        "- Skipped ambiguous Makefile lines: {}\n",
        stats.skipped_makefile_lines.len()
    ));

    out.push_str("\n## Preprocessor reducer report\n\n");
    out.push_str(&format!(
        "- Branches folded: {}\n",
        stats.cpp_report.branches_folded
    ));
    out.push_str(&format!(
        "- Files touched: {}\n",
        stats.cpp_report.files_touched
    ));
    out.push_str(&format!(
        "- Unsupported preprocessor forms: {}\n",
        stats.unsupported_cpp_expressions.len()
    ));
    out.push_str(&format!(
        "- Skipped nested edge cases: {}\n",
        stats.skipped_cpp_nested_edge_cases.len()
    ));

    out.push_str("\n## Include reducer report\n\n");
    out.push_str(&format!(
        "- Removed include lines: {}\n",
        stats.include_report.removed_include_lines
    ));
    out.push_str(&format!(
        "- Live missing includes: {}\n",
        stats.include_report.live_missing_includes
    ));
    out.push_str(&format!(
        "- Public headers preserved: {}\n",
        stats.include_report.public_headers_preserved
    ));
    out.push_str(&format!(
        "- Ambiguous includes: {}\n",
        stats.include_report.ambiguous_includes
    ));

    out.push_str("\n## Deterministic fixups\n\n");
    out.push_str(&format!(
        "- Applied fixups: {}\n",
        stats.applied_fixups.len()
    ));
    out.push_str(&format!(
        "- Skipped diagnostics: {}\n",
        stats.skipped_fixups.len()
    ));

    if !stats.applied_fixups.is_empty() {
        out.push_str("\n### Applied fixup details\n\n");
        for fixup in sorted_applied_fixups(&stats.applied_fixups) {
            out.push_str(&format!(
                "- {} | {} | edits: {}\n",
                fixup.fixer_name,
                render_classified_diagnostic_md(&fixup.diagnostic),
                fixup.edits.len()
            ));
            for proof in &fixup.proof_sources {
                out.push_str(&format!("  - proof: {}\n", render_fixup_proof_md(proof)));
            }
        }
    }

    if !stats.skipped_fixups.is_empty() {
        out.push_str("\n### Skipped diagnostics\n\n");
        for skipped in sorted_skipped_fixups(&stats.skipped_fixups) {
            out.push_str(&format!(
                "- {} | fixer: {} | reason: {}\n",
                render_classified_diagnostic_md(&skipped.diagnostic),
                skipped.fixer_name.unwrap_or("<none>"),
                sanitize_committed_result_text(&skipped.reason)
            ));
        }
    }

    if !by_pass.is_empty() {
        out.push_str("\n## Edits by pass\n\n");
        for (pass, count) in by_pass {
            out.push_str(&format!("- {}: {}\n", pass, count));
        }
    }

    out
}
