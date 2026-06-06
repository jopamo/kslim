use super::*;

#[test]
fn parse_kconfig_document_preserves_unknown_syntax_as_skipped_sites() {
    let document = parse_kconfig_document(concat!(
        "modules\n",
        "optional FOO # keep raw unknown\n",
        "config KNOWN\n",
        "\tbool \"Known\"\n",
        "\tweird body syntax\n",
    ))
    .unwrap();

    let skipped_sites = document.skipped_sites().collect::<Vec<_>>();
    let config = document.configs().next().unwrap();

    assert_eq!(document.nodes().len(), 3);
    assert_eq!(skipped_sites.len(), 2);
    assert_eq!(
        skipped_site_texts(&document),
        vec!["modules", "optional FOO # keep raw unknown"]
    );
    assert_eq!(skipped_sites[0].line(), 1);
    assert_eq!(skipped_sites[0].end_line(), 1);
    assert_eq!(skipped_sites[0].raw().text(), "modules");
    assert_eq!(skipped_sites[1].line(), 2);
    assert_eq!(skipped_sites[1].end_line(), 2);
    assert_eq!(
        skipped_sites[1].raw().text(),
        "optional FOO # keep raw unknown"
    );
    assert_eq!(config.line(), 3);
    assert_eq!(
        config
            .body()
            .iter()
            .map(KconfigRawLine::text)
            .collect::<Vec<_>>(),
        vec!["\tbool \"Known\"", "\tweird body syntax"]
    );
}
#[test]
fn parse_kconfig_document_preserves_formatting_lines() {
    let document = parse_kconfig_document(concat!(
        "\n",
        "source \"Kconfig.extra\"\n",
        "\t \n",
        "config FOO\n",
        "\tbool \"Foo\"\n",
        "\t  \n",
        "config BAR\n",
        "\tbool \"Bar\"\n",
    ))
    .unwrap();

    let blank_lines = document.blank_lines().collect::<Vec<_>>();
    let configs = document.configs().collect::<Vec<_>>();

    assert_eq!(document.nodes().len(), 5);
    assert_eq!(blank_lines.len(), 2);
    assert_eq!(blank_line_texts(&document), vec!["", "\t "]);
    assert_eq!(blank_lines[0].line(), 1);
    assert_eq!(blank_lines[0].end_line(), 1);
    assert_eq!(blank_lines[0].raw().text(), "");
    assert_eq!(blank_lines[1].line(), 3);
    assert_eq!(blank_lines[1].end_line(), 3);
    assert_eq!(
        configs[0]
            .body()
            .iter()
            .map(KconfigRawLine::text)
            .collect::<Vec<_>>(),
        vec!["\tbool \"Foo\"", "\t  "]
    );
    assert_eq!(configs[1].line(), 7);
}
#[test]
fn parse_kconfig_document_preserves_line_comments() {
    let document = parse_kconfig_document(concat!(
        "# SPDX-License-Identifier: GPL-2.0\n",
        "source \"Kconfig.extra\"\n",
        "\t# source boundary note\n",
        "config FOO # inline directive note\n",
        "\t# body note\n",
        "\tbool \"Foo\" # inline body note\n",
    ))
    .unwrap();

    let line_comments = document.line_comments().collect::<Vec<_>>();
    let config = document.configs().next().unwrap();

    assert_eq!(document.nodes().len(), 4);
    assert_eq!(line_comments.len(), 2);
    assert_eq!(
        line_comment_texts(&document),
        vec!["# SPDX-License-Identifier: GPL-2.0", "\t# source boundary note"]
    );
    assert_eq!(line_comments[0].line(), 1);
    assert_eq!(line_comments[0].end_line(), 1);
    assert_eq!(
        line_comments[0].raw().text(),
        "# SPDX-License-Identifier: GPL-2.0"
    );
    assert_eq!(line_comments[1].line(), 3);
    assert_eq!(line_comments[1].end_line(), 3);
    assert_eq!(config.directive().text(), "config FOO # inline directive note");
    assert_eq!(
        config
            .body()
            .iter()
            .map(KconfigRawLine::text)
            .collect::<Vec<_>>(),
        vec!["\t# body note", "\tbool \"Foo\" # inline body note"]
    );
}
#[test]
fn parse_kconfig_document_ignores_config_text_inside_help() {
    let document = parse_kconfig_document(concat!(
        "config LIVE\n",
        "\tbool \"Live\"\n",
        "\thelp\n",
        "\t  config NOT_A_SYMBOL\n",
        "\t    bool \"not syntax\"\n",
        "config NEXT\n",
        "\tbool \"Next\"\n",
    ))
    .unwrap();

    assert_eq!(config_symbols(&document), vec!["LIVE", "NEXT"]);
    let live = document.configs().next().unwrap();
    assert_eq!(live.line(), 1);
    assert_eq!(live.end_line(), 5);
}
