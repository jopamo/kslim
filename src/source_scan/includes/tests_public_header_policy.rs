use super::*;

#[test]
fn test_classify_include_targets_marks_public_preserved_headers() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("include/linux")).unwrap();
    std::fs::write(root.join("include/linux/module.h"), "#define MODULE 1\n").unwrap();

    let classified = classify_include_targets(
        root,
        &[ResolvedIncludeTarget {
            path: PathBuf::from("include/linux/module.h"),
            rule: IncludeResolveRule::IncludeRoot,
        }],
    );

    assert_eq!(
        classified,
        vec![ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("include/linux/module.h"),
                rule: IncludeResolveRule::IncludeRoot,
            },
            classification: IncludeTargetClassification::PublicPreservedHeader,
        }]
    );
}

#[test]
fn test_classify_include_targets_preserves_live_public_header_under_broad_manifest_path() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("include/linux")).unwrap();
    std::fs::write(root.join("include/linux/module.h"), "#define MODULE 1\n").unwrap();
    let removal_proofs = removal_proofs_with_removed_headers(&["include"], &[]);

    let classified = classify_include_targets_with_removal_proofs(
        root,
        &[ResolvedIncludeTarget {
            path: PathBuf::from("include/linux/module.h"),
            rule: IncludeResolveRule::IncludeRoot,
        }],
        Some(&removal_proofs),
    );

    assert_eq!(
        classified,
        vec![ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("include/linux/module.h"),
                rule: IncludeResolveRule::IncludeRoot,
            },
            classification: IncludeTargetClassification::PublicPreservedHeader,
        }]
    );
}

#[test]
fn test_classify_include_targets_marks_include_generated_headers() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("include/generated")).unwrap();
    std::fs::write(
        root.join("include/generated/autoconf.h"),
        "#define CONFIG_X 1\n",
    )
    .unwrap();

    let classified = classify_include_targets(
        root,
        &[ResolvedIncludeTarget {
            path: PathBuf::from("include/generated/autoconf.h"),
            rule: IncludeResolveRule::IncludeRoot,
        }],
    );

    assert_eq!(
        classified,
        vec![ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("include/generated/autoconf.h"),
                rule: IncludeResolveRule::IncludeRoot,
            },
            classification: IncludeTargetClassification::GeneratedHeader,
        }]
    );
}

#[test]
fn test_classify_include_targets_marks_configured_generated_headers() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("build/include/generated/linux")).unwrap();
    std::fs::write(
        root.join("build/include/generated/linux/version.h"),
        "#define LINUX_VERSION 1\n",
    )
    .unwrap();

    let classified = classify_include_targets(
        root,
        &[ResolvedIncludeTarget {
            path: PathBuf::from("build/include/generated/linux/version.h"),
            rule: IncludeResolveRule::ConfiguredGeneratedRoot,
        }],
    );

    assert_eq!(
        classified,
        vec![ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("build/include/generated/linux/version.h"),
                rule: IncludeResolveRule::ConfiguredGeneratedRoot,
            },
            classification: IncludeTargetClassification::GeneratedHeader,
        }]
    );
}

#[test]
fn test_rewrite_removed_header_includes_preserves_public_header_under_broad_manifest_path() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
    std::fs::create_dir_all(root.join("include/linux")).unwrap();
    std::fs::write(
        root.join("include/linux/drm_public.h"),
        "#define DRM_PUBLIC 1\n",
    )
    .unwrap();
    std::fs::write(
        root.join("drivers/gpu/drm/helper.c"),
        "#include <linux/drm_public.h>\nint drm_helper;\n",
    )
    .unwrap();
    let removal_proofs = removal_proofs_with_removed_paths(&["include"]);

    let report = rewrite_removed_header_includes_report(root, &removal_proofs).unwrap();
    apply_include_rewrite_report(root, &report).unwrap();

    assert_eq!(report.counts.public_headers_preserved, 1);
    assert_eq!(report.counts.removed_include_lines, 0);
    assert!(report.edits.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/gpu/drm/helper.c")).unwrap(),
        "#include <linux/drm_public.h>\nint drm_helper;\n",
    );
}

#[test]
fn test_rewrite_removed_header_includes_preserves_explicit_public_header_without_abi_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(
        root.join("drivers/foo/helper.c"),
        "#include <linux/removed_public.h>\nint helper;\n",
    )
    .unwrap();
    let removal_proofs = removal_proofs_with_removed_paths(&["include/linux/removed_public.h"]);
    let removed_header_paths = vec![PathBuf::from("include/linux/removed_public.h")];

    let report = rewrite_removed_header_includes_report(
        root,
        &removal_proofs.with_removed_header_paths(&removed_header_paths),
    )
    .unwrap();
    apply_include_rewrite_report(root, &report).unwrap();

    assert_eq!(report.counts.removed_include_lines, 0);
    assert_eq!(report.counts.live_missing_includes, 1);
    assert!(report.edits.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/helper.c")).unwrap(),
        "#include <linux/removed_public.h>\nint helper;\n",
    );
}

#[test]
fn test_rewrite_removed_header_includes_removes_explicitly_removed_public_header_with_abi_policy(
) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(
        root.join("drivers/foo/helper.c"),
        "#include <linux/removed_public.h>\nint helper;\n",
    )
    .unwrap();
    let removal_proofs = removal_proofs_with_removed_headers_and_abi_policy(
        &["include/linux/removed_public.h"],
        &[PathBuf::from("include/linux/removed_public.h")],
        &allow_public_header_removal(),
    );

    let report = rewrite_removed_header_includes_report(root, &removal_proofs).unwrap();
    apply_include_rewrite_report(root, &report).unwrap();

    assert_eq!(report.counts.removed_include_lines, 1);
    assert_eq!(report.counts.live_missing_includes, 0);
    assert!(matches!(
        report.edits.as_slice(),
        [EditRecord {
            reason: EditReason::RemovedHeader { header },
            proof_source:
                EditProofSource::RemovalManifest {
                    key: crate::edit_reason::RemovalKey::Header {
                        header: proof_header,
                        path,
                    },
                },
            ..
        }] if header == "linux/removed_public.h"
            && proof_header == "linux/removed_public.h"
            && path == Path::new("include/linux/removed_public.h")
    ));
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/helper.c")).unwrap(),
        "int helper;\n",
    );
}

#[test]
fn test_rewrite_removed_header_includes_preserves_public_header_removed_by_broad_manifest_dir()
{
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(
        root.join("drivers/foo/helper.c"),
        "#include <linux/public_from_dir.h>\nint helper;\n",
    )
    .unwrap();
    let removal_proofs = removal_proofs_with_removed_paths(&["include/linux"]);
    let removed_header_paths = vec![PathBuf::from("include/linux/public_from_dir.h")];

    let report = rewrite_removed_header_includes_report(
        root,
        &removal_proofs.with_removed_header_paths(&removed_header_paths),
    )
    .unwrap();
    apply_include_rewrite_report(root, &report).unwrap();

    assert_eq!(report.counts.removed_include_lines, 0);
    assert_eq!(report.counts.live_missing_includes, 1);
    assert!(report.edits.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/helper.c")).unwrap(),
        "#include <linux/public_from_dir.h>\nint helper;\n",
    );
}

#[test]
fn test_rewrite_removed_header_includes_preserves_uapi_header_removed_by_broad_manifest_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(
        root.join("drivers/foo/helper.c"),
        "#include <uapi/linux/abi.h>\nint helper;\n",
    )
    .unwrap();
    let removal_proofs = removal_proofs_with_removed_paths(&["include/uapi"]);
    let removed_header_paths = vec![PathBuf::from("include/uapi/linux/abi.h")];

    let report = rewrite_removed_header_includes_report(
        root,
        &removal_proofs.with_removed_header_paths(&removed_header_paths),
    )
    .unwrap();
    apply_include_rewrite_report(root, &report).unwrap();

    assert_eq!(report.counts.removed_include_lines, 0);
    assert_eq!(report.counts.live_missing_includes, 1);
    assert!(report.edits.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/helper.c")).unwrap(),
        "#include <uapi/linux/abi.h>\nint helper;\n",
    );
}

#[test]
fn test_rewrite_removed_header_includes_preserves_explicit_uapi_header_without_uapi_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(
        root.join("drivers/foo/helper.c"),
        "#include <uapi/linux/removed_abi.h>\nint helper;\n",
    )
    .unwrap();
    let removal_proofs = removal_proofs_with_removed_headers_and_abi_policy(
        &["include/uapi/linux/removed_abi.h"],
        &[PathBuf::from("include/uapi/linux/removed_abi.h")],
        &allow_public_header_removal(),
    );

    let report = rewrite_removed_header_includes_report(root, &removal_proofs).unwrap();
    apply_include_rewrite_report(root, &report).unwrap();

    assert_eq!(report.counts.removed_include_lines, 0);
    assert_eq!(report.counts.live_missing_includes, 1);
    assert!(report.edits.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/helper.c")).unwrap(),
        "#include <uapi/linux/removed_abi.h>\nint helper;\n",
    );
}

#[test]
fn test_rewrite_removed_header_includes_removes_explicit_uapi_header_with_uapi_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(
        root.join("drivers/foo/helper.c"),
        "#include <uapi/linux/removed_abi.h>\nint helper;\n",
    )
    .unwrap();
    let removal_proofs = removal_proofs_with_removed_headers_and_abi_policy(
        &["include/uapi/linux/removed_abi.h"],
        &[PathBuf::from("include/uapi/linux/removed_abi.h")],
        &allow_uapi_header_removal(),
    );

    let report = rewrite_removed_header_includes_report(root, &removal_proofs).unwrap();
    apply_include_rewrite_report(root, &report).unwrap();

    assert_eq!(report.counts.removed_include_lines, 1);
    assert_eq!(report.counts.live_missing_includes, 0);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/helper.c")).unwrap(),
        "int helper;\n",
    );
}

#[test]
fn test_rewrite_removed_header_includes_reports_missing_public_header() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
    std::fs::write(
        root.join("drivers/gpu/drm/helper.c"),
        "#include <linux/missing.h>\nint drm_helper;\n",
    )
    .unwrap();
    let removal_proofs = removal_proofs_with_removed_paths(&[]);

    let report = rewrite_removed_header_includes_report(root, &removal_proofs).unwrap();
    apply_include_rewrite_report(root, &report).unwrap();

    assert_eq!(report.counts.live_missing_includes, 1);
    assert_eq!(report.counts.public_headers_preserved, 0);
    assert_eq!(report.counts.removed_include_lines, 0);
    assert!(report.edits.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/gpu/drm/helper.c")).unwrap(),
        "#include <linux/missing.h>\nint drm_helper;\n",
    );
}

#[test]
fn test_rewrite_removed_header_includes_does_not_report_dead_public_header_site() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(
        root.join("drivers/foo/helper.c"),
        concat!(
            "#ifdef CONFIG_REMOVED\n",
            "#include <linux/public_from_dir.h>\n",
            "#endif\n",
            "int helper;\n",
        ),
    )
    .unwrap();
    let removal_proofs = removal_proofs_with_removed_paths(&["include/linux"]);
    let removed_header_paths = vec![PathBuf::from("include/linux/public_from_dir.h")];

    let report = rewrite_removed_header_includes_report_with_removed_configs(
        root,
        &removal_proofs.with_removed_header_paths(&removed_header_paths),
        &[String::from("REMOVED")],
    )
    .unwrap();
    apply_include_rewrite_report(root, &report).unwrap();

    assert_eq!(report.counts.removed_include_lines, 1);
    assert_eq!(report.counts.live_missing_includes, 0);
    assert!(report.manual_sites.is_empty());
    assert!(matches!(
        report.edits.as_slice(),
        [EditRecord {
            reason:
                EditReason::RemovedDeadBranchInclude {
                    header,
                    symbol,
                },
            ..
        }] if header == "linux/public_from_dir.h" && symbol == "REMOVED"
    ));
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/helper.c")).unwrap(),
        "#ifdef CONFIG_REMOVED\n#endif\nint helper;\n",
    );
}
