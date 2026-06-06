use super::*;

#[test]
fn test_reducer_result_serializes_stable_public_shape() {
    let result = ReducerResult::default();

    let value = serde_json::to_value(&result).unwrap();
    let object = value.as_object().unwrap();

    assert_eq!(value["status"], "success");
    assert_eq!(value["publishable"], true);
    assert_eq!(value["final_build_status"], "not_run");
    assert_eq!(value["convergence"], "converged");
    assert!(object.contains_key("passes"));
    assert!(object.contains_key("edit_summary"));
    assert!(object.contains_key("diagnostic_summary"));
    assert!(object.contains_key("touched_files"));
    assert!(object.contains_key("skipped_sites"));
    assert!(object.contains_key("fixups_applied"));

    for internal_field in [
        "manifest",
        "initial_index",
        "declared_prune",
        "post_prune_index",
        "post_kconfig_index",
        "post_kbuild_index",
        "post_cpp_index",
        "post_include_index",
        "stats",
    ] {
        assert!(
            !object.contains_key(internal_field),
            "serialized ReducerResult must not expose internal artifact field {internal_field}"
        );
    }

    assert_eq!(
        serde_json::to_value(ReducerStatus::FailedUnsupportedSyntax).unwrap(),
        "failed_unsupported_syntax"
    );
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
fn test_reducer_result_marks_unknown_class_diagnostic_unpublishable() {
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
            classified_diagnostics: vec![ClassifiedDiagnostic::UndeclaredIdentifier {
                source_file: PathBuf::from("drivers/gpu/drm/helper.c"),
                line: 7,
                symbol: String::from("amdgpu_magic"),
                build_target: Some(String::from("modules")),
                arch: None,
                config: Some(String::from("defconfig")),
            }],
            ..ReducerStats::default()
        },
    );

    assert_eq!(result.status, ReducerStatus::FailedUnknownDiagnostic);
    assert!(!result.publishable);
    assert_eq!(result.diagnostic_summary.unknown_diagnostics, 1);
}
#[test]
fn test_reducer_result_serialization_is_deterministic_for_unordered_inputs() {
    let edit_z = manifest_path_edit("z/removed.c");
    let edit_a = manifest_path_edit("a/removed.c");
    let fixup_z = AppliedFixup {
        fixer_name: "z.fixup",
        diagnostic: missing_header_diagnostic("z/live.c", 9, "z.h"),
        edits: vec![edit_z.clone()],
        proof_sources: vec![FixupProof::ManifestPath {
            path: PathBuf::from("z/removed.c"),
        }],
    };
    let fixup_a = AppliedFixup {
        fixer_name: "a.fixup",
        diagnostic: missing_header_diagnostic("a/live.c", 1, "a.h"),
        edits: vec![edit_a.clone()],
        proof_sources: vec![FixupProof::ManifestPath {
            path: PathBuf::from("a/removed.c"),
        }],
    };
    let skipped_z = SkippedFixup {
        fixer_name: Some("z.skip"),
        diagnostic: missing_header_diagnostic("z/skipped.c", 7, "z.h"),
        reason: String::from("z reason"),
    };
    let skipped_a = SkippedFixup {
        fixer_name: Some("a.skip"),
        diagnostic: missing_header_diagnostic("a/skipped.c", 2, "a.h"),
        reason: String::from("a reason"),
    };
    let mut first = ReducerStats {
        ran: true,
        files_removed: 2,
        dirs_removed: 2,
        removal: RemovalAccounting {
            removed_files: vec![PathBuf::from("z/removed.c"), PathBuf::from("a/removed.c")],
            removed_dirs: vec![
                PathBuf::from("z/removed-dir"),
                PathBuf::from("a/removed-dir"),
            ],
            ..RemovalAccounting::default()
        },
        edits: vec![edit_z, edit_a],
        unsupported_kconfig_expressions: vec![
            crate::kconfig::UnsupportedKconfigExpression {
                file: PathBuf::from("z/Kconfig"),
                line: 9,
                directive: String::from("if"),
                expression: String::from("Z"),
                reason: String::from("z"),
            },
            crate::kconfig::UnsupportedKconfigExpression {
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
                file: PathBuf::from("z-nested.c"),
                line: 8,
                reason: String::from("z"),
            },
            crate::cpp::SkippedCppNestedEdgeCase {
                file: PathBuf::from("a-nested.c"),
                line: 3,
                reason: String::from("a"),
            },
        ],
        skipped_makefile_lines: vec![
            crate::kbuild::KbuildSkippedLine {
                file: PathBuf::from("z/Makefile"),
                line: 4,
                assignment_lhs: String::from("z-y"),
                reason: String::from("z"),
            },
            crate::kbuild::KbuildSkippedLine {
                file: PathBuf::from("a/Makefile"),
                line: 2,
                assignment_lhs: String::from("a-y"),
                reason: String::from("a"),
            },
        ],
        applied_fixups: vec![fixup_z, fixup_a],
        skipped_fixups: vec![skipped_z, skipped_a],
        ..ReducerStats::default()
    };
    let second = first.clone();
    first.removal.removed_files.reverse();
    first.removal.removed_dirs.reverse();
    first.edits.reverse();
    first.unsupported_kconfig_expressions.reverse();
    first.unsupported_cpp_expressions.reverse();
    first.skipped_cpp_nested_edge_cases.reverse();
    first.skipped_makefile_lines.reverse();
    first.applied_fixups.reverse();
    first.skipped_fixups.reverse();

    let first_json = serde_json::to_string(&ReducerResult::from_pipeline_artifacts(
        None, None, None, None, None, None, None, None, first,
    ))
    .unwrap();
    let second_json = serde_json::to_string(&ReducerResult::from_pipeline_artifacts(
        None, None, None, None, None, None, None, None, second,
    ))
    .unwrap();

    assert_eq!(first_json, second_json);

    let value: serde_json::Value = serde_json::from_str(&first_json).unwrap();
    assert_eq!(
        value["touched_files"],
        serde_json::json!([
            "a/removed-dir",
            "a/removed.c",
            "z/removed-dir",
            "z/removed.c"
        ])
    );
    let skipped_site_keys = value["skipped_sites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|site| {
            format!(
                "{}:{}:{}",
                site["kind"].as_str().unwrap(),
                site["file"].as_str().unwrap_or(""),
                site["line"].as_u64().unwrap_or(0)
            )
        })
        .collect::<Vec<_>>();
    let mut sorted_skipped_site_keys = skipped_site_keys.clone();
    sorted_skipped_site_keys.sort();
    assert_eq!(skipped_site_keys, sorted_skipped_site_keys);
    assert_eq!(value["fixups_applied"][0]["fixer_name"], "a.fixup");
    assert_eq!(value["fixups_applied"][1]["fixer_name"], "z.fixup");
}
#[test]
fn test_reducer_result_committed_serialization_redacts_host_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let host_path = tmp.path().join("candidate-tree/source.c");
    let host_path_string = host_path.to_string_lossy().to_string();
    let redaction = crate::reducer::result::REDUCER_RESULT_HOST_PATH_REDACTION;
    let mut host_index = TreeIndex::default();
    host_index.files.insert(host_path.clone());
    let host_edit = EditRecord::new(
        host_path.clone(),
        None,
        String::from("before\n"),
        String::new(),
        EditReason::ManifestPath {
            path: host_path.clone(),
        },
        EditProofSource::removal_manifest_path(host_path.clone()),
        "test.reducer_result",
    );
    let result = ReducerResult::from_pipeline_artifacts(
        None,
        Some(host_index),
        None,
        None,
        None,
        None,
        None,
        None,
        ReducerStats {
            ran: true,
            removal: RemovalAccounting {
                removed_files: vec![host_path.clone()],
                ..RemovalAccounting::default()
            },
            edits: vec![host_edit],
            skipped_makefile_lines: vec![crate::kbuild::KbuildSkippedLine {
                file: host_path.clone(),
                line: 3,
                assignment_lhs: String::from("obj-y"),
                reason: format!("candidate={host_path_string}"),
            }],
            skipped_fixups: vec![SkippedFixup {
                fixer_name: Some("test.fixup"),
                diagnostic: missing_header_diagnostic(&host_path_string, 3, "source.h"),
                reason: format!("log=file://{host_path_string}"),
            }],
            ..ReducerStats::default()
        },
    );

    assert_eq!(result.touched_files, vec![PathBuf::from(redaction)]);
    assert!(result
        .skipped_sites
        .iter()
        .all(|site| site.file.as_deref() != Some(host_path.as_path())));
    assert!(result
        .skipped_sites
        .iter()
        .any(|site| site.reason == redaction));

    let serialized = serde_json::to_string(&result).unwrap();
    assert!(!serialized.contains(&host_path_string));
    assert!(serialized.contains(redaction));

    let directly_constructed = ReducerResult {
        passes: vec![ReducerPassReport {
            name: String::from("test.pass"),
            changed: true,
            touched_files: vec![host_path.clone()],
            edit_count: 1,
            diagnostic_count: 1,
            skipped_site_count: 1,
        }],
        touched_files: vec![host_path.clone()],
        skipped_sites: vec![SkippedSite {
            kind: String::from("skipped_fixup"),
            file: Some(host_path.clone()),
            line: Some(3),
            reason: format!("candidate={host_path_string}"),
        }],
        ..ReducerResult::default()
    };
    let direct_json = serde_json::to_string(&directly_constructed).unwrap();
    assert!(!direct_json.contains(&host_path_string));
    assert!(direct_json.contains(redaction));
}
