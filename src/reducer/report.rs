mod json;
mod model;
mod render;
mod summary;
mod text;

#[cfg(test)]
use super::diagnostics::RawDiagnosticExcerpt;
#[cfg(test)]
use super::{ReducerResult, ReducerStats};
#[cfg(test)]
use crate::diagnostics::ClassifiedDiagnostic;
#[cfg(test)]
use crate::kconfig::UnsupportedKconfigExpression;

#[allow(unused_imports)]
pub(crate) use json::{
    render_edit_records_json, render_kconfig_rewrite_report_json,
    render_kconfig_solver_report_json, render_reducer_diagnostics_json,
    render_reducer_skipped_sites_json,
};
#[allow(unused_imports)]
pub(crate) use model::{
    ReducerFailureReport, ReducerReportArtifactNames, RenderedReducerReportArtifacts,
    REDUCER_REPORT_SCHEMA_VERSION,
};
pub use render::ensure_supported_fallout;
#[allow(unused_imports)]
pub(crate) use render::{
    render_reducer_result_report_artifacts, render_reducer_stats_report_artifacts,
    render_reducer_stats_report_artifacts_with_manifest,
};
#[allow(unused_imports)]
pub(crate) use summary::render_unsupported_expression_report;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ReducerConfig, SlimConfig};
    use crate::hardware::DeviceBindingRemovalProof;
    use crate::edit_reason::{DiagnosticClass, EditProofSource, EditReason, EditRecord, LineRange};
    use crate::exported_symbols::ExportedSymbolRemovalProof;
    use crate::fixups::{AppliedFixup, FixupProof, SkippedFixup};
    use crate::includes::{
        ClassifiedIncludeTarget, IncludeKind, IncludeResolveRule, IncludeSite,
        IncludeTargetClassification, ManualIncludeHandlingKind, ManualIncludeHandlingSite,
        ResolvedIncludeTarget,
    };
    use crate::kbuild::KbuildSkippedLine;
    use crate::model::{DeviceCompatible, ExportedSymbol};
    use crate::prune::RemovalAccounting;
    use crate::reducer::{BuildMatrixStatus, ConvergenceStatus, ReducerStatus};
    use crate::removal_manifest::RemovalManifest;
    use crate::runtime::RuntimeRegistrationRemovalProof;
    use serde_json::Value;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn artifact_names() -> ReducerReportArtifactNames<'static> {
        ReducerReportArtifactNames {
            markdown: "reducer-report.md",
            summary_json: "reducer-report.json",
            diagnostics_json: "diagnostics.json",
            edit_summary_json: "edit-summary.json",
            kconfig_solver_report_json: "kconfig-solver-report.json",
            kconfig_rewrite_report_json: "kconfig-rewrite-report.json",
            skipped_sites_json: "skipped-sites.json",
        }
    }

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
            "prune.remove_path",
        )
    }

    fn missing_header_diagnostic(path: &str, line: usize, header: &str) -> ClassifiedDiagnostic {
        ClassifiedDiagnostic::MissingHeader {
            source_file: PathBuf::from(path),
            line,
            header: header.to_string(),
            build_target: Some(String::from("modules")),
            arch: Some(String::from("arm64")),
            config: Some(String::from("defconfig")),
        }
    }

    fn unreasoned_diagnostic_edit() -> EditRecord {
        EditRecord::new(
            PathBuf::from("drivers/foo/test.c"),
            Some(LineRange { start: 1, end: 1 }),
            String::from("before\n"),
            String::from("after\n"),
            EditReason::BuildDiagnostic {
                class: DiagnosticClass::Unknown,
            },
            EditProofSource::ClassifiedDiagnostic {
                diagnostic_id: DiagnosticClass::Unknown.into(),
            },
            "test.unreasoned",
        )
    }

    fn speculative_fallout_edit() -> EditRecord {
        EditRecord::new(
            PathBuf::from("drivers/foo/test.c"),
            Some(LineRange { start: 1, end: 1 }),
            String::from("before\n"),
            String::from("after\n"),
            EditReason::BuildDiagnostic {
                class: DiagnosticClass::UndefinedReference,
            },
            EditProofSource::ClassifiedDiagnostic {
                diagnostic_id: DiagnosticClass::UndefinedReference.into(),
            },
            "test.speculative_fallout",
        )
    }

    #[test]
    fn reducer_report_rejects_unreasoned_edits_when_policy_enabled() {
        let stats = ReducerStats {
            ran: true,
            edits: vec![unreasoned_diagnostic_edit()],
            ..ReducerStats::default()
        };

        let err = render_reducer_stats_report_artifacts(
            &stats,
            Some(&ReducerConfig::default()),
            artifact_names(),
        )
        .unwrap_err();
        let err = format!("{err:#}");

        assert!(err.contains("unreasoned EditReason"));

        let mut relaxed = ReducerConfig::default();
        relaxed.reject_unreasoned_edits = false;
        render_reducer_stats_report_artifacts(&stats, Some(&relaxed), artifact_names()).unwrap();
    }

    #[test]
    fn reducer_report_rejects_speculative_fallout_edits_when_policy_enabled() {
        let stats = ReducerStats {
            ran: true,
            edits: vec![speculative_fallout_edit()],
            ..ReducerStats::default()
        };

        let err = render_reducer_stats_report_artifacts(
            &stats,
            Some(&ReducerConfig::default()),
            artifact_names(),
        )
        .unwrap_err();
        let err = format!("{err:#}");

        assert!(err.contains("broad speculative fallout edit"));

        let mut relaxed = ReducerConfig::default();
        relaxed.reject_speculative_fallout_edits = false;
        render_reducer_stats_report_artifacts(&stats, Some(&relaxed), artifact_names()).unwrap();
    }

    #[test]
    fn reducer_report_marks_unsupported_syntax_publishable_only_when_policy_disabled() {
        let stats = ReducerStats {
            ran: true,
            unsupported_kconfig_expressions: vec![UnsupportedKconfigExpression {
                file: PathBuf::from("Kconfig"),
                line: 3,
                directive: String::from("depends on"),
                expression: String::from("REMOVED + LIVE"),
                reason: String::from("unsupported expression"),
            }],
            ..ReducerStats::default()
        };

        let strict = render_reducer_stats_report_artifacts(
            &stats,
            Some(&ReducerConfig::default()),
            artifact_names(),
        )
        .unwrap();
        let strict: Value = serde_json::from_str(&strict.summary_json).unwrap();
        assert_eq!(
            strict["final_status"]["status"],
            "failed_unsupported_syntax"
        );
        assert_eq!(strict["final_status"]["publishable"], false);

        let mut relaxed = ReducerConfig::default();
        relaxed.report_unsupported_expressions = false;
        let relaxed =
            render_reducer_stats_report_artifacts(&stats, Some(&relaxed), artifact_names())
                .unwrap();
        let relaxed: Value = serde_json::from_str(&relaxed.summary_json).unwrap();
        assert_eq!(relaxed["final_status"]["status"], "success");
        assert_eq!(relaxed["final_status"]["publishable"], true);
    }

    #[test]
    fn reducer_report_marks_unknown_diagnostics_publishable_only_when_policy_disabled() {
        let stats = ReducerStats {
            ran: true,
            classified_diagnostics: vec![ClassifiedDiagnostic::Unknown],
            ..ReducerStats::default()
        };

        let strict = render_reducer_stats_report_artifacts(
            &stats,
            Some(&ReducerConfig::default()),
            artifact_names(),
        )
        .unwrap();
        let strict: Value = serde_json::from_str(&strict.summary_json).unwrap();
        assert_eq!(
            strict["final_status"]["status"],
            "failed_unknown_diagnostic"
        );
        assert_eq!(strict["final_status"]["publishable"], false);

        let mut relaxed = ReducerConfig::default();
        relaxed.fail_on_unknown_diagnostics = false;
        let relaxed =
            render_reducer_stats_report_artifacts(&stats, Some(&relaxed), artifact_names())
                .unwrap();
        let relaxed: Value = serde_json::from_str(&relaxed.summary_json).unwrap();
        assert_eq!(relaxed["final_status"]["status"], "success");
        assert_eq!(relaxed["final_status"]["publishable"], true);
    }

    #[test]
    fn strict_unsupported_syntax_policy_rejects_reported_sites() {
        let stats = ReducerStats {
            ran: true,
            unsupported_kconfig_expressions: vec![UnsupportedKconfigExpression {
                file: PathBuf::from("Kconfig"),
                line: 3,
                directive: String::from("depends on"),
                expression: String::from("REMOVED + LIVE"),
                reason: String::from("unsupported expression"),
            }],
            ..ReducerStats::default()
        };

        let err = ensure_supported_fallout(&stats, &ReducerConfig::default())
            .unwrap_err()
            .to_string();

        assert!(err.contains("unsupported Kconfig expressions"));
        assert!(err.contains("Kconfig:3"));

        let mut relaxed = ReducerConfig::default();
        relaxed.report_unsupported_expressions = false;
        ensure_supported_fallout(&stats, &relaxed).unwrap();
    }

    #[test]
    fn strict_unknown_diagnostic_policy_rejects_reported_unknowns() {
        let stats = ReducerStats {
            ran: true,
            skipped_fixups: vec![SkippedFixup {
                fixer_name: None,
                diagnostic: ClassifiedDiagnostic::Unknown,
                reason: String::from("unknown diagnostic"),
            }],
            ..ReducerStats::default()
        };

        let err = ensure_supported_fallout(&stats, &ReducerConfig::default())
            .unwrap_err()
            .to_string();

        assert!(err.contains("unknown diagnostic in strict mode"));
        assert!(err.contains("skipped diagnostic"));

        let mut relaxed = ReducerConfig::default();
        relaxed.fail_on_unknown_diagnostics = false;
        ensure_supported_fallout(&stats, &relaxed).unwrap();
    }

    #[test]
    fn reducer_report_json_contains_required_public_sections() {
        let mut manifest = RemovalManifest::from_slim_config(&SlimConfig {
            remove_paths: vec![String::from("drivers/foo/bar.c")],
            remove_configs: vec![String::from("REMOVE_ME")],
            set_defaults: BTreeMap::from([(String::from("KEEP_ME"), String::from("n"))]),
            unsafe_allow_root_path_removal: false,
        })
        .unwrap();
        manifest
            .removed_exported_symbols
            .insert(ExportedSymbolRemovalProof {
                symbol: ExportedSymbol::new("foo_api").unwrap(),
                provider: PathBuf::from("drivers/foo/bar.c"),
                export_macro: String::from("EXPORT_SYMBOL_GPL"),
                line: 7,
            });
        manifest
            .removed_device_bindings
            .insert(DeviceBindingRemovalProof {
                binding: PathBuf::from("Documentation/devicetree/bindings/vendor/foo.yaml"),
                compatible_strings: vec![DeviceCompatible::new("vendor,foo").unwrap()],
                schema_references: vec![String::from("/schemas/vendor/foo.yaml")],
            });
        manifest
            .removed_runtime_registrations
            .insert(RuntimeRegistrationRemovalProof {
                provider: PathBuf::from("drivers/foo/bar.c"),
                registration_macro: String::from("module_init"),
                entry_points: vec![String::from("foo_init")],
                line: 9,
            });
        let stats = ReducerStats {
            ran: true,
            files_removed: 1,
            removal: RemovalAccounting {
                removed_files: vec![PathBuf::from("drivers/foo/bar.c")],
                removed_config_symbols: vec![String::from("REMOVE_ME")],
                ..RemovalAccounting::default()
            },
            edits: vec![manifest_path_edit("drivers/foo/bar.c")],
            ..ReducerStats::default()
        };
        let mut result = ReducerResult::from_pipeline_artifacts(
            Some(manifest),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            stats,
        );
        result.set_publication_state(
            ReducerStatus::FailedBuildMatrix,
            BuildMatrixStatus::Failed,
            ConvergenceStatus::NotEvaluated,
        );
        let reducer_config = ReducerConfig {
            max_fixup_passes: 7,
            report_unsupported_expressions: false,
            fail_on_unknown_diagnostics: false,
            reject_unproven_fixups: true,
            reject_unreasoned_edits: true,
            reject_speculative_fallout_edits: true,
            fail_on_missing_prune_paths: false,
            ignore_unsupported_special_removals: false,
        };

        let artifacts = render_reducer_result_report_artifacts(
            &result,
            Some(&reducer_config),
            artifact_names(),
        )
        .unwrap();
        let report: Value = serde_json::from_str(&artifacts.summary_json).unwrap();

        assert_eq!(report["schema_version"], REDUCER_REPORT_SCHEMA_VERSION);
        assert_eq!(
            report["artifacts"]["kconfig_solver_report_json"],
            "kconfig-solver-report.json"
        );
        assert_eq!(
            report["artifacts"]["kconfig_rewrite_report_json"],
            "kconfig-rewrite-report.json"
        );
        assert_eq!(report["reducer_config"]["max_fixup_passes"], 7);
        assert_eq!(report["reducer_config"]["reject_unreasoned_edits"], true);
        assert_eq!(
            report["reducer_config"]["reject_speculative_fallout_edits"],
            true
        );
        assert_eq!(
            report["reducer_config"]["fail_on_missing_prune_paths"],
            false
        );
        assert_eq!(
            report["reducer_config"]["ignore_unsupported_special_removals"],
            false
        );
        assert_eq!(
            report["normalized_removal_manifest"]["declared_paths"],
            serde_json::json!(["drivers/foo/bar.c"])
        );
        assert_eq!(
            report["normalized_removal_manifest"]["removed_config_symbols"],
            serde_json::json!(["REMOVE_ME"])
        );
        assert_eq!(
            report["normalized_removal_manifest"]["default_overrides"]["KEEP_ME"],
            "n"
        );
        assert_eq!(
            report["normalized_removal_manifest"]["removed_exported_symbol_count"],
            1
        );
        assert_eq!(
            report["normalized_removal_manifest"]["removed_device_binding_count"],
            1
        );
        assert_eq!(
            report["normalized_removal_manifest"]["removed_runtime_registration_count"],
            1
        );
        assert_eq!(
            report["normalized_removal_manifest"]["removed_device_bindings"][0]["binding"],
            "Documentation/devicetree/bindings/vendor/foo.yaml"
        );
        assert_eq!(
            report["normalized_removal_manifest"]["removed_runtime_registrations"][0]
                ["entry_points"],
            serde_json::json!(["foo_init"])
        );
        assert_eq!(
            report["normalized_removal_manifest"]["removed_exported_symbols"][0]["symbol"],
            "foo_api"
        );
        assert_eq!(
            report["normalized_removal_manifest"]["removed_exported_symbols"][0]["provider"],
            "drivers/foo/bar.c"
        );
        assert_eq!(report["passes"][0]["name"], "reducer.pipeline");
        assert_eq!(report["per_pass_edit_counts"]["prune.remove_path"], 1);
        assert_eq!(
            report["per_file_edit_records"][0]["file"],
            "drivers/foo/bar.c"
        );
        assert_eq!(report["matrix_status"], "failed");
        assert_eq!(report["convergence_status"], "not_evaluated");
        assert_eq!(report["final_status"]["status"], "failed_build_matrix");
        assert_eq!(report["final_status"]["publishable"], false);
        assert!(artifacts.markdown.contains("## Reducer config"));
        assert!(artifacts
            .markdown
            .contains("Final status: failed_build_matrix"));
        assert!(artifacts.markdown.contains("Build matrix status: failed"));
        assert!(artifacts
            .markdown
            .contains("Convergence status: not_evaluated"));
        assert!(artifacts
            .edit_summary_json
            .contains("\"schema_version\": 1"));
        assert!(artifacts
            .kconfig_solver_report_json
            .contains("\"removed_symbols\""));
        assert!(artifacts
            .kconfig_rewrite_report_json
            .contains("\"kconfig_edit_count\""));
        let edit_summary: Value = serde_json::from_str(&artifacts.edit_summary_json).unwrap();
        assert_eq!(edit_summary["byte_explanation"]["edit_records"], 1);
        assert_eq!(edit_summary["byte_explanation"]["old_bytes"], 7);
        assert_eq!(edit_summary["byte_explanation"]["new_bytes"], 0);
        assert_eq!(edit_summary["edit_record_details"][0]["old"]["byte_len"], 7);
        assert_eq!(
            edit_summary["edit_record_details"][0]["old"]["sha256"],
            "9160d4be34c8695bd172a76c7c7966587ea5a4d991ad22c87b2b91af54aa9ebb"
        );
        assert_eq!(edit_summary["edit_record_details"][0]["new"]["byte_len"], 0);
        assert_eq!(
            edit_summary["edit_record_details"][0]["new"]["sha256"],
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn reducer_report_sorts_removed_config_symbols_without_manifest() {
        let stats = ReducerStats {
            ran: true,
            removal: RemovalAccounting {
                removed_config_symbols: vec![
                    String::from("Z_SYMBOL"),
                    String::from("A_SYMBOL"),
                    String::from("Z_SYMBOL"),
                ],
                ..RemovalAccounting::default()
            },
            ..ReducerStats::default()
        };

        let artifacts = render_reducer_stats_report_artifacts(
            &stats,
            Some(&ReducerConfig::default()),
            artifact_names(),
        )
        .unwrap();
        let report: Value = serde_json::from_str(&artifacts.summary_json).unwrap();

        assert_eq!(
            report["normalized_removal_manifest"]["removed_config_symbols"],
            serde_json::json!(["A_SYMBOL", "Z_SYMBOL"])
        );
        assert_eq!(
            report["normalized_removal_manifest"]["removed_config_symbol_count"],
            2
        );
    }

    #[test]
    fn reducer_report_sorts_dynamic_report_maps_by_key() {
        fn stale_kbuild_edit(path: &str, reference: &str) -> EditRecord {
            EditRecord::new(
                PathBuf::from(path),
                Some(LineRange { start: 4, end: 4 }),
                format!("obj-y += {reference}\n"),
                String::new(),
                EditReason::RemovedKbuildRef {
                    reference: reference.to_string(),
                },
                EditProofSource::stale_kbuild_reference(reference.to_string()),
                "prune.rewrite_makefiles",
            )
        }

        fn index_of(haystack: &str, needle: &str) -> usize {
            haystack
                .find(needle)
                .unwrap_or_else(|| panic!("missing {needle:?} in rendered report"))
        }

        fn assert_before(haystack: &str, left: &str, right: &str) {
            assert!(
                index_of(haystack, left) < index_of(haystack, right),
                "expected {left:?} before {right:?} in {haystack}"
            );
        }

        fn section_between<'a>(haystack: &'a str, start: &str, end: &str) -> &'a str {
            let start_index = index_of(haystack, start) + start.len();
            let tail = &haystack[start_index..];
            let end_index = tail
                .find(end)
                .unwrap_or_else(|| panic!("missing section end {end:?} after {start:?}"));
            &tail[..end_index]
        }

        let manifest = RemovalManifest::from_slim_config(&SlimConfig {
            remove_paths: vec![String::from("drivers/z.c")],
            remove_configs: Vec::new(),
            set_defaults: BTreeMap::from([
                (String::from("Z_DEFAULT"), String::from("n")),
                (String::from("A_DEFAULT"), String::from("y")),
            ]),
            unsafe_allow_root_path_removal: false,
        })
        .unwrap();
        let stats = ReducerStats {
            ran: true,
            edits: vec![
                stale_kbuild_edit("drivers/Makefile", "z.o"),
                manifest_path_edit("drivers/z.c"),
            ],
            ..ReducerStats::default()
        };
        let result = ReducerResult::from_pipeline_artifacts(
            Some(manifest),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            stats,
        );
        let artifacts = render_reducer_result_report_artifacts(
            &result,
            Some(&ReducerConfig::default()),
            artifact_names(),
        )
        .unwrap();

        assert_before(&artifacts.summary_json, "\"A_DEFAULT\"", "\"Z_DEFAULT\"");

        let summary_proofs = section_between(
            &artifacts.summary_json,
            "    \"proof_sources\": {\n",
            "\n    }\n  },\n  \"unsupported_fallout\"",
        );
        assert_before(
            summary_proofs,
            "\"classified_build_diagnostic\"",
            "\"kconfig_solver_proof\"",
        );
        assert_before(
            summary_proofs,
            "\"kconfig_solver_proof\"",
            "\"removal_manifest_entry\"",
        );
        assert_before(
            summary_proofs,
            "\"removal_manifest_entry\"",
            "\"stale_reference\"",
        );
        assert_before(
            summary_proofs,
            "\"stale_reference\"",
            "\"tree_index_entry\"",
        );

        let per_pass = section_between(
            &artifacts.summary_json,
            "  \"per_pass_edit_counts\": ",
            ",\n  \"per_file_edit_records\"",
        );
        assert_before(
            per_pass,
            "\"prune.remove_path\"",
            "\"prune.rewrite_makefiles\"",
        );

        let edit_summary_proofs = section_between(
            &artifacts.edit_summary_json,
            "  \"proof_sources\": {\n",
            "\n  },\n  \"edits_by_pass\"",
        );
        assert_before(
            edit_summary_proofs,
            "\"classified_build_diagnostic\"",
            "\"kconfig_solver_proof\"",
        );
        assert_before(
            edit_summary_proofs,
            "\"kconfig_solver_proof\"",
            "\"removal_manifest_entry\"",
        );
        assert_before(
            edit_summary_proofs,
            "\"removal_manifest_entry\"",
            "\"stale_reference\"",
        );
        assert_before(
            edit_summary_proofs,
            "\"stale_reference\"",
            "\"tree_index_entry\"",
        );

        let edits_by_pass = section_between(
            &artifacts.edit_summary_json,
            "  \"edits_by_pass\": {\n",
            "\n  },\n  \"edits_by_reason\"",
        );
        assert_before(
            edits_by_pass,
            "\"prune.remove_path\"",
            "\"prune.rewrite_makefiles\"",
        );

        let edits_by_reason = section_between(
            &artifacts.edit_summary_json,
            "  \"edits_by_reason\": {\n",
            "\n  }\n}",
        );
        assert_before(
            edits_by_reason,
            "\"manifest_path\"",
            "\"removed_kbuild_ref\"",
        );
    }

    #[test]
    fn reducer_report_redacts_host_paths_in_committed_artifacts() {
        let temp = tempfile::tempdir().unwrap();
        let host_path = temp.path().join("candidate-tree/removed.c");
        let host_path_string = host_path.to_string_lossy().to_string();
        let stats = ReducerStats {
            ran: true,
            files_removed: 1,
            removal: RemovalAccounting {
                removed_files: vec![host_path.clone()],
                ..RemovalAccounting::default()
            },
            ..ReducerStats::default()
        };

        let artifacts =
            render_reducer_stats_report_artifacts(&stats, None, artifact_names()).unwrap();

        for content in [
            &artifacts.markdown,
            &artifacts.summary_json,
            &artifacts.diagnostics_json,
            &artifacts.edit_summary_json,
            &artifacts.kconfig_solver_report_json,
            &artifacts.kconfig_rewrite_report_json,
        ] {
            assert!(!content.contains(&host_path_string));
        }
        assert!(artifacts
            .summary_json
            .contains(crate::reducer::result::REDUCER_RESULT_HOST_PATH_REDACTION));
    }

    #[test]
    fn diagnostic_report_contains_normalized_log_summaries_and_structured_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let host_path = temp.path().join("candidate-tree/drivers/foo/test.c");
        let host_path_string = host_path.to_string_lossy().to_string();
        let missing_header = missing_header_diagnostic("drivers/foo/test.c", 9, "missing/header.h");
        let consumed_edit = EditRecord::new(
            PathBuf::from("drivers/foo/test.c"),
            Some(LineRange { start: 9, end: 9 }),
            String::from("#include <missing/header.h>\n"),
            String::new(),
            EditReason::BuildDiagnostic {
                class: DiagnosticClass::MissingHeader,
            },
            EditProofSource::ClassifiedDiagnostic {
                diagnostic_id: DiagnosticClass::MissingHeader.into(),
            },
            "fixups.remove_missing_header_include",
        );
        let stats = ReducerStats {
            ran: true,
            classified_diagnostics: vec![missing_header.clone(), ClassifiedDiagnostic::Unknown],
            raw_diagnostic_excerpts: vec![
                RawDiagnosticExcerpt {
                    command_context: String::from("make modules"),
                    build_target: Some(String::from("modules")),
                    raw_excerpt: format!("{host_path_string}:9: fatal error: missing/header.h"),
                },
                RawDiagnosticExcerpt {
                    command_context: String::from("make modules"),
                    build_target: Some(String::from("modules")),
                    raw_excerpt: String::from("second failure line"),
                },
                RawDiagnosticExcerpt {
                    command_context: String::from("built-in selftest: kconfig-sources"),
                    build_target: None,
                    raw_excerpt: String::from("missing Kconfig source"),
                },
            ],
            applied_fixups: vec![AppliedFixup {
                fixer_name: "fixups.remove_missing_header_include",
                diagnostic: missing_header,
                edits: vec![consumed_edit],
                proof_sources: vec![FixupProof::ManifestPath {
                    path: PathBuf::from("drivers/foo/missing/header.h"),
                }],
            }],
            skipped_fixups: vec![SkippedFixup {
                fixer_name: None,
                diagnostic: ClassifiedDiagnostic::Unknown,
                reason: String::from("unknown diagnostic"),
            }],
            ..ReducerStats::default()
        };

        let diagnostics: Value =
            serde_json::from_str(&render_reducer_diagnostics_json(&stats)).unwrap();

        assert_eq!(diagnostics["schema_version"], REDUCER_REPORT_SCHEMA_VERSION);
        assert_eq!(
            diagnostics["diagnostic_log_summaries_by_command"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        let make_group = diagnostics["diagnostic_log_summaries_by_command"]
            .as_array()
            .unwrap()
            .iter()
            .find(|group| group["command_context"] == "make modules")
            .unwrap();
        assert_eq!(make_group["build_target"], "modules");
        assert_eq!(make_group["log_excerpt_count"], 2);
        let rendered = diagnostics.to_string();
        assert!(!rendered.contains(&host_path_string));
        assert!(!rendered.contains("fatal error: missing/header.h"));
        assert!(!rendered.contains("second failure line"));
        assert!(diagnostics.get("raw_diagnostics_by_command").is_none());

        let classified_classes = diagnostics["classified_diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|diagnostic| diagnostic["class"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert!(classified_classes.contains(&"MissingHeader"));
        assert!(classified_classes.contains(&"Unknown"));
        assert_eq!(diagnostics["unknown_diagnostics"][0]["class"], "Unknown");
        assert_eq!(
            diagnostics["consumed_diagnostics"][0]["fixer_name"],
            "fixups.remove_missing_header_include"
        );
        assert_eq!(diagnostics["consumed_diagnostics"][0]["edit_count"], 1);
        assert_eq!(
            diagnostics["skipped_diagnostics"][0]["reason"],
            "unknown diagnostic"
        );
        assert_eq!(
            diagnostics["skipped_fixup_diagnostics"][0]["kind"],
            "skipped_fixup_diagnostic"
        );
    }

    #[test]
    fn diagnostic_report_sorts_log_summaries_by_command() {
        let stats = ReducerStats {
            ran: true,
            raw_diagnostic_excerpts: vec![
                RawDiagnosticExcerpt {
                    command_context: String::from("make modules"),
                    build_target: Some(String::from("modules")),
                    raw_excerpt: String::from("z failure\n"),
                },
                RawDiagnosticExcerpt {
                    command_context: String::from("make modules"),
                    build_target: Some(String::from("modules")),
                    raw_excerpt: String::from("a failure\n"),
                },
                RawDiagnosticExcerpt {
                    command_context: String::from("make modules"),
                    build_target: Some(String::from("modules")),
                    raw_excerpt: String::from("z failure\n"),
                },
                RawDiagnosticExcerpt {
                    command_context: String::from("built-in selftest: makefiles"),
                    build_target: None,
                    raw_excerpt: String::from("builtin failure\n"),
                },
            ],
            ..ReducerStats::default()
        };

        let diagnostics: Value =
            serde_json::from_str(&render_reducer_diagnostics_json(&stats)).unwrap();
        let groups = diagnostics["diagnostic_log_summaries_by_command"]
            .as_array()
            .unwrap();
        assert_eq!(
            groups
                .iter()
                .map(|group| group["command_context"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["built-in selftest: makefiles", "make modules"]
        );
        let make_group = groups
            .iter()
            .find(|group| group["command_context"] == "make modules")
            .unwrap();

        assert_eq!(make_group["log_excerpt_count"], 3);
        assert!(make_group.get("raw_excerpts").is_none());
        assert!(!diagnostics.to_string().contains("z failure"));
        assert!(!diagnostics.to_string().contains("a failure"));
    }

    #[test]
    fn diagnostic_report_sorts_and_deduplicates_classified_diagnostics() {
        let alpha = missing_header_diagnostic("drivers/a.c", 1, "a.h");
        let zed = missing_header_diagnostic("drivers/z.c", 7, "z.h");
        let stats = ReducerStats {
            ran: true,
            classified_diagnostics: vec![ClassifiedDiagnostic::Unknown, zed.clone()],
            applied_fixups: vec![AppliedFixup {
                fixer_name: "fixups.remove_missing_header_include",
                diagnostic: alpha.clone(),
                edits: Vec::new(),
                proof_sources: Vec::new(),
            }],
            skipped_fixups: vec![SkippedFixup {
                fixer_name: Some("fixups.remove_missing_header_include"),
                diagnostic: zed,
                reason: String::from("not enough proof"),
            }],
            ..ReducerStats::default()
        };

        let diagnostics: Value =
            serde_json::from_str(&render_reducer_diagnostics_json(&stats)).unwrap();
        let classified = diagnostics["classified_diagnostics"].as_array().unwrap();

        assert_eq!(classified.len(), 3);
        assert_eq!(classified[0]["class"], "MissingHeader");
        assert_eq!(classified[0]["file"], "drivers/a.c");
        assert_eq!(classified[0]["subject"], "a.h");
        assert_eq!(classified[1]["class"], "MissingHeader");
        assert_eq!(classified[1]["file"], "drivers/z.c");
        assert_eq!(classified[1]["subject"], "z.h");
        assert_eq!(classified[2]["class"], "Unknown");
    }

    #[test]
    fn skipped_site_report_contains_all_manual_site_categories() {
        let stats = ReducerStats {
            ran: true,
            unsupported_kconfig_expressions: vec![UnsupportedKconfigExpression {
                file: PathBuf::from("Kconfig"),
                line: 3,
                directive: String::from("depends on"),
                expression: String::from("REMOVED + LIVE"),
                reason: String::from("unsupported expression"),
            }],
            unsupported_cpp_expressions: vec![crate::cpp::UnsupportedCppExpression {
                file: PathBuf::from("drivers/foo/test.c"),
                line: 8,
                directive: String::from("if"),
                expression: String::from("defined(CONFIG_REMOVED) + LIVE"),
                reason: String::from("unsupported preprocessor form"),
            }],
            skipped_cpp_nested_edge_cases: vec![crate::cpp::SkippedCppNestedEdgeCase {
                file: PathBuf::from("drivers/foo/nested.c"),
                line: 12,
                reason: String::from("unknown enclosing condition"),
            }],
            skipped_makefile_lines: vec![KbuildSkippedLine {
                file: PathBuf::from("drivers/foo/Makefile"),
                line: 6,
                assignment_lhs: String::from("ccflags-y"),
                reason: String::from("ambiguous include path"),
            }],
            manual_include_sites: vec![
                ManualIncludeHandlingSite {
                    site: IncludeSite {
                        file: PathBuf::from("drivers/foo/test.c"),
                        line: 4,
                        header: String::from("shared.h"),
                        kind: IncludeKind::Quoted,
                    },
                    kind: ManualIncludeHandlingKind::AmbiguousInclude,
                    classified_targets: vec![
                        ClassifiedIncludeTarget {
                            target: ResolvedIncludeTarget {
                                path: PathBuf::from("drivers/foo/shared.h"),
                                rule: IncludeResolveRule::LocalDirectory,
                            },
                            classification: IncludeTargetClassification::Exists,
                        },
                        ClassifiedIncludeTarget {
                            target: ResolvedIncludeTarget {
                                path: PathBuf::from("include/shared.h"),
                                rule: IncludeResolveRule::IncludeRoot,
                            },
                            classification: IncludeTargetClassification::PublicPreservedHeader,
                        },
                    ],
                },
                ManualIncludeHandlingSite {
                    site: IncludeSite {
                        file: PathBuf::from("drivers/foo/live.c"),
                        line: 5,
                        header: String::from("linux/missing.h"),
                        kind: IncludeKind::Angle,
                    },
                    kind: ManualIncludeHandlingKind::LiveMissingInclude,
                    classified_targets: Vec::new(),
                },
            ],
            ..ReducerStats::default()
        };

        let report: Value =
            serde_json::from_str(&render_reducer_skipped_sites_json(&stats)).unwrap();

        assert_eq!(report["schema_version"], REDUCER_REPORT_SCHEMA_VERSION);
        assert_eq!(
            report["ambiguous_include_sites"][0]["kind"],
            "ambiguous_include_site"
        );
        assert_eq!(
            report["ambiguous_include_sites"][0]["classified_targets"][0]["rule"],
            "local_directory"
        );
        assert_eq!(
            report["live_missing_includes"][0]["kind"],
            "live_missing_include"
        );
        assert_eq!(
            report["unsupported_kconfig_expressions"][0]["kind"],
            "unsupported_kconfig_expression"
        );
        assert_eq!(
            report["unsupported_cpp_forms"][0]["kind"],
            "skipped_cpp_nested_edge_case"
        );
        assert!(report["unsupported_cpp_forms"]
            .as_array()
            .unwrap()
            .iter()
            .any(|site| site["kind"] == "unsupported_cpp_expression"));
        assert_eq!(
            report["ambiguous_makefile_lines"][0]["kind"],
            "ambiguous_makefile_line"
        );
    }
}
