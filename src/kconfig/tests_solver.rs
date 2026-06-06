use super::*;

#[test]
fn test_simplify_kconfig_expr_applies_declared_tristate_rules() {
    let removed = removed_set(&["REMOVED"]);

    assert_eq!(
        simplify_kconfig_expr(&parse_kconfig_expr("REMOVED").unwrap(), &removed),
        KconfigExpr::Literal(TristateLiteral::N)
    );
    assert_eq!(
        simplify_kconfig_expr(&parse_kconfig_expr("n && LIVE").unwrap(), &removed),
        KconfigExpr::Literal(TristateLiteral::N)
    );
    assert_eq!(
        simplify_kconfig_expr(&parse_kconfig_expr("y && LIVE").unwrap(), &removed),
        KconfigExpr::Symbol(String::from("LIVE"))
    );
    assert_eq!(
        simplify_kconfig_expr(&parse_kconfig_expr("n || LIVE").unwrap(), &removed),
        KconfigExpr::Symbol(String::from("LIVE"))
    );
    assert_eq!(
        simplify_kconfig_expr(&parse_kconfig_expr("y || LIVE").unwrap(), &removed),
        KconfigExpr::Literal(TristateLiteral::Y)
    );
    assert_eq!(
        simplify_kconfig_expr(&parse_kconfig_expr("!y").unwrap(), &removed),
        KconfigExpr::Literal(TristateLiteral::N)
    );
    assert_eq!(
        simplify_kconfig_expr(&parse_kconfig_expr("!n").unwrap(), &removed),
        KconfigExpr::Literal(TristateLiteral::Y)
    );
    assert_eq!(
        simplify_kconfig_expr(&parse_kconfig_expr("!m").unwrap(), &removed),
        KconfigExpr::Literal(TristateLiteral::M)
    );
    assert_eq!(
        simplify_kconfig_expr(&parse_kconfig_expr("m && y").unwrap(), &removed),
        KconfigExpr::Literal(TristateLiteral::M)
    );
    assert_eq!(
        simplify_kconfig_expr(&parse_kconfig_expr("m && n").unwrap(), &removed),
        KconfigExpr::Literal(TristateLiteral::N)
    );
    assert_eq!(
        simplify_kconfig_expr(&parse_kconfig_expr("m || n").unwrap(), &removed),
        KconfigExpr::Literal(TristateLiteral::M)
    );
    assert_eq!(
        simplify_kconfig_expr(&parse_kconfig_expr("m || y").unwrap(), &removed),
        KconfigExpr::Literal(TristateLiteral::Y)
    );
    assert_eq!(
        simplify_kconfig_expr(&parse_kconfig_expr("REMOVED = y").unwrap(), &removed),
        KconfigExpr::Literal(TristateLiteral::N)
    );
    assert_eq!(
        simplify_kconfig_expr(&parse_kconfig_expr("REMOVED = \"y\"").unwrap(), &removed),
        KconfigExpr::Literal(TristateLiteral::N)
    );
    assert_eq!(
        simplify_kconfig_expr(&parse_kconfig_expr("REMOVED = \"n\"").unwrap(), &removed),
        KconfigExpr::Literal(TristateLiteral::Y)
    );
    assert_eq!(
        simplify_kconfig_expr(&parse_kconfig_expr("REMOVED = n").unwrap(), &removed),
        KconfigExpr::Literal(TristateLiteral::Y)
    );
    assert_eq!(
        simplify_kconfig_expr(
            &parse_kconfig_expr("REMOVED = y || LIVE = y").unwrap(),
            &removed
        ),
        KconfigExpr::Eq(
            Box::new(KconfigExpr::Symbol(String::from("LIVE"))),
            Box::new(KconfigExpr::Literal(TristateLiteral::Y)),
        )
    );
}

#[test]
fn test_kconfig_expression_simplification_requires_tristate_equivalence() {
    let removed = removed_set(&["REMOVED"]);
    let original = parse_kconfig_expr("REMOVED || LIVE").unwrap();
    let simplified = equivalent_kconfig_expr_simplification(&original, &removed).unwrap();

    assert_eq!(simplified, KconfigExpr::Symbol(String::from("LIVE")));
    assert!(kconfig_expr_rewrite_is_tristate_equivalent(
        &original,
        &simplified,
        &removed,
    ));
    assert!(!kconfig_expr_rewrite_is_tristate_equivalent(
        &original,
        &KconfigExpr::Literal(TristateLiteral::N),
        &removed,
    ));

    let too_many_symbols = parse_kconfig_expr("A && B && C && D && E && F && G && H && I")
        .unwrap();
    assert!(!kconfig_expr_rewrite_is_tristate_equivalent(
        &too_many_symbols,
        &too_many_symbols,
        &removed,
    ));
    assert_eq!(
        equivalent_kconfig_expr_simplification(&too_many_symbols, &removed),
        None
    );
}

#[test]
fn test_rewrite_dead_kconfig_symbol_definitions_requires_solver_proof() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let kconfig = root.join("Kconfig");
    let original = concat!(
        "config LIVE\n",
        "\tbool \"Live\"\n",
        "config DEAD\n",
        "\tbool \"Dead\"\n",
        "\tdepends on REMOVED_GATE\n",
        "\thelp\n",
        "\t  dead help\n",
        "config DEFAULTED\n",
        "\tbool \"Defaulted\"\n",
        "\tdepends on REMOVED_GATE\n",
        "\tdefault y\n",
        "menuconfig DEAD_MENU\n",
        "\tbool \"Dead menu\"\n",
        "\tdepends on REMOVED_GATE\n",
        "config ARCH_USED\n",
        "\tbool \"Arch used\"\n",
        "\tdepends on REMOVED_GATE\n",
    );
    std::fs::write(&kconfig, original).unwrap();
    std::fs::create_dir_all(root.join("arch/x86")).unwrap();
    std::fs::write(
        root.join("arch/x86/Kconfig"),
        concat!(
            "config X86_PLATFORM\n",
            "\tbool \"x86 platform\"\n",
            "\tselect ARCH_USED\n",
        ),
    )
    .unwrap();

    let selected = selected_profile_values(&[
        ("LIVE", "y"),
        ("DEAD", "n"),
        ("DEFAULTED", "n"),
        ("DEAD_MENU", "n"),
        ("ARCH_USED", "n"),
        ("X86_PLATFORM", "n"),
    ]);
    let proofs = prove_dead_kconfig_symbol_definitions(
        root,
        &selected,
        &[String::from("REMOVED_GATE")],
    )
    .unwrap();
    assert!(proofs.iter().all(|proof| proof.symbol != "ARCH_USED"));

    assert_eq!(
        proofs,
        vec![
            KconfigDeadSymbolDefinitionProof {
                file: PathBuf::from("Kconfig"),
                symbol: String::from("DEAD"),
                definition_kind: KconfigSymbolDefinitionKind::Config,
                start_line: 3,
                end_line: 7,
            },
            KconfigDeadSymbolDefinitionProof {
                file: PathBuf::from("Kconfig"),
                symbol: String::from("DEAD_MENU"),
                definition_kind: KconfigSymbolDefinitionKind::Menuconfig,
                start_line: 12,
                end_line: 14,
            },
        ]
    );

    let (count, edits) = rewrite_dead_kconfig_symbol_definitions(root, &[]).unwrap();

    assert_eq!(count, 0);
    assert!(edits.is_empty());
    assert_eq!(std::fs::read_to_string(&kconfig).unwrap(), original);

    let mut mismatched = proofs.clone();
    mismatched[0].start_line = 2;
    let (count, edits) = rewrite_dead_kconfig_symbol_definitions(root, &mismatched).unwrap();

    assert_eq!(count, 1);
    assert_eq!(edits.len(), 1);
    assert_eq!(
        std::fs::read_to_string(&kconfig).unwrap(),
        concat!(
            "config LIVE\n",
            "\tbool \"Live\"\n",
            "config DEAD\n",
            "\tbool \"Dead\"\n",
            "\tdepends on REMOVED_GATE\n",
            "\thelp\n",
            "\t  dead help\n",
            "config DEFAULTED\n",
            "\tbool \"Defaulted\"\n",
            "\tdepends on REMOVED_GATE\n",
            "\tdefault y\n",
            "# kslim: removed unreachable menuconfig DEAD_MENU\n",
            "config ARCH_USED\n",
            "\tbool \"Arch used\"\n",
            "\tdepends on REMOVED_GATE\n",
        )
    );

    std::fs::write(&kconfig, original).unwrap();
    let (count, edits) = rewrite_dead_kconfig_symbol_definitions(root, &proofs).unwrap();
    let (second_count, second_edits) =
        rewrite_dead_kconfig_symbol_definitions(root, &proofs).unwrap();

    assert_eq!(count, 2);
    assert_eq!(edits.len(), 2);
    assert_eq!(second_count, 0);
    assert!(second_edits.is_empty());
    assert!(edits.iter().all(|edit| matches!(
        edit.reason,
        EditReason::RemovedDeadKconfigSymbolDefinition { .. }
    )));
    assert!(edits.iter().all(|edit| matches!(
        edit.proof_source,
        EditProofSource::KconfigSolver { .. }
    )));
    assert_eq!(
        std::fs::read_to_string(&kconfig).unwrap(),
        concat!(
            "config LIVE\n",
            "\tbool \"Live\"\n",
            "# kslim: removed unreachable config DEAD\n",
            "config DEFAULTED\n",
            "\tbool \"Defaulted\"\n",
            "\tdepends on REMOVED_GATE\n",
            "\tdefault y\n",
            "# kslim: removed unreachable menuconfig DEAD_MENU\n",
            "config ARCH_USED\n",
            "\tbool \"Arch used\"\n",
            "\tdepends on REMOVED_GATE\n",
        )
    );
}

#[test]
fn test_rewrite_empty_kconfig_menus_requires_solver_cleanup_proof() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let kconfig = root.join("Kconfig");
    let original = concat!(
        "menu \"Dead drivers\"\n",
        "\tdepends on LIVE\n",
        "# kslim: removed config DEAD_DRIVER\n",
        "endmenu\n",
        "menu \"Still has content\"\n",
        "# kslim: removed config REMOVED_SIBLING\n",
        "config LIVE_DRIVER\n",
        "\tbool \"Live\"\n",
        "endmenu\n",
        "menu \"Not pruned yet\"\n",
        "config REMOVED_DRIVER\n",
        "\tbool \"Removed\"\n",
        "endmenu\n",
        "menu \"Outer empty\"\n",
        "menu \"Inner empty\"\n",
        "# kslim: removed config INNER_DEAD\n",
        "endmenu\n",
        "endmenu\n",
    );
    std::fs::write(&kconfig, original).unwrap();

    let selected = selected_profile_values(&[("LIVE", "y"), ("LIVE_DRIVER", "y")]);
    let removed = vec![
        String::from("DEAD_DRIVER"),
        String::from("INNER_DEAD"),
        String::from("REMOVED_DRIVER"),
        String::from("REMOVED_SIBLING"),
    ];
    let proofs = prove_empty_kconfig_menus(root, &selected, &removed).unwrap();

    assert_eq!(
        proofs,
        vec![
            KconfigEmptyMenuRemovalProof {
                file: PathBuf::from("Kconfig"),
                prompt: String::from("Dead drivers"),
                start_line: 1,
                end_line: 4,
            },
            KconfigEmptyMenuRemovalProof {
                file: PathBuf::from("Kconfig"),
                prompt: String::from("Not pruned yet"),
                start_line: 10,
                end_line: 13,
            },
            KconfigEmptyMenuRemovalProof {
                file: PathBuf::from("Kconfig"),
                prompt: String::from("Outer empty"),
                start_line: 14,
                end_line: 18,
            },
            KconfigEmptyMenuRemovalProof {
                file: PathBuf::from("Kconfig"),
                prompt: String::from("Inner empty"),
                start_line: 15,
                end_line: 17,
            },
        ]
    );

    let (count, edits) = rewrite_empty_kconfig_menus(root, &[]).unwrap();
    assert_eq!(count, 0);
    assert!(edits.is_empty());
    assert_eq!(std::fs::read_to_string(&kconfig).unwrap(), original);

    let mut mismatched = proofs.clone();
    mismatched[0].end_line = 3;
    let (count, edits) = rewrite_empty_kconfig_menus(root, &mismatched[..1]).unwrap();
    assert_eq!(count, 0);
    assert!(edits.is_empty());
    assert_eq!(std::fs::read_to_string(&kconfig).unwrap(), original);

    let (count, edits) = rewrite_empty_kconfig_menus(root, &proofs).unwrap();
    let (second_count, second_edits) = rewrite_empty_kconfig_menus(root, &proofs).unwrap();

    assert_eq!(count, 2);
    assert_eq!(edits.len(), 2);
    assert_eq!(second_count, 0);
    assert!(second_edits.is_empty());
    assert!(edits.iter().all(|edit| matches!(
        edit.reason,
        EditReason::RemovedEmptyKconfigMenu { .. }
    )));
    assert!(edits.iter().all(|edit| matches!(
        edit.proof_source,
        EditProofSource::KconfigSolver { .. }
    )));
    assert_eq!(
        std::fs::read_to_string(&kconfig).unwrap(),
        concat!(
            "# kslim: removed empty menu \"Dead drivers\"\n",
            "menu \"Still has content\"\n",
            "# kslim: removed config REMOVED_SIBLING\n",
            "config LIVE_DRIVER\n",
            "\tbool \"Live\"\n",
            "endmenu\n",
            "menu \"Not pruned yet\"\n",
            "config REMOVED_DRIVER\n",
            "\tbool \"Removed\"\n",
            "endmenu\n",
            "# kslim: removed empty menu \"Outer empty\"\n",
        )
    );
}
