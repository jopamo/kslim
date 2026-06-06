use super::*;

#[test]
fn parse_kconfig_expr_parses_and_expression() {
    let expr = parse_kconfig_expr("FOO && BAR && BAZ").unwrap();

    assert_eq!(
        expr,
        KconfigExpr::And(
            Box::new(KconfigExpr::And(
                Box::new(KconfigExpr::Symbol(String::from("FOO"))),
                Box::new(KconfigExpr::Symbol(String::from("BAR"))),
            )),
            Box::new(KconfigExpr::Symbol(String::from("BAZ"))),
        )
    );
    assert_eq!(render_kconfig_expr(&expr), "FOO && BAR && BAZ");
    assert_eq!(parse_kconfig_expr("FOO & BAR"), None);
    assert_eq!(parse_kconfig_expr("FOO &&"), None);
}

#[test]
fn parse_kconfig_expr_parses_or_expression() {
    let expr = parse_kconfig_expr("FOO || BAR || BAZ").unwrap();
    let mixed = parse_kconfig_expr("FOO || BAR && BAZ").unwrap();

    assert_eq!(
        expr,
        KconfigExpr::Or(
            Box::new(KconfigExpr::Or(
                Box::new(KconfigExpr::Symbol(String::from("FOO"))),
                Box::new(KconfigExpr::Symbol(String::from("BAR"))),
            )),
            Box::new(KconfigExpr::Symbol(String::from("BAZ"))),
        )
    );
    assert_eq!(
        mixed,
        KconfigExpr::Or(
            Box::new(KconfigExpr::Symbol(String::from("FOO"))),
            Box::new(KconfigExpr::And(
                Box::new(KconfigExpr::Symbol(String::from("BAR"))),
                Box::new(KconfigExpr::Symbol(String::from("BAZ"))),
            )),
        )
    );
    assert_eq!(render_kconfig_expr(&expr), "FOO || BAR || BAZ");
    assert_eq!(render_kconfig_expr(&mixed), "FOO || BAR && BAZ");
    assert_eq!(parse_kconfig_expr("FOO | BAR"), None);
    assert_eq!(parse_kconfig_expr("FOO ||"), None);
}

#[test]
fn parse_kconfig_expr_parses_not_expression() {
    let expr = parse_kconfig_expr("!FOO").unwrap();
    let nested = parse_kconfig_expr("!!FOO").unwrap();
    let grouped = parse_kconfig_expr("!(FOO || BAR)").unwrap();
    let mixed = parse_kconfig_expr("!FOO && BAR").unwrap();

    assert_eq!(
        expr,
        KconfigExpr::Not(Box::new(KconfigExpr::Symbol(String::from("FOO"))))
    );
    assert_eq!(
        nested,
        KconfigExpr::Not(Box::new(KconfigExpr::Not(Box::new(KconfigExpr::Symbol(
            String::from("FOO")
        )))))
    );
    assert_eq!(
        grouped,
        KconfigExpr::Not(Box::new(KconfigExpr::Or(
            Box::new(KconfigExpr::Symbol(String::from("FOO"))),
            Box::new(KconfigExpr::Symbol(String::from("BAR"))),
        )))
    );
    assert_eq!(
        mixed,
        KconfigExpr::And(
            Box::new(KconfigExpr::Not(Box::new(KconfigExpr::Symbol(
                String::from("FOO")
            )))),
            Box::new(KconfigExpr::Symbol(String::from("BAR"))),
        )
    );
    assert_eq!(render_kconfig_expr(&expr), "!FOO");
    assert_eq!(render_kconfig_expr(&nested), "!!FOO");
    assert_eq!(render_kconfig_expr(&grouped), "!(FOO || BAR)");
    assert_eq!(render_kconfig_expr(&mixed), "!FOO && BAR");
    assert_eq!(parse_kconfig_expr("!"), None);
    assert_eq!(parse_kconfig_expr("FOO ! BAR"), None);
}

#[test]
fn parse_kconfig_expr_parses_equality_comparison() {
    let expr = parse_kconfig_expr("FOO = y").unwrap();
    let string_literal = parse_kconfig_expr("FOO = \"bar\"").unwrap();
    let mixed = parse_kconfig_expr("FOO = y && BAR").unwrap();

    assert_eq!(
        expr,
        KconfigExpr::Eq(
            Box::new(KconfigExpr::Symbol(String::from("FOO"))),
            Box::new(KconfigExpr::Literal(TristateLiteral::Y)),
        )
    );
    assert_eq!(
        string_literal,
        KconfigExpr::Eq(
            Box::new(KconfigExpr::Symbol(String::from("FOO"))),
            Box::new(KconfigExpr::StringLiteral(String::from("bar"))),
        )
    );
    assert_eq!(
        mixed,
        KconfigExpr::And(
            Box::new(KconfigExpr::Eq(
                Box::new(KconfigExpr::Symbol(String::from("FOO"))),
                Box::new(KconfigExpr::Literal(TristateLiteral::Y)),
            )),
            Box::new(KconfigExpr::Symbol(String::from("BAR"))),
        )
    );
    assert_eq!(render_kconfig_expr(&expr), "FOO = y");
    assert_eq!(render_kconfig_expr(&string_literal), "FOO = \"bar\"");
    assert_eq!(render_kconfig_expr(&mixed), "FOO = y && BAR");
    assert_eq!(parse_kconfig_expr("FOO ="), None);
    assert_eq!(parse_kconfig_expr("FOO == y"), None);
    assert_eq!(parse_kconfig_expr("\"y\""), None);
    assert_eq!(parse_kconfig_expr("FOO = \"unterminated"), None);
}

#[test]
fn parse_kconfig_expr_parses_inequality_comparison() {
    let expr = parse_kconfig_expr("FOO != n").unwrap();
    let string_literal = parse_kconfig_expr("FOO != \"bar\"").unwrap();
    let mixed = parse_kconfig_expr("FOO != n || BAR").unwrap();

    assert_eq!(
        expr,
        KconfigExpr::Ne(
            Box::new(KconfigExpr::Symbol(String::from("FOO"))),
            Box::new(KconfigExpr::Literal(TristateLiteral::N)),
        )
    );
    assert_eq!(
        string_literal,
        KconfigExpr::Ne(
            Box::new(KconfigExpr::Symbol(String::from("FOO"))),
            Box::new(KconfigExpr::StringLiteral(String::from("bar"))),
        )
    );
    assert_eq!(
        mixed,
        KconfigExpr::Or(
            Box::new(KconfigExpr::Ne(
                Box::new(KconfigExpr::Symbol(String::from("FOO"))),
                Box::new(KconfigExpr::Literal(TristateLiteral::N)),
            )),
            Box::new(KconfigExpr::Symbol(String::from("BAR"))),
        )
    );
    assert_eq!(render_kconfig_expr(&expr), "FOO != n");
    assert_eq!(render_kconfig_expr(&string_literal), "FOO != \"bar\"");
    assert_eq!(render_kconfig_expr(&mixed), "FOO != n || BAR");
    assert_eq!(parse_kconfig_expr("FOO !="), None);
    assert_eq!(parse_kconfig_expr("FOO !== n"), None);
    assert_eq!(parse_kconfig_expr("FOO ! = n"), None);
}

#[test]
fn parse_kconfig_expr_parses_symbol_references() {
    let expr = parse_kconfig_expr("DRM_AMDGPU").unwrap();
    let prefixed = parse_kconfig_expr("CONFIG_FOO").unwrap();
    let numeric_prefix = parse_kconfig_expr("64BIT").unwrap();
    let mixed = parse_kconfig_expr("DRM_AMDGPU && CONFIG_FOO").unwrap();

    assert_eq!(expr, KconfigExpr::Symbol(String::from("DRM_AMDGPU")));
    assert_eq!(prefixed, KconfigExpr::Symbol(String::from("CONFIG_FOO")));
    assert_eq!(numeric_prefix, KconfigExpr::Symbol(String::from("64BIT")));
    assert_eq!(
        mixed,
        KconfigExpr::And(
            Box::new(KconfigExpr::Symbol(String::from("DRM_AMDGPU"))),
            Box::new(KconfigExpr::Symbol(String::from("CONFIG_FOO"))),
        )
    );
    assert_eq!(render_kconfig_expr(&expr), "DRM_AMDGPU");
    assert_eq!(render_kconfig_expr(&prefixed), "CONFIG_FOO");
    assert_eq!(render_kconfig_expr(&numeric_prefix), "64BIT");
    assert_eq!(render_kconfig_expr(&mixed), "DRM_AMDGPU && CONFIG_FOO");
    assert_eq!(parse_kconfig_expr("DRM-AMDGPU"), None);
    assert_eq!(parse_kconfig_expr("drivers/foo"), None);
    assert_eq!(parse_kconfig_expr("FOO BAR"), None);
}

#[test]
fn parse_kconfig_expr_parses_y_literal() {
    let expr = parse_kconfig_expr("y").unwrap();
    let mixed = parse_kconfig_expr("y && FOO").unwrap();
    let uppercase = parse_kconfig_expr("Y").unwrap();

    assert_eq!(expr, KconfigExpr::Literal(TristateLiteral::Y));
    assert_eq!(
        mixed,
        KconfigExpr::And(
            Box::new(KconfigExpr::Literal(TristateLiteral::Y)),
            Box::new(KconfigExpr::Symbol(String::from("FOO"))),
        )
    );
    assert_eq!(uppercase, KconfigExpr::Symbol(String::from("Y")));
    assert_eq!(render_kconfig_expr(&expr), "y");
    assert_eq!(render_kconfig_expr(&mixed), "y && FOO");
    assert_eq!(render_kconfig_expr(&uppercase), "Y");
}

#[test]
fn parse_kconfig_expr_parses_m_literal() {
    let expr = parse_kconfig_expr("m").unwrap();
    let mixed = parse_kconfig_expr("m || FOO").unwrap();
    let uppercase = parse_kconfig_expr("M").unwrap();

    assert_eq!(expr, KconfigExpr::Literal(TristateLiteral::M));
    assert_eq!(
        mixed,
        KconfigExpr::Or(
            Box::new(KconfigExpr::Literal(TristateLiteral::M)),
            Box::new(KconfigExpr::Symbol(String::from("FOO"))),
        )
    );
    assert_eq!(uppercase, KconfigExpr::Symbol(String::from("M")));
    assert_eq!(render_kconfig_expr(&expr), "m");
    assert_eq!(render_kconfig_expr(&mixed), "m || FOO");
    assert_eq!(render_kconfig_expr(&uppercase), "M");
}

#[test]
fn parse_kconfig_expr_parses_n_literal() {
    let expr = parse_kconfig_expr("n").unwrap();
    let mixed = parse_kconfig_expr("n && FOO").unwrap();
    let uppercase = parse_kconfig_expr("N").unwrap();

    assert_eq!(expr, KconfigExpr::Literal(TristateLiteral::N));
    assert_eq!(
        mixed,
        KconfigExpr::And(
            Box::new(KconfigExpr::Literal(TristateLiteral::N)),
            Box::new(KconfigExpr::Symbol(String::from("FOO"))),
        )
    );
    assert_eq!(uppercase, KconfigExpr::Symbol(String::from("N")));
    assert_eq!(render_kconfig_expr(&expr), "n");
    assert_eq!(render_kconfig_expr(&mixed), "n && FOO");
    assert_eq!(render_kconfig_expr(&uppercase), "N");
}

#[test]
fn simplify_kconfig_expr_evaluates_tristate_min_max() {
    let removed = std::collections::HashSet::new();

    for (source, expected) in [
        ("y && y", TristateLiteral::Y),
        ("y && m", TristateLiteral::M),
        ("m && y", TristateLiteral::M),
        ("m && n", TristateLiteral::N),
        ("n && m", TristateLiteral::N),
        ("n && y", TristateLiteral::N),
        ("n || n", TristateLiteral::N),
        ("n || m", TristateLiteral::M),
        ("m || n", TristateLiteral::M),
        ("m || y", TristateLiteral::Y),
        ("y || m", TristateLiteral::Y),
        ("y || y", TristateLiteral::Y),
        ("(m && y) || n", TristateLiteral::M),
        ("(m || n) && y", TristateLiteral::M),
    ] {
        let simplified = simplify_kconfig_expr(&parse_kconfig_expr(source).unwrap(), &removed);

        assert_eq!(simplified, KconfigExpr::Literal(expected), "{source}");
        assert_eq!(
            render_kconfig_expr(&simplified),
            render_kconfig_expr(&KconfigExpr::Literal(expected)),
            "{source}"
        );
    }
}

#[test]
fn evaluate_kconfig_visibility_lowers_dependencies_by_tristate_minimum() {
    let document = parse_kconfig_document(concat!(
        "config LOWERED_TO_M\n",
        "\ttristate \"Lowered to m\"\n",
        "\tdepends on DEP_Y\n",
        "\tdepends on DEP_M\n",
        "config LOWERED_TO_N\n",
        "\ttristate \"Lowered to n\"\n",
        "\tdepends on DEP_M\n",
        "\tdepends on DEP_N\n",
        "config PROMPT_LOWERED_TO_M\n",
        "\ttristate \"Prompt lowered\" if PROMPT_M\n",
        "\tdepends on DEP_Y\n",
    ))
    .unwrap();
    let configs = document.configs().collect::<Vec<_>>();
    let symbol_values = std::collections::BTreeMap::from([
        (String::from("DEP_Y"), TristateLiteral::Y),
        (String::from("DEP_M"), TristateLiteral::M),
        (String::from("DEP_N"), TristateLiteral::N),
        (String::from("PROMPT_M"), TristateLiteral::M),
    ]);

    assert_eq!(
        evaluate_kconfig_visibility(
            configs[0].prompt_definitions(),
            configs[0].dependency_definitions(),
            &symbol_values,
        ),
        Some(TristateLiteral::M)
    );
    assert_eq!(
        evaluate_kconfig_reachability_under_selected_profile(
            configs[0].prompt_definitions(),
            configs[0].dependency_definitions(),
            &symbol_values,
        ),
        Some(true)
    );
    assert_eq!(
        evaluate_kconfig_visibility(
            configs[1].prompt_definitions(),
            configs[1].dependency_definitions(),
            &symbol_values,
        ),
        Some(TristateLiteral::N)
    );
    assert_eq!(
        evaluate_kconfig_reachability_under_selected_profile(
            configs[1].prompt_definitions(),
            configs[1].dependency_definitions(),
            &symbol_values,
        ),
        Some(false)
    );
    assert_eq!(
        evaluate_kconfig_visibility(
            configs[2].prompt_definitions(),
            configs[2].dependency_definitions(),
            &symbol_values,
        ),
        Some(TristateLiteral::M)
    );
}

#[test]
fn evaluate_kconfig_visibility_combines_prompt_and_dependency_expressions() {
    let document = parse_kconfig_document(concat!(
        "config VISIBLE\n",
        "\ttristate \"Visible\" if PROMPT\n",
        "\tdepends on DEP_A\n",
        "\tdepends on DEP_B || m\n",
        "config NO_PROMPT\n",
        "\tbool\n",
        "\tdepends on DEP_A\n",
        "config UNKNOWN_PROMPT\n",
        "\tbool \"Unknown\" if MISSING\n",
        "config COMPARE\n",
        "\tbool \"Compare\" if MODE = \"y\"\n",
        "\tdepends on !BLOCKED\n",
    ))
    .unwrap();
    let configs = document.configs().collect::<Vec<_>>();
    let symbol_values = std::collections::BTreeMap::from([
        (String::from("PROMPT"), TristateLiteral::Y),
        (String::from("DEP_A"), TristateLiteral::M),
        (String::from("DEP_B"), TristateLiteral::N),
        (String::from("MODE"), TristateLiteral::Y),
        (String::from("BLOCKED"), TristateLiteral::N),
    ]);

    assert_eq!(
        evaluate_kconfig_visibility(
            configs[0].prompt_definitions(),
            configs[0].dependency_definitions(),
            &symbol_values,
        ),
        Some(TristateLiteral::M)
    );
    assert_eq!(
        evaluate_kconfig_visibility(
            configs[1].prompt_definitions(),
            configs[1].dependency_definitions(),
            &symbol_values,
        ),
        Some(TristateLiteral::N)
    );
    assert_eq!(
        evaluate_kconfig_visibility(
            configs[2].prompt_definitions(),
            configs[2].dependency_definitions(),
            &symbol_values,
        ),
        None
    );
    assert_eq!(
        evaluate_kconfig_visibility(
            configs[3].prompt_definitions(),
            configs[3].dependency_definitions(),
            &symbol_values,
        ),
        Some(TristateLiteral::Y)
    );
}

#[test]
fn evaluate_kconfig_reachability_under_selected_profile_uses_visibility() {
    let document = parse_kconfig_document(concat!(
        "config REACHABLE\n",
        "\ttristate \"Reachable\" if FEATURE\n",
        "\tdepends on MODULES\n",
        "config BLOCKED\n",
        "\ttristate \"Blocked\" if FEATURE\n",
        "\tdepends on BLOCKER\n",
        "config NO_PROMPT\n",
        "\tbool\n",
        "\tdepends on FEATURE\n",
        "config UNKNOWN\n",
        "\tbool \"Unknown\" if MISSING\n",
    ))
    .unwrap();
    let configs = document.configs().collect::<Vec<_>>();
    let selected_profile_values = std::collections::BTreeMap::from([
        (String::from("FEATURE"), TristateLiteral::Y),
        (String::from("MODULES"), TristateLiteral::M),
        (String::from("BLOCKER"), TristateLiteral::N),
    ]);

    assert_eq!(
        evaluate_kconfig_reachability_under_selected_profile(
            configs[0].prompt_definitions(),
            configs[0].dependency_definitions(),
            &selected_profile_values,
        ),
        Some(true)
    );
    assert_eq!(
        evaluate_kconfig_reachability_under_selected_profile(
            configs[1].prompt_definitions(),
            configs[1].dependency_definitions(),
            &selected_profile_values,
        ),
        Some(false)
    );
    assert_eq!(
        evaluate_kconfig_reachability_under_selected_profile(
            configs[2].prompt_definitions(),
            configs[2].dependency_definitions(),
            &selected_profile_values,
        ),
        Some(false)
    );
    assert_eq!(
        evaluate_kconfig_reachability_under_selected_profile(
            configs[3].prompt_definitions(),
            configs[3].dependency_definitions(),
            &selected_profile_values,
        ),
        None
    );
}

#[test]
fn evaluate_kconfig_removed_symbol_effect_forces_removed_symbols_to_n() {
    let removed_symbols = std::collections::HashSet::from(["REMOVED"]);
    let selected_profile_values = std::collections::BTreeMap::from([
        (String::from("REMOVED"), TristateLiteral::Y),
        (String::from("LIVE"), TristateLiteral::Y),
        (String::from("DEP"), TristateLiteral::M),
    ]);
    let removed_and_live = parse_kconfig_expr("REMOVED && LIVE").unwrap();
    let removed_equals_n = parse_kconfig_expr("REMOVED = n").unwrap();

    assert_eq!(
        evaluate_kconfig_expr(&removed_and_live, &selected_profile_values),
        Some(TristateLiteral::Y)
    );
    assert_eq!(
        evaluate_kconfig_expr_after_removed_symbols(
            &removed_and_live,
            &selected_profile_values,
            &removed_symbols,
        ),
        Some(TristateLiteral::N)
    );
    assert_eq!(
        evaluate_kconfig_expr_after_removed_symbols(
            &removed_equals_n,
            &selected_profile_values,
            &removed_symbols,
        ),
        Some(TristateLiteral::Y)
    );

    let document = parse_kconfig_document(concat!(
        "config STILL_REACHABLE\n",
        "\ttristate \"Still\" if LIVE\n",
        "\tdepends on REMOVED || DEP\n",
        "config BLOCKED_BY_REMOVAL\n",
        "\ttristate \"Blocked\" if LIVE\n",
        "\tdepends on REMOVED && DEP\n",
        "config EQUALITY_AFTER_REMOVAL\n",
        "\tbool \"Equals\" if REMOVED = n\n",
    ))
    .unwrap();
    let configs = document.configs().collect::<Vec<_>>();

    assert_eq!(
        evaluate_kconfig_visibility_after_removed_symbols(
            configs[0].prompt_definitions(),
            configs[0].dependency_definitions(),
            &selected_profile_values,
            &removed_symbols,
        ),
        Some(TristateLiteral::M)
    );
    assert_eq!(
        evaluate_kconfig_reachability_after_removed_symbols(
            configs[0].prompt_definitions(),
            configs[0].dependency_definitions(),
            &selected_profile_values,
            &removed_symbols,
        ),
        Some(true)
    );
    assert_eq!(
        evaluate_kconfig_reachability_after_removed_symbols(
            configs[1].prompt_definitions(),
            configs[1].dependency_definitions(),
            &selected_profile_values,
            &removed_symbols,
        ),
        Some(false)
    );
    assert_eq!(
        evaluate_kconfig_reachability_after_removed_symbols(
            configs[2].prompt_definitions(),
            configs[2].dependency_definitions(),
            &selected_profile_values,
            &removed_symbols,
        ),
        Some(true)
    );
}

#[test]
fn evaluate_kconfig_defaults_after_removal_uses_first_active_default() {
    let removed_symbols = std::collections::HashSet::from(["REMOVED"]);
    let selected_profile_values = std::collections::BTreeMap::from([
        (String::from("REMOVED"), TristateLiteral::Y),
        (String::from("LIVE"), TristateLiteral::Y),
        (String::from("OTHER"), TristateLiteral::M),
    ]);
    let document = parse_kconfig_document(concat!(
        "config FIRST_AFTER_REMOVAL\n",
        "\ttristate \"First\"\n",
        "\tdefault y if REMOVED\n",
        "\tdefault m if LIVE\n",
        "\tdefault y\n",
        "config VALUE_FORCED_TO_N\n",
        "\ttristate \"Value\"\n",
        "\tdefault REMOVED if LIVE\n",
        "\tdefault y\n",
        "config UNCONDITIONAL_SYMBOL\n",
        "\ttristate \"Unconditional\"\n",
        "\tdefault OTHER\n",
        "config EQUALITY_AFTER_REMOVAL\n",
        "\tbool \"Equals\"\n",
        "\tdefault y if REMOVED = n\n",
        "config NO_ACTIVE_DEFAULT\n",
        "\ttristate \"No active\"\n",
        "\tdefault y if REMOVED && LIVE\n",
    ))
    .unwrap();
    let configs = document.configs().collect::<Vec<_>>();

    assert_eq!(
        evaluate_kconfig_defaults(
            configs[0].default_definitions(),
            &selected_profile_values,
        ),
        Some(TristateLiteral::Y)
    );
    assert_eq!(
        evaluate_kconfig_defaults_after_removed_symbols(
            configs[0].default_definitions(),
            &selected_profile_values,
            &removed_symbols,
        ),
        Some(TristateLiteral::M)
    );
    assert_eq!(
        evaluate_kconfig_defaults_after_removed_symbols(
            configs[1].default_definitions(),
            &selected_profile_values,
            &removed_symbols,
        ),
        Some(TristateLiteral::N)
    );
    assert_eq!(
        evaluate_kconfig_defaults_after_removed_symbols(
            configs[2].default_definitions(),
            &selected_profile_values,
            &removed_symbols,
        ),
        Some(TristateLiteral::M)
    );
    assert_eq!(
        evaluate_kconfig_defaults_after_removed_symbols(
            configs[3].default_definitions(),
            &selected_profile_values,
            &removed_symbols,
        ),
        Some(TristateLiteral::Y)
    );
    assert_eq!(
        evaluate_kconfig_defaults_after_removed_symbols(
            configs[4].default_definitions(),
            &selected_profile_values,
            &removed_symbols,
        ),
        Some(TristateLiteral::N)
    );
}

#[test]
fn detect_kconfig_symbols_reenabled_by_defaults_reports_removed_non_n_defaults() {
    let removed_symbols = std::collections::HashSet::from([
        "REMOVED_CHOICE",
        "REMOVED_CONDITION_OFF",
        "REMOVED_DUP",
        "REMOVED_GATE",
        "REMOVED_M",
        "REMOVED_MENU",
        "REMOVED_VALUE_FORCED_N",
        "REMOVED_Y",
    ]);
    let selected_profile_values = std::collections::BTreeMap::from([
        (String::from("LIVE"), TristateLiteral::Y),
        (String::from("OTHER"), TristateLiteral::M),
        (String::from("REMOVED_GATE"), TristateLiteral::Y),
    ]);
    let document = parse_kconfig_document(concat!(
        "config REMOVED_Y\n",
        "\ttristate \"Removed y\"\n",
        "\tdefault y if LIVE\n",
        "config REMOVED_M\n",
        "\ttristate \"Removed m\"\n",
        "\tdefault OTHER\n",
        "config REMOVED_CONDITION_OFF\n",
        "\ttristate \"Off\"\n",
        "\tdefault y if REMOVED_GATE\n",
        "\tdefault n\n",
        "config REMOVED_VALUE_FORCED_N\n",
        "\ttristate \"Value forced n\"\n",
        "\tdefault REMOVED_GATE if LIVE\n",
        "\tdefault y\n",
        "config REMOVED_DUP\n",
        "\ttristate \"Dup one\"\n",
        "\tdefault n if LIVE\n",
        "config REMOVED_DUP\n",
        "\ttristate \"Dup two\"\n",
        "\tdefault y\n",
        "menuconfig REMOVED_MENU\n",
        "\ttristate \"Removed menu\"\n",
        "\tdefault m if LIVE\n",
        "choice REMOVED_CHOICE\n",
        "\ttristate \"Removed choice\"\n",
        "\tdefault y if LIVE\n",
        "config LIVE_SYMBOL\n",
        "\ttristate \"Live\"\n",
        "\tdefault y\n",
    ))
    .unwrap();

    let reenabled = detect_kconfig_symbols_reenabled_by_defaults(
        &document,
        &selected_profile_values,
        &removed_symbols,
    )
    .unwrap();
    let observed = reenabled
        .iter()
        .map(|symbol| (symbol.symbol().to_string(), symbol.value()))
        .collect::<Vec<_>>();

    assert_eq!(
        observed,
        vec![
            (String::from("REMOVED_CHOICE"), TristateLiteral::Y),
            (String::from("REMOVED_M"), TristateLiteral::M),
            (String::from("REMOVED_MENU"), TristateLiteral::M),
            (String::from("REMOVED_Y"), TristateLiteral::Y),
        ]
    );
}

#[test]
fn detect_kconfig_removed_symbols_forced_by_select_reports_live_select_edges() {
    let removed_symbols = std::collections::HashSet::from([
        "REMOVED_BY_CHOICE",
        "REMOVED_BY_M",
        "REMOVED_BY_MENU",
        "REMOVED_BY_N",
        "REMOVED_BY_Y",
        "REMOVED_CONDITION_OFF",
        "REMOVED_FROM_REMOVED_SOURCE",
        "REMOVED_GATE",
        "REMOVED_SOURCE",
    ]);
    let selected_profile_values = std::collections::BTreeMap::from([
        (String::from("LIVE_CHOICE"), TristateLiteral::Y),
        (String::from("LIVE_COND"), TristateLiteral::Y),
        (String::from("LIVE_M"), TristateLiteral::M),
        (String::from("LIVE_MENU"), TristateLiteral::M),
        (String::from("LIVE_N"), TristateLiteral::N),
        (String::from("LIVE_Y"), TristateLiteral::Y),
        (String::from("REMOVED_GATE"), TristateLiteral::Y),
        (String::from("REMOVED_SOURCE"), TristateLiteral::Y),
    ]);
    let document = parse_kconfig_document(concat!(
        "config LIVE_Y\n",
        "\ttristate \"Live y\"\n",
        "\tselect REMOVED_BY_Y\n",
        "\tselect LIVE_TARGET\n",
        "\tselect REMOVED_CONDITION_OFF if REMOVED_GATE\n",
        "config LIVE_M\n",
        "\ttristate \"Live m\"\n",
        "\tselect REMOVED_BY_M if LIVE_COND\n",
        "config LIVE_N\n",
        "\ttristate \"Live n\"\n",
        "\tselect REMOVED_BY_N\n",
        "config REMOVED_SOURCE\n",
        "\ttristate \"Removed source\"\n",
        "\tselect REMOVED_FROM_REMOVED_SOURCE\n",
        "menuconfig LIVE_MENU\n",
        "\ttristate \"Live menu\"\n",
        "\tselect REMOVED_BY_MENU if LIVE_COND\n",
        "choice LIVE_CHOICE\n",
        "\ttristate \"Live choice\"\n",
        "\tselect REMOVED_BY_CHOICE\n",
    ))
    .unwrap();

    let forced = detect_kconfig_removed_symbols_forced_by_select(
        &document,
        &selected_profile_values,
        &removed_symbols,
    )
    .unwrap();
    let observed = forced
        .iter()
        .map(|selection| {
            (
                selection.source_symbol().to_string(),
                selection.target_symbol().to_string(),
                selection.value(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        observed,
        vec![
            (
                String::from("LIVE_Y"),
                String::from("REMOVED_BY_Y"),
                TristateLiteral::Y,
            ),
            (
                String::from("LIVE_M"),
                String::from("REMOVED_BY_M"),
                TristateLiteral::M,
            ),
            (
                String::from("LIVE_MENU"),
                String::from("REMOVED_BY_MENU"),
                TristateLiteral::M,
            ),
            (
                String::from("LIVE_CHOICE"),
                String::from("REMOVED_BY_CHOICE"),
                TristateLiteral::Y,
            ),
        ]
    );
}

#[test]
fn detect_kconfig_removed_symbols_forced_by_select_bypasses_target_dependencies() {
    let removed_symbols = std::collections::HashSet::from([
        "REMOVED_COND_DEP_BLOCKED",
        "REMOVED_DEP_BLOCKED",
    ]);
    let selected_profile_values = std::collections::BTreeMap::from([
        (String::from("BLOCKED_DEP"), TristateLiteral::N),
        (String::from("LIVE_COND"), TristateLiteral::M),
        (String::from("LIVE_SELECT"), TristateLiteral::Y),
    ]);
    let document = parse_kconfig_document(concat!(
        "config REMOVED_DEP_BLOCKED\n",
        "\ttristate \"Removed dependency-blocked target\"\n",
        "\tdepends on BLOCKED_DEP\n",
        "config REMOVED_COND_DEP_BLOCKED\n",
        "\ttristate \"Removed conditioned dependency-blocked target\"\n",
        "\tdepends on BLOCKED_DEP\n",
        "config LIVE_SELECT\n",
        "\ttristate \"Live select source\"\n",
        "\tselect REMOVED_DEP_BLOCKED\n",
        "\tselect REMOVED_COND_DEP_BLOCKED if LIVE_COND\n",
    ))
    .unwrap();

    for target_symbol in ["REMOVED_DEP_BLOCKED", "REMOVED_COND_DEP_BLOCKED"] {
        let target = document
            .configs()
            .find(|config| config.symbol().as_str() == target_symbol)
            .unwrap();
        assert_eq!(
            evaluate_kconfig_visibility_after_removed_symbols(
                target.prompt_definitions(),
                target.dependency_definitions(),
                &selected_profile_values,
                &removed_symbols,
            ),
            Some(TristateLiteral::N),
            "{target_symbol}"
        );
    }

    let forced = detect_kconfig_removed_symbols_forced_by_select(
        &document,
        &selected_profile_values,
        &removed_symbols,
    )
    .unwrap();
    let observed = forced
        .iter()
        .map(|selection| {
            (
                selection.source_symbol().to_string(),
                selection.target_symbol().to_string(),
                selection.value(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        observed,
        vec![
            (
                String::from("LIVE_SELECT"),
                String::from("REMOVED_DEP_BLOCKED"),
                TristateLiteral::Y,
            ),
            (
                String::from("LIVE_SELECT"),
                String::from("REMOVED_COND_DEP_BLOCKED"),
                TristateLiteral::M,
            ),
        ]
    );
}

#[test]
fn detect_kconfig_removed_symbols_weakly_enabled_by_imply_reports_live_imply_edges() {
    let removed_symbols = std::collections::HashSet::from([
        "REMOVED_BY_CHOICE",
        "REMOVED_BY_M",
        "REMOVED_BY_MENU",
        "REMOVED_BY_N",
        "REMOVED_BY_Y",
        "REMOVED_CONDITION_OFF",
        "REMOVED_FROM_REMOVED_SOURCE",
        "REMOVED_GATE",
        "REMOVED_SOURCE",
    ]);
    let selected_profile_values = std::collections::BTreeMap::from([
        (String::from("LIVE_CHOICE"), TristateLiteral::Y),
        (String::from("LIVE_COND"), TristateLiteral::Y),
        (String::from("LIVE_M"), TristateLiteral::M),
        (String::from("LIVE_MENU"), TristateLiteral::M),
        (String::from("LIVE_N"), TristateLiteral::N),
        (String::from("LIVE_Y"), TristateLiteral::Y),
        (String::from("REMOVED_GATE"), TristateLiteral::Y),
        (String::from("REMOVED_SOURCE"), TristateLiteral::Y),
    ]);
    let document = parse_kconfig_document(concat!(
        "config LIVE_Y\n",
        "\ttristate \"Live y\"\n",
        "\timply REMOVED_BY_Y\n",
        "\timply LIVE_TARGET\n",
        "\timply REMOVED_CONDITION_OFF if REMOVED_GATE\n",
        "config LIVE_M\n",
        "\ttristate \"Live m\"\n",
        "\timply REMOVED_BY_M if LIVE_COND\n",
        "config LIVE_N\n",
        "\ttristate \"Live n\"\n",
        "\timply REMOVED_BY_N\n",
        "config REMOVED_SOURCE\n",
        "\ttristate \"Removed source\"\n",
        "\timply REMOVED_FROM_REMOVED_SOURCE\n",
        "menuconfig LIVE_MENU\n",
        "\ttristate \"Live menu\"\n",
        "\timply REMOVED_BY_MENU if LIVE_COND\n",
        "choice LIVE_CHOICE\n",
        "\ttristate \"Live choice\"\n",
        "\timply REMOVED_BY_CHOICE\n",
    ))
    .unwrap();

    let weakly_enabled = detect_kconfig_removed_symbols_weakly_enabled_by_imply(
        &document,
        &selected_profile_values,
        &removed_symbols,
    )
    .unwrap();
    let observed = weakly_enabled
        .iter()
        .map(|implication| {
            (
                implication.source_symbol().to_string(),
                implication.target_symbol().to_string(),
                implication.value(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        observed,
        vec![
            (
                String::from("LIVE_Y"),
                String::from("REMOVED_BY_Y"),
                TristateLiteral::Y,
            ),
            (
                String::from("LIVE_M"),
                String::from("REMOVED_BY_M"),
                TristateLiteral::M,
            ),
            (
                String::from("LIVE_MENU"),
                String::from("REMOVED_BY_MENU"),
                TristateLiteral::M,
            ),
            (
                String::from("LIVE_CHOICE"),
                String::from("REMOVED_BY_CHOICE"),
                TristateLiteral::Y,
            ),
        ]
    );
}

#[test]
fn detect_kconfig_removed_symbols_weakly_enabled_by_imply_respects_target_dependencies() {
    let removed_symbols = std::collections::HashSet::from([
        "REMOVED_DEP_BLOCKED",
        "REMOVED_DEP_LOWERED_TO_M",
    ]);
    let selected_profile_values = std::collections::BTreeMap::from([
        (String::from("BLOCKED_DEP"), TristateLiteral::N),
        (String::from("LIVE_IMPLY"), TristateLiteral::Y),
        (String::from("MODULE_DEP"), TristateLiteral::M),
    ]);
    let document = parse_kconfig_document(concat!(
        "config REMOVED_DEP_BLOCKED\n",
        "\ttristate\n",
        "\tdepends on BLOCKED_DEP\n",
        "config REMOVED_DEP_LOWERED_TO_M\n",
        "\ttristate\n",
        "\tdepends on MODULE_DEP\n",
        "config LIVE_IMPLY\n",
        "\ttristate \"Live imply source\"\n",
        "\timply REMOVED_DEP_BLOCKED\n",
        "\timply REMOVED_DEP_LOWERED_TO_M\n",
    ))
    .unwrap();

    assert_eq!(
        evaluate_kconfig_symbol_dependency_upper_bound_after_removed_symbols(
            &document,
            "REMOVED_DEP_BLOCKED",
            &selected_profile_values,
            &removed_symbols,
        ),
        Some(TristateLiteral::N)
    );
    assert_eq!(
        evaluate_kconfig_symbol_dependency_upper_bound_after_removed_symbols(
            &document,
            "REMOVED_DEP_LOWERED_TO_M",
            &selected_profile_values,
            &removed_symbols,
        ),
        Some(TristateLiteral::M)
    );

    let weakly_enabled = detect_kconfig_removed_symbols_weakly_enabled_by_imply(
        &document,
        &selected_profile_values,
        &removed_symbols,
    )
    .unwrap();
    let observed = weakly_enabled
        .iter()
        .map(|implication| {
            (
                implication.source_symbol().to_string(),
                implication.target_symbol().to_string(),
                implication.value(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        observed,
        vec![(
            String::from("LIVE_IMPLY"),
            String::from("REMOVED_DEP_LOWERED_TO_M"),
            TristateLiteral::M,
        )]
    );
}

#[test]
fn detect_kconfig_impossible_choices_reports_unreachable_mandatory_choices() {
    let removed_symbols = std::collections::HashSet::from([
        "REMOVED_ANON_MEMBER",
        "REMOVED_HIDDEN_MEMBER",
        "REMOVED_MEMBER",
        "REMOVED_OPTIONAL_MEMBER",
        "REMOVED_WITH_LIVE_MEMBER",
    ]);
    let selected_profile_values = std::collections::BTreeMap::from([
        (String::from("LIVE"), TristateLiteral::Y),
        (String::from("MOD_GATE"), TristateLiteral::M),
        (String::from("OFF"), TristateLiteral::N),
    ]);
    let document = parse_kconfig_document(concat!(
        "choice BROKEN_CHOICE\n",
        "\ttristate \"Broken\"\n",
        "\tdepends on LIVE\n",
        "config REMOVED_MEMBER\n",
        "\tbool \"Removed\"\n",
        "config UNREACHABLE_MEMBER\n",
        "\tbool \"Unreachable\"\n",
        "\tdepends on OFF\n",
        "endchoice\n",
        "choice HAS_LIVE_MEMBER\n",
        "\tbool \"Has live\"\n",
        "config REMOVED_WITH_LIVE_MEMBER\n",
        "\tbool \"Removed\"\n",
        "config LIVE_MEMBER\n",
        "\tbool \"Live\"\n",
        "endchoice\n",
        "choice HIDDEN_CHOICE\n",
        "\tbool \"Hidden\" if OFF\n",
        "config REMOVED_HIDDEN_MEMBER\n",
        "\tbool \"Removed hidden\"\n",
        "endchoice\n",
        "choice OPTIONAL_CHOICE\n",
        "\tbool \"Optional\"\n",
        "\toptional\n",
        "config REMOVED_OPTIONAL_MEMBER\n",
        "\tbool \"Removed optional\"\n",
        "endchoice\n",
        "choice\n",
        "\ttristate \"Anonymous\"\n",
        "\tdepends on MOD_GATE\n",
        "config REMOVED_ANON_MEMBER\n",
        "\ttristate \"Removed anon\"\n",
        "endchoice\n",
    ))
    .unwrap();

    let impossible = detect_kconfig_impossible_choices(
        &document,
        &selected_profile_values,
        &removed_symbols,
    )
    .unwrap();
    let observed = impossible
        .iter()
        .map(|choice| {
            (
                choice.choice_symbol().map(str::to_string),
                choice.line(),
                choice.visibility(),
                choice.member_symbols().to_vec(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        observed,
        vec![
            (
                Some(String::from("BROKEN_CHOICE")),
                1,
                TristateLiteral::Y,
                vec![
                    String::from("REMOVED_MEMBER"),
                    String::from("UNREACHABLE_MEMBER"),
                ],
            ),
            (
                None,
                28,
                TristateLiteral::M,
                vec![String::from("REMOVED_ANON_MEMBER")],
            ),
        ]
    );
}

#[test]
fn detect_kconfig_choice_invalidation_reports_removed_selected_choice_without_live_replacement() {
    let removed_symbols = std::collections::HashSet::from([
        "REMOVED_SELECTED_MEMBER",
        "REMOVED_VALID_MEMBER",
    ]);
    let selected_profile_values = std::collections::BTreeMap::from([
        (String::from("BLOCKED_DEP"), TristateLiteral::N),
        (String::from("CHOICE_GATE"), TristateLiteral::Y),
        (String::from("REMOVED_SELECTED_MEMBER"), TristateLiteral::Y),
    ]);
    let document = parse_kconfig_document(concat!(
        "choice INVALIDATED_CHOICE\n",
        "\tbool \"Invalidated\"\n",
        "\tdepends on CHOICE_GATE\n",
        "config REMOVED_SELECTED_MEMBER\n",
        "\tbool \"Removed selected\"\n",
        "config BLOCKED_REPLACEMENT\n",
        "\tbool \"Blocked replacement\"\n",
        "\tdepends on BLOCKED_DEP\n",
        "endchoice\n",
        "choice STILL_VALID_CHOICE\n",
        "\tbool \"Still valid\"\n",
        "config REMOVED_VALID_MEMBER\n",
        "\tbool \"Removed valid\"\n",
        "config LIVE_REPLACEMENT\n",
        "\tbool \"Live replacement\"\n",
        "endchoice\n",
    ))
    .unwrap();

    let impossible = detect_kconfig_impossible_choices(
        &document,
        &selected_profile_values,
        &removed_symbols,
    )
    .unwrap();
    let observed = impossible
        .iter()
        .map(|choice| {
            (
                choice.choice_symbol().map(str::to_string),
                choice.line(),
                choice.visibility(),
                choice.member_symbols().to_vec(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        observed,
        vec![(
            Some(String::from("INVALIDATED_CHOICE")),
            1,
            TristateLiteral::Y,
            vec![
                String::from("REMOVED_SELECTED_MEMBER"),
                String::from("BLOCKED_REPLACEMENT"),
            ],
        )]
    );
}

#[test]
fn detect_kconfig_empty_menus_reports_reachable_menus_without_live_content() {
    let removed_symbols = std::collections::HashSet::from([
        "REMOVED_COMMENT_MEMBER",
        "REMOVED_HIDDEN_MENU_MEMBER",
        "REMOVED_MENU_MEMBER",
        "REMOVED_MOD_MEMBER",
        "REMOVED_SOURCE_MEMBER",
        "REMOVED_WITH_LIVE_MEMBER",
    ]);
    let selected_profile_values = std::collections::BTreeMap::from([
        (String::from("LIVE"), TristateLiteral::Y),
        (String::from("MOD_GATE"), TristateLiteral::M),
        (String::from("OFF"), TristateLiteral::N),
    ]);
    let document = parse_kconfig_document(concat!(
        "menu \"Empty\"\n",
        "\tdepends on LIVE\n",
        "config REMOVED_MENU_MEMBER\n",
        "\tbool \"Removed\"\n",
        "config HIDDEN_MENU_MEMBER\n",
        "\tbool \"Hidden\"\n",
        "\tdepends on OFF\n",
        "endmenu\n",
        "menu \"Module empty\"\n",
        "\tvisible if MOD_GATE\n",
        "config REMOVED_MOD_MEMBER\n",
        "\ttristate \"Removed mod\"\n",
        "endmenu\n",
        "menu \"Has live\"\n",
        "config REMOVED_WITH_LIVE_MEMBER\n",
        "\tbool \"Removed\"\n",
        "config LIVE_MEMBER\n",
        "\tbool \"Live\"\n",
        "endmenu\n",
        "menu \"Hidden menu\"\n",
        "\tvisible if OFF\n",
        "config REMOVED_HIDDEN_MENU_MEMBER\n",
        "\tbool \"Removed hidden\"\n",
        "endmenu\n",
        "menu \"Comment menu\"\n",
        "comment \"Still visible\"\n",
        "config REMOVED_COMMENT_MEMBER\n",
        "\tbool \"Removed comment sibling\"\n",
        "endmenu\n",
        "menu \"Source menu\"\n",
        "source \"Kconfig.live\"\n",
        "config REMOVED_SOURCE_MEMBER\n",
        "\tbool \"Removed source sibling\"\n",
        "endmenu\n",
    ))
    .unwrap();

    let empty = detect_kconfig_empty_menus(
        &document,
        &selected_profile_values,
        &removed_symbols,
    )
    .unwrap();
    let observed = empty
        .iter()
        .map(|menu| {
            (
                menu.prompt().to_string(),
                menu.line(),
                menu.visibility(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        observed,
        vec![
            (String::from("Empty"), 1, TristateLiteral::Y),
            (String::from("Module empty"), 9, TristateLiteral::M),
        ]
    );
}

#[test]
fn detect_kconfig_orphaned_symbol_definitions_reports_unactivated_unreachable_definitions() {
    let removed_symbols = std::collections::HashSet::from([
        "REMOVED_GATE",
        "REMOVED_SYMBOL",
    ]);
    let selected_profile_values = std::collections::BTreeMap::from([
        (String::from("DEFAULT_GATE"), TristateLiteral::Y),
        (String::from("DEFAULT_VALUE"), TristateLiteral::M),
        (String::from("IMPLY_SOURCE"), TristateLiteral::M),
        (String::from("OFF"), TristateLiteral::N),
        (String::from("PROFILE_SELECTED"), TristateLiteral::Y),
        (String::from("SELECT_SOURCE"), TristateLiteral::Y),
    ]);
    let document = parse_kconfig_document(concat!(
        "config ORPHANED_DEP\n",
        "\tbool \"Orphaned dep\"\n",
        "\tdepends on REMOVED_GATE\n",
        "config ORPHANED_HIDDEN\n",
        "\tbool \"Orphaned hidden\" if OFF\n",
        "config LIVE_PROMPT\n",
        "\tbool \"Live\"\n",
        "config SELECT_SOURCE\n",
        "\tbool \"Select source\"\n",
        "\tselect SELECTED_TARGET\n",
        "config SELECTED_TARGET\n",
        "\tbool \"Selected target\"\n",
        "\tdepends on OFF\n",
        "config IMPLY_SOURCE\n",
        "\ttristate \"Imply source\"\n",
        "\timply IMPLIED_TARGET\n",
        "config IMPLIED_TARGET\n",
        "\ttristate \"Implied target\"\n",
        "\tdepends on OFF\n",
        "config DEFAULTED_TARGET\n",
        "\ttristate \"Defaulted\"\n",
        "\tdepends on OFF\n",
        "\tdefault DEFAULT_VALUE if DEFAULT_GATE\n",
        "config PROFILE_SELECTED\n",
        "\tbool \"Profile selected\"\n",
        "\tdepends on OFF\n",
        "config REMOVED_SYMBOL\n",
        "\tbool \"Removed\"\n",
        "\tdepends on REMOVED_GATE\n",
        "menuconfig ORPHANED_MENU\n",
        "\tbool \"Orphaned menu\"\n",
        "\tdepends on REMOVED_GATE\n",
        "choice ORPHANED_CHOICE\n",
        "\tbool \"Orphaned choice\"\n",
        "\tdepends on REMOVED_GATE\n",
    ))
    .unwrap();

    let orphaned = detect_kconfig_orphaned_symbol_definitions(
        &document,
        &selected_profile_values,
        &removed_symbols,
    )
    .unwrap();
    let observed = orphaned
        .iter()
        .map(|definition| {
            (
                definition.symbol().to_string(),
                definition.definition_kind(),
                definition.line(),
                definition.visibility(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        observed,
        vec![
            (
                String::from("ORPHANED_DEP"),
                KconfigSymbolDefinitionKind::Config,
                1,
                TristateLiteral::N,
            ),
            (
                String::from("ORPHANED_HIDDEN"),
                KconfigSymbolDefinitionKind::Config,
                4,
                TristateLiteral::N,
            ),
            (
                String::from("ORPHANED_MENU"),
                KconfigSymbolDefinitionKind::Menuconfig,
                30,
                TristateLiteral::N,
            ),
            (
                String::from("ORPHANED_CHOICE"),
                KconfigSymbolDefinitionKind::Choice,
                33,
                TristateLiteral::N,
            ),
        ]
    );
}
