use super::*;

#[test]
fn test_report_live_include_site_needing_manual_handling_reports_live_missing_include() {
    let site = IncludeSite {
        file: PathBuf::from("drivers/foo/test.c"),
        line: 3,
        header: String::from("linux/missing.h"),
        kind: IncludeKind::Angle,
    };

    assert_eq!(
        report_live_include_site_needing_manual_handling(&site, &[], true),
        Some(ManualIncludeHandlingSite {
            site,
            kind: ManualIncludeHandlingKind::LiveMissingInclude,
            classified_targets: Vec::new(),
        })
    );
}

#[test]
fn test_report_live_include_site_needing_manual_handling_reports_ambiguous_live_include() {
    let site = IncludeSite {
        file: PathBuf::from("drivers/foo/test.c"),
        line: 4,
        header: String::from("shared.h"),
        kind: IncludeKind::Quoted,
    };

    let classified_targets = vec![
        ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("drivers/foo/shared.h"),
                rule: IncludeResolveRule::LocalDirectory,
            },
            classification: IncludeTargetClassification::Exists,
        },
        ClassifiedIncludeTarget {
            target: ResolvedIncludeTarget {
                path: PathBuf::from("include/linux/shared.h"),
                rule: IncludeResolveRule::IncludeRoot,
            },
            classification: IncludeTargetClassification::PublicPreservedHeader,
        },
    ];

    assert_eq!(
        report_live_include_site_needing_manual_handling(&site, &classified_targets, true),
        Some(ManualIncludeHandlingSite {
            site,
            kind: ManualIncludeHandlingKind::AmbiguousInclude,
            classified_targets,
        })
    );
}

#[test]
fn test_report_live_include_site_needing_manual_handling_ignores_dead_or_safe_live_sites() {
    let dead_site = IncludeSite {
        file: PathBuf::from("drivers/foo/test.c"),
        line: 5,
        header: String::from("linux/missing.h"),
        kind: IncludeKind::Angle,
    };
    assert_eq!(
        report_live_include_site_needing_manual_handling(&dead_site, &[], false),
        None
    );

    let safe_live_site = IncludeSite {
        file: PathBuf::from("drivers/foo/test.c"),
        line: 6,
        header: String::from("internal.h"),
        kind: IncludeKind::Quoted,
    };
    assert_eq!(
        report_live_include_site_needing_manual_handling(
            &safe_live_site,
            &[ClassifiedIncludeTarget {
                target: ResolvedIncludeTarget {
                    path: PathBuf::from("drivers/foo/internal.h"),
                    rule: IncludeResolveRule::LocalDirectory,
                },
                classification: IncludeTargetClassification::Exists,
            }],
            true,
        ),
        None
    );
}

#[test]
fn test_rewrite_removed_header_includes_does_not_remove_commented_include() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    let original = concat!("/*\n", "#include \"removed.h\"\n", "*/\n", "int helper;\n",);
    std::fs::write(root.join("drivers/foo/helper.c"), original).unwrap();
    let removal_proofs = removal_proofs_with_removed_paths(&["drivers/foo/removed.h"]);
    let removed_header_paths = vec![PathBuf::from("drivers/foo/removed.h")];

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
        original,
    );
}

#[test]
fn test_rewrite_removed_header_includes_removes_dead_branch_backed_include() {
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
            "#include <linux/live_missing.h>\n",
            "int helper;\n",
        ),
    )
    .unwrap();
    let removal_proofs = removal_proofs_with_removed_paths(&[]);

    let report = rewrite_removed_header_includes_report_with_removed_configs(
        root,
        &removal_proofs,
        &[String::from("REMOVED")],
    )
    .unwrap();
    apply_include_rewrite_report(root, &report).unwrap();

    assert_eq!(report.counts.removed_include_lines, 1);
    assert_eq!(report.counts.live_missing_includes, 1);
    assert_eq!(report.edits.len(), 1);
    assert!(matches!(
        report.edits.as_slice(),
        [EditRecord {
            reason:
                EditReason::RemovedDeadBranchInclude {
                    header,
                    symbol,
                },
            proof_source:
                EditProofSource::RemovalManifest {
                    key: crate::edit_reason::RemovalKey::Config(proof_symbol),
                },
            ..
        }] if header == "linux/dead_missing.h"
            && symbol == "REMOVED"
            && proof_symbol == "REMOVED"
    ));
    assert_eq!(report.manual_sites[0].site.header, "linux/live_missing.h",);
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/helper.c")).unwrap(),
        concat!(
            "#if defined(CONFIG_LIVE)\n",
            "#ifdef CONFIG_REMOVED\n",
            "#endif\n",
            "#endif\n",
            "#include <linux/live_missing.h>\n",
            "int helper;\n",
        ),
    );
}

#[test]
fn test_rewrite_removed_header_includes_preserves_unsupported_dead_like_include() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    let original = concat!(
        "#if defined(CONFIG_REMOVED) + defined(CONFIG_OTHER)\n",
        "#include <linux/maybe_live.h>\n",
        "#endif\n",
    );
    std::fs::write(root.join("drivers/foo/helper.c"), original).unwrap();
    let removal_proofs = removal_proofs_with_removed_paths(&[]);

    let report = rewrite_removed_header_includes_report_with_removed_configs(
        root,
        &removal_proofs,
        &[String::from("REMOVED")],
    )
    .unwrap();
    apply_include_rewrite_report(root, &report).unwrap();

    assert_eq!(report.counts.removed_include_lines, 0);
    assert!(report.edits.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/helper.c")).unwrap(),
        original,
    );
}

#[test]
fn test_rewrite_removed_header_includes_reports_ambiguous_include() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::create_dir_all(root.join("include")).unwrap();
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
    let removal_proofs = removal_proofs_with_removed_paths(&[]);

    let report = rewrite_removed_header_includes_report(root, &removal_proofs).unwrap();
    apply_include_rewrite_report(root, &report).unwrap();

    assert_eq!(report.counts.ambiguous_includes, 1);
    assert_eq!(report.counts.removed_include_lines, 0);
    assert_eq!(report.counts.live_missing_includes, 0);
    assert_eq!(report.counts.public_headers_preserved, 0);
    assert!(report.edits.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/test.c")).unwrap(),
        "#include \"shared.h\"\nint test;\n",
    );
}
