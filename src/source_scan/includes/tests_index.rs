use super::*;

#[test]
fn test_parse_include_site_supports_quoted_and_angle_forms() {
    assert_eq!(
        parse_include_site("#include <linux/module.h>"),
        Some((IncludeKind::Angle, "linux/module.h"))
    );
    assert_eq!(
        parse_include_site("  # include \"local/header.h\" // keep"),
        Some((IncludeKind::Quoted, "local/header.h"))
    );
    assert_eq!(
        parse_include_site("\t#include \"quoted.h\" /* trailing comment */"),
        Some((IncludeKind::Quoted, "quoted.h"))
    );
}

#[test]
fn test_parse_include_site_ignores_non_literal_directives() {
    assert_eq!(parse_include_site("#define include <linux/module.h>"), None);
    assert_eq!(parse_include_site("#include SOME_MACRO"), None);
    assert_eq!(parse_include_site("// #include <linux/module.h>"), None);
    assert_eq!(parse_include_site("#include <linux/module.h> extra"), None);
    assert_eq!(parse_include_site("#include \"\""), None);
}

#[test]
fn test_index_include_sites_collects_quoted_and_angle_sites() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::create_dir_all(root.join("include/linux")).unwrap();
    std::fs::write(
        root.join("drivers/foo/test.c"),
        concat!(
            "#include \"local.h\"\n",
            "#include <linux/module.h>\n",
            "#include SOME_MACRO\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("include/linux/test.h"),
        "# include <linux/other.h>\n",
    )
    .unwrap();
    std::fs::write(root.join("README"), "#include <ignored.h>\n").unwrap();

    let sites = index_include_sites(root).unwrap();

    assert_eq!(
        sites,
        vec![
            IncludeSite {
                file: PathBuf::from("drivers/foo/test.c"),
                line: 1,
                header: String::from("local.h"),
                kind: IncludeKind::Quoted,
            },
            IncludeSite {
                file: PathBuf::from("drivers/foo/test.c"),
                line: 2,
                header: String::from("linux/module.h"),
                kind: IncludeKind::Angle,
            },
            IncludeSite {
                file: PathBuf::from("include/linux/test.h"),
                line: 1,
                header: String::from("linux/other.h"),
                kind: IncludeKind::Angle,
            },
        ]
    );
}

#[test]
fn test_index_include_sites_ignores_commented_and_continued_directives() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(
        root.join("drivers/foo/test.c"),
        concat!(
            "/*\n",
            "#include <linux/commented.h>\n",
            "*/\n",
            "#define TEXT \\\n",
            "#include <linux/stringized.h>\n",
            "#include <linux/live.h>\n",
        ),
    )
    .unwrap();

    let sites = index_include_sites(root).unwrap();

    assert_eq!(
        sites,
        vec![IncludeSite {
            file: PathBuf::from("drivers/foo/test.c"),
            line: 6,
            header: String::from("linux/live.h"),
            kind: IncludeKind::Angle,
        }]
    );
}

#[test]
fn test_resolve_include_targets_uses_local_directory_for_angle_include() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/gpu/drm/amd/amdgpu")).unwrap();
    std::fs::write(
        root.join("drivers/gpu/drm/amd/amdgpu/internal.h"),
        "#define PRIVATE 1\n",
    )
    .unwrap();

    let targets = resolve_include_targets(
        root,
        &IncludeSite {
            file: PathBuf::from("drivers/gpu/drm/helper.c"),
            line: 1,
            header: String::from("amd/amdgpu/internal.h"),
            kind: IncludeKind::Angle,
        },
    );

    assert_eq!(
        targets,
        vec![ResolvedIncludeTarget {
            path: PathBuf::from("drivers/gpu/drm/amd/amdgpu/internal.h"),
            rule: IncludeResolveRule::LocalDirectory,
        }]
    );
}

#[test]
fn test_resolve_include_targets_uses_file_relative_rule_for_quoted_include() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo/shared")).unwrap();
    std::fs::write(
        root.join("drivers/foo/shared/header.h"),
        "#define SHARED 1\n",
    )
    .unwrap();

    let targets = resolve_include_targets(
        root,
        &IncludeSite {
            file: PathBuf::from("drivers/foo/sub/test.c"),
            line: 1,
            header: String::from("../shared/header.h"),
            kind: IncludeKind::Quoted,
        },
    );

    assert_eq!(
        targets,
        vec![ResolvedIncludeTarget {
            path: PathBuf::from("drivers/foo/shared/header.h"),
            rule: IncludeResolveRule::FileRelativeQuoted,
        }]
    );
}

#[test]
fn test_resolve_include_targets_rejects_quoted_path_that_escapes_tree_root() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    let targets = resolve_include_targets(
        root,
        &IncludeSite {
            file: PathBuf::from("drivers/foo/test.c"),
            line: 1,
            header: String::from("../../../outside.h"),
            kind: IncludeKind::Quoted,
        },
    );

    assert!(targets.is_empty());
}

#[test]
fn test_resolve_include_targets_uses_include_root_for_kernel_header() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::create_dir_all(root.join("include/linux")).unwrap();
    std::fs::write(root.join("include/linux/module.h"), "#define MODULE 1\n").unwrap();

    let targets = resolve_include_targets(
        root,
        &IncludeSite {
            file: PathBuf::from("drivers/foo/test.c"),
            line: 1,
            header: String::from("linux/module.h"),
            kind: IncludeKind::Angle,
        },
    );

    assert_eq!(
        targets,
        vec![ResolvedIncludeTarget {
            path: PathBuf::from("include/linux/module.h"),
            rule: IncludeResolveRule::IncludeRoot,
        }]
    );
}

#[test]
fn test_resolve_include_targets_does_not_use_include_root_for_relative_quoted_header() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("include/shared")).unwrap();
    std::fs::write(root.join("include/shared/header.h"), "#define SHARED 1\n").unwrap();

    let targets = resolve_include_targets(
        root,
        &IncludeSite {
            file: PathBuf::from("drivers/foo/test.c"),
            line: 1,
            header: String::from("../shared/header.h"),
            kind: IncludeKind::Quoted,
        },
    );

    assert!(targets.is_empty());
}

#[test]
fn test_resolve_include_targets_uses_arch_include_root_when_source_arch_is_known() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("arch/x86/kernel")).unwrap();
    std::fs::create_dir_all(root.join("arch/x86/include/asm")).unwrap();
    std::fs::write(
        root.join("arch/x86/include/asm/processor.h"),
        "#define X86_PROCESSOR 1\n",
    )
    .unwrap();

    let targets = resolve_include_targets(
        root,
        &IncludeSite {
            file: PathBuf::from("arch/x86/kernel/test.c"),
            line: 1,
            header: String::from("asm/processor.h"),
            kind: IncludeKind::Angle,
        },
    );

    assert_eq!(
        targets,
        vec![ResolvedIncludeTarget {
            path: PathBuf::from("arch/x86/include/asm/processor.h"),
            rule: IncludeResolveRule::ArchIncludeRoot,
        }]
    );
}

#[test]
fn test_resolve_include_targets_does_not_guess_arch_root_for_non_arch_source() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::create_dir_all(root.join("arch/x86/include/asm")).unwrap();
    std::fs::write(
        root.join("arch/x86/include/asm/processor.h"),
        "#define X86_PROCESSOR 1\n",
    )
    .unwrap();

    let targets = resolve_include_targets(
        root,
        &IncludeSite {
            file: PathBuf::from("drivers/foo/test.c"),
            line: 1,
            header: String::from("asm/processor.h"),
            kind: IncludeKind::Angle,
        },
    );

    assert!(targets.is_empty());
}

#[test]
fn test_resolve_include_targets_uses_configured_generated_include_root() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::create_dir_all(root.join("build/include/generated/linux")).unwrap();
    std::fs::write(
        root.join("build/include/generated/linux/version.h"),
        "#define LINUX_VERSION 1\n",
    )
    .unwrap();

    let targets = resolve_include_targets_with_generated_roots(
        root,
        &IncludeSite {
            file: PathBuf::from("drivers/foo/test.c"),
            line: 1,
            header: String::from("linux/version.h"),
            kind: IncludeKind::Angle,
        },
        &[PathBuf::from("build/include/generated")],
    );

    assert_eq!(
        targets,
        vec![ResolvedIncludeTarget {
            path: PathBuf::from("build/include/generated/linux/version.h"),
            rule: IncludeResolveRule::ConfiguredGeneratedRoot,
        }]
    );
}

#[test]
fn test_resolve_include_targets_ignores_escaping_generated_include_root_config() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();

    let targets = resolve_include_targets_with_generated_roots(
        root,
        &IncludeSite {
            file: PathBuf::from("drivers/foo/test.c"),
            line: 1,
            header: String::from("linux/version.h"),
            kind: IncludeKind::Angle,
        },
        &[PathBuf::from("../build/include/generated")],
    );

    assert!(targets.is_empty());
}

#[test]
fn test_classify_include_targets_marks_existing_non_public_targets() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(root.join("drivers/foo/local.h"), "#define LOCAL 1\n").unwrap();

    let classified = classify_include_targets(
        root,
        &[ResolvedIncludeTarget {
            path: PathBuf::from("drivers/foo/local.h"),
            rule: IncludeResolveRule::LocalDirectory,
        }],
    );

    assert_eq!(
        classified,
        vec![ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("drivers/foo/local.h"),
                rule: IncludeResolveRule::LocalDirectory,
            },
            classification: IncludeTargetClassification::Exists,
        }]
    );
}

#[test]
fn test_classify_include_targets_marks_absent_unknown_targets() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    let classified = classify_include_targets(
        root,
        &[ResolvedIncludeTarget {
            path: PathBuf::from("include/linux/missing.h"),
            rule: IncludeResolveRule::IncludeRoot,
        }],
    );

    assert_eq!(
        classified,
        vec![ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("include/linux/missing.h"),
                rule: IncludeResolveRule::IncludeRoot,
            },
            classification: IncludeTargetClassification::AbsentForUnknownReason,
        }]
    );
}
