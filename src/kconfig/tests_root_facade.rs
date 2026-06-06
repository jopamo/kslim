use super::*;

#[test]
fn test_parse_kconfig_directive_defines_minimal_ast_nodes() {
    assert_eq!(
        parse_kconfig_directive("config FOO"),
        Some(KconfigDirective::Entry {
            kind: KconfigEntryKind::Config,
            symbol: String::from("FOO"),
        })
    );
    assert_eq!(
        parse_kconfig_directive("config\tTAB"),
        Some(KconfigDirective::Entry {
            kind: KconfigEntryKind::Config,
            symbol: String::from("TAB"),
        })
    );
    assert_eq!(
        parse_kconfig_directive("menuconfig BAR"),
        Some(KconfigDirective::Entry {
            kind: KconfigEntryKind::Menuconfig,
            symbol: String::from("BAR"),
        })
    );
    assert_eq!(
        parse_kconfig_directive("\tdepends on FOO || BAR"),
        Some(KconfigDirective::DependsOn {
            expr: String::from("FOO || BAR"),
        })
    );
    assert_eq!(
        parse_kconfig_directive("\tselect BAZ if FOO"),
        Some(KconfigDirective::Select {
            symbol: String::from("BAZ"),
            condition: Some(String::from("FOO")),
        })
    );
    assert_eq!(
        parse_kconfig_directive("\timply QUX"),
        Some(KconfigDirective::Imply {
            symbol: String::from("QUX"),
            condition: None,
        })
    );
    assert_eq!(
        parse_kconfig_directive("\tvisible if FOO && BAR"),
        Some(KconfigDirective::VisibleIf {
            expr: String::from("FOO && BAR"),
        })
    );
    assert_eq!(
        parse_kconfig_directive("if FOO || BAR"),
        Some(KconfigDirective::If {
            expr: String::from("FOO || BAR"),
        })
    );
    assert_eq!(
        parse_kconfig_directive("\tdefault y if FOO"),
        Some(KconfigDirective::Default {
            value: String::from("y"),
            condition: Some(String::from("FOO")),
        })
    );
    assert_eq!(
        parse_kconfig_directive("\tdefault \"value if REMOVED\""),
        Some(KconfigDirective::Default {
            value: String::from("\"value if REMOVED\""),
            condition: None,
        })
    );
    assert_eq!(
        parse_kconfig_directive(r#"source "drivers/foo/Kconfig""#),
        Some(KconfigDirective::Source {
            source: KconfigSource {
                path: String::from("drivers/foo/Kconfig"),
                optional: false,
                relative: false,
            },
        })
    );
}

#[test]
fn test_parse_kconfig_source_ignores_trailing_comment() {
    let source =
        parse_kconfig_source(r#"    source "drivers/foo/Kconfig"   # keep note"#).unwrap();

    assert_eq!(source.path, "drivers/foo/Kconfig");
    assert!(!source.optional);
    assert!(!source.relative);
}

#[test]
fn test_defined_symbols_in_file_returns_sorted_unique_symbols() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("Kconfig");
    std::fs::write(
        &path,
        "config ZED\n\tbool \"Z\"\nmenuconfig ALPHA\n\tbool \"A\"\nconfig ZED\n\tbool \"Z again\"\n",
    )
    .unwrap();

    let symbols = defined_symbols_in_file(&path).unwrap();

    assert_eq!(symbols, vec![String::from("ALPHA"), String::from("ZED")]);
}

#[test]
fn test_render_kconfig_expr_preserves_parentheses_when_needed() {
    let expr = parse_kconfig_expr("!(FOO || BAR) && BAZ").unwrap();

    assert_eq!(render_kconfig_expr(&expr), "!(FOO || BAR) && BAZ");
}
