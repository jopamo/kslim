use super::*;

#[test]
fn test_classify_include_targets_marks_manifest_removed_file() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let removal_proofs = removal_proofs_with_removed_headers(
        &["include/linux/removed.h"],
        &[PathBuf::from("include/linux/removed.h")],
    );

    let classified = classify_include_targets_with_removal_proofs(
        root,
        &[ResolvedIncludeTarget {
            path: PathBuf::from("include/linux/removed.h"),
            rule: IncludeResolveRule::IncludeRoot,
        }],
        Some(&removal_proofs),
    );

    assert_eq!(
        classified,
        vec![ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("include/linux/removed.h"),
                rule: IncludeResolveRule::IncludeRoot,
            },
            classification: IncludeTargetClassification::RemovedByManifest,
        }]
    );
}

#[test]
fn test_classify_include_targets_marks_manifest_removed_directory_descendant() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let removal_proofs = removal_proofs_with_removed_headers(
        &["drivers/gpu/drm/amd/amdgpu"],
        &[PathBuf::from("drivers/gpu/drm/amd/amdgpu/internal.h")],
    );

    let classified = classify_include_targets_with_removal_proofs(
        root,
        &[ResolvedIncludeTarget {
            path: PathBuf::from("drivers/gpu/drm/amd/amdgpu/internal.h"),
            rule: IncludeResolveRule::LocalDirectory,
        }],
        Some(&removal_proofs),
    );

    assert_eq!(
        classified,
        vec![ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("drivers/gpu/drm/amd/amdgpu/internal.h"),
                rule: IncludeResolveRule::LocalDirectory,
            },
            classification: IncludeTargetClassification::RemovedByManifest,
        }]
    );
}

#[test]
fn test_target_is_gone_from_reduced_tree_accepts_removed_and_absent_targets() {
    assert!(target_is_gone_from_reduced_tree(&[
        ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("include/linux/removed.h"),
                rule: IncludeResolveRule::IncludeRoot,
            },
            classification: IncludeTargetClassification::RemovedByManifest,
        },
        ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("include/linux/missing.h"),
                rule: IncludeResolveRule::IncludeRoot,
            },
            classification: IncludeTargetClassification::AbsentForUnknownReason,
        },
    ]));
}

#[test]
fn test_target_is_gone_from_reduced_tree_rejects_any_live_target() {
    assert!(!target_is_gone_from_reduced_tree(&[
        ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("include/linux/removed.h"),
                rule: IncludeResolveRule::IncludeRoot,
            },
            classification: IncludeTargetClassification::RemovedByManifest,
        },
        ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("include/linux/drm_public.h"),
                rule: IncludeResolveRule::IncludeRoot,
            },
            classification: IncludeTargetClassification::PublicPreservedHeader,
        },
    ]));
    assert!(!target_is_gone_from_reduced_tree(&[
        ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("build/include/generated/linux/version.h"),
                rule: IncludeResolveRule::ConfiguredGeneratedRoot,
            },
            classification: IncludeTargetClassification::GeneratedHeader,
        },
    ]));
}

#[test]
fn test_target_is_gone_from_reduced_tree_rejects_empty_classification_set() {
    assert!(!target_is_gone_from_reduced_tree(&[]));
}

#[test]
fn test_target_is_covered_by_removal_manifest_accepts_only_manifest_removed_targets() {
    assert!(target_is_covered_by_removal_manifest(&[
        ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("include/linux/removed.h"),
                rule: IncludeResolveRule::IncludeRoot,
            },
            classification: IncludeTargetClassification::RemovedByManifest,
        },
        ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("drivers/gpu/drm/amd/amdgpu/internal.h"),
                rule: IncludeResolveRule::LocalDirectory,
            },
            classification: IncludeTargetClassification::RemovedByManifest,
        },
    ]));
}

#[test]
fn test_target_is_covered_by_removal_manifest_rejects_non_manifest_targets() {
    assert!(!target_is_covered_by_removal_manifest(&[
        ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("include/linux/removed.h"),
                rule: IncludeResolveRule::IncludeRoot,
            },
            classification: IncludeTargetClassification::RemovedByManifest,
        },
        ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("include/linux/missing.h"),
                rule: IncludeResolveRule::IncludeRoot,
            },
            classification: IncludeTargetClassification::AbsentForUnknownReason,
        },
    ]));
    assert!(!target_is_covered_by_removal_manifest(&[
        ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("include/linux/drm_public.h"),
                rule: IncludeResolveRule::IncludeRoot,
            },
            classification: IncludeTargetClassification::PublicPreservedHeader,
        },
    ]));
}

#[test]
fn test_target_is_covered_by_removal_manifest_rejects_empty_classification_set() {
    assert!(!target_is_covered_by_removal_manifest(&[]));
}

#[test]
fn test_local_removal_rule_applies_to_local_resolution_rules_only() {
    assert!(local_removal_rule_applies(&[
        ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("drivers/foo/internal.h"),
                rule: IncludeResolveRule::LocalDirectory,
            },
            classification: IncludeTargetClassification::RemovedByManifest,
        },
        ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("drivers/foo/../shared/header.h"),
                rule: IncludeResolveRule::FileRelativeQuoted,
            },
            classification: IncludeTargetClassification::RemovedByManifest,
        },
    ]));
    assert!(!local_removal_rule_applies(&[ClassifiedIncludeTarget {
        target: ResolvedIncludeTarget {
            path: PathBuf::from("include/linux/module.h"),
            rule: IncludeResolveRule::IncludeRoot,
        },
        classification: IncludeTargetClassification::RemovedByManifest,
    },]));
    assert!(!local_removal_rule_applies(&[]));
}

#[test]
fn test_include_site_passes_preprocessor_or_local_rule_gate_accepts_dead_site() {
    assert!(include_site_passes_preprocessor_or_local_rule_gate(
        &[ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("include/linux/removed.h"),
                rule: IncludeResolveRule::IncludeRoot,
            },
            classification: IncludeTargetClassification::RemovedByManifest,
        }],
        false,
    ));
}

#[test]
fn test_include_site_passes_preprocessor_or_local_rule_gate_accepts_live_local_rule() {
    assert!(include_site_passes_preprocessor_or_local_rule_gate(
        &[ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("drivers/foo/internal.h"),
                rule: IncludeResolveRule::LocalDirectory,
            },
            classification: IncludeTargetClassification::RemovedByManifest,
        }],
        true,
    ));
}

#[test]
fn test_include_site_passes_preprocessor_or_local_rule_gate_rejects_live_nonlocal_rule() {
    assert!(!include_site_passes_preprocessor_or_local_rule_gate(
        &[ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("include/linux/removed.h"),
                rule: IncludeResolveRule::IncludeRoot,
            },
            classification: IncludeTargetClassification::RemovedByManifest,
        }],
        true,
    ));
    assert!(!include_site_passes_preprocessor_or_local_rule_gate(
        &[],
        false
    ));
}

#[test]
fn test_preserve_subsystem_looking_include_when_resolved_header_exists() {
    let site = IncludeSite {
        file: PathBuf::from("drivers/gpu/drm/helper.c"),
        line: 6,
        header: String::from("amd/amdgpu/internal.h"),
        kind: IncludeKind::Angle,
    };
    let classified_targets = vec![ClassifiedIncludeTarget {
        target: ResolvedIncludeTarget {
            path: PathBuf::from("drivers/gpu/drm/amd/amdgpu/internal.h"),
            rule: IncludeResolveRule::LocalDirectory,
        },
        classification: IncludeTargetClassification::Exists,
    }];

    assert!(
        preserve_subsystem_looking_include_when_resolved_header_exists(
            &site,
            &classified_targets,
        )
    );
    assert_eq!(
        report_live_include_site_needing_manual_handling(&site, &classified_targets, true),
        None
    );
}

#[test]
fn test_preserve_subsystem_looking_include_requires_single_live_local_target() {
    let public_site = IncludeSite {
        file: PathBuf::from("drivers/foo/test.c"),
        line: 7,
        header: String::from("linux/drm_public.h"),
        kind: IncludeKind::Angle,
    };
    assert!(
        !preserve_subsystem_looking_include_when_resolved_header_exists(
            &public_site,
            &[ClassifiedIncludeTarget {
                target: ResolvedIncludeTarget {
                    path: PathBuf::from("include/linux/drm_public.h"),
                    rule: IncludeResolveRule::IncludeRoot,
                },
                classification: IncludeTargetClassification::PublicPreservedHeader,
            }],
        )
    );

    let ambiguous_site = IncludeSite {
        file: PathBuf::from("drivers/foo/test.c"),
        line: 8,
        header: String::from("amd/amdgpu/internal.h"),
        kind: IncludeKind::Angle,
    };
    assert!(
        !preserve_subsystem_looking_include_when_resolved_header_exists(
            &ambiguous_site,
            &[
                ClassifiedIncludeTarget {
                    target: ResolvedIncludeTarget {
                        path: PathBuf::from("drivers/gpu/drm/amd/amdgpu/internal.h"),
                        rule: IncludeResolveRule::LocalDirectory,
                    },
                    classification: IncludeTargetClassification::Exists,
                },
                ClassifiedIncludeTarget {
                    target: ResolvedIncludeTarget {
                        path: PathBuf::from("include/linux/internal.h"),
                        rule: IncludeResolveRule::IncludeRoot,
                    },
                    classification: IncludeTargetClassification::PublicPreservedHeader,
                },
            ],
        )
    );
}

#[test]
fn test_rewrite_removed_header_includes_removes_manifest_removed_private_header() {
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
        concat!(
            "#include <amd/amdgpu/internal.h>\n",
            "#include <linux/drm_public.h>\n",
            "int drm_helper;\n",
        ),
    )
    .unwrap();
    let removal_proofs = removal_proofs_with_removed_paths(&["drivers/gpu/drm/amd/amdgpu"]);
    let removed_header_paths = vec![PathBuf::from("drivers/gpu/drm/amd/amdgpu/internal.h")];

    let report = rewrite_removed_header_includes_report(
        root,
        &removal_proofs.with_removed_header_paths(&removed_header_paths),
    )
    .unwrap();
    apply_include_rewrite_report(root, &report).unwrap();
    let after_first = std::fs::read_to_string(root.join("drivers/gpu/drm/helper.c")).unwrap();
    let second_report = rewrite_removed_header_includes_report(
        root,
        &removal_proofs.with_removed_header_paths(&removed_header_paths),
    )
    .unwrap();
    apply_include_rewrite_report(root, &second_report).unwrap();

    assert_eq!(report.counts.removed_include_lines, 1);
    assert_eq!(report.counts.public_headers_preserved, 1);
    assert_eq!(report.edits.len(), 1);
    assert_eq!(second_report.counts.removed_include_lines, 0);
    assert!(second_report.edits.is_empty());
    assert!(matches!(
        report.edits[0].reason,
        EditReason::RemovedHeader { ref header } if header == "amd/amdgpu/internal.h"
    ));
    assert!(matches!(
        report.edits[0].proof_source,
        EditProofSource::RemovalManifest {
            key: crate::edit_reason::RemovalKey::Header {
                ref header,
                ref path,
            }
        } if header == "amd/amdgpu/internal.h"
            && path == Path::new("drivers/gpu/drm/amd/amdgpu")
    ));
    assert_eq!(
        report.edits[0].pass_name,
        "includes.rewrite_removed_headers"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/gpu/drm/helper.c")).unwrap(),
        after_first,
    );
    assert_eq!(after_first, "#include <linux/drm_public.h>\nint drm_helper;\n");
}

#[test]
fn test_rewrite_removed_header_includes_does_not_remove_unknown_missing_header_under_removed_dir(
) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
    std::fs::write(
        root.join("drivers/gpu/drm/helper.c"),
        "#include <amd/amdgpu/amdgpu_missing.h>\nint drm_helper;\n",
    )
    .unwrap();
    let removal_proofs = removal_proofs_with_removed_paths(&["drivers/gpu/drm/amd/amdgpu"]);

    let report = rewrite_removed_header_includes_report(root, &removal_proofs).unwrap();
    apply_include_rewrite_report(root, &report).unwrap();

    assert_eq!(report.counts.removed_include_lines, 0);
    assert!(report.edits.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/gpu/drm/helper.c")).unwrap(),
        "#include <amd/amdgpu/amdgpu_missing.h>\nint drm_helper;\n",
    );
}

#[test]
fn test_rewrite_removed_header_includes_does_not_remove_live_header_even_with_manifest_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(root.join("drivers/foo/internal.h"), "#define LIVE 1\n").unwrap();
    std::fs::write(
        root.join("drivers/foo/helper.c"),
        "#include \"internal.h\"\nint helper;\n",
    )
    .unwrap();
    let removal_proofs = removal_proofs_with_removed_paths(&["drivers/foo/internal.h"]);
    let removed_header_paths = vec![PathBuf::from("drivers/foo/internal.h")];

    let report = rewrite_removed_header_includes_report(
        root,
        &removal_proofs.with_removed_header_paths(&removed_header_paths),
    )
    .unwrap();
    apply_include_rewrite_report(root, &report).unwrap();

    assert_eq!(report.counts.removed_include_lines, 0);
    assert!(report.edits.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/helper.c")).unwrap(),
        "#include \"internal.h\"\nint helper;\n",
    );
}

#[test]
fn test_rewrite_removed_header_includes_removes_angle_private_header_for_exact_removed_file() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/gpu/drm/amd/amdgpu")).unwrap();
    std::fs::create_dir_all(root.join("drivers/gpu/drm")).unwrap();
    std::fs::write(
        root.join("drivers/gpu/drm/helper.c"),
        "#include <amd/amdgpu/internal.h>\nint drm_helper;\n",
    )
    .unwrap();
    let removal_proofs =
        removal_proofs_with_removed_paths(&["drivers/gpu/drm/amd/amdgpu/internal.h"]);
    let removed_header_paths = vec![PathBuf::from("drivers/gpu/drm/amd/amdgpu/internal.h")];

    let report = rewrite_removed_header_includes_report(
        root,
        &removal_proofs.with_removed_header_paths(&removed_header_paths),
    )
    .unwrap();
    apply_include_rewrite_report(root, &report).unwrap();

    assert_eq!(report.counts.removed_include_lines, 1);
    assert_eq!(report.counts.public_headers_preserved, 0);
    assert_eq!(report.counts.live_missing_includes, 0);
    assert!(matches!(
        report.edits.as_slice(),
        [EditRecord {
            reason: EditReason::RemovedHeader { header },
            ..
        }] if header == "amd/amdgpu/internal.h"
    ));
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/gpu/drm/helper.c")).unwrap(),
        "int drm_helper;\n",
    );
}

#[test]
fn test_rewrite_removed_header_includes_removes_file_relative_quoted_private_header() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo/include")).unwrap();
    std::fs::create_dir_all(root.join("drivers/foo/sub")).unwrap();
    std::fs::write(
        root.join("drivers/foo/sub/helper.c"),
        "#include \"../include/private.h\"\nint helper;\n",
    )
    .unwrap();
    let removal_proofs = removal_proofs_with_removed_paths(&["drivers/foo/include/private.h"]);
    let removed_header_paths = vec![PathBuf::from("drivers/foo/include/private.h")];

    let report = rewrite_removed_header_includes_report(
        root,
        &removal_proofs.with_removed_header_paths(&removed_header_paths),
    )
    .unwrap();
    apply_include_rewrite_report(root, &report).unwrap();

    assert_eq!(report.counts.removed_include_lines, 1);
    assert_eq!(report.counts.public_headers_preserved, 0);
    assert_eq!(report.counts.live_missing_includes, 0);
    assert!(matches!(
        report.edits.as_slice(),
        [EditRecord {
            reason: EditReason::RemovedHeader { header },
            ..
        }] if header == "../include/private.h"
    ));
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/sub/helper.c")).unwrap(),
        "int helper;\n",
    );
}
