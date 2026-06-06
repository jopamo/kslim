use super::*;

#[test]
fn test_strict_reducer_edit_provenance_rejects_unreasoned_edit() {
    let mut stats = ReducerStats::default();
    stats.ran = true;
    stats.edits.push(EditRecord::new(
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
    ));

    let err = validate_reducer_edit_provenance(&stats, &ReducerConfig::default())
        .unwrap_err()
        .to_string();

    assert!(err.contains("unreasoned EditReason"));
}
#[test]
fn test_non_strict_reducer_edit_provenance_accepts_structural_unreasoned_edit() {
    let mut stats = ReducerStats::default();
    stats.ran = true;
    stats.edits.push(EditRecord::new(
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
    ));
    let mut config = ReducerConfig::default();
    config.reject_unreasoned_edits = false;

    validate_reducer_edit_provenance(&stats, &config).unwrap();
}
#[test]
fn test_reducer_edit_provenance_rejects_unreasoned_edit_without_unrelated_strict_flags() {
    let mut stats = ReducerStats::default();
    stats.ran = true;
    stats.edits.push(EditRecord::new(
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
    ));
    let mut config = ReducerConfig::default();
    config.report_unsupported_expressions = false;
    assert!(!config.strict_mode());

    let err = validate_reducer_edit_provenance(&stats, &config)
        .unwrap_err()
        .to_string();

    assert!(err.contains("unreasoned EditReason"));
}
#[test]
fn test_strict_reducer_edit_provenance_rejects_competing_proof_sources() {
    let mut edit = EditRecord::new(
        PathBuf::from("Kconfig"),
        Some(LineRange { start: 1, end: 1 }),
        String::from("before\n"),
        String::from("after\n"),
        EditReason::ManifestConfig {
            symbol: String::from("FOO"),
        },
        EditProofSource::removal_manifest_config(String::from("FOO")),
        "test.competing_proofs",
    );
    edit.proof_source = EditProofSource::stale_kbuild_reference(String::from("foo.o"));

    let mut stats = ReducerStats::default();
    stats.ran = true;
    stats.edits.push(edit);

    let err = validate_reducer_edit_provenance(&stats, &ReducerConfig::default())
        .unwrap_err()
        .to_string();

    assert!(err.contains("multiple competing proof sources"));
}
#[test]
fn test_strict_reducer_edit_provenance_rejects_speculative_fallout_edit() {
    let mut stats = ReducerStats::default();
    stats.ran = true;
    stats.edits.push(EditRecord::new(
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
    ));

    let err = validate_reducer_edit_provenance(&stats, &ReducerConfig::default())
        .unwrap_err()
        .to_string();

    assert!(err.contains("broad speculative fallout edit"));

    let mut relaxed = ReducerConfig::default();
    relaxed.reject_speculative_fallout_edits = false;
    validate_reducer_edit_provenance(&stats, &relaxed).unwrap();
}
#[test]
fn test_apply_selftest_fixup_removes_proven_missing_header_include() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/gpu/drm/amd/amdgpu")).unwrap();
    std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
    std::fs::write(root.join("drivers/gpu/drm/amd/amdgpu/.keep"), "").unwrap();
    std::fs::write(
        root.join("drivers/gpu/drm/helper.c"),
        "#include <amd/amdgpu/amdgpu_missing.h>\nint helper;\n",
    )
    .unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec!["drivers/gpu/drm/amd/amdgpu".to_string()],
        ..SlimConfig::default()
    });

    let mut stats = run(root.to_str().unwrap(), &profile).unwrap();
    let applied = apply_selftest_fixup(
        root.to_str().unwrap(),
        &profile,
        &mut stats,
        &SelfTestFailure::KernelBuild {
            label: "build".to_string(),
            output_dir: PathBuf::from("out"),
            details: CapturedCommandFailure {
                command: "make".to_string(),
                target: Some("modules".to_string()),
                arch: None,
                config: Some("defconfig".to_string()),
                stdout: String::new(),
                stderr: String::from(
                    "drivers/gpu/drm/helper.c:1:10: fatal error: amd/amdgpu/amdgpu_missing.h: No such file or directory\n",
                ),
                exit_status: Some(2),
                elapsed: Duration::ZERO,
            },
        },
    )
    .unwrap();

    assert!(applied);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/gpu/drm/helper.c")).unwrap(),
        "int helper;\n"
    );
    assert!(stats
        .edits
        .iter()
        .any(|edit| edit.pass_name == "fixups.remove_missing_header_include"));
    assert_eq!(stats.applied_fixups.len(), 1);
    assert_eq!(
        stats.applied_fixups[0].fixer_name,
        "fixups.remove_missing_header_include"
    );
}
#[test]
fn test_apply_selftest_fixup_does_not_mutate_on_undefined_symbol_fallout() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
    std::fs::write(
        root.join("drivers/gpu/drm/helper.c"),
        "int helper(void) { return amdgpu_magic(); }\n",
    )
    .unwrap();

    let profile = default_profile_config("v1.0");
    let mut stats = ReducerStats {
        ran: true,
        ..ReducerStats::default()
    };

    let applied = apply_selftest_fixup(
        root.to_str().unwrap(),
        &profile,
        &mut stats,
        &SelfTestFailure::KernelBuild {
            label: "build".to_string(),
            output_dir: PathBuf::from("out"),
            details: CapturedCommandFailure {
                command: "make".to_string(),
                target: Some("modules".to_string()),
                arch: None,
                config: Some("defconfig".to_string()),
                stdout: String::new(),
                stderr: String::from(
                    "drivers/gpu/drm/helper.c:(.text+0x10): undefined reference to `amdgpu_magic'\n",
                ),
                exit_status: Some(2),
                elapsed: Duration::ZERO,
            },
        },
    )
    .unwrap();

    assert!(!applied);
    assert!(stats.edits.is_empty());
    assert!(stats.applied_fixups.is_empty());
    assert_eq!(stats.skipped_fixups.len(), 1);
    assert!(matches!(
        stats.skipped_fixups[0].diagnostic,
        ClassifiedDiagnostic::UndefinedReference { .. }
    ));
    assert!(stats.skipped_fixups[0]
        .reason
        .contains("broad speculative edits are forbidden"));
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/gpu/drm/helper.c")).unwrap(),
        "int helper(void) { return amdgpu_magic(); }\n",
    );
}
#[test]
fn test_apply_selftest_fixup_removes_proven_stale_kbuild_directory_ref() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        "subdir-y += remove/ keep/\n",
    )
    .unwrap();

    let profile = default_profile_config("v1.0");
    let mut stats = ReducerStats {
        ran: true,
        removal: RemovalAccounting {
            removed_files: Vec::new(),
            removed_dirs: vec![PathBuf::from("drivers/foo/remove")],
            removed_config_symbols: Vec::new(),
            empty_parents_cleaned: Vec::new(),
            missing_paths: Vec::new(),
        },
        ..ReducerStats::default()
    };

    let applied = apply_selftest_fixup(
        root.to_str().unwrap(),
        &profile,
        &mut stats,
        &SelfTestFailure::Command {
            details: CapturedCommandFailure {
                command: "make".to_string(),
                target: Some("modules".to_string()),
                arch: Some("arm64".to_string()),
                config: Some("defconfig".to_string()),
                stdout: String::new(),
                stderr: String::from(
                    "make[1]: *** No rule to make target 'drivers/foo/remove/', needed by 'drivers/foo/'. Stop.\n",
                ),
                exit_status: Some(2),
                elapsed: Duration::ZERO,
            },
        },
    )
    .unwrap();

    assert!(applied);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        "subdir-y += keep/\n"
    );
    assert!(stats
        .edits
        .iter()
        .any(|edit| edit.pass_name == "fixups.remove_stale_kbuild_directory_ref"));
    assert_eq!(stats.applied_fixups.len(), 1);
    assert_eq!(
        stats.applied_fixups[0].fixer_name,
        "fixups.remove_stale_kbuild_directory_ref"
    );
}
#[test]
fn test_apply_selftest_fixup_removes_proven_stale_kbuild_object_ref() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(
        root.join("drivers/foo/Makefile"),
        "obj-y += remove.o keep.o\n",
    )
    .unwrap();

    let profile = default_profile_config("v1.0");
    let mut stats = ReducerStats {
        ran: true,
        removal: RemovalAccounting {
            removed_files: vec![PathBuf::from("drivers/foo/remove.c")],
            removed_dirs: Vec::new(),
            removed_config_symbols: Vec::new(),
            empty_parents_cleaned: Vec::new(),
            missing_paths: Vec::new(),
        },
        ..ReducerStats::default()
    };

    let applied = apply_selftest_fixup(
        root.to_str().unwrap(),
        &profile,
        &mut stats,
        &SelfTestFailure::Command {
            details: CapturedCommandFailure {
                command: "make".to_string(),
                target: Some("modules".to_string()),
                arch: Some("arm64".to_string()),
                config: Some("defconfig".to_string()),
                stdout: String::new(),
                stderr: String::from(
                    "make[1]: *** No rule to make target 'drivers/foo/remove.o', needed by 'drivers/foo/built-in.a'. Stop.\n",
                ),
                exit_status: Some(2),
                elapsed: Duration::ZERO,
            },
        },
    )
    .unwrap();

    assert!(applied);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Makefile")).unwrap(),
        "obj-y += keep.o\n"
    );
    assert!(stats
        .edits
        .iter()
        .any(|edit| edit.pass_name == "fixups.remove_stale_kbuild_object_ref"));
    assert_eq!(stats.applied_fixups.len(), 1);
    assert_eq!(
        stats.applied_fixups[0].fixer_name,
        "fixups.remove_stale_kbuild_object_ref"
    );
}
#[test]
fn test_apply_selftest_fixup_removes_proven_missing_kconfig_source() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(root.join("Kconfig"), "source \"drivers/foo/Kconfig\"\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/test.c"),
        "#ifdef CONFIG_REMOVED\nint dead;\n#endif\nint live;\n",
    )
    .unwrap();

    let profile = default_profile_config("v1.0");
    let mut stats = ReducerStats {
        ran: true,
        removal: RemovalAccounting {
            removed_files: vec![PathBuf::from("drivers/foo/Kconfig")],
            removed_dirs: Vec::new(),
            removed_config_symbols: vec![String::from("REMOVED")],
            empty_parents_cleaned: Vec::new(),
            missing_paths: Vec::new(),
        },
        ..ReducerStats::default()
    };

    let applied = apply_selftest_fixup(
        root.to_str().unwrap(),
        &profile,
        &mut stats,
        &SelfTestFailure::BuiltIn {
            check: "kconfig-sources",
            message: format!(
                "selftest failed: {}:1 references missing Kconfig source 'drivers/foo/Kconfig'",
                root.join("Kconfig").display()
            ),
        },
    )
    .unwrap();

    assert!(applied);
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        "# kslim: removed source \"drivers/foo/Kconfig\"\n"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/test.c")).unwrap(),
        "int live;\n"
    );
    assert!(stats
        .edits
        .iter()
        .any(|edit| edit.pass_name == "fixups.remove_missing_kconfig_source"));
    assert!(stats
        .edits
        .iter()
        .any(|edit| edit.pass_name == "cpp.fold_removed_config_branches"));
    assert_eq!(stats.applied_fixups.len(), 1);
    assert_eq!(
        stats.applied_fixups[0].fixer_name,
        "fixups.remove_missing_kconfig_source"
    );
}
#[test]
fn test_apply_selftest_fixup_stops_on_unknown_diagnostic_without_editing() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/gpu/drm/amd/amdgpu")).unwrap();
    std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
    std::fs::write(root.join("drivers/gpu/drm/amd/amdgpu/.keep"), "").unwrap();
    std::fs::write(root.join("drivers/gpu/drm/helper.c"), "int helper;\n").unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec!["drivers/gpu/drm/amd/amdgpu".to_string()],
        ..SlimConfig::default()
    });
    profile.reducer.fail_on_unknown_diagnostics = false;

    let mut stats = run(root.to_str().unwrap(), &profile).unwrap();
    let edit_count_before = stats.edits.len();
    let applied = apply_selftest_fixup(
        root.to_str().unwrap(),
        &profile,
        &mut stats,
        &SelfTestFailure::Command {
            details: CapturedCommandFailure {
                command: "make".to_string(),
                target: Some("modules".to_string()),
                arch: Some("arm64".to_string()),
                config: Some("defconfig".to_string()),
                stdout: String::new(),
                stderr: String::from("totally unrecognized failure output\n"),
                exit_status: Some(2),
                elapsed: Duration::ZERO,
            },
        },
    )
    .unwrap();

    assert!(!applied);
    assert_eq!(stats.skipped_fixups.len(), 1);
    assert_eq!(stats.skipped_fixups[0].reason, "unknown diagnostic");
    assert_eq!(stats.edits.len(), edit_count_before);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/gpu/drm/helper.c")).unwrap(),
        "int helper;\n"
    );
}
