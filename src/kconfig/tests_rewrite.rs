use super::*;

#[test]
fn test_prune_configs_removes_multiple_symbol_definitions() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/foo")).unwrap();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "config KEEP_ROOT\n",
            "\tbool \"Keep root\"\n",
            "config DUP_SYMBOL\n",
            "\tbool \"First definition\"\n",
            "\tdefault y\n",
            "\thelp\n",
            "\t  first definition help\n",
            "config AFTER_ROOT\n",
            "\tbool \"After root\"\n",
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("drivers/foo/Kconfig"),
        concat!(
            "menuconfig DUP_SYMBOL\n",
            "\ttristate \"Second definition\"\n",
            "\tdepends on KEEP_ROOT\n",
            "config KEEP_DRIVER\n",
            "\tbool \"Keep driver\"\n",
        ),
    )
    .unwrap();

    let (removed, edits) = prune_configs(root, &[String::from("DUP_SYMBOL")]).unwrap();
    let (second_removed, second_edits) =
        prune_configs(root, &[String::from("DUP_SYMBOL")]).unwrap();

    assert_eq!(removed, 2);
    assert_eq!(edits.len(), 2);
    assert_eq!(second_removed, 0);
    assert!(second_edits.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        concat!(
            "config KEEP_ROOT\n",
            "\tbool \"Keep root\"\n",
            "# kslim: removed config DUP_SYMBOL\n",
            "config AFTER_ROOT\n",
            "\tbool \"After root\"\n",
        )
    );
    assert_eq!(
        std::fs::read_to_string(root.join("drivers/foo/Kconfig")).unwrap(),
        concat!(
            "# kslim: removed config DUP_SYMBOL\n",
            "config KEEP_DRIVER\n",
            "\tbool \"Keep driver\"\n",
        )
    );
    assert!(edits
        .iter()
        .any(|edit| edit.file == PathBuf::from("Kconfig") && edit.before.contains("help")));
    assert!(edits.iter().any(|edit| {
        edit.file == PathBuf::from("drivers/foo/Kconfig")
            && edit.before.contains("menuconfig DUP_SYMBOL")
    }));
}

#[test]
fn test_kconfig_symbol_rewrites_ignore_help_text_that_looks_like_ast() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let path = root.join("Kconfig");
    std::fs::write(
        &path,
        concat!(
            "config LIVE\n",
            "\tbool \"Live\"\n",
            "\thelp\n",
            "\t  config REMOVED\n",
            "\t    bool \"not real\"\n",
            "\t  config OVERRIDE\n",
            "\t    default n\n",
            "config OVERRIDE\n",
            "\tbool \"Override\"\n",
            "\tdefault n\n",
        ),
    )
    .unwrap();

    let symbols = defined_symbols_in_file(&path).unwrap();
    let (removed, _) = prune_configs(root, &[String::from("REMOVED")]).unwrap();
    let mut overrides = BTreeMap::new();
    overrides.insert(String::from("OVERRIDE"), String::from("y"));
    let (rewritten, _) = rewrite_kconfig_defaults(root, &overrides).unwrap();
    let (second_rewritten, second_edits) = rewrite_kconfig_defaults(root, &overrides).unwrap();

    assert_eq!(
        symbols,
        vec![String::from("LIVE"), String::from("OVERRIDE")]
    );
    assert_eq!(removed, 0);
    assert_eq!(rewritten, 1);
    assert_eq!(second_rewritten, 0);
    assert!(second_edits.is_empty());
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        concat!(
            "config LIVE\n",
            "\tbool \"Live\"\n",
            "\thelp\n",
            "\t  config REMOVED\n",
            "\t    bool \"not real\"\n",
            "\t  config OVERRIDE\n",
            "\t    default n\n",
            "config OVERRIDE\n",
            "\tbool \"Override\"\n",
            "\tdefault y\n",
        )
    );
}

#[test]
fn test_kconfig_relation_and_source_rewrites_ignore_help_text_ast_lookalikes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "config LIVE\n",
            "\tbool \"Live\"\n",
            "\thelp\n",
            "\t  depends on REMOVED || LIVE\n",
            "\t  if REMOVED || LIVE\n",
            "\t  source \"drivers/missing/Kconfig\"\n",
            "config REAL\n",
            "\tbool \"Real\"\n",
            "\tdepends on REMOVED || LIVE\n",
            "\tsource \"drivers/missing/Kconfig\"\n",
        ),
    )
    .unwrap();

    let relation_stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();
    let (removed_sources, _) = rewrite_kconfig_sources(
        root,
        &[source_removal_proof(10, "drivers/missing/Kconfig")],
    )
    .unwrap();

    assert_eq!(relation_stats.rewrites, 1);
    assert_eq!(removed_sources, 1);
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        concat!(
            "config LIVE\n",
            "\tbool \"Live\"\n",
            "\thelp\n",
            "\t  depends on REMOVED || LIVE\n",
            "\t  if REMOVED || LIVE\n",
            "\t  source \"drivers/missing/Kconfig\"\n",
            "config REAL\n",
            "\tbool \"Real\"\n",
            "\tdepends on LIVE\n",
            "\t# kslim: removed source \"drivers/missing/Kconfig\"\n",
        )
    );
}

#[test]
fn test_kconfig_rewrites_preserve_help_text_unless_removing_full_symbol_block() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "config LIVE\n",
            "\tbool \"Live\"\n",
            "\tdepends on REMOVED || LIVE\n",
            "\thelp\n",
            "\t  depends on REMOVED || LIVE\n",
            "\t  source \"drivers/missing/Kconfig\"\n",
            "\t  default y if REMOVED\n",
            "config DEFAULTED\n",
            "\tbool \"Defaulted\"\n",
            "\tdefault n\n",
            "\thelp\n",
            "\t  default y if REMOVED\n",
            "\t  config NOT_REAL\n",
            "config REMOVE_BLOCK\n",
            "\tbool \"Remove\"\n",
            "\thelp\n",
            "\t  remove help\n",
            "\t  depends on REMOVED || LIVE\n",
        ),
    )
    .unwrap();

    let relation_stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();
    let (removed_sources, _) = rewrite_kconfig_sources(
        root,
        &[source_removal_proof(6, "drivers/missing/Kconfig")],
    )
    .unwrap();
    let mut overrides = BTreeMap::new();
    overrides.insert(String::from("DEFAULTED"), String::from("y"));
    let (rewritten_defaults, _) = rewrite_kconfig_defaults(root, &overrides).unwrap();
    let (removed_blocks, _) = prune_configs(root, &[String::from("REMOVE_BLOCK")]).unwrap();

    assert_eq!(relation_stats.rewrites, 1);
    assert_eq!(removed_sources, 0);
    assert_eq!(rewritten_defaults, 1);
    assert_eq!(removed_blocks, 1);
    let rewritten = std::fs::read_to_string(root.join("Kconfig")).unwrap();
    assert_eq!(
        rewritten,
        concat!(
            "config LIVE\n",
            "\tbool \"Live\"\n",
            "\tdepends on LIVE\n",
            "\thelp\n",
            "\t  depends on REMOVED || LIVE\n",
            "\t  source \"drivers/missing/Kconfig\"\n",
            "\t  default y if REMOVED\n",
            "config DEFAULTED\n",
            "\tbool \"Defaulted\"\n",
            "\tdefault y\n",
            "\thelp\n",
            "\t  default y if REMOVED\n",
            "\t  config NOT_REAL\n",
            "# kslim: removed config REMOVE_BLOCK\n",
        )
    );
    assert!(!rewritten.contains("remove help"));
}

#[test]
fn test_rewrite_kconfig_relations_preserves_prompt_text_unless_removing_full_symbol_block() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "config LIVE\n",
            "\tbool \"Live prompt\" if REMOVED || LIVE\n",
            "\tprompt \"Explicit prompt\" if REMOVED\n",
            "\tdepends on REMOVED || LIVE\n",
            "config REMOVE_BLOCK\n",
            "\tbool \"Removed prompt\" if REMOVED\n",
            "\tprompt \"Removed explicit prompt\" if REMOVED\n",
            "\tdepends on REMOVED || LIVE\n",
        ),
    )
    .unwrap();

    let stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();

    assert_eq!(stats.rewrites, 2);
    assert_eq!(stats.report.simplified_depends, 2);
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        concat!(
            "config LIVE\n",
            "\tbool \"Live prompt\" if REMOVED || LIVE\n",
            "\tprompt \"Explicit prompt\" if REMOVED\n",
            "\tdepends on LIVE\n",
            "config REMOVE_BLOCK\n",
            "\tbool \"Removed prompt\" if REMOVED\n",
            "\tprompt \"Removed explicit prompt\" if REMOVED\n",
            "\tdepends on LIVE\n",
        )
    );

    let (removed, _) = prune_configs(root, &[String::from("REMOVE_BLOCK")]).unwrap();

    assert_eq!(removed, 1);
    let rewritten = std::fs::read_to_string(root.join("Kconfig")).unwrap();
    assert_eq!(
        rewritten,
        concat!(
            "config LIVE\n",
            "\tbool \"Live prompt\" if REMOVED || LIVE\n",
            "\tprompt \"Explicit prompt\" if REMOVED\n",
            "\tdepends on LIVE\n",
            "# kslim: removed config REMOVE_BLOCK\n",
        )
    );
    assert!(!rewritten.contains("Removed prompt"));
    assert!(!rewritten.contains("Removed explicit prompt"));
}

#[test]
fn test_rewrite_kconfig_relations_uses_sorted_removed_symbol_for_proof() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "config FOO\n",
            "\tbool \"Foo\"\n",
            "\tdepends on ZED || ALPHA\n",
        ),
    )
    .unwrap();

    let stats =
        rewrite_kconfig_relations(root, &[String::from("ZED"), String::from("ALPHA")]).unwrap();

    assert_eq!(stats.rewrites, 1);
    assert!(matches!(
        stats.edits[0].reason,
        EditReason::SimplifiedTristateExpr { ref symbol } if symbol == "ALPHA"
    ));
    assert!(matches!(
        stats.edits[0].proof_source,
        EditProofSource::RemovalManifest {
            key: crate::edit_reason::RemovalKey::Config(ref symbol),
            ..
        } if symbol == "ALPHA"
    ));
}

#[test]
fn test_rewrite_kconfig_relations_preserves_default_line_order() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "config FOO\n",
            "\tbool \"Foo\"\n",
            "\tdefault FIRST if KEEP_A\n",
            "\tdepends on KEEP\n",
            "\tdefault DROPPED if REMOVED\n",
            "\tselect LIVE\n",
            "\tdefault LAST if REMOVED || KEEP_B\n",
        ),
    )
    .unwrap();

    let stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();

    assert_eq!(stats.rewrites, 2);
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        concat!(
            "config FOO\n",
            "\tbool \"Foo\"\n",
            "\tdefault FIRST if KEEP_A\n",
            "\tdepends on KEEP\n",
            "\tselect LIVE\n",
            "\tdefault LAST if KEEP_B\n",
        )
    );
}

#[test]
fn test_rewrite_kconfig_relations_preserves_inline_comment_formatting() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "config FOO\n",
            "\tbool \"Foo\"\n",
            "\tdepends on REMOVED || LIVE   # keep relation note\n",
            "\tdefault y if REMOVED || KEEP  # keep default note\n",
            "\tselect LIVE if REMOVED || BAR\t# keep select note\n",
        ),
    )
    .unwrap();

    let stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();

    assert_eq!(stats.rewrites, 3);
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        concat!(
            "config FOO\n",
            "\tbool \"Foo\"\n",
            "\tdepends on LIVE   # keep relation note\n",
            "\tdefault y if KEEP  # keep default note\n",
            "\tselect LIVE if BAR\t# keep select note\n",
        )
    );
}

#[test]
fn test_rewrite_kconfig_relations_collapses_removed_and_branch_to_n() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        "config FOO\n\tbool \"Foo\"\n\tdepends on REMOVED && LIVE\n",
    )
    .unwrap();

    let stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();

    assert_eq!(stats.rewrites, 1);
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        "config FOO\n\tbool \"Foo\"\n\tdepends on n\n"
    );
}

#[test]
fn test_rewrite_kconfig_relations_simplifies_visible_if_removed() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        "config FOO\n\tbool \"Foo\"\n\tvisible if REMOVED\n",
    )
    .unwrap();

    let stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();

    assert_eq!(stats.rewrites, 1);
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        "config FOO\n\tbool \"Foo\"\n\tvisible if n\n"
    );
}

#[test]
fn test_rewrite_kconfig_relations_simplifies_select_condition_and_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "config FOO\n",
            "\tbool \"Foo\"\n",
            "\tselect LIVE if REMOVED || BAR\n",
            "\timply BAR if !REMOVED\n",
        ),
    )
    .unwrap();

    let first_stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();
    let after_first = std::fs::read_to_string(root.join("Kconfig")).unwrap();
    let second_stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();

    assert_eq!(first_stats.rewrites, 2);
    assert_eq!(
        after_first,
        concat!(
            "config FOO\n",
            "\tbool \"Foo\"\n",
            "\tselect LIVE if BAR\n",
            "\timply BAR\n",
        )
    );
    assert_eq!(second_stats.rewrites, 0);
    assert!(second_stats.edits.is_empty());
}

#[test]
fn test_rewrite_kconfig_relations_supports_symbol_equals_tristate_expression_syntax() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "config FOO\n",
            "\tbool \"Foo\"\n",
            "\tdepends on REMOVED = y || LIVE = y\n",
            "\tdefault y if REMOVED = m && LIVE = m\n",
        ),
    )
    .unwrap();

    let stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();

    assert_eq!(stats.rewrites, 2);
    assert!(stats.unsupported.is_empty());
    assert_eq!(stats.report.simplified_depends, 1);
    assert_eq!(stats.report.simplified_defaults, 1);
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        concat!(
            "config FOO\n",
            "\tbool \"Foo\"\n",
            "\tdepends on LIVE = y\n",
        )
    );
}

#[test]
fn test_rewrite_kconfig_relations_supports_quoted_tristate_expression_syntax() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "config FOO\n",
            "\tbool \"Foo\"\n",
            "\tdepends on REMOVED = \"y\" || LIVE = \"y\"\n",
            "\tdefault y if REMOVED = \"n\"\n",
        ),
    )
    .unwrap();

    let stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();

    assert_eq!(stats.rewrites, 2);
    assert!(stats.unsupported.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        concat!(
            "config FOO\n",
            "\tbool \"Foo\"\n",
            "\tdepends on LIVE = \"y\"\n",
            "\tdefault y\n",
        )
    );
}

#[test]
fn test_rewrite_kconfig_relations_does_not_treat_quoted_text_as_removed_symbol() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        "config FOO\n\tstring \"Foo\"\n\tdefault \"literal if REMOVED\"\n",
    )
    .unwrap();

    let stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();

    assert_eq!(stats.rewrites, 0);
    assert!(stats.unsupported.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        "config FOO\n\tstring \"Foo\"\n\tdefault \"literal if REMOVED\"\n",
    );
}

#[test]
fn test_rewrite_kconfig_relations_simplifies_nested_if_block_condition() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        "if REMOVED || LIVE\nconfig FOO\n\tbool \"Foo\"\nendif\n",
    )
    .unwrap();

    let stats = rewrite_kconfig_relations(root, &[String::from("REMOVED")]).unwrap();

    assert_eq!(stats.rewrites, 1);
    assert!(stats.unsupported.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        "if LIVE\nconfig FOO\n\tbool \"Foo\"\nendif\n",
    );
}

#[test]
fn test_rewrite_kconfig_sources_preserves_local_indentation_and_comment_suffix() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "menu \"Drivers\"\n",
            "\tsource \"drivers/missing/Kconfig\"   # keep source note\n",
            "endmenu\n",
        ),
    )
    .unwrap();

    let proofs = [source_removal_proof(2, "drivers/missing/Kconfig")];
    let (count, _) = rewrite_kconfig_sources(root, &proofs).unwrap();
    let (second_count, second_edits) = rewrite_kconfig_sources(root, &proofs).unwrap();

    assert_eq!(count, 1);
    assert_eq!(second_count, 0);
    assert!(second_edits.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        concat!(
            "menu \"Drivers\"\n",
            "\t# kslim: removed source \"drivers/missing/Kconfig\"   # keep source note\n",
            "endmenu\n",
        )
    );
}

#[test]
fn test_rewrite_kconfig_sources_removes_dead_source_and_preserves_live_source() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("drivers/live")).unwrap();
    std::fs::write(root.join("drivers/live/Kconfig"), "config LIVE_DRIVER\n").unwrap();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "menu \"Drivers\"\n",
            "\tsource \"drivers/dead/Kconfig\"\n",
            "\tsource \"drivers/live/Kconfig\"\n",
            "\tsource \"drivers/unproven-dead/Kconfig\"\n",
            "endmenu\n",
        ),
    )
    .unwrap();

    let (count, edits) = rewrite_kconfig_sources(
        root,
        &[source_removal_proof(2, "drivers/dead/Kconfig")],
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(edits.len(), 1);
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        concat!(
            "menu \"Drivers\"\n",
            "\t# kslim: removed source \"drivers/dead/Kconfig\"\n",
            "\tsource \"drivers/live/Kconfig\"\n",
            "\tsource \"drivers/unproven-dead/Kconfig\"\n",
            "endmenu\n",
        )
    );
    assert_eq!(edits[0].file, PathBuf::from("Kconfig"));
    assert_eq!(
        edits[0].span,
        Some(LineRange {
            start: 2,
            end: 2,
        })
    );
    assert_eq!(edits[0].reason, EditReason::RemovedKconfigSource);
    assert_eq!(
        edits[0].proof_source,
        EditProofSource::removal_manifest_kconfig_source(PathBuf::from(
            "drivers/dead/Kconfig"
        ))
    );
}

#[test]
fn test_rewrite_kconfig_sources_preserves_optional_missing_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("Kconfig"),
        concat!(
            "menu \"Drivers\"\n",
            "\tosource \"drivers/optional/Kconfig\"\n",
            "\torsource \"drivers/optional-relative/Kconfig\"\n",
            "\tsource \"drivers/dead/Kconfig\"\n",
            "endmenu\n",
        ),
    )
    .unwrap();

    let (count, edits) = rewrite_kconfig_sources(
        root,
        &[
            KconfigSourceRemovalProof {
                file: PathBuf::from("Kconfig"),
                line: 2,
                source: String::from("drivers/optional/Kconfig"),
                optional: true,
                relative: false,
                removed_target: PathBuf::from("drivers/optional/Kconfig"),
            },
            KconfigSourceRemovalProof {
                file: PathBuf::from("Kconfig"),
                line: 3,
                source: String::from("drivers/optional-relative/Kconfig"),
                optional: true,
                relative: true,
                removed_target: PathBuf::from("drivers/optional-relative/Kconfig"),
            },
            source_removal_proof(4, "drivers/dead/Kconfig"),
        ],
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(edits.len(), 1);
    assert_eq!(
        std::fs::read_to_string(root.join("Kconfig")).unwrap(),
        concat!(
            "menu \"Drivers\"\n",
            "\tosource \"drivers/optional/Kconfig\"\n",
            "\torsource \"drivers/optional-relative/Kconfig\"\n",
            "\t# kslim: removed source \"drivers/dead/Kconfig\"\n",
            "endmenu\n",
        )
    );
    assert_eq!(
        edits[0].proof_source,
        EditProofSource::removal_manifest_kconfig_source(PathBuf::from(
            "drivers/dead/Kconfig"
        ))
    );
}

#[test]
fn test_rewrite_kconfig_sources_requires_manifest_index_proof() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let kconfig = root.join("Kconfig");
    let original = concat!(
        "menu \"Drivers\"\n",
        "\tsource \"drivers/missing/Kconfig\"\n",
        "endmenu\n",
    );
    std::fs::write(&kconfig, original).unwrap();

    let (count, edits) = rewrite_kconfig_sources(root, &[]).unwrap();

    assert_eq!(count, 0);
    assert!(edits.is_empty());
    assert_eq!(std::fs::read_to_string(&kconfig).unwrap(), original);

    let (count, edits) = rewrite_kconfig_sources(
        root,
        &[source_removal_proof(1, "drivers/missing/Kconfig")],
    )
    .unwrap();

    assert_eq!(count, 0);
    assert!(edits.is_empty());
    assert_eq!(std::fs::read_to_string(&kconfig).unwrap(), original);

    let (count, edits) = rewrite_kconfig_sources(
        root,
        &[KconfigSourceRemovalProof {
            file: PathBuf::from("Kconfig"),
            line: 2,
            source: String::from("drivers/missing/Kconfig"),
            optional: false,
            relative: false,
            removed_target: PathBuf::from("drivers/missing/Kconfig"),
        }],
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(edits.len(), 1);
    assert_eq!(
        std::fs::read_to_string(&kconfig).unwrap(),
        concat!(
            "menu \"Drivers\"\n",
            "\t# kslim: removed source \"drivers/missing/Kconfig\"\n",
            "endmenu\n",
        )
    );
}
