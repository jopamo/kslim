use super::*;

#[test]
fn test_reducer_run_fails_closed_on_unsupported_cpp_expression_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("test.c"),
        "#if defined(CONFIG_REMOVED) + defined(CONFIG_LIVE)\nint maybe_kept;\n#endif\n",
    )
    .unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec![],
        remove_configs: vec!["REMOVED".to_string()],
        set_defaults: BTreeMap::new(),
        unsafe_allow_root_path_removal: false,
    });

    let err = run(root.to_str().unwrap(), &profile)
        .unwrap_err()
        .to_string();
    assert!(err.contains("unsupported preprocessor expressions"));
    assert!(err.contains("test.c:1"));
    assert!(err.contains("if"));
    assert!(err.contains("defined(CONFIG_REMOVED) + defined(CONFIG_LIVE)"));
    assert_eq!(
        std::fs::read_to_string(root.join("test.c")).unwrap(),
        "#if defined(CONFIG_REMOVED) + defined(CONFIG_LIVE)\nint maybe_kept;\n#endif\n"
    );
}
#[test]
fn test_reducer_run_can_report_unsupported_cpp_expression_without_failing() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("test.c"),
        concat!(
            "#ifdef CONFIG_REMOVED\n",
            "int removed;\n",
            "#else\n",
            "int kept;\n",
            "#endif\n",
            "#if defined(CONFIG_REMOVED) + defined(CONFIG_LIVE)\n",
            "int maybe_kept;\n",
            "#endif\n",
        ),
    )
    .unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec![],
        remove_configs: vec!["REMOVED".to_string()],
        set_defaults: BTreeMap::new(),
        unsafe_allow_root_path_removal: false,
    });
    profile.reducer.report_unsupported_expressions = false;

    let stats = run(root.to_str().unwrap(), &profile).unwrap();

    assert!(stats.ran);
    assert_eq!(stats.unsupported_cpp_expressions.len(), 1);
    let site = &stats.unsupported_cpp_expressions[0];
    assert_eq!(site.file, PathBuf::from("test.c"));
    assert_eq!(site.line, 6);
    assert_eq!(site.directive, "if");
    assert_eq!(
        site.expression,
        "defined(CONFIG_REMOVED) + defined(CONFIG_LIVE)"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("test.c")).unwrap(),
        "int kept;\n#if defined(CONFIG_REMOVED) + defined(CONFIG_LIVE)\nint maybe_kept;\n#endif\n"
    );
}
#[test]
fn test_reducer_run_ignores_unsupported_cpp_expression_in_dead_nested_branch() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("test.c"),
        concat!(
            "#ifdef CONFIG_REMOVED\n",
            "#if defined(CONFIG_OTHER_REMOVED) + defined(CONFIG_LIVE)\n",
            "int dead_unsupported;\n",
            "#endif\n",
            "#else\n",
            "#ifdef CONFIG_OTHER_REMOVED\n",
            "int dead_inner;\n",
            "#else\n",
            "int kept;\n",
            "#endif\n",
            "#endif\n",
        ),
    )
    .unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec![],
        remove_configs: vec!["REMOVED".to_string(), "OTHER_REMOVED".to_string()],
        set_defaults: BTreeMap::new(),
        unsafe_allow_root_path_removal: false,
    });

    let stats = run(root.to_str().unwrap(), &profile).unwrap();

    assert!(stats.ran);
    assert!(stats.unsupported_cpp_expressions.is_empty());
    assert!(stats.skipped_cpp_nested_edge_cases.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("test.c")).unwrap(),
        "int kept;\n"
    );
}
#[test]
fn test_reducer_run_reports_skipped_unknown_nested_edge_case_without_failing() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let original = concat!(
        "#if defined(CONFIG_LIVE)\n",
        "#ifdef CONFIG_REMOVED\n",
        "int removed;\n",
        "#else\n",
        "int kept;\n",
        "#endif\n",
        "#endif\n",
    );
    std::fs::write(root.join("test.c"), original).unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec![],
        remove_configs: vec!["REMOVED".to_string()],
        set_defaults: BTreeMap::new(),
        unsafe_allow_root_path_removal: false,
    });

    let stats = run(root.to_str().unwrap(), &profile).unwrap();

    assert!(stats.ran);
    assert!(stats.unsupported_cpp_expressions.is_empty());
    assert_eq!(stats.cpp_report.skipped_nested_edge_cases, 1);
    assert_eq!(stats.skipped_cpp_nested_edge_cases.len(), 1);
    assert_eq!(
        stats.skipped_cpp_nested_edge_cases[0].file,
        PathBuf::from("test.c")
    );
    assert_eq!(stats.skipped_cpp_nested_edge_cases[0].line, 1);
    assert_eq!(
        stats.skipped_cpp_nested_edge_cases[0].reason,
        "unknown enclosing condition prevents folding nested branches"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("test.c")).unwrap(),
        original
    );
}
#[test]
fn test_reducer_run_cpp_folding_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("test.c"),
        concat!(
            "#ifdef CONFIG_REMOVED\n",
            "int removed_outer;\n",
            "#else\n",
            "#ifdef CONFIG_OTHER_REMOVED\n",
            "int removed_inner;\n",
            "#else\n",
            "int kept;\n",
            "#endif\n",
            "#endif\n",
        ),
    )
    .unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec![],
        remove_configs: vec!["REMOVED".to_string(), "OTHER_REMOVED".to_string()],
        set_defaults: BTreeMap::new(),
        unsafe_allow_root_path_removal: false,
    });

    let first = run(root.to_str().unwrap(), &profile).unwrap();
    let after_first = std::fs::read_to_string(root.join("test.c")).unwrap();
    let second = run(root.to_str().unwrap(), &profile).unwrap();

    assert!(first.ran);
    assert_eq!(after_first, "int kept;\n");
    assert_eq!(
        std::fs::read_to_string(root.join("test.c")).unwrap(),
        after_first
    );
    assert_eq!(first.cpp_report.branches_folded, 2);
    assert_eq!(first.cpp_report.files_touched, 1);
    assert_eq!(
        first
            .edits
            .iter()
            .filter(|edit| edit.pass_name == "cpp.fold_removed_config_branches")
            .count(),
        2
    );
    assert!(second.ran);
    assert_eq!(second.cpp_report.branches_folded, 0);
    assert_eq!(second.cpp_report.files_touched, 0);
    assert!(second.unsupported_cpp_expressions.is_empty());
    assert!(second
        .edits
        .iter()
        .all(|edit| edit.pass_name != "cpp.fold_removed_config_branches"));
}
#[test]
fn test_reducer_run_rewrites_removed_private_header_include() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/gpu/drm/amd/amdgpu")).unwrap();
    std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
    std::fs::create_dir_all(root.join("include/linux")).unwrap();
    std::fs::write(
        root.join("drivers/gpu/drm/amd/amdgpu/internal.h"),
        "#define AMDGPU_PRIVATE 1\n",
    )
    .unwrap();
    std::fs::write(
        root.join("include/linux/drm_public.h"),
        "#define DRM_PUBLIC 1\n",
    )
    .unwrap();
    std::fs::write(
        root.join("drivers/gpu/drm/helper.c"),
        concat!(
            "#include <amd/amdgpu/internal.h>\n",
            "#include <linux/drm_public.h>\n",
            "int drm_helper;\n",
        ),
    )
    .unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec!["drivers/gpu/drm/amd/amdgpu".to_string()],
        ..SlimConfig::default()
    });

    let stats = run(root.to_str().unwrap(), &profile).unwrap();

    assert!(stats.ran);
    assert_eq!(stats.include_report.removed_include_lines, 1);
    assert_eq!(stats.include_report.public_headers_preserved, 1);
    assert!(stats.edits.iter().any(|edit| matches!(
        edit.reason,
        crate::edit_reason::EditReason::RemovedHeader { ref header }
            if header == "amd/amdgpu/internal.h"
    )));
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/gpu/drm/helper.c")).unwrap(),
        "#include <linux/drm_public.h>\nint drm_helper;\n",
    );
}
#[test]
fn test_reducer_run_reports_missing_public_header_include() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("Documentation")).unwrap();
    std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
    std::fs::write(root.join("Documentation/unused.txt"), "unused\n").unwrap();
    std::fs::write(
        root.join("drivers/gpu/drm/helper.c"),
        "#include <linux/missing.h>\nint drm_helper;\n",
    )
    .unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec!["Documentation/unused.txt".to_string()],
        ..SlimConfig::default()
    });

    let stats = run(root.to_str().unwrap(), &profile).unwrap();

    assert!(stats.ran);
    assert_eq!(stats.include_report.live_missing_includes, 1);
    assert_eq!(stats.manual_include_sites.len(), 1);
    assert_eq!(
        stats.manual_include_sites[0].kind,
        crate::includes::ManualIncludeHandlingKind::LiveMissingInclude
    );
    assert_eq!(stats.manual_include_sites[0].site.header, "linux/missing.h");
    assert_eq!(stats.include_report.removed_include_lines, 0);
    assert!(stats
        .edits
        .iter()
        .all(|edit| edit.pass_name != "includes.rewrite_removed_headers"));
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/gpu/drm/helper.c")).unwrap(),
        "#include <linux/missing.h>\nint drm_helper;\n",
    );
}
#[test]
fn test_reducer_run_reports_ambiguous_include() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("Documentation")).unwrap();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::create_dir_all(root.join("include")).unwrap();
    std::fs::write(root.join("Documentation/unused.txt"), "unused\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/shared.h"),
        "#define LOCAL_SHARED 1\n",
    )
    .unwrap();
    std::fs::write(root.join("include/shared.h"), "#define ROOT_SHARED 1\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/test.c"),
        "#include \"shared.h\"\nint test;\n",
    )
    .unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec!["Documentation/unused.txt".to_string()],
        ..SlimConfig::default()
    });

    let stats = run(root.to_str().unwrap(), &profile).unwrap();

    assert!(stats.ran);
    assert_eq!(stats.include_report.ambiguous_includes, 1);
    assert_eq!(stats.manual_include_sites.len(), 1);
    assert_eq!(
        stats.manual_include_sites[0].kind,
        crate::includes::ManualIncludeHandlingKind::AmbiguousInclude
    );
    assert_eq!(stats.manual_include_sites[0].site.header, "shared.h");
    assert_eq!(stats.manual_include_sites[0].classified_targets.len(), 2);
    assert_eq!(stats.include_report.removed_include_lines, 0);
    assert!(stats
        .edits
        .iter()
        .all(|edit| edit.pass_name != "includes.rewrite_removed_headers"));
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/test.c")).unwrap(),
        "#include \"shared.h\"\nint test;\n",
    );
}
#[test]
fn test_reducer_run_ignores_include_inside_folded_dead_branch() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/gpu/drm/amd/amdgpu")).unwrap();
    std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
    std::fs::write(
        root.join("drivers/gpu/drm/amd/amdgpu/internal.h"),
        "#define AMDGPU_PRIVATE 1\n",
    )
    .unwrap();
    std::fs::write(
        root.join("drivers/gpu/drm/helper.c"),
        concat!(
            "#ifdef CONFIG_REMOVED\n",
            "#include <amd/amdgpu/internal.h>\n",
            "int dead;\n",
            "#else\n",
            "int live;\n",
            "#endif\n",
        ),
    )
    .unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec!["drivers/gpu/drm/amd/amdgpu".to_string()],
        remove_configs: vec!["REMOVED".to_string()],
        set_defaults: BTreeMap::new(),
        unsafe_allow_root_path_removal: false,
    });

    let stats = run(root.to_str().unwrap(), &profile).unwrap();

    assert!(stats.ran);
    assert_eq!(stats.cpp_report.branches_folded, 1);
    assert_eq!(stats.include_report.removed_include_lines, 0);
    assert_eq!(stats.include_report.live_missing_includes, 0);
    assert_eq!(stats.include_report.public_headers_preserved, 0);
    assert_eq!(stats.include_report.ambiguous_includes, 0);
    assert!(stats
        .edits
        .iter()
        .all(|edit| edit.pass_name != "includes.rewrite_removed_headers"));
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/gpu/drm/helper.c")).unwrap(),
        "int live;\n",
    );
}
#[test]
fn test_reducer_run_removes_include_inside_proven_dead_nested_branch() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(
        root.join("drivers/foo/helper.c"),
        concat!(
            "#if defined(CONFIG_LIVE)\n",
            "#ifdef CONFIG_REMOVED\n",
            "#include <linux/dead_missing.h>\n",
            "#endif\n",
            "#endif\n",
            "int helper;\n",
        ),
    )
    .unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: Vec::new(),
        remove_configs: vec!["REMOVED".to_string()],
        set_defaults: BTreeMap::new(),
        unsafe_allow_root_path_removal: false,
    });

    let stats = run(root.to_str().unwrap(), &profile).unwrap();

    assert!(stats.ran);
    assert_eq!(stats.cpp_report.branches_folded, 0);
    assert_eq!(stats.cpp_report.skipped_nested_edge_cases, 1);
    assert_eq!(stats.include_report.removed_include_lines, 1);
    assert_eq!(stats.include_report.live_missing_includes, 0);
    assert!(stats.edits.iter().any(|edit| matches!(
        edit.reason,
        crate::edit_reason::EditReason::RemovedDeadBranchInclude {
            ref header,
            ref symbol,
        } if header == "linux/dead_missing.h" && symbol == "REMOVED"
    )));
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/helper.c")).unwrap(),
        concat!(
            "#if defined(CONFIG_LIVE)\n",
            "#ifdef CONFIG_REMOVED\n",
            "#endif\n",
            "#endif\n",
            "int helper;\n",
        ),
    );
}
#[test]
fn test_reducer_run_rewrites_file_relative_quoted_private_header_include() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo/include")).unwrap();
    std::fs::create_dir_all(root.join("drivers/foo/sub")).unwrap();
    std::fs::write(
        root.join("drivers/foo/include/private.h"),
        "#define PRIVATE 1\n",
    )
    .unwrap();
    std::fs::write(
        root.join("drivers/foo/sub/helper.c"),
        "#include \"../include/private.h\"\nint helper;\n",
    )
    .unwrap();

    let mut profile = default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec!["drivers/foo/include/private.h".to_string()],
        ..SlimConfig::default()
    });

    let stats = run(root.to_str().unwrap(), &profile).unwrap();

    assert!(stats.ran);
    assert_eq!(stats.include_report.removed_include_lines, 1);
    assert!(stats.edits.iter().any(|edit| matches!(
        edit.reason,
        crate::edit_reason::EditReason::RemovedHeader { ref header }
            if header == "../include/private.h"
    )));
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/sub/helper.c")).unwrap(),
        "int helper;\n",
    );
}
