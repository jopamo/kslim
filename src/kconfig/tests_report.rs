use super::*;

#[test]
fn test_rewrite_kconfig_relations_drops_removed_selects_and_implies_only_from_valid_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "\tselect REMOVED\n",
            "\timply REMOVED\n",
            "config FOO\n",
            "\tbool \"Foo\"\n",
            "\tselect REMOVED\n",
            "\timply REMOVED if LIVE\n",
            "\tselect LIVE\n",
            "config REMOVED_SOURCE\n",
            "\tbool \"Removed source\"\n",
            "\tselect REMOVED\n",
            "\timply REMOVED\n",
        ),
    )
    .unwrap();

    let stats = rewrite_kconfig_relations(
        root,
        &[String::from("REMOVED"), String::from("REMOVED_SOURCE")],
    )
    .unwrap();

    assert_eq!(stats.rewrites, 2);
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        concat!(
            "\tselect REMOVED\n",
            "\timply REMOVED\n",
            "config FOO\n",
            "\tbool \"Foo\"\n",
            "\tselect LIVE\n",
            "config REMOVED_SOURCE\n",
            "\tbool \"Removed source\"\n",
            "\tselect REMOVED\n",
            "\timply REMOVED\n",
        )
    );
    assert_eq!(stats.edits.len(), 2);
    assert_eq!(stats.report.dropped_selects, 1);
    assert_eq!(stats.report.dropped_implies, 1);
    assert!(stats.edits.iter().all(|edit| {
        matches!(
            edit.reason,
            EditReason::ManifestConfig { ref symbol } if symbol == "REMOVED"
        )
    }));
}

#[test]
fn test_rewrite_kconfig_relations_simplifies_depends_visible_and_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "config FOO\n",
            "\tbool \"Foo\"\n",
            "\tdepends on REMOVED || LIVE\n",
            "\tvisible if REMOVED && LIVE\n",
            "\tdefault y if REMOVED\n",
            "\tdefault n if LIVE || REMOVED\n",
        ),
    )
    .unwrap();

    let stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();

    assert_eq!(stats.rewrites, 4);
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        concat!(
            "config FOO\n",
            "\tbool \"Foo\"\n",
            "\tdepends on LIVE\n",
            "\tvisible if n\n",
            "\tdefault n if LIVE\n",
        )
    );
    assert_eq!(stats.edits.len(), 4);
    assert_eq!(stats.report.simplified_depends, 1);
    assert_eq!(stats.report.simplified_visible_if, 1);
    assert_eq!(stats.report.simplified_defaults, 2);
    assert!(stats.edits.iter().all(|edit| {
        matches!(
            edit.reason,
            EditReason::SimplifiedTristateExpr { ref symbol } if symbol == "REMOVED"
        )
    }));
}

#[test]
fn test_rewrite_kconfig_relations_handles_tristate_m_edge_cases() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "config FOO\n",
            "\ttristate \"Foo\"\n",
            "\tdepends on REMOVED || m\n",
            "\tvisible if REMOVED && m\n",
            "\tdefault y if REMOVED || m\n",
        ),
    )
    .unwrap();

    let stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();

    assert_eq!(stats.rewrites, 3);
    assert_eq!(stats.report.simplified_depends, 1);
    assert_eq!(stats.report.simplified_visible_if, 1);
    assert_eq!(stats.report.simplified_defaults, 1);
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        concat!(
            "config FOO\n",
            "\ttristate \"Foo\"\n",
            "\tdepends on m\n",
            "\tvisible if n\n",
            "\tdefault y if m\n",
        )
    );
}

#[test]
fn test_rewrite_kconfig_relations_reports_unsupported_expression_syntax() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        "config FOO\n\tbool \"Foo\"\n\tdepends on REMOVED + LIVE\n",
    )
    .unwrap();

    let stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();

    assert_eq!(stats.rewrites, 0);
    assert!(stats.edits.is_empty());
    assert_eq!(
        stats.unsupported,
        vec![UnsupportedKconfigExpression {
            file: PathBuf::from("Kconfig"),
            line: 3,
            directive: String::from("depends on"),
            expression: String::from("REMOVED + LIVE"),
            reason: String::from(
                "expression syntax referencing removed symbols is not supported"
            ),
        }]
    );
    assert_eq!(stats.report.skipped_expressions, 1);
}

#[test]
fn test_rewrite_kconfig_relations_preserves_unknown_removed_target_conditions() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let original = concat!(
        "config LIVE\n",
        "\tbool \"Live\"\n",
        "\tselect REMOVED if LIVE + OTHER\n",
        "\timply REMOVED if OTHER ? LIVE\n",
        "\tselect REMOVED\n",
    );
    std::fs::write(root.join("Kconfig"), original).unwrap();

    let stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();

    assert_eq!(stats.rewrites, 1);
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        concat!(
            "config LIVE\n",
            "\tbool \"Live\"\n",
            "\tselect REMOVED if LIVE + OTHER\n",
            "\timply REMOVED if OTHER ? LIVE\n",
        )
    );
    assert_eq!(
        stats.unsupported,
        vec![
            UnsupportedKconfigExpression {
                file: PathBuf::from("Kconfig"),
                line: 3,
                directive: String::from("select"),
                expression: String::from("LIVE + OTHER"),
                reason: String::from(KCONFIG_UNKNOWN_REMOVED_TARGET_CONDITION_REASON),
            },
            UnsupportedKconfigExpression {
                file: PathBuf::from("Kconfig"),
                line: 4,
                directive: String::from("imply"),
                expression: String::from("OTHER ? LIVE"),
                reason: String::from(KCONFIG_UNKNOWN_REMOVED_TARGET_CONDITION_REASON),
            },
        ]
    );
    assert_eq!(stats.report.dropped_selects, 1);
    assert_eq!(stats.report.dropped_implies, 0);
    assert_eq!(stats.report.skipped_expressions, 2);
}

#[test]
fn test_rewrite_kconfig_relations_reports_unsupported_if_block_expression_syntax() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(root.join("Kconfig"), "if REMOVED + LIVE\nendif\n").unwrap();

    let stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();

    assert_eq!(stats.rewrites, 0);
    assert_eq!(
        stats.unsupported,
        vec![UnsupportedKconfigExpression {
            file: PathBuf::from("Kconfig"),
            line: 1,
            directive: String::from("if"),
            expression: String::from("REMOVED + LIVE"),
            reason: String::from(
                "expression syntax referencing removed symbols is not supported"
            ),
        }]
    );
    assert_eq!(stats.report.skipped_expressions, 1);
}
