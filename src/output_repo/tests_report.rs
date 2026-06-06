use super::*;

#[test]
fn test_reducer_artifact_path_uses_worktree_metadata_dir_without_git_repo() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");

    let path = reducer_artifact_path(&output, REDUCER_REMOVAL_MANIFEST).unwrap();

    assert_eq!(path, output.join(".kslim").join(REDUCER_REMOVAL_MANIFEST));
}

#[test]
fn test_reducer_artifact_path_uses_git_metadata_dir_for_repo_output() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");
    std::fs::create_dir_all(output.join(".git")).unwrap();

    let path = reducer_artifact_path(&output, REDUCER_REPORT_JSON).unwrap();

    assert_eq!(path, output.join(".git/kslim").join(REDUCER_REPORT_JSON));
}

#[test]
fn test_write_reducer_artifact_creates_metadata_file() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");
    std::fs::create_dir_all(output.join(".git")).unwrap();

    write_reducer_artifact(
        output.to_str().unwrap(),
        REDUCER_REPORT_JSON,
        "{\"status\":\"ok\"}\n",
    )
    .unwrap();

    assert_eq!(
        std::fs::read_to_string(output.join(".git/kslim").join(REDUCER_REPORT_JSON)).unwrap(),
        "{\"status\":\"ok\"}\n"
    );
}

#[test]
fn test_write_failure_report_stores_stage_enum_stable_name() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");
    let config = crate::config::default_kslim_config("demo", output.to_str().unwrap());
    let profile = crate::config::default_profile_config("v1.0");

    for stage in GenerateStage::ALL {
        let report_path = tmp.path().join(format!("failure-{}.txt", stage.as_str()));
        write_failure_report(
            report_path.as_path(),
            &config,
            &profile,
            None,
            Some("slimmed"),
            None,
            stage,
            "failed",
            None,
            None,
            None,
        )
        .unwrap();

        let report = std::fs::read_to_string(report_path).unwrap();
        assert!(
            report.contains(&format!("Stage: {}\n", stage.as_str())),
            "failure report should store stable GenerateStage name for {stage}"
        );
    }
}

#[test]
fn test_render_reducer_diagnostics_json_sorts_diagnostics_by_stable_keys() {
    let stats = ReducerStats {
        unsupported_kconfig_expressions: vec![
            UnsupportedKconfigExpression {
                file: PathBuf::from("z/Kconfig"),
                line: 9,
                directive: String::from("if"),
                expression: String::from("Z"),
                reason: String::from("z"),
            },
            UnsupportedKconfigExpression {
                file: PathBuf::from("a/Kconfig"),
                line: 1,
                directive: String::from("if"),
                expression: String::from("A"),
                reason: String::from("a"),
            },
        ],
        unsupported_cpp_expressions: vec![
            crate::cpp::UnsupportedCppExpression {
                file: PathBuf::from("z.c"),
                line: 5,
                directive: String::from("if"),
                expression: String::from("Z"),
                reason: String::from("z"),
            },
            crate::cpp::UnsupportedCppExpression {
                file: PathBuf::from("a.c"),
                line: 1,
                directive: String::from("if"),
                expression: String::from("A"),
                reason: String::from("a"),
            },
        ],
        skipped_cpp_nested_edge_cases: vec![
            crate::cpp::SkippedCppNestedEdgeCase {
                file: PathBuf::from("nested-z.c"),
                line: 4,
                reason: String::from("z"),
            },
            crate::cpp::SkippedCppNestedEdgeCase {
                file: PathBuf::from("nested-a.c"),
                line: 2,
                reason: String::from("a"),
            },
        ],
        skipped_makefile_lines: vec![
            KbuildSkippedLine {
                file: PathBuf::from("z/Makefile"),
                line: 4,
                assignment_lhs: String::from("z"),
                reason: String::from("z"),
            },
            KbuildSkippedLine {
                file: PathBuf::from("a/Makefile"),
                line: 2,
                assignment_lhs: String::from("a"),
                reason: String::from("a"),
            },
        ],
        skipped_fixups: vec![
            SkippedFixup {
                fixer_name: None,
                diagnostic: ClassifiedDiagnostic::Unknown,
                reason: String::from("z"),
            },
            SkippedFixup {
                fixer_name: Some("fixups.remove_missing_header_include"),
                diagnostic: ClassifiedDiagnostic::MissingHeader {
                    source_file: PathBuf::from("a.c"),
                    line: 1,
                    header: String::from("a.h"),
                    build_target: None,
                    arch: None,
                    config: None,
                },
                reason: String::from("a"),
            },
        ],
        ..ReducerStats::default()
    };

    let json = crate::reducer::render_reducer_diagnostics_json(&stats);

    fn index_of(haystack: &str, needle: &str) -> usize {
        haystack
            .find(needle)
            .unwrap_or_else(|| panic!("missing {needle:?} in rendered diagnostics"))
    }

    assert!(
        index_of(&json, "\"file\": \"a/Kconfig\"") < index_of(&json, "\"file\": \"z/Kconfig\"")
    );
    assert!(index_of(&json, "\"file\": \"a.c\"") < index_of(&json, "\"file\": \"z.c\""));
    assert!(
        index_of(&json, "\"file\": \"nested-a.c\"")
            < index_of(&json, "\"file\": \"nested-z.c\"")
    );
    assert!(
        index_of(&json, "\"file\": \"a/Makefile\"")
            < index_of(&json, "\"file\": \"z/Makefile\"")
    );
    assert!(
        index_of(&json, "\"class\":\"MissingHeader\"")
            < index_of(&json, "\"class\":\"Unknown\"")
    );
}

#[test]
fn test_render_edit_records_json_sorts_records_by_stable_keys() {
    fn remove_path(path: &str) -> EditRecord {
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

    fn stale_kbuild_edit(
        path: &str,
        line: usize,
        after: &str,
        pass_name: &'static str,
        reference: &str,
    ) -> EditRecord {
        EditRecord::new(
            PathBuf::from(path),
            Some(LineRange {
                start: line,
                end: line,
            }),
            format!("{reference}\n"),
            after.to_string(),
            EditReason::RemovedKbuildRef {
                reference: reference.to_string(),
            },
            EditProofSource::stale_kbuild_reference(reference.to_string()),
            pass_name,
        )
    }

    let edits = vec![
        stale_kbuild_edit(
            "a.c",
            1,
            "",
            "cpp.fold_removed_config_branches",
            "CONFIG_REMOVED",
        ),
        stale_kbuild_edit(
            "drivers/Makefile",
            9,
            "# kslim: removed stale make refs from obj-y\n",
            "prune.rewrite_makefiles",
            "late.o",
        ),
        remove_path("z/removed.c"),
        stale_kbuild_edit(
            "drivers/Makefile",
            2,
            "# kslim: removed stale make refs from obj-y\n",
            "prune.rewrite_makefiles",
            "rewrite.o",
        ),
        stale_kbuild_edit(
            "drivers/Makefile",
            2,
            "",
            "prune.rewrite_makefiles",
            "remove.o",
        ),
        remove_path("a/removed.c"),
        remove_path("a/removed.c"),
    ];

    let json = crate::reducer::render_edit_records_json(&edits);

    fn index_of(haystack: &str, needle: &str) -> usize {
        haystack
            .find(needle)
            .unwrap_or_else(|| panic!("missing {needle:?} in rendered edit records"))
    }

    assert!(
        index_of(&json, "\"file\": \"a/removed.c\"")
            < index_of(&json, "\"file\": \"z/removed.c\"")
    );
    assert_eq!(json.matches("\"file\": \"a/removed.c\"").count(), 1);
    assert!(
        index_of(&json, "\"file\": \"z/removed.c\"")
            < index_of(&json, "\"pass_name\": \"prune.rewrite_makefiles\"")
    );
    assert!(
        index_of(&json, "\"logical_item\": \"remove.o\\n\"")
            < index_of(&json, "\"logical_item\": \"rewrite.o\\n\"")
    );
    assert!(
        index_of(&json, "\"logical_item\": \"rewrite.o\\n\"")
            < index_of(&json, "\"logical_item\": \"late.o\\n\"")
    );
    assert!(
        index_of(&json, "\"logical_item\": \"late.o\\n\"")
            < index_of(&json, "\"pass_name\": \"cpp.fold_removed_config_branches\"")
    );
}

#[test]
fn test_write_reducer_metadata_writes_report_and_summary_when_reducer_ran() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");
    std::fs::create_dir_all(output.join(".git")).unwrap();

    write_reducer_metadata(
        output.to_str().unwrap(),
        Some(&ReducerStats {
            ran: true,
            files_removed: 1,
            dirs_removed: 2,
            configs_disabled: 3,
            defaults_overridden: 4,
            kconfig_refs_removed: 5,
            makefile_refs_removed: 6,
            kconfig_report: KconfigReportCounts {
                dropped_selects: 7,
                dropped_implies: 8,
                simplified_depends: 9,
                simplified_visible_if: 10,
                simplified_defaults: 11,
                removed_sources: 12,
                removed_empty_menus: 13,
                skipped_expressions: 1,
            },
            kconfig_solver_report: KconfigSolverReport {
                files_analyzed: 1,
                removed_symbols: vec![String::from("REMOVED")],
                default_reenabled_symbols: vec![
                    KconfigSolverDefaultReenabledSymbol {
                        symbol: String::from("REMOVED"),
                        value: String::from("y"),
                    },
                ],
                ..KconfigSolverReport::default()
            },
            cpp_report: crate::cpp::CppReportCounts {
                branches_folded: 13,
                files_touched: 2,
                skipped_nested_edge_cases: 1,
            },
            include_report: crate::includes::IncludeReportCounts {
                removed_include_lines: 14,
                live_missing_includes: 15,
                public_headers_preserved: 16,
                ambiguous_includes: 17,
            },
            unsupported_kconfig_expressions: vec![UnsupportedKconfigExpression {
                file: "Kconfig".into(),
                line: 7,
                directive: String::from("if"),
                expression: String::from("REMOVED = y"),
                reason: String::from("expression syntax referencing removed symbols is not supported"),
            }],
            unsupported_cpp_expressions: vec![crate::cpp::UnsupportedCppExpression {
                file: PathBuf::from("drivers/foo/test.c"),
                line: 8,
                directive: String::from("if"),
                expression: String::from(
                    "defined(CONFIG_REMOVED) || defined(CONFIG_LIVE)",
                ),
                reason: String::from(
                    "preprocessor expression syntax referencing removed symbols is not supported",
                ),
            }],
            skipped_cpp_nested_edge_cases: vec![crate::cpp::SkippedCppNestedEdgeCase {
                file: PathBuf::from("drivers/foo/test.c"),
                line: 12,
                reason: String::from(
                    "unknown enclosing condition prevents folding nested branches",
                ),
            }],
            skipped_makefile_lines: vec![KbuildSkippedLine {
                file: PathBuf::from("drivers/foo/Makefile"),
                line: 6,
                assignment_lhs: String::from("ccflags-y"),
                reason: String::from(
                    "ambiguous include path flag '-Iinclude' resolves to both removed and live paths",
                ),
            }],
            removal: RemovalAccounting {
                removed_config_symbols: vec![String::from("REMOVED")],
                ..RemovalAccounting::default()
            },
            edits: vec![
                EditRecord::new(
                    PathBuf::from("drivers/foo/Makefile"),
                    Some(LineRange { start: 1, end: 1 }),
                    String::from("obj-y += remove/\n"),
                    String::from("# kslim: removed stale make refs from obj-y\n"),
                    EditReason::RemovedKbuildRef {
                        reference: String::from("remove/"),
                    },
                    EditProofSource::stale_kbuild_reference(String::from("remove/")),
                    "prune.rewrite_makefiles",
                ),
                EditRecord::new(
                    PathBuf::from("drivers/foo/Makefile"),
                    Some(LineRange { start: 2, end: 2 }),
                    String::from("obj-y += remove.o\n"),
                    String::from("# kslim: removed stale make refs from obj-y\n"),
                    EditReason::RemovedKbuildRef {
                        reference: String::from("helper.o"),
                    },
                    EditProofSource::stale_kbuild_reference(String::from("helper.o")),
                    "prune.rewrite_makefiles",
                ),
                EditRecord::new(
                    PathBuf::from("drivers/foo/Makefile"),
                    Some(LineRange { start: 3, end: 3 }),
                    String::from("foo-y += helper.o\n"),
                    String::from("# kslim: removed stale make refs from foo-y\n"),
                    EditReason::RemovedKbuildRef {
                        reference: String::from("helper.o"),
                    },
                    EditProofSource::stale_kbuild_reference(String::from("helper.o")),
                    "prune.rewrite_makefiles",
                ),
                EditRecord::new(
                    PathBuf::from("drivers/foo/Makefile"),
                    Some(LineRange { start: 4, end: 4 }),
                    String::from("obj-y += foo.o\n"),
                    String::from("# kslim: removed stale make refs from obj-y\n"),
                    EditReason::RemovedKbuildRef {
                        reference: String::from("foo.o"),
                    },
                    EditProofSource::stale_kbuild_reference(String::from("foo.o")),
                    "prune.rewrite_makefiles",
                ),
                EditRecord::new(
                    PathBuf::from("drivers/foo/Makefile"),
                    Some(LineRange { start: 5, end: 5 }),
                    String::from("ccflags-y += -Idrivers/foo/include\n"),
                    String::from("# kslim: removed stale make refs from ccflags-y\n"),
                    EditReason::RemovedKbuildRef {
                        reference: String::from("-Idrivers/foo/include"),
                    },
                    EditProofSource::stale_kbuild_reference(String::from(
                        "-Idrivers/foo/include",
                    )),
                    "prune.rewrite_makefiles",
                ),
            ],
            applied_fixups: vec![AppliedFixup {
                fixer_name: "fixups.remove_missing_header_include",
                diagnostic: ClassifiedDiagnostic::MissingHeader {
                    source_file: PathBuf::from("drivers/foo/test.c"),
                    line: 9,
                    header: String::from("missing/header.h"),
                    build_target: Some(String::from("modules")),
                    arch: Some(String::from("arm64")),
                    config: Some(String::from("defconfig")),
                },
                edits: vec![EditRecord::new(
                    PathBuf::from("drivers/foo/test.c"),
                    Some(LineRange { start: 9, end: 9 }),
                    String::from("#include <missing/header.h>\n"),
                    String::new(),
                    EditReason::BuildDiagnostic {
                        class: crate::edit_reason::DiagnosticClass::MissingHeader,
                    },
                    EditProofSource::ClassifiedDiagnostic {
                        diagnostic_id: crate::edit_reason::DiagnosticClass::MissingHeader.into(),
                    },
                    "fixups.remove_missing_header_include",
                )],
                proof_sources: vec![
                    FixupProof::ManifestPath {
                        path: PathBuf::from("drivers/foo/missing/header.h"),
                    },
                    FixupProof::TreeIndexIncludeSite {
                        file: PathBuf::from("drivers/foo/test.c"),
                        line: 9,
                        target: String::from("missing/header.h"),
                    },
                ],
            }],
            skipped_fixups: vec![SkippedFixup {
                fixer_name: None,
                diagnostic: ClassifiedDiagnostic::Unknown,
                reason: String::from("unknown diagnostic"),
            }],
            classified_diagnostics: Vec::new(),
            raw_diagnostic_excerpts: Vec::new(),
            manual_include_sites: Vec::new(),
        }),
    )
    .unwrap();

    let report =
        std::fs::read_to_string(output.join(".git/kslim").join(REDUCER_REPORT_MD)).unwrap();
    assert!(report.contains("# kslim reducer report"));
    assert!(report.contains("- Files removed: 1"));
    assert!(report.contains("- Unsupported Kconfig expressions: 1"));
    assert!(report.contains("- Dropped selects: 7"));
    assert!(report.contains("- Dropped implies: 8"));
    assert!(report.contains("- Simplified depends: 9"));
    assert!(report.contains("- Simplified defaults: 11"));
    assert!(report.contains("- Removed sources: 12"));
    assert!(report.contains("- Removed empty menus: 13"));
    assert!(report.contains("- Skipped expressions: 1"));
    assert!(report.contains("- Removed directory refs: 1"));
    assert!(report.contains("- Removed object refs: 2"));
    assert!(report.contains("- Cleaned composite objects: 1"));
    assert!(report.contains("- Removed stale include paths: 1"));
    assert!(report.contains("- Skipped ambiguous Makefile lines: 1"));
    assert!(report.contains("- Branches folded: 13"));
    assert!(report.contains("- Files touched: 2"));
    assert!(report.contains("- Unsupported preprocessor forms: 1"));
    assert!(report.contains("- Skipped nested edge cases: 1"));
    assert!(report.contains("- Removed include lines: 14"));
    assert!(report.contains("- Live missing includes: 15"));
    assert!(report.contains("- Public headers preserved: 16"));
    assert!(report.contains("- Ambiguous includes: 17"));
    assert!(report.contains("## Deterministic fixups"));
    assert!(report.contains("- Applied fixups: 1"));
    assert!(report.contains("- Skipped diagnostics: 1"));
    assert!(report.contains("fixups.remove_missing_header_include"));
    assert!(report.contains("proof: manifest path drivers/foo/missing/header.h"));

    let report_json =
        std::fs::read_to_string(output.join(".git/kslim").join(REDUCER_REPORT_JSON)).unwrap();
    assert!(report_json.contains("\"files_removed\": 1"));
    assert!(report_json.contains("\"present\": true"));
    assert!(report_json.contains("\"unsupported_cpp_expressions\": 1"));
    assert!(report_json.contains("\"diagnostics_json\": \"diagnostics.json\""));
    assert!(report_json.contains(
        "\"kconfig_solver_report_json\": \"kconfig-solver-report.json\""
    ));
    assert!(report_json.contains(
        "\"kconfig_rewrite_report_json\": \"kconfig-rewrite-report.json\""
    ));

    let solver_report = std::fs::read_to_string(
        output
            .join(".git/kslim")
            .join(REDUCER_KCONFIG_SOLVER_REPORT_JSON),
    )
    .unwrap();
    assert!(solver_report.contains("\"files_analyzed\": 1"));
    assert!(solver_report.contains("\"removed_symbols\": [\"REMOVED\"]"));
    assert!(solver_report.contains("\"default_reenabled_symbols\""));

    let rewrite_report = std::fs::read_to_string(
        output
            .join(".git/kslim")
            .join(REDUCER_KCONFIG_REWRITE_REPORT_JSON),
    )
    .unwrap();
    assert!(rewrite_report.contains("\"removed_symbols\": [\"REMOVED\"]"));
    assert!(rewrite_report.contains("\"dropped_selects\": 7"));

    let summary =
        std::fs::read_to_string(output.join(".git/kslim").join(REDUCER_EDIT_SUMMARY_JSON))
            .unwrap();
    assert!(summary.contains("\"files_removed\": 1"));
    assert!(summary.contains("\"makefile_refs_removed\": 6"));
    assert!(summary.contains("\"unsupported_kconfig_expressions\": 1"));
    assert!(summary.contains("\"dropped_selects\": 7"));
    assert!(summary.contains("\"dropped_implies\": 8"));
    assert!(summary.contains("\"simplified_depends\": 9"));
    assert!(summary.contains("\"simplified_defaults\": 11"));
    assert!(summary.contains("\"removed_sources\": 12"));
    assert!(summary.contains("\"removed_empty_menus\": 13"));
    assert!(summary.contains("\"skipped_expressions\": 1"));
    assert!(summary.contains("\"removed_directory_refs\": 1"));
    assert!(summary.contains("\"removed_object_refs\": 2"));
    assert!(summary.contains("\"cleaned_composite_objects\": 1"));
    assert!(summary.contains("\"removed_stale_include_paths\": 1"));
    assert!(summary.contains("\"skipped_ambiguous_makefile_lines\": 1"));
    assert!(summary.contains("\"branches_folded\": 13"));
    assert!(summary.contains("\"files_touched\": 2"));
    assert!(summary.contains("\"unsupported_preprocessor_forms\": 1"));
    assert!(summary.contains("\"skipped_nested_edge_cases\": 1"));
    assert!(summary.contains("\"removed_include_lines\": 14"));
    assert!(summary.contains("\"live_missing_includes\": 15"));
    assert!(summary.contains("\"public_headers_preserved\": 16"));
    assert!(summary.contains("\"ambiguous_includes\": 17"));
    assert!(summary.contains("\"fixer_name\": \"fixups.remove_missing_header_include\""));
    assert!(summary.contains("\"proof_sources\""));
    assert!(summary.contains("\"edit_record_details\""));
    assert!(summary.contains("\"edit_kind\": \"rewrite_line\""));
    assert!(summary.contains("\"edit_reason\": {\"kind\":\"removed_kbuild_ref\""));
    assert!(summary.contains("\"proof_source\": {\"kind\":\"stale_reference\""));
    assert!(summary.contains("\"logical_item\": \"obj-y += remove/\\n\""));
    assert!(summary.contains("\"idempotence_marker\""));
    assert!(summary.contains("\"fixups_skipped\""));
    assert!(summary.contains("\"reason\": \"unknown diagnostic\""));

    let diagnostics =
        std::fs::read_to_string(output.join(".git/kslim").join(REDUCER_DIAGNOSTICS_JSON))
            .unwrap();
    assert!(diagnostics.contains("\"unsupported_kconfig_expression\""));
    assert!(diagnostics.contains("\"unsupported_cpp_expression\""));
    assert!(diagnostics.contains("\"skipped_fixup_diagnostic\""));

    let skipped =
        std::fs::read_to_string(output.join(".git/kslim").join(REDUCER_SKIPPED_SITES_JSON))
            .unwrap();
    assert!(skipped.contains("\"unsupported_kconfig_expression\""));
    assert!(skipped.contains("\"directive\": \"if\""));
    assert!(skipped.contains("\"unsupported_cpp_expression\""));
    assert!(
        skipped.contains("\"expression\": \"defined(CONFIG_REMOVED) || defined(CONFIG_LIVE)\"")
    );
    assert!(skipped.contains("\"skipped_cpp_nested_edge_case\""));
    assert!(skipped.contains(
        "\"reason\": \"unknown enclosing condition prevents folding nested branches\""
    ));
    assert!(skipped.contains("\"ambiguous_makefile_line\""));
    assert!(skipped.contains("\"assignment_lhs\": \"ccflags-y\""));
    assert!(skipped.contains("\"skipped_fixup_diagnostic\""));
}

#[test]
fn test_write_reducer_metadata_at_dir_bypasses_git_metadata_selection() {
    let tmp = tempfile::tempdir().unwrap();
    let project_root = tmp.path().join("project");
    std::fs::create_dir_all(project_root.join(".git")).unwrap();

    write_reducer_metadata_at_dir(
        &project_root.join(".kslim"),
        Some(&ReducerStats {
            ran: true,
            ..ReducerStats::default()
        }),
    )
    .unwrap();

    assert!(project_root.join(".kslim").join(REDUCER_REPORT_MD).exists());
    assert!(project_root
        .join(".kslim")
        .join(REDUCER_REPORT_JSON)
        .exists());
    assert!(project_root
        .join(".kslim")
        .join(REDUCER_DIAGNOSTICS_JSON)
        .exists());
    assert!(project_root
        .join(".kslim")
        .join(REDUCER_KCONFIG_SOLVER_REPORT_JSON)
        .exists());
    assert!(project_root
        .join(".kslim")
        .join(REDUCER_KCONFIG_REWRITE_REPORT_JSON)
        .exists());
    assert!(!project_root
        .join(".git/kslim")
        .join(REDUCER_REPORT_MD)
        .exists());
}

#[test]
fn test_write_reducer_metadata_removes_stale_files_when_reducer_did_not_run() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");
    std::fs::create_dir_all(output.join(".git/kslim")).unwrap();
    std::fs::write(output.join(".git/kslim").join(REDUCER_REPORT_MD), "stale\n").unwrap();
    std::fs::write(
        output.join(".git/kslim").join(REDUCER_REPORT_JSON),
        "stale\n",
    )
    .unwrap();
    std::fs::write(
        output.join(".git/kslim").join(REDUCER_DIAGNOSTICS_JSON),
        "stale\n",
    )
    .unwrap();
    std::fs::write(
        output.join(".git/kslim").join(REDUCER_EDIT_SUMMARY_JSON),
        "stale\n",
    )
    .unwrap();
    std::fs::write(
        output
            .join(".git/kslim")
            .join(REDUCER_KCONFIG_SOLVER_REPORT_JSON),
        "stale\n",
    )
    .unwrap();
    std::fs::write(
        output
            .join(".git/kslim")
            .join(REDUCER_KCONFIG_REWRITE_REPORT_JSON),
        "stale\n",
    )
    .unwrap();
    std::fs::write(
        output.join(".git/kslim").join(REDUCER_SKIPPED_SITES_JSON),
        "stale\n",
    )
    .unwrap();

    write_reducer_metadata(output.to_str().unwrap(), None).unwrap();

    assert!(!output.join(".git/kslim").join(REDUCER_REPORT_MD).exists());
    assert!(!output.join(".git/kslim").join(REDUCER_REPORT_JSON).exists());
    assert!(!output
        .join(".git/kslim")
        .join(REDUCER_DIAGNOSTICS_JSON)
        .exists());
    assert!(!output
        .join(".git/kslim")
        .join(REDUCER_EDIT_SUMMARY_JSON)
        .exists());
    assert!(!output
        .join(".git/kslim")
        .join(REDUCER_KCONFIG_SOLVER_REPORT_JSON)
        .exists());
    assert!(!output
        .join(".git/kslim")
        .join(REDUCER_KCONFIG_REWRITE_REPORT_JSON)
        .exists());
    assert!(!output
        .join(".git/kslim")
        .join(REDUCER_SKIPPED_SITES_JSON)
        .exists());
}
