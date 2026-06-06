use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::config::ReducerConfig;
use crate::diagnostics::ClassifiedDiagnostic;
use crate::edit_reason::{
    grouped_edit_record_refs_by_reason, sorted_edit_record_refs, DiagnosticClass,
    EditReason, EditRecord,
};
use crate::fixups::{AppliedFixup, FixupProof, SkippedFixup};
use crate::includes::{
    ClassifiedIncludeTarget, IncludeKind, IncludeResolveRule, IncludeTargetClassification,
    ManualIncludeHandlingKind, ManualIncludeHandlingSite,
};
use crate::kconfig::{
    KconfigSolverDeadSymbolDefinitionProof, KconfigSolverDefaultReenabledSymbol,
    KconfigSolverEmptyMenu, KconfigSolverImpossibleChoice,
    KconfigSolverOrphanedSymbolDefinition, KconfigSolverReverseDependency,
    KconfigSolverSkippedFile,
};

use super::super::super::diagnostics::RawDiagnosticExcerpt;
use super::super::super::result::sanitize_committed_result_text;
use super::super::super::{ReducerResult, ReducerStats};
use super::super::model::ReducerReportArtifactNames;
use super::schema::REDUCER_REPORT_SCHEMA_VERSION;
use super::super::summary::{
    classified_diagnostics_from_stats, committed_path_string, committed_paths_as_strings,
    count_edit_proof_sources, has_reducer_diagnostics, render_kbuild_edit_report,
    sorted_applied_fixups, sorted_refs, sorted_skipped_fixups,
};
use super::canonical::{render_edit_proof_source_count_entries, sort_strings};
use super::escaping::{bool_json, json_compact, json_escape};

pub(crate) fn render_reducer_report_json(
    result: &ReducerResult,
    reducer_config: Option<&ReducerConfig>,
    artifact_names: ReducerReportArtifactNames<'_>,
) -> String {
    let stats = &result.stats;
    let proof_counts = count_edit_proof_sources(&stats.edits);
    let proof_source_counts = render_edit_proof_source_count_entries(&proof_counts, "      ");

    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": {},\n",
            "  \"reducer_config\": {},\n",
            "  \"normalized_removal_manifest\": {},\n",
            "  \"ran\": {},\n",
            "  \"summary\": {{\n",
            "    \"files_removed\": {},\n",
            "    \"dirs_removed\": {},\n",
            "    \"empty_parents_cleaned\": {},\n",
            "    \"missing_declared_paths\": {},\n",
            "    \"configs_disabled\": {},\n",
            "    \"defaults_overridden\": {},\n",
            "    \"kconfig_refs_removed\": {},\n",
            "    \"makefile_refs_removed\": {},\n",
            "    \"edit_records\": {},\n",
            "    \"proof_sources\": {{\n",
            "{}\n",
            "    }}\n",
            "  }},\n",
            "  \"unsupported_fallout\": {{\n",
            "    \"present\": {},\n",
            "    \"unsupported_kconfig_expressions\": {},\n",
            "    \"unsupported_cpp_expressions\": {},\n",
            "    \"skipped_cpp_nested_edge_cases\": {},\n",
            "    \"ambiguous_makefile_lines\": {},\n",
            "    \"skipped_fixup_diagnostics\": {}\n",
            "  }},\n",
            "  \"artifacts\": {{\n",
            "    \"markdown\": \"{}\",\n",
            "    \"summary_json\": \"{}\",\n",
            "    \"diagnostics_json\": \"{}\",\n",
            "    \"edit_summary_json\": \"{}\",\n",
            "    \"kconfig_solver_report_json\": \"{}\",\n",
            "    \"kconfig_rewrite_report_json\": \"{}\",\n",
            "    \"skipped_sites_json\": {}\n",
            "  }},\n",
            "  \"passes\": {},\n",
            "  \"per_pass_edit_counts\": {},\n",
            "  \"per_file_edit_records\": {},\n",
            "  \"edit_summary\": {},\n",
            "  \"diagnostic_summary\": {},\n",
            "  \"fixup_summary\": {},\n",
            "  \"skipped_sites\": {},\n",
            "  \"matrix_status\": {},\n",
            "  \"convergence_status\": {},\n",
            "  \"final_status\": {{\n",
            "    \"status\": {},\n",
            "    \"publishable\": {}\n",
            "  }}\n",
            "}}\n"
        ),
        REDUCER_REPORT_SCHEMA_VERSION,
        render_reducer_config_json(reducer_config),
        render_normalized_removal_manifest_summary_json(result),
        if stats.ran { "true" } else { "false" },
        stats.files_removed,
        stats.dirs_removed,
        stats.removal.empty_parents_cleaned.len(),
        stats.removal.missing_paths.len(),
        stats.configs_disabled,
        stats.defaults_overridden,
        stats.kconfig_refs_removed,
        stats.makefile_refs_removed,
        stats.edits.len(),
        proof_source_counts,
        if has_reducer_diagnostics(stats) {
            "true"
        } else {
            "false"
        },
        stats.unsupported_kconfig_expressions.len(),
        stats.unsupported_cpp_expressions.len(),
        stats.skipped_cpp_nested_edge_cases.len(),
        stats.skipped_makefile_lines.len(),
        stats.skipped_fixups.len(),
        json_escape(artifact_names.markdown),
        json_escape(artifact_names.summary_json),
        json_escape(artifact_names.diagnostics_json),
        json_escape(artifact_names.edit_summary_json),
        json_escape(artifact_names.kconfig_solver_report_json),
        json_escape(artifact_names.kconfig_rewrite_report_json),
        if has_reducer_diagnostics(stats) {
            format!("\"{}\"", json_escape(artifact_names.skipped_sites_json))
        } else {
            String::from("null")
        },
        json_compact(&result.passes),
        render_per_pass_edit_counts_json(&stats.edits),
        render_per_file_edit_records_json(&stats.edits),
        json_compact(&result.edit_summary),
        json_compact(&result.diagnostic_summary),
        render_fixup_summary_json(result),
        json_compact(&result.skipped_sites),
        json_compact(&result.final_build_status),
        json_compact(&result.convergence),
        json_compact(&result.status),
        if result.publishable { "true" } else { "false" },
    )
}

fn render_reducer_config_json(reducer_config: Option<&ReducerConfig>) -> String {
    match reducer_config {
        Some(config) => format!(
            concat!(
                "{{",
                "\"max_fixup_passes\":{},",
                "\"report_unsupported_expressions\":{},",
                "\"fail_on_unknown_diagnostics\":{},",
                "\"reject_unproven_fixups\":{},",
                "\"reject_unreasoned_edits\":{},",
                "\"reject_speculative_fallout_edits\":{},",
                "\"fail_on_missing_prune_paths\":{},",
                "\"ignore_unsupported_special_removals\":{}",
                "}}"
            ),
            config.max_fixup_passes,
            bool_json(config.report_unsupported_expressions),
            bool_json(config.fail_on_unknown_diagnostics),
            bool_json(config.reject_unproven_fixups),
            bool_json(config.reject_unreasoned_edits),
            bool_json(config.reject_speculative_fallout_edits),
            bool_json(config.fail_on_missing_prune_paths),
            bool_json(config.ignore_unsupported_special_removals),
        ),
        None => String::from("null"),
    }
}

fn render_normalized_removal_manifest_summary_json(result: &ReducerResult) -> String {
    let (
        manifest_available,
        declared_paths,
        preserved_paths,
        removed_headers,
        removed_public_headers,
        removed_kconfig_sources,
        removed_kbuild_objects,
        removed_device_bindings,
        removed_runtime_registrations,
        removed_exported_symbols,
        removed_config_symbols,
        preserved_config_symbols,
        default_overrides,
    ) = match &result.manifest {
        Some(manifest) => (
            true,
            manifest
                .removed_paths()
                .iter()
                .map(|path| committed_path_string(path))
                .collect::<Vec<_>>(),
            manifest
                .preserved_paths()
                .iter()
                .map(|path| committed_path_string(path))
                .collect::<Vec<_>>(),
            manifest.removed_headers.iter().cloned().collect::<Vec<_>>(),
            manifest
                .removed_public_headers
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            manifest
                .removed_kconfig_sources_vec()
                .iter()
                .map(|path| committed_path_string(path))
                .collect::<Vec<_>>(),
            manifest.removed_kbuild_objects_vec(),
            manifest.removed_device_bindings_vec(),
            manifest.removed_runtime_registrations_vec(),
            manifest.removed_exported_symbols_vec(),
            manifest.removed_config_symbols_vec(),
            manifest.preserved_config_symbols_vec(),
            manifest.default_overrides().clone(),
        ),
        None => (
            false,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            result.stats.removal.removed_config_symbols.clone(),
            Vec::new(),
            BTreeMap::new(),
        ),
    };
    let mut removed_config_symbols = removed_config_symbols;
    sort_strings(&mut removed_config_symbols);
    let mut preserved_config_symbols = preserved_config_symbols;
    sort_strings(&mut preserved_config_symbols);
    let default_overrides_json = render_string_string_map_json(&default_overrides);
    let removed_files = committed_paths_as_strings(&result.stats.removal.removed_files);
    let removed_dirs = committed_paths_as_strings(&result.stats.removal.removed_dirs);
    let missing_paths = committed_paths_as_strings(&result.stats.removal.missing_paths);

    format!(
        concat!(
            "{{",
            "\"manifest_available\":{},",
            "\"declared_path_count\":{},",
            "\"declared_paths\":{},",
            "\"preserved_path_count\":{},",
            "\"preserved_paths\":{},",
            "\"removed_header_count\":{},",
            "\"removed_headers\":{},",
            "\"removed_public_header_count\":{},",
            "\"removed_public_headers\":{},",
            "\"removed_kconfig_source_count\":{},",
            "\"removed_kconfig_sources\":{},",
            "\"removed_kbuild_object_count\":{},",
            "\"removed_kbuild_objects\":{},",
            "\"removed_device_binding_count\":{},",
            "\"removed_device_bindings\":{},",
            "\"removed_runtime_registration_count\":{},",
            "\"removed_runtime_registrations\":{},",
            "\"removed_exported_symbol_count\":{},",
            "\"removed_exported_symbols\":{},",
            "\"removed_config_symbol_count\":{},",
            "\"removed_config_symbols\":{},",
            "\"preserved_config_symbol_count\":{},",
            "\"preserved_config_symbols\":{},",
            "\"default_override_count\":{},",
            "\"default_overrides\":{},",
            "\"removed_file_count\":{},",
            "\"removed_files\":{},",
            "\"removed_dir_count\":{},",
            "\"removed_dirs\":{},",
            "\"missing_declared_path_count\":{},",
            "\"missing_declared_paths\":{}",
            "}}"
        ),
        bool_json(manifest_available),
        declared_paths.len(),
        json_compact(&declared_paths),
        preserved_paths.len(),
        json_compact(&preserved_paths),
        removed_headers.len(),
        json_compact(&removed_headers),
        removed_public_headers.len(),
        json_compact(&removed_public_headers),
        removed_kconfig_sources.len(),
        json_compact(&removed_kconfig_sources),
        removed_kbuild_objects.len(),
        json_compact(&removed_kbuild_objects),
        removed_device_bindings.len(),
        render_removed_device_bindings_json(&removed_device_bindings),
        removed_runtime_registrations.len(),
        render_removed_runtime_registrations_json(&removed_runtime_registrations),
        removed_exported_symbols.len(),
        render_removed_exported_symbols_json(&removed_exported_symbols),
        removed_config_symbols.len(),
        json_compact(&removed_config_symbols),
        preserved_config_symbols.len(),
        json_compact(&preserved_config_symbols),
        default_overrides.len(),
        default_overrides_json,
        removed_files.len(),
        json_compact(&removed_files),
        removed_dirs.len(),
        json_compact(&removed_dirs),
        missing_paths.len(),
        json_compact(&missing_paths),
    )
}

fn render_removed_exported_symbols_json(
    proofs: &[crate::exported_symbols::ExportedSymbolRemovalProof],
) -> String {
    let mut proofs = proofs.to_vec();
    proofs.sort();
    proofs.dedup();
    let entries = proofs
        .iter()
        .map(|proof| {
            format!(
                concat!(
                    "{{",
                    "\"symbol\":\"{}\",",
                    "\"provider\":\"{}\",",
                    "\"export_macro\":\"{}\",",
                    "\"line\":{}",
                    "}}"
                ),
                json_escape(proof.symbol.as_str()),
                json_escape(&committed_path_string(&proof.provider)),
                json_escape(&proof.export_macro),
                proof.line,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn render_removed_device_bindings_json(
    proofs: &[crate::hardware::DeviceBindingRemovalProof],
) -> String {
    let mut proofs = proofs.to_vec();
    for proof in &mut proofs {
        proof.compatible_strings.sort();
        proof.compatible_strings.dedup();
        sort_strings(&mut proof.schema_references);
    }
    proofs.sort();
    proofs.dedup();
    let entries = proofs
        .iter()
        .map(|proof| {
            format!(
                concat!(
                    "{{",
                    "\"binding\":\"{}\",",
                    "\"compatible_strings\":{},",
                    "\"schema_references\":{}",
                    "}}"
                ),
                json_escape(&committed_path_string(&proof.binding)),
                json_compact(&proof.compatible_strings),
                json_compact(&proof.schema_references),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn render_removed_runtime_registrations_json(
    proofs: &[crate::runtime::RuntimeRegistrationRemovalProof],
) -> String {
    let mut proofs = proofs.to_vec();
    for proof in &mut proofs {
        sort_strings(&mut proof.entry_points);
    }
    proofs.sort();
    proofs.dedup();
    let entries = proofs
        .iter()
        .map(|proof| {
            format!(
                concat!(
                    "{{",
                    "\"provider\":\"{}\",",
                    "\"registration_macro\":\"{}\",",
                    "\"entry_points\":{},",
                    "\"line\":{}",
                    "}}"
                ),
                json_escape(&committed_path_string(&proof.provider)),
                json_escape(&proof.registration_macro),
                json_compact(&proof.entry_points),
                proof.line,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn render_string_array_json(values: &[String]) -> String {
    let entries = values
        .iter()
        .map(|value| {
            format!(
                "\"{}\"",
                json_escape(&sanitize_committed_result_text(value))
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn render_kconfig_solver_default_reenabled_symbols_json(
    symbols: &[KconfigSolverDefaultReenabledSymbol],
) -> String {
    let entries = sorted_refs(symbols)
        .into_iter()
        .map(|symbol| {
            format!(
                "{{\"symbol\":\"{}\",\"value\":\"{}\"}}",
                json_escape(&sanitize_committed_result_text(&symbol.symbol)),
                json_escape(&sanitize_committed_result_text(&symbol.value)),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn render_kconfig_solver_reverse_dependencies_json(
    dependencies: &[KconfigSolverReverseDependency],
) -> String {
    let entries = sorted_refs(dependencies)
        .into_iter()
        .map(|dependency| {
            format!(
                concat!(
                    "{{",
                    "\"source_symbol\":\"{}\",",
                    "\"target_symbol\":\"{}\",",
                    "\"value\":\"{}\"",
                    "}}"
                ),
                json_escape(&sanitize_committed_result_text(&dependency.source_symbol)),
                json_escape(&sanitize_committed_result_text(&dependency.target_symbol)),
                json_escape(&sanitize_committed_result_text(&dependency.value)),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn render_kconfig_solver_impossible_choices_json(
    choices: &[KconfigSolverImpossibleChoice],
) -> String {
    let entries = sorted_refs(choices)
        .into_iter()
        .map(|choice| {
            format!(
                concat!(
                    "{{",
                    "\"choice_symbol\":{},",
                    "\"line\":{},",
                    "\"visibility\":\"{}\",",
                    "\"member_symbols\":{}",
                    "}}"
                ),
                choice
                    .choice_symbol
                    .as_deref()
                    .map(sanitize_committed_result_text)
                    .map(|symbol| format!("\"{}\"", json_escape(&symbol)))
                    .unwrap_or_else(|| String::from("null")),
                choice.line,
                json_escape(&sanitize_committed_result_text(&choice.visibility)),
                render_string_array_json(&choice.member_symbols),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn render_kconfig_solver_empty_menus_json(menus: &[KconfigSolverEmptyMenu]) -> String {
    let entries = sorted_refs(menus)
        .into_iter()
        .map(|menu| {
            format!(
                "{{\"prompt\":\"{}\",\"line\":{},\"visibility\":\"{}\"}}",
                json_escape(&sanitize_committed_result_text(&menu.prompt)),
                menu.line,
                json_escape(&sanitize_committed_result_text(&menu.visibility)),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn render_kconfig_solver_orphaned_symbol_definitions_json(
    definitions: &[KconfigSolverOrphanedSymbolDefinition],
) -> String {
    let entries = sorted_refs(definitions)
        .into_iter()
        .map(|definition| {
            format!(
                concat!(
                    "{{",
                    "\"symbol\":\"{}\",",
                    "\"definition_kind\":\"{}\",",
                    "\"line\":{},",
                    "\"visibility\":\"{}\"",
                    "}}"
                ),
                json_escape(&sanitize_committed_result_text(&definition.symbol)),
                json_escape(&sanitize_committed_result_text(
                    &definition.definition_kind,
                )),
                definition.line,
                json_escape(&sanitize_committed_result_text(&definition.visibility)),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn render_kconfig_solver_dead_symbol_definition_proofs_json(
    proofs: &[KconfigSolverDeadSymbolDefinitionProof],
) -> String {
    let entries = sorted_refs(proofs)
        .into_iter()
        .map(|proof| {
            format!(
                concat!(
                    "{{",
                    "\"file\":\"{}\",",
                    "\"symbol\":\"{}\",",
                    "\"definition_kind\":\"{}\",",
                    "\"start_line\":{},",
                    "\"end_line\":{}",
                    "}}"
                ),
                json_escape(&committed_path_string(&proof.file)),
                json_escape(&sanitize_committed_result_text(&proof.symbol)),
                json_escape(&sanitize_committed_result_text(&proof.definition_kind)),
                proof.start_line,
                proof.end_line,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn render_kconfig_solver_skipped_files_json(skipped_files: &[KconfigSolverSkippedFile]) -> String {
    let entries = sorted_refs(skipped_files)
        .into_iter()
        .map(|skipped| {
            format!(
                concat!(
                    "{{",
                    "\"file\":\"{}\",",
                    "\"analysis\":\"{}\",",
                    "\"reason\":\"{}\"",
                    "}}"
                ),
                json_escape(&committed_path_string(&skipped.file)),
                json_escape(&sanitize_committed_result_text(&skipped.analysis)),
                json_escape(&sanitize_committed_result_text(&skipped.reason)),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn render_string_string_map_json(map: &BTreeMap<String, String>) -> String {
    let entries = map
        .iter()
        .map(|(key, value)| {
            format!(
                "\"{}\":\"{}\"",
                json_escape(&sanitize_committed_result_text(key)),
                json_escape(&sanitize_committed_result_text(value))
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{entries}}}")
}

fn render_string_usize_map_json<K>(map: &BTreeMap<K, usize>) -> String
where
    K: AsRef<str> + Ord,
{
    let entries = map
        .iter()
        .map(|(key, count)| {
            format!(
                "\"{}\":{}",
                json_escape(&sanitize_committed_result_text(key.as_ref())),
                count
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{entries}}}")
}

fn render_string_usize_map_entries<K>(map: &BTreeMap<K, usize>, indent: &str) -> String
where
    K: AsRef<str> + Ord,
{
    map.iter()
        .map(|(key, count)| {
            format!(
                "{}\"{}\": {}",
                indent,
                json_escape(&sanitize_committed_result_text(key.as_ref())),
                count
            )
        })
        .collect::<Vec<_>>()
        .join(",\n")
}

fn render_per_pass_edit_counts_json(edits: &[EditRecord]) -> String {
    let mut by_pass = BTreeMap::<String, usize>::new();
    for edit in edits {
        *by_pass.entry(edit.pass_name.to_string()).or_default() += 1;
    }
    render_string_usize_map_json(&by_pass)
}

fn render_per_file_edit_records_json(edits: &[EditRecord]) -> String {
    let mut by_file = BTreeMap::<String, Vec<String>>::new();
    for edit in sorted_edit_record_refs(edits) {
        by_file
            .entry(committed_path_string(&edit.file))
            .or_default()
            .push(render_edit_record_json(edit));
    }

    let files = by_file
        .into_iter()
        .map(|(file, records)| {
            format!(
                concat!(
                    "    {{\n",
                    "      \"file\": \"{}\",\n",
                    "      \"edit_count\": {},\n",
                    "      \"edit_record_details\": [\n",
                    "{}\n",
                    "      ]\n",
                    "    }}"
                ),
                json_escape(&file),
                records.len(),
                records.join(",\n"),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");

    format!("[\n{}\n  ]", files)
}

fn render_fixup_summary_json(result: &ReducerResult) -> String {
    format!(
        concat!(
            "{{",
            "\"applied_fixups\":{},",
            "\"skipped_fixups\":{},",
            "\"applications\":{},",
            "\"skipped_diagnostics\":[\n",
            "{}\n",
            "  ]",
            "}}"
        ),
        result.stats.applied_fixups.len(),
        result.stats.skipped_fixups.len(),
        json_compact(&result.fixups_applied),
        render_skipped_fixups_json(&result.stats.skipped_fixups),
    )
}

fn is_kconfig_rewrite_edit_record(edit: &EditRecord) -> bool {
    matches!(
        edit.pass_name,
        "prune.prune_configs"
            | "prune.rewrite_kconfig_defaults"
            | "kconfig.rewrite_relations"
            | "prune.rewrite_kconfig_sources"
            | "kconfig.rewrite_dead_symbol_definitions"
            | "kconfig.rewrite_empty_menus"
    )
}

fn kconfig_rewrite_edit_records(stats: &ReducerStats) -> Vec<EditRecord> {
    stats
        .edits
        .iter()
        .filter(|edit| is_kconfig_rewrite_edit_record(edit))
        .cloned()
        .collect()
}

pub(crate) fn render_kconfig_rewrite_report_json(stats: &ReducerStats) -> String {
    let kconfig_edits = kconfig_rewrite_edit_records(stats);
    let mut by_pass = BTreeMap::<&str, usize>::new();
    for edit in &kconfig_edits {
        *by_pass.entry(edit.pass_name).or_default() += 1;
    }
    let relation_rewrite_count = stats.kconfig_report.dropped_selects
        + stats.kconfig_report.dropped_implies
        + stats.kconfig_report.simplified_depends
        + stats.kconfig_report.simplified_visible_if
        + stats.kconfig_report.simplified_defaults;
    let dead_symbol_definition_removal_count = kconfig_edits
        .iter()
        .filter(|edit| edit.pass_name == "kconfig.rewrite_dead_symbol_definitions")
        .count();
    let empty_menu_removal_count = stats.kconfig_report.removed_empty_menus;

    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": {},\n",
            "  \"ran\": {},\n",
            "  \"removed_symbol_count\": {},\n",
            "  \"config_block_removal_count\": {},\n",
            "  \"default_override_count\": {},\n",
            "  \"relation_rewrite_count\": {},\n",
            "  \"source_removal_count\": {},\n",
            "  \"dead_symbol_definition_removal_count\": {},\n",
            "  \"empty_menu_removal_count\": {},\n",
            "  \"skipped_expression_count\": {},\n",
            "  \"kconfig_edit_count\": {},\n",
            "  \"removed_symbols\": {},\n",
            "  \"counts\": {{\n",
            "    \"dropped_selects\": {},\n",
            "    \"dropped_implies\": {},\n",
            "    \"simplified_depends\": {},\n",
            "    \"simplified_visible_if\": {},\n",
            "    \"simplified_defaults\": {},\n",
            "    \"removed_sources\": {},\n",
            "    \"removed_empty_menus\": {},\n",
            "    \"skipped_expressions\": {}\n",
            "  }},\n",
            "  \"unsupported_kconfig_expressions\": [\n",
            "{}\n",
            "  ],\n",
            "  \"edits_by_pass\": {{\n",
            "{}\n",
            "  }},\n",
            "  \"edits_by_reason\": {{\n",
            "{}\n",
            "  }},\n",
            "  \"edit_record_details\": [\n",
            "{}\n",
            "  ]\n",
            "}}\n"
        ),
        REDUCER_REPORT_SCHEMA_VERSION,
        if stats.ran { "true" } else { "false" },
        stats.removal.removed_config_symbols.len(),
        stats.configs_disabled,
        stats.defaults_overridden,
        relation_rewrite_count,
        stats.kconfig_report.removed_sources,
        dead_symbol_definition_removal_count,
        empty_menu_removal_count,
        stats.kconfig_report.skipped_expressions,
        kconfig_edits.len(),
        render_string_array_json(&stats.removal.removed_config_symbols),
        stats.kconfig_report.dropped_selects,
        stats.kconfig_report.dropped_implies,
        stats.kconfig_report.simplified_depends,
        stats.kconfig_report.simplified_visible_if,
        stats.kconfig_report.simplified_defaults,
        stats.kconfig_report.removed_sources,
        stats.kconfig_report.removed_empty_menus,
        stats.kconfig_report.skipped_expressions,
        render_unsupported_kconfig_sites_json(stats),
        render_string_usize_map_entries(&by_pass, "    "),
        render_edit_records_by_reason_json(&kconfig_edits),
        render_edit_records_json(&kconfig_edits),
    )
}

pub(crate) fn render_kconfig_solver_report_json(stats: &ReducerStats) -> String {
    let report = &stats.kconfig_solver_report;
    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": {},\n",
            "  \"files_analyzed\": {},\n",
            "  \"removed_symbol_count\": {},\n",
            "  \"default_reenabled_symbol_count\": {},\n",
            "  \"forced_select_count\": {},\n",
            "  \"weak_imply_count\": {},\n",
            "  \"impossible_choice_count\": {},\n",
            "  \"empty_menu_count\": {},\n",
            "  \"orphaned_symbol_definition_count\": {},\n",
            "  \"dead_symbol_definition_proof_count\": {},\n",
            "  \"skipped_file_count\": {},\n",
            "  \"removed_symbols\": {},\n",
            "  \"default_reenabled_symbols\": {},\n",
            "  \"forced_selects\": {},\n",
            "  \"weak_implies\": {},\n",
            "  \"impossible_choices\": {},\n",
            "  \"empty_menus\": {},\n",
            "  \"orphaned_symbol_definitions\": {},\n",
            "  \"dead_symbol_definition_proofs\": {},\n",
            "  \"skipped_files\": {}\n",
            "}}\n"
        ),
        REDUCER_REPORT_SCHEMA_VERSION,
        report.files_analyzed,
        report.removed_symbols.len(),
        report.default_reenabled_symbols.len(),
        report.forced_selects.len(),
        report.weak_implies.len(),
        report.impossible_choices.len(),
        report.empty_menus.len(),
        report.orphaned_symbol_definitions.len(),
        report.dead_symbol_definition_proofs.len(),
        report.skipped_files.len(),
        render_string_array_json(&report.removed_symbols),
        render_kconfig_solver_default_reenabled_symbols_json(&report.default_reenabled_symbols),
        render_kconfig_solver_reverse_dependencies_json(&report.forced_selects),
        render_kconfig_solver_reverse_dependencies_json(&report.weak_implies),
        render_kconfig_solver_impossible_choices_json(&report.impossible_choices),
        render_kconfig_solver_empty_menus_json(&report.empty_menus),
        render_kconfig_solver_orphaned_symbol_definitions_json(
            &report.orphaned_symbol_definitions,
        ),
        render_kconfig_solver_dead_symbol_definition_proofs_json(
            &report.dead_symbol_definition_proofs,
        ),
        render_kconfig_solver_skipped_files_json(&report.skipped_files),
    )
}

pub(crate) fn render_reducer_edit_summary_json(stats: &ReducerStats) -> String {
    let mut by_pass = BTreeMap::<&str, usize>::new();
    for edit in &stats.edits {
        *by_pass.entry(edit.pass_name).or_default() += 1;
    }
    let kbuild_report = render_kbuild_edit_report(&stats.edits);
    let proof_counts = count_edit_proof_sources(&stats.edits);

    let passes = render_string_usize_map_entries(&by_pass, "    ");
    let proof_sources = render_edit_proof_source_count_entries(&proof_counts, "    ");
    let applied_fixups = render_applied_fixups_json(&stats.applied_fixups);
    let skipped_fixups = render_skipped_fixup_sites_json(&stats.skipped_fixups);
    let edit_record_details = render_edit_records_json(&stats.edits);
    let edits_by_reason = render_edit_records_by_reason_json(&stats.edits);
    let byte_explanation = render_byte_explanation_summary_json(&stats.edits);

    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": {},\n",
            "  \"ran\": {},\n",
            "  \"files_removed\": {},\n",
            "  \"dirs_removed\": {},\n",
            "  \"empty_parents_cleaned\": {},\n",
            "  \"missing_paths\": {},\n",
            "  \"configs_disabled\": {},\n",
            "  \"defaults_overridden\": {},\n",
            "  \"kconfig_refs_removed\": {},\n",
            "  \"makefile_refs_removed\": {},\n",
            "  \"unsupported_kconfig_expressions\": {},\n",
            "  \"kconfig_report\": {{\n",
            "    \"dropped_selects\": {},\n",
            "    \"dropped_implies\": {},\n",
            "    \"simplified_depends\": {},\n",
            "    \"simplified_visible_if\": {},\n",
            "    \"simplified_defaults\": {},\n",
            "    \"removed_sources\": {},\n",
            "    \"removed_empty_menus\": {},\n",
            "    \"skipped_expressions\": {}\n",
            "  }},\n",
            "  \"kbuild_report\": {{\n",
            "    \"removed_directory_refs\": {},\n",
            "    \"removed_object_refs\": {},\n",
            "    \"cleaned_composite_objects\": {},\n",
            "    \"removed_stale_include_paths\": {},\n",
            "    \"skipped_ambiguous_makefile_lines\": {}\n",
            "  }},\n",
            "  \"cpp_report\": {{\n",
            "    \"branches_folded\": {},\n",
            "    \"files_touched\": {},\n",
            "    \"unsupported_preprocessor_forms\": {},\n",
            "    \"skipped_nested_edge_cases\": {}\n",
            "  }},\n",
            "  \"include_report\": {{\n",
            "    \"removed_include_lines\": {},\n",
            "    \"live_missing_includes\": {},\n",
            "    \"public_headers_preserved\": {},\n",
            "    \"ambiguous_includes\": {}\n",
            "  }},\n",
            "  \"fixups_applied\": [\n",
            "{}\n",
            "  ],\n",
            "  \"fixups_skipped\": [\n",
            "{}\n",
            "  ],\n",
            "  \"byte_explanation\": {},\n",
            "  \"edit_records\": {},\n",
            "  \"edit_record_details\": [\n",
            "{}\n",
            "  ],\n",
            "  \"proof_sources\": {{\n",
            "{}\n",
            "  }},\n",
            "  \"edits_by_pass\": {{\n",
            "{}\n",
            "  }},\n",
            "  \"edits_by_reason\": {{\n",
            "{}\n",
            "  }}\n",
            "}}\n"
        ),
        REDUCER_REPORT_SCHEMA_VERSION,
        if stats.ran { "true" } else { "false" },
        stats.files_removed,
        stats.dirs_removed,
        stats.removal.empty_parents_cleaned.len(),
        stats.removal.missing_paths.len(),
        stats.configs_disabled,
        stats.defaults_overridden,
        stats.kconfig_refs_removed,
        stats.makefile_refs_removed,
        stats.unsupported_kconfig_expressions.len(),
        stats.kconfig_report.dropped_selects,
        stats.kconfig_report.dropped_implies,
        stats.kconfig_report.simplified_depends,
        stats.kconfig_report.simplified_visible_if,
        stats.kconfig_report.simplified_defaults,
        stats.kconfig_report.removed_sources,
        stats.kconfig_report.removed_empty_menus,
        stats.kconfig_report.skipped_expressions,
        kbuild_report.removed_directory_refs,
        kbuild_report.removed_object_refs,
        kbuild_report.cleaned_composite_objects,
        kbuild_report.removed_stale_include_paths,
        stats.skipped_makefile_lines.len(),
        stats.cpp_report.branches_folded,
        stats.cpp_report.files_touched,
        stats.unsupported_cpp_expressions.len(),
        stats.skipped_cpp_nested_edge_cases.len(),
        stats.include_report.removed_include_lines,
        stats.include_report.live_missing_includes,
        stats.include_report.public_headers_preserved,
        stats.include_report.ambiguous_includes,
        applied_fixups,
        skipped_fixups,
        byte_explanation,
        stats.edits.len(),
        edit_record_details,
        proof_sources,
        passes,
        edits_by_reason,
    )
}

fn render_byte_explanation_summary_json(edits: &[EditRecord]) -> String {
    let old_bytes = edits
        .iter()
        .map(|edit| {
            sanitize_committed_result_text(&edit.before)
                .as_bytes()
                .len()
        })
        .sum::<usize>();
    let new_bytes = edits
        .iter()
        .map(|edit| sanitize_committed_result_text(&edit.after).as_bytes().len())
        .sum::<usize>();

    format!(
        concat!(
            "{{",
            "\"edit_records\":{},",
            "\"old_bytes\":{},",
            "\"new_bytes\":{},",
            "\"all_edit_records_have_byte_evidence\":true",
            "}}"
        ),
        edits.len(),
        old_bytes,
        new_bytes,
    )
}

fn render_edit_records_by_reason_json(edits: &[EditRecord]) -> String {
    grouped_edit_record_refs_by_reason(edits)
        .into_iter()
        .map(|(reason, records)| {
            format!(
                concat!(
                    "    \"{}\": {{\n",
                    "      \"edit_count\": {},\n",
                    "      \"edit_record_details\": [\n",
                    "{}\n",
                    "      ]\n",
                    "    }}"
                ),
                json_escape(reason),
                records.len(),
                records
                    .into_iter()
                    .map(render_edit_record_json)
                    .collect::<Vec<_>>()
                    .join(",\n"),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n")
}

fn render_diagnostic_log_summaries_by_command_json(excerpts: &[RawDiagnosticExcerpt]) -> String {
    let mut by_command = BTreeMap::<(String, Option<String>), usize>::new();
    for excerpt in excerpts {
        let key = (
            sanitize_committed_result_text(&excerpt.command_context),
            excerpt
                .build_target
                .as_deref()
                .map(sanitize_committed_result_text),
        );
        *by_command.entry(key).or_default() += 1;
    }

    by_command
        .into_iter()
        .map(|((command_context, build_target), log_excerpt_count)| {
            format!(
                concat!(
                    "    {{\n",
                    "      \"command_context\": \"{}\",\n",
                    "      \"build_target\": {},\n",
                    "      \"log_excerpt_count\": {}\n",
                    "    }}"
                ),
                json_escape(&command_context),
                build_target
                    .as_deref()
                    .map(|target| format!("\"{}\"", json_escape(target)))
                    .unwrap_or_else(|| String::from("null")),
                log_excerpt_count,
            )
        })
        .collect::<Vec<_>>()
        .join(",\n")
}

fn render_classified_diagnostics_json(stats: &ReducerStats) -> String {
    classified_diagnostics_from_stats(stats)
        .into_iter()
        .map(|diagnostic| format!("    {}", render_classified_diagnostic_json(diagnostic)))
        .collect::<Vec<_>>()
        .join(",\n")
}

fn render_unknown_diagnostics_json(stats: &ReducerStats) -> String {
    classified_diagnostics_from_stats(stats)
        .into_iter()
        .filter(|diagnostic| diagnostic.class() == DiagnosticClass::Unknown)
        .map(|diagnostic| format!("    {}", render_classified_diagnostic_json(diagnostic)))
        .collect::<Vec<_>>()
        .join(",\n")
}

fn render_consumed_diagnostics_json(fixups: &[AppliedFixup]) -> String {
    sorted_applied_fixups(fixups)
        .into_iter()
        .map(|fixup| {
            format!(
                concat!(
                    "    {{\n",
                    "      \"fixer_name\": \"{}\",\n",
                    "      \"diagnostic\": {},\n",
                    "      \"edit_count\": {},\n",
                    "      \"proof_source_count\": {}\n",
                    "    }}"
                ),
                json_escape(fixup.fixer_name),
                render_classified_diagnostic_json(&fixup.diagnostic),
                fixup.edits.len(),
                fixup.proof_sources.len(),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n")
}

pub(crate) fn render_reducer_diagnostics_json(stats: &ReducerStats) -> String {
    let diagnostic_log_summaries_by_command =
        render_diagnostic_log_summaries_by_command_json(&stats.raw_diagnostic_excerpts);
    let classified_diagnostics = render_classified_diagnostics_json(stats);
    let unknown_diagnostics = render_unknown_diagnostics_json(stats);
    let consumed_diagnostics = render_consumed_diagnostics_json(&stats.applied_fixups);
    let skipped_diagnostics = render_skipped_fixups_json(&stats.skipped_fixups);
    let unsupported = sorted_refs(&stats.unsupported_kconfig_expressions)
        .into_iter()
        .map(|site| {
            format!(
                concat!(
                    "    {{\n",
                    "      \"kind\": \"unsupported_kconfig_expression\",\n",
                    "      \"file\": \"{}\",\n",
                    "      \"line\": {},\n",
                    "      \"directive\": \"{}\",\n",
                    "      \"expression\": \"{}\",\n",
                    "      \"reason\": \"{}\"\n",
                    "    }}"
                ),
                json_escape(&committed_path_string(&site.file)),
                site.line,
                json_escape(&sanitize_committed_result_text(&site.directive)),
                json_escape(&sanitize_committed_result_text(&site.expression)),
                json_escape(&sanitize_committed_result_text(&site.reason)),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");

    let unsupported_cpp = sorted_refs(&stats.unsupported_cpp_expressions)
        .into_iter()
        .map(|site| {
            format!(
                concat!(
                    "    {{\n",
                    "      \"kind\": \"unsupported_cpp_expression\",\n",
                    "      \"file\": \"{}\",\n",
                    "      \"line\": {},\n",
                    "      \"directive\": \"{}\",\n",
                    "      \"expression\": \"{}\",\n",
                    "      \"reason\": \"{}\"\n",
                    "    }}"
                ),
                json_escape(&committed_path_string(&site.file)),
                site.line,
                json_escape(&sanitize_committed_result_text(&site.directive)),
                json_escape(&sanitize_committed_result_text(&site.expression)),
                json_escape(&sanitize_committed_result_text(&site.reason)),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");

    let ambiguous = sorted_refs(&stats.skipped_makefile_lines)
        .into_iter()
        .map(|site| {
            format!(
                concat!(
                    "    {{\n",
                    "      \"kind\": \"ambiguous_makefile_line\",\n",
                    "      \"file\": \"{}\",\n",
                    "      \"line\": {},\n",
                    "      \"assignment_lhs\": \"{}\",\n",
                    "      \"reason\": \"{}\"\n",
                    "    }}"
                ),
                json_escape(&committed_path_string(&site.file)),
                site.line,
                json_escape(&sanitize_committed_result_text(&site.assignment_lhs)),
                json_escape(&sanitize_committed_result_text(&site.reason)),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");

    let skipped_nested = sorted_refs(&stats.skipped_cpp_nested_edge_cases)
        .into_iter()
        .map(|site| {
            format!(
                concat!(
                    "    {{\n",
                    "      \"kind\": \"skipped_cpp_nested_edge_case\",\n",
                    "      \"file\": \"{}\",\n",
                    "      \"line\": {},\n",
                    "      \"reason\": \"{}\"\n",
                    "    }}"
                ),
                json_escape(&committed_path_string(&site.file)),
                site.line,
                json_escape(&sanitize_committed_result_text(&site.reason)),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");

    let skipped_fixups = sorted_skipped_fixups(&stats.skipped_fixups)
        .into_iter()
        .map(|skipped| {
            format!(
                concat!(
                    "    {{\n",
                    "      \"kind\": \"skipped_fixup_diagnostic\",\n",
                    "      \"fixer_name\": {},\n",
                    "      \"reason\": \"{}\",\n",
                    "      \"diagnostic\": {}\n",
                    "    }}"
                ),
                skipped
                    .fixer_name
                    .map(|name| format!("\"{}\"", json_escape(name)))
                    .unwrap_or_else(|| String::from("null")),
                json_escape(&sanitize_committed_result_text(&skipped.reason)),
                render_classified_diagnostic_json(&skipped.diagnostic),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");

    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": {},\n",
            "  \"diagnostic_log_summaries_by_command\": [\n",
            "{}\n",
            "  ],\n",
            "  \"classified_diagnostics\": [\n",
            "{}\n",
            "  ],\n",
            "  \"unknown_diagnostics\": [\n",
            "{}\n",
            "  ],\n",
            "  \"consumed_diagnostics\": [\n",
            "{}\n",
            "  ],\n",
            "  \"skipped_diagnostics\": [\n",
            "{}\n",
            "  ],\n",
            "  \"unsupported_kconfig_expressions\": [\n",
            "{}\n",
            "  ],\n",
            "  \"unsupported_cpp_expressions\": [\n",
            "{}\n",
            "  ],\n",
            "  \"skipped_cpp_nested_edge_cases\": [\n",
            "{}\n",
            "  ],\n",
            "  \"ambiguous_makefile_lines\": [\n",
            "{}\n",
            "  ],\n",
            "  \"skipped_fixup_diagnostics\": [\n",
            "{}\n",
            "  ]\n",
            "}}\n"
        ),
        REDUCER_REPORT_SCHEMA_VERSION,
        diagnostic_log_summaries_by_command,
        classified_diagnostics,
        unknown_diagnostics,
        consumed_diagnostics,
        skipped_diagnostics,
        unsupported,
        unsupported_cpp,
        skipped_nested,
        ambiguous,
        skipped_fixups,
    )
}

pub(crate) fn render_reducer_skipped_sites_json(stats: &ReducerStats) -> String {
    let ambiguous_include_sites =
        render_manual_include_sites_json(stats, ManualIncludeHandlingKind::AmbiguousInclude);
    let live_missing_includes =
        render_manual_include_sites_json(stats, ManualIncludeHandlingKind::LiveMissingInclude);
    let unsupported_kconfig = render_unsupported_kconfig_sites_json(stats);
    let unsupported_cpp_forms = render_unsupported_cpp_forms_json(stats);
    let unsupported_cpp = render_unsupported_cpp_expression_sites_json(stats);
    let skipped_nested = render_skipped_cpp_nested_sites_json(stats);
    let ambiguous_makefile = render_ambiguous_makefile_sites_json(stats);
    let skipped_fixups = render_skipped_fixup_sites_json(&stats.skipped_fixups);

    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": {},\n",
            "  \"ambiguous_include_sites\": [\n",
            "{}\n",
            "  ],\n",
            "  \"unsupported_kconfig_expressions\": [\n",
            "{}\n",
            "  ],\n",
            "  \"unsupported_cpp_forms\": [\n",
            "{}\n",
            "  ],\n",
            "  \"unsupported_cpp_expressions\": [\n",
            "{}\n",
            "  ],\n",
            "  \"skipped_cpp_nested_edge_cases\": [\n",
            "{}\n",
            "  ],\n",
            "  \"ambiguous_makefile_lines\": [\n",
            "{}\n",
            "  ],\n",
            "  \"live_missing_includes\": [\n",
            "{}\n",
            "  ],\n",
            "  \"skipped_fixup_diagnostics\": [\n",
            "{}\n",
            "  ]\n",
            "}}\n"
        ),
        REDUCER_REPORT_SCHEMA_VERSION,
        ambiguous_include_sites,
        unsupported_kconfig,
        unsupported_cpp_forms,
        unsupported_cpp,
        skipped_nested,
        ambiguous_makefile,
        live_missing_includes,
        skipped_fixups,
    )
}

fn render_manual_include_sites_json(
    stats: &ReducerStats,
    kind: ManualIncludeHandlingKind,
) -> String {
    let mut sites = stats
        .manual_include_sites
        .iter()
        .filter(|site| site.kind == kind)
        .collect::<Vec<_>>();
    sites.sort_by(|left, right| {
        manual_include_site_sort_key(left).cmp(&manual_include_site_sort_key(right))
    });

    sites
        .into_iter()
        .map(render_manual_include_site_json)
        .collect::<Vec<_>>()
        .join(",\n")
}

fn manual_include_site_sort_key(
    site: &ManualIncludeHandlingSite,
) -> (String, usize, String, &'static str) {
    (
        committed_path_string(&site.site.file),
        site.site.line,
        site.site.header.clone(),
        manual_include_kind_json(site.kind),
    )
}

fn render_manual_include_site_json(site: &ManualIncludeHandlingSite) -> String {
    let targets = site
        .classified_targets
        .iter()
        .map(render_classified_include_target_json)
        .map(|target| format!("        {}", target))
        .collect::<Vec<_>>()
        .join(",\n");

    format!(
        concat!(
            "    {{\n",
            "      \"kind\": \"{}\",\n",
            "      \"file\": \"{}\",\n",
            "      \"line\": {},\n",
            "      \"header\": \"{}\",\n",
            "      \"include_kind\": \"{}\",\n",
            "      \"classified_targets\": [\n",
            "{}\n",
            "      ]\n",
            "    }}"
        ),
        manual_include_kind_json(site.kind),
        json_escape(&committed_path_string(&site.site.file)),
        site.site.line,
        json_escape(&sanitize_committed_result_text(&site.site.header)),
        include_kind_json(site.site.kind),
        targets,
    )
}

fn render_classified_include_target_json(target: &ClassifiedIncludeTarget) -> String {
    format!(
        concat!(
            "{{",
            "\"path\":\"{}\",",
            "\"rule\":\"{}\",",
            "\"classification\":\"{}\"",
            "}}"
        ),
        json_escape(&committed_path_string(&target.target.path)),
        include_resolve_rule_json(target.target.rule),
        include_target_classification_json(target.classification),
    )
}

fn render_unsupported_kconfig_sites_json(stats: &ReducerStats) -> String {
    sorted_refs(&stats.unsupported_kconfig_expressions)
        .into_iter()
        .map(|site| {
            format!(
                concat!(
                    "    {{\n",
                    "      \"kind\": \"unsupported_kconfig_expression\",\n",
                    "      \"file\": \"{}\",\n",
                    "      \"line\": {},\n",
                    "      \"directive\": \"{}\",\n",
                    "      \"expression\": \"{}\",\n",
                    "      \"reason\": \"{}\"\n",
                    "    }}"
                ),
                json_escape(&committed_path_string(&site.file)),
                site.line,
                json_escape(&sanitize_committed_result_text(&site.directive)),
                json_escape(&sanitize_committed_result_text(&site.expression)),
                json_escape(&sanitize_committed_result_text(&site.reason)),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n")
}

fn render_unsupported_cpp_expression_sites_json(stats: &ReducerStats) -> String {
    sorted_refs(&stats.unsupported_cpp_expressions)
        .into_iter()
        .map(|site| {
            format!(
                concat!(
                    "    {{\n",
                    "      \"kind\": \"unsupported_cpp_expression\",\n",
                    "      \"file\": \"{}\",\n",
                    "      \"line\": {},\n",
                    "      \"directive\": \"{}\",\n",
                    "      \"expression\": \"{}\",\n",
                    "      \"reason\": \"{}\"\n",
                    "    }}"
                ),
                json_escape(&committed_path_string(&site.file)),
                site.line,
                json_escape(&sanitize_committed_result_text(&site.directive)),
                json_escape(&sanitize_committed_result_text(&site.expression)),
                json_escape(&sanitize_committed_result_text(&site.reason)),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n")
}

fn render_skipped_cpp_nested_sites_json(stats: &ReducerStats) -> String {
    sorted_refs(&stats.skipped_cpp_nested_edge_cases)
        .into_iter()
        .map(|site| {
            format!(
                concat!(
                    "    {{\n",
                    "      \"kind\": \"skipped_cpp_nested_edge_case\",\n",
                    "      \"file\": \"{}\",\n",
                    "      \"line\": {},\n",
                    "      \"reason\": \"{}\"\n",
                    "    }}"
                ),
                json_escape(&committed_path_string(&site.file)),
                site.line,
                json_escape(&sanitize_committed_result_text(&site.reason)),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n")
}

fn render_unsupported_cpp_forms_json(stats: &ReducerStats) -> String {
    let mut forms = Vec::new();
    forms.extend(
        sorted_refs(&stats.unsupported_cpp_expressions)
            .into_iter()
            .map(|site| {
                format!(
                    concat!(
                        "    {{\n",
                        "      \"kind\": \"unsupported_cpp_expression\",\n",
                        "      \"file\": \"{}\",\n",
                        "      \"line\": {},\n",
                        "      \"directive\": \"{}\",\n",
                        "      \"expression\": \"{}\",\n",
                        "      \"reason\": \"{}\"\n",
                        "    }}"
                    ),
                    json_escape(&committed_path_string(&site.file)),
                    site.line,
                    json_escape(&sanitize_committed_result_text(&site.directive)),
                    json_escape(&sanitize_committed_result_text(&site.expression)),
                    json_escape(&sanitize_committed_result_text(&site.reason)),
                )
            }),
    );
    forms.extend(
        sorted_refs(&stats.skipped_cpp_nested_edge_cases)
            .into_iter()
            .map(|site| {
                format!(
                    concat!(
                        "    {{\n",
                        "      \"kind\": \"skipped_cpp_nested_edge_case\",\n",
                        "      \"file\": \"{}\",\n",
                        "      \"line\": {},\n",
                        "      \"reason\": \"{}\"\n",
                        "    }}"
                    ),
                    json_escape(&committed_path_string(&site.file)),
                    site.line,
                    json_escape(&sanitize_committed_result_text(&site.reason)),
                )
            }),
    );
    forms.sort();
    forms.join(",\n")
}

fn render_ambiguous_makefile_sites_json(stats: &ReducerStats) -> String {
    sorted_refs(&stats.skipped_makefile_lines)
        .into_iter()
        .map(|site| {
            format!(
                concat!(
                    "    {{\n",
                    "      \"kind\": \"ambiguous_makefile_line\",\n",
                    "      \"file\": \"{}\",\n",
                    "      \"line\": {},\n",
                    "      \"assignment_lhs\": \"{}\",\n",
                    "      \"reason\": \"{}\"\n",
                    "    }}"
                ),
                json_escape(&committed_path_string(&site.file)),
                site.line,
                json_escape(&sanitize_committed_result_text(&site.assignment_lhs)),
                json_escape(&sanitize_committed_result_text(&site.reason)),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n")
}

fn render_skipped_fixup_sites_json(skipped_fixups: &[SkippedFixup]) -> String {
    sorted_skipped_fixups(skipped_fixups)
        .into_iter()
        .map(|skipped| {
            format!(
                concat!(
                    "    {{\n",
                    "      \"kind\": \"skipped_fixup_diagnostic\",\n",
                    "      \"fixer_name\": {},\n",
                    "      \"reason\": \"{}\",\n",
                    "      \"diagnostic\": {}\n",
                    "    }}"
                ),
                skipped
                    .fixer_name
                    .map(|name| format!("\"{}\"", json_escape(name)))
                    .unwrap_or_else(|| String::from("null")),
                json_escape(&sanitize_committed_result_text(&skipped.reason)),
                render_classified_diagnostic_json(&skipped.diagnostic),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n")
}

fn manual_include_kind_json(kind: ManualIncludeHandlingKind) -> &'static str {
    match kind {
        ManualIncludeHandlingKind::LiveMissingInclude => "live_missing_include",
        ManualIncludeHandlingKind::AmbiguousInclude => "ambiguous_include_site",
    }
}

fn include_kind_json(kind: IncludeKind) -> &'static str {
    match kind {
        IncludeKind::Quoted => "quoted",
        IncludeKind::Angle => "angle",
    }
}

fn include_resolve_rule_json(rule: IncludeResolveRule) -> &'static str {
    match rule {
        IncludeResolveRule::LocalDirectory => "local_directory",
        IncludeResolveRule::FileRelativeQuoted => "file_relative_quoted",
        IncludeResolveRule::IncludeRoot => "include_root",
        IncludeResolveRule::ArchIncludeRoot => "arch_include_root",
        IncludeResolveRule::ConfiguredGeneratedRoot => "configured_generated_root",
    }
}

fn include_target_classification_json(classification: IncludeTargetClassification) -> &'static str {
    match classification {
        IncludeTargetClassification::Exists => "exists",
        IncludeTargetClassification::RemovedByManifest => "removed_by_manifest",
        IncludeTargetClassification::AbsentForUnknownReason => "absent_for_unknown_reason",
        IncludeTargetClassification::PublicPreservedHeader => "public_preserved_header",
        IncludeTargetClassification::GeneratedHeader => "generated_header",
    }
}

fn render_classified_diagnostic_json(diagnostic: &ClassifiedDiagnostic) -> String {
    crate::diagnostics::render_classified_diagnostic_json(
        diagnostic,
        committed_path_string,
        sanitize_committed_result_text,
        json_escape,
    )
}

fn render_fixup_proof_json(proof: &FixupProof) -> String {
    match proof {
        FixupProof::ManifestPath { path } => format!(
            "{{\"kind\":\"manifest_path\",\"path\":\"{}\"}}",
            json_escape(&committed_path_string(path))
        ),
        FixupProof::TreeIndexIncludeSite { file, line, target } => format!(
            concat!(
                "{{",
                "\"kind\":\"tree_index_include_site\",",
                "\"file\":\"{}\",",
                "\"line\":{},",
                "\"target\":\"{}\"",
                "}}"
            ),
            json_escape(&committed_path_string(file)),
            line,
            json_escape(&sanitize_committed_result_text(target)),
        ),
        FixupProof::TreeIndexKbuildDirectoryRef {
            file,
            line,
            assignment_lhs,
            directory,
            resolved_path,
        } => format!(
            concat!(
                "{{",
                "\"kind\":\"tree_index_kbuild_directory_ref\",",
                "\"file\":\"{}\",",
                "\"line\":{},",
                "\"assignment_lhs\":\"{}\",",
                "\"directory\":\"{}\",",
                "\"resolved_path\":\"{}\"",
                "}}"
            ),
            json_escape(&committed_path_string(file)),
            line,
            json_escape(&sanitize_committed_result_text(assignment_lhs)),
            json_escape(&sanitize_committed_result_text(directory)),
            json_escape(&committed_path_string(resolved_path)),
        ),
        FixupProof::TreeIndexKbuildObjectRef {
            file,
            line,
            assignment_lhs,
            object,
            resolved_path,
        } => format!(
            concat!(
                "{{",
                "\"kind\":\"tree_index_kbuild_object_ref\",",
                "\"file\":\"{}\",",
                "\"line\":{},",
                "\"assignment_lhs\":\"{}\",",
                "\"object\":\"{}\",",
                "\"resolved_path\":\"{}\"",
                "}}"
            ),
            json_escape(&committed_path_string(file)),
            line,
            json_escape(&sanitize_committed_result_text(assignment_lhs)),
            json_escape(&sanitize_committed_result_text(object)),
            json_escape(&committed_path_string(resolved_path)),
        ),
        FixupProof::TreeIndexKconfigSourceRef {
            file,
            line,
            source,
            optional,
            relative,
        } => format!(
            concat!(
                "{{",
                "\"kind\":\"tree_index_kconfig_source_ref\",",
                "\"file\":\"{}\",",
                "\"line\":{},",
                "\"source\":\"{}\",",
                "\"optional\":{},",
                "\"relative\":{}",
                "}}"
            ),
            json_escape(&committed_path_string(file)),
            line,
            json_escape(&sanitize_committed_result_text(source)),
            if *optional { "true" } else { "false" },
            if *relative { "true" } else { "false" },
        ),
        FixupProof::ClassifiedDiagnostic {
            class,
            file,
            line,
            subject,
        } => format!(
            concat!(
                "{{",
                "\"kind\":\"classified_diagnostic\",",
                "\"class\":\"{}\",",
                "\"file\":{},",
                "\"line\":{},",
                "\"subject\":{}",
                "}}"
            ),
            class.stable_name(),
            file.as_deref()
                .map(|path| format!("\"{}\"", json_escape(&committed_path_string(path))))
                .unwrap_or_else(|| String::from("null")),
            line.map(|line| line.to_string())
                .unwrap_or_else(|| String::from("null")),
            subject
                .as_deref()
                .map(sanitize_committed_result_text)
                .map(|subject| format!("\"{}\"", json_escape(&subject)))
                .unwrap_or_else(|| String::from("null")),
        ),
    }
}

fn render_edit_reason_json(reason: &EditReason) -> String {
    format!(
        "{{\"kind\":\"{}\",\"payload\":\"{}\"}}",
        reason.json_key(),
        json_escape(&sanitize_committed_result_text(&reason.payload_label()))
    )
}

fn render_edit_proof_source_json(edit: &EditRecord) -> String {
    format!(
        "{{\"kind\":\"{}\",\"payload\":\"{}\"}}",
        edit.proof_source.kind().json_key(),
        json_escape(&sanitize_committed_result_text(
            &edit.proof_source.payload_label()
        ))
    )
}

fn byte_sha256(content: &str) -> String {
    hex::encode(Sha256::digest(content.as_bytes()))
}

fn render_edit_record_json(edit: &EditRecord) -> String {
    let line_start = edit
        .line_range
        .map(|range| range.start.to_string())
        .unwrap_or_else(|| String::from("null"));
    let line_end = edit
        .line_range
        .map(|range| range.end.to_string())
        .unwrap_or_else(|| String::from("null"));
    let before = sanitize_committed_result_text(&edit.before);
    let after = sanitize_committed_result_text(&edit.after);
    let before_byte_len = before.as_bytes().len();
    let after_byte_len = after.as_bytes().len();
    let before_sha256 = byte_sha256(&before);
    let after_sha256 = byte_sha256(&after);

    format!(
        concat!(
            "    {{\n",
            "      \"file\": \"{}\",\n",
            "      \"pass_name\": \"{}\",\n",
            "      \"edit_kind\": \"{}\",\n",
            "      \"edit_reason\": {},\n",
            "      \"proof_source\": {},\n",
            "      \"old\": {{\n",
            "        \"line_start\": {},\n",
            "        \"line_end\": {},\n",
            "        \"logical_item\": \"{}\",\n",
            "        \"byte_len\": {},\n",
            "        \"sha256\": \"{}\"\n",
            "      }},\n",
            "      \"new\": {{\n",
            "        \"logical_item\": \"{}\",\n",
            "        \"byte_len\": {},\n",
            "        \"sha256\": \"{}\"\n",
            "      }},\n",
            "      \"idempotence_marker\": \"{}\"\n",
            "    }}"
        ),
        json_escape(&committed_path_string(&edit.file)),
        json_escape(edit.pass_name),
        edit.edit_kind.json_key(),
        render_edit_reason_json(&edit.reason),
        render_edit_proof_source_json(edit),
        line_start,
        line_end,
        json_escape(&before),
        before_byte_len,
        before_sha256,
        json_escape(&after),
        after_byte_len,
        after_sha256,
        edit.idempotence_marker.as_str(),
    )
}

pub(crate) fn render_edit_records_json(edits: &[EditRecord]) -> String {
    sorted_edit_record_refs(edits)
        .into_iter()
        .map(render_edit_record_json)
        .collect::<Vec<_>>()
        .join(",\n")
}

fn render_fixup_edit_json(edit: &EditRecord) -> String {
    format!(
        concat!(
            "      {{",
            "\"file\":\"{}\",",
            "\"line_start\":{},",
            "\"line_end\":{},",
            "\"edit_kind\":\"{}\",",
            "\"pass_name\":\"{}\",",
            "\"idempotence_marker\":\"{}\"",
            "}}"
        ),
        json_escape(&committed_path_string(&edit.file)),
        edit.line_range
            .map(|range| range.start.to_string())
            .unwrap_or_else(|| String::from("null")),
        edit.line_range
            .map(|range| range.end.to_string())
            .unwrap_or_else(|| String::from("null")),
        edit.edit_kind.json_key(),
        json_escape(edit.pass_name),
        edit.idempotence_marker.as_str(),
    )
}

fn render_applied_fixups_json(fixups: &[AppliedFixup]) -> String {
    sorted_applied_fixups(fixups)
        .into_iter()
        .map(|fixup| {
            let edits = sorted_edit_record_refs(&fixup.edits)
                .into_iter()
                .map(render_fixup_edit_json)
                .collect::<Vec<_>>()
                .join(",\n");
            let proofs = fixup
                .proof_sources
                .iter()
                .map(render_fixup_proof_json)
                .map(|proof| format!("      {}", proof))
                .collect::<Vec<_>>()
                .join(",\n");
            format!(
                concat!(
                    "    {{\n",
                    "      \"fixer_name\": \"{}\",\n",
                    "      \"diagnostic\": {},\n",
                    "      \"edits_made\": [\n",
                    "{}\n",
                    "      ],\n",
                    "      \"proof_sources\": [\n",
                    "{}\n",
                    "      ]\n",
                    "    }}"
                ),
                json_escape(fixup.fixer_name),
                render_classified_diagnostic_json(&fixup.diagnostic),
                edits,
                proofs,
            )
        })
        .collect::<Vec<_>>()
        .join(",\n")
}

fn render_skipped_fixups_json(skipped_fixups: &[SkippedFixup]) -> String {
    sorted_skipped_fixups(skipped_fixups)
        .into_iter()
        .map(|skipped| {
            format!(
                concat!(
                    "    {{\n",
                    "      \"fixer_name\": {},\n",
                    "      \"reason\": \"{}\",\n",
                    "      \"diagnostic\": {}\n",
                    "    }}"
                ),
                skipped
                    .fixer_name
                    .map(|name| format!("\"{}\"", json_escape(name)))
                    .unwrap_or_else(|| String::from("null")),
                json_escape(&sanitize_committed_result_text(&skipped.reason)),
                render_classified_diagnostic_json(&skipped.diagnostic),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n")
}
