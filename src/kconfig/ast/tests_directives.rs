use super::*;

#[test]
fn parse_kconfig_document_parses_config_entries() {
    let document = parse_kconfig_document(concat!(
        "# SPDX-License-Identifier: GPL-2.0\n",
        "config FOO # keep directive note\n",
        "\tbool \"Foo\"\n",
        "\tdefault y\n",
        "\n",
        "config\tBAR\n",
        "\tdepends on FOO\n",
    ))
    .unwrap();

    let configs = document.configs().collect::<Vec<_>>();
    assert_eq!(configs.len(), 2);
    assert_eq!(document.nodes().len(), 3);
    assert_eq!(config_symbols(&document), vec!["FOO", "BAR"]);
    assert_eq!(configs[0].line(), 2);
    assert_eq!(configs[0].end_line(), 5);
    assert_eq!(
        configs[0].directive().text(),
        "config FOO # keep directive note"
    );
    assert_eq!(
        configs[0]
            .body()
            .iter()
            .map(KconfigRawLine::text)
            .collect::<Vec<_>>(),
        vec!["\tbool \"Foo\"", "\tdefault y", ""]
    );
    assert_eq!(configs[1].directive().text(), "config\tBAR");
    assert_eq!(configs[1].line(), 6);
    assert_eq!(configs[1].end_line(), 7);
}
#[test]
fn parse_kconfig_document_parses_menuconfig_entries() {
    let document = parse_kconfig_document(concat!(
        "menuconfig NETDEVICES # keep directive note\n",
        "\tbool \"Network device support\"\n",
        "\tdefault y\n",
        "\n",
        "menuconfig\tBT\n",
        "\ttristate \"Bluetooth subsystem support\"\n",
        "\tdepends on NET\n",
        "config AFTER\n",
        "\tbool \"After\"\n",
    ))
    .unwrap();

    let menuconfigs = document.menuconfigs().collect::<Vec<_>>();
    assert_eq!(menuconfigs.len(), 2);
    assert_eq!(document.nodes().len(), 3);
    assert_eq!(config_symbols(&document), vec!["AFTER"]);
    assert_eq!(menuconfig_symbols(&document), vec!["NETDEVICES", "BT"]);
    assert_eq!(menuconfigs[0].line(), 1);
    assert_eq!(menuconfigs[0].end_line(), 4);
    assert_eq!(
        menuconfigs[0].directive().text(),
        "menuconfig NETDEVICES # keep directive note"
    );
    assert_eq!(
        menuconfigs[0]
            .body()
            .iter()
            .map(KconfigRawLine::text)
            .collect::<Vec<_>>(),
        vec!["\tbool \"Network device support\"", "\tdefault y", ""]
    );
    assert_eq!(menuconfigs[1].directive().text(), "menuconfig\tBT");
    assert_eq!(menuconfigs[1].line(), 5);
    assert_eq!(menuconfigs[1].end_line(), 7);
}
#[test]
fn parse_kconfig_document_parses_choice_entries() {
    let document = parse_kconfig_document(concat!(
        "choice NET_VENDOR # keep directive note\n",
        "\tprompt \"Vendor\"\n",
        "\tdefault VENDOR_A\n",
        "config VENDOR_A\n",
        "\tbool \"Vendor A\"\n",
        "endchoice\n",
        "choice\n",
        "\tprompt \"Anonymous\"\n",
        "endchoice\n",
    ))
    .unwrap();

    let choices = document.choices().collect::<Vec<_>>();
    assert_eq!(choices.len(), 2);
    assert_eq!(document.nodes().len(), 5);
    assert_eq!(config_symbols(&document), vec!["VENDOR_A"]);
    assert_eq!(choice_symbols(&document), vec![Some("NET_VENDOR"), None]);
    assert_eq!(endchoice_lines(&document), vec![6, 9]);
    assert_eq!(choices[0].line(), 1);
    assert_eq!(choices[0].end_line(), 3);
    assert_eq!(
        choices[0].directive().text(),
        "choice NET_VENDOR # keep directive note"
    );
    assert_eq!(
        choices[0]
            .body()
            .iter()
            .map(KconfigRawLine::text)
            .collect::<Vec<_>>(),
        vec!["\tprompt \"Vendor\"", "\tdefault VENDOR_A"]
    );
    assert_eq!(choices[1].symbol(), None);
    assert_eq!(choices[1].line(), 7);
    assert_eq!(choices[1].end_line(), 8);
}
#[test]
fn parse_kconfig_document_parses_endchoice_markers() {
    let document = parse_kconfig_document(concat!(
        "choice\n",
        "\tprompt \"Anonymous\"\n",
        "endchoice # keep end marker note\n",
        "config AFTER\n",
        "\tbool \"After\"\n",
    ))
    .unwrap();

    let endchoices = document.endchoices().collect::<Vec<_>>();
    assert_eq!(endchoices.len(), 1);
    assert_eq!(document.nodes().len(), 3);
    assert_eq!(endchoices[0].line(), 3);
    assert_eq!(endchoices[0].end_line(), 3);
    assert_eq!(
        endchoices[0].directive().text(),
        "endchoice # keep end marker note"
    );
    assert_eq!(config_symbols(&document), vec!["AFTER"]);
}
#[test]
fn parse_kconfig_document_parses_menu_entries() {
    let document = parse_kconfig_document(concat!(
        "menu \"Device drivers\" # keep directive note\n",
        "\tdepends on HAS_IOMEM\n",
        "config FOO\n",
        "\tbool \"Foo\"\n",
        "endmenu\n",
        "menu\t\"Networking \\\"stack\\\"\"\n",
        "\tvisible if NET\n",
        "endmenu\n",
    ))
    .unwrap();

    let menus = document.menus().collect::<Vec<_>>();
    assert_eq!(menus.len(), 2);
    assert_eq!(document.nodes().len(), 5);
    assert_eq!(
        menu_prompts(&document),
        vec!["Device drivers", "Networking \"stack\""]
    );
    assert_eq!(endmenu_lines(&document), vec![5, 8]);
    assert_eq!(config_symbols(&document), vec!["FOO"]);
    assert_eq!(menus[0].line(), 1);
    assert_eq!(menus[0].end_line(), 2);
    assert_eq!(
        menus[0].directive().text(),
        "menu \"Device drivers\" # keep directive note"
    );
    assert_eq!(
        menus[0]
            .body()
            .iter()
            .map(KconfigRawLine::text)
            .collect::<Vec<_>>(),
        vec!["\tdepends on HAS_IOMEM"]
    );
    assert_eq!(
        menus[1].directive().text(),
        "menu\t\"Networking \\\"stack\\\"\""
    );
    assert_eq!(menus[1].line(), 6);
    assert_eq!(menus[1].end_line(), 7);
}
#[test]
fn parse_kconfig_document_parses_comment_entries() {
    let document = parse_kconfig_document(concat!(
        "comment \"Driver note\" # keep comment note\n",
        "\tdepends on HAS_IOMEM\n",
        "config AFTER\n",
        "\tbool \"After\"\n",
        "comment\t\"Debug \\\"notes\\\"\"\n",
        "\tdepends on DEBUG\n",
        "comment 'Single quoted note'\n",
    ))
    .unwrap();

    let comments = document.comments().collect::<Vec<_>>();
    assert_eq!(comments.len(), 3);
    assert_eq!(document.nodes().len(), 4);
    assert_eq!(
        comment_prompts(&document),
        vec!["Driver note", "Debug \"notes\"", "Single quoted note"]
    );
    assert_eq!(config_symbols(&document), vec!["AFTER"]);
    assert_eq!(comments[0].line(), 1);
    assert_eq!(comments[0].end_line(), 2);
    assert_eq!(
        comments[0].directive().text(),
        "comment \"Driver note\" # keep comment note"
    );
    assert_eq!(
        comments[0]
            .body()
            .iter()
            .map(KconfigRawLine::text)
            .collect::<Vec<_>>(),
        vec!["\tdepends on HAS_IOMEM"]
    );
    assert_eq!(
        comments[1].directive().text(),
        "comment\t\"Debug \\\"notes\\\"\""
    );
    assert_eq!(comments[1].line(), 5);
    assert_eq!(comments[1].end_line(), 6);
    assert_eq!(comments[2].directive().text(), "comment 'Single quoted note'");
    assert_eq!(comments[2].line(), 7);
    assert_eq!(comments[2].end_line(), 7);
}
#[test]
fn parse_kconfig_document_parses_endmenu_markers() {
    let document = parse_kconfig_document(concat!(
        "menu \"Drivers\"\n",
        "endmenu # keep end marker note\n",
        "config AFTER\n",
        "\tbool \"After\"\n",
    ))
    .unwrap();

    let endmenus = document.endmenus().collect::<Vec<_>>();
    assert_eq!(endmenus.len(), 1);
    assert_eq!(document.nodes().len(), 3);
    assert_eq!(endmenus[0].line(), 2);
    assert_eq!(endmenus[0].end_line(), 2);
    assert_eq!(
        endmenus[0].directive().text(),
        "endmenu # keep end marker note"
    );
    assert_eq!(config_symbols(&document), vec!["AFTER"]);
}
#[test]
fn parse_kconfig_document_parses_if_entries() {
    let document = parse_kconfig_document(concat!(
        "if NET && (PCI || USB) # keep directive note\n",
        "\tdepends on HAS_IOMEM\n",
        "config FOO\n",
        "\tbool \"Foo\"\n",
        "endif\n",
        "if\tEXPERT\n",
        "\tvisible if DEBUG\n",
        "endif\n",
    ))
    .unwrap();

    let ifs = document.ifs().collect::<Vec<_>>();
    assert_eq!(ifs.len(), 2);
    assert_eq!(document.nodes().len(), 5);
    assert_eq!(
        if_conditions(&document),
        vec!["NET && (PCI || USB)", "EXPERT"]
    );
    assert_eq!(endif_lines(&document), vec![5, 8]);
    assert_eq!(config_symbols(&document), vec!["FOO"]);
    assert_eq!(ifs[0].line(), 1);
    assert_eq!(ifs[0].end_line(), 2);
    assert_eq!(
        ifs[0].directive().text(),
        "if NET && (PCI || USB) # keep directive note"
    );
    assert_eq!(
        ifs[0]
            .body()
            .iter()
            .map(KconfigRawLine::text)
            .collect::<Vec<_>>(),
        vec!["\tdepends on HAS_IOMEM"]
    );
    assert_eq!(ifs[1].directive().text(), "if\tEXPERT");
    assert_eq!(ifs[1].line(), 6);
    assert_eq!(ifs[1].end_line(), 7);
}
#[test]
fn parse_kconfig_document_parses_endif_markers() {
    let document = parse_kconfig_document(concat!(
        "if FOO\n",
        "endif # keep end marker note\n",
        "config AFTER\n",
        "\tbool \"After\"\n",
    ))
    .unwrap();

    let endifs = document.endifs().collect::<Vec<_>>();
    assert_eq!(endifs.len(), 1);
    assert_eq!(document.nodes().len(), 3);
    assert_eq!(endifs[0].line(), 2);
    assert_eq!(endifs[0].end_line(), 2);
    assert_eq!(
        endifs[0].directive().text(),
        "endif # keep end marker note"
    );
    assert_eq!(if_conditions(&document), vec!["FOO"]);
    assert_eq!(config_symbols(&document), vec!["AFTER"]);
}
#[test]
fn parse_kconfig_document_parses_source_entries() {
    let document = parse_kconfig_document(concat!(
        "source \"drivers/foo/Kconfig\" # keep source note\n",
        "config AFTER\n",
        "\tbool \"After\"\n",
        "source\t\"arch/$ARCH/Kconfig\"\n",
    ))
    .unwrap();

    let sources = document.sources().collect::<Vec<_>>();
    assert_eq!(sources.len(), 2);
    assert_eq!(document.nodes().len(), 3);
    assert_eq!(
        source_paths(&document),
        vec!["drivers/foo/Kconfig", "arch/$ARCH/Kconfig"]
    );
    assert_eq!(config_symbols(&document), vec!["AFTER"]);
    assert_eq!(sources[0].line(), 1);
    assert_eq!(sources[0].end_line(), 1);
    assert_eq!(
        sources[0].directive().text(),
        "source \"drivers/foo/Kconfig\" # keep source note"
    );
    assert_eq!(sources[1].directive().text(), "source\t\"arch/$ARCH/Kconfig\"");
    assert_eq!(sources[1].line(), 4);
    assert_eq!(sources[1].end_line(), 4);
}
#[test]
fn parse_kconfig_document_parses_rsource_entries() {
    let document = parse_kconfig_document(concat!(
        "rsource \"subsys/Kconfig\" # keep rsource note\n",
        "config AFTER\n",
        "\tbool \"After\"\n",
        "rsource\t\"../shared/Kconfig\"\n",
    ))
    .unwrap();

    let rsources = document.rsources().collect::<Vec<_>>();
    assert_eq!(rsources.len(), 2);
    assert_eq!(document.nodes().len(), 3);
    assert_eq!(
        rsource_paths(&document),
        vec!["subsys/Kconfig", "../shared/Kconfig"]
    );
    assert_eq!(config_symbols(&document), vec!["AFTER"]);
    assert_eq!(rsources[0].line(), 1);
    assert_eq!(rsources[0].end_line(), 1);
    assert_eq!(
        rsources[0].directive().text(),
        "rsource \"subsys/Kconfig\" # keep rsource note"
    );
    assert_eq!(
        rsources[1].directive().text(),
        "rsource\t\"../shared/Kconfig\""
    );
    assert_eq!(rsources[1].line(), 4);
    assert_eq!(rsources[1].end_line(), 4);
}
#[test]
fn parse_kconfig_document_parses_osource_entries() {
    let document = parse_kconfig_document(concat!(
        "osource \"drivers/optional/Kconfig\" # keep osource note\n",
        "config AFTER\n",
        "\tbool \"After\"\n",
        "osource\t\"arch/$ARCH/Kconfig.optional\"\n",
    ))
    .unwrap();

    let osources = document.osources().collect::<Vec<_>>();
    assert_eq!(osources.len(), 2);
    assert_eq!(document.nodes().len(), 3);
    assert_eq!(
        osource_paths(&document),
        vec!["drivers/optional/Kconfig", "arch/$ARCH/Kconfig.optional"]
    );
    assert_eq!(config_symbols(&document), vec!["AFTER"]);
    assert_eq!(osources[0].line(), 1);
    assert_eq!(osources[0].end_line(), 1);
    assert_eq!(
        osources[0].directive().text(),
        "osource \"drivers/optional/Kconfig\" # keep osource note"
    );
    assert_eq!(
        osources[1].directive().text(),
        "osource\t\"arch/$ARCH/Kconfig.optional\""
    );
    assert_eq!(osources[1].line(), 4);
    assert_eq!(osources[1].end_line(), 4);
}
#[test]
fn parse_kconfig_document_parses_orsource_entries() {
    let document = parse_kconfig_document(concat!(
        "orsource \"optional/subsys/Kconfig\" # keep orsource note\n",
        "config AFTER\n",
        "\tbool \"After\"\n",
        "orsource\t\"../optional/Kconfig\"\n",
    ))
    .unwrap();

    let orsources = document.orsources().collect::<Vec<_>>();
    assert_eq!(orsources.len(), 2);
    assert_eq!(document.nodes().len(), 3);
    assert_eq!(
        orsource_paths(&document),
        vec!["optional/subsys/Kconfig", "../optional/Kconfig"]
    );
    assert_eq!(config_symbols(&document), vec!["AFTER"]);
    assert_eq!(orsources[0].line(), 1);
    assert_eq!(orsources[0].end_line(), 1);
    assert_eq!(
        orsources[0].directive().text(),
        "orsource \"optional/subsys/Kconfig\" # keep orsource note"
    );
    assert_eq!(
        orsources[1].directive().text(),
        "orsource\t\"../optional/Kconfig\""
    );
    assert_eq!(orsources[1].line(), 4);
    assert_eq!(orsources[1].end_line(), 4);
}
#[test]
fn parse_kconfig_document_parses_mainmenu_entries() {
    let document = parse_kconfig_document(concat!(
        "mainmenu \"Linux/kslim\" # keep mainmenu note\n",
        "config AFTER\n",
        "\tbool \"After\"\n",
        "mainmenu\t\"Second \\\"menu\\\"\"\n",
    ))
    .unwrap();

    let mainmenus = document.mainmenus().collect::<Vec<_>>();
    assert_eq!(mainmenus.len(), 2);
    assert_eq!(document.nodes().len(), 3);
    assert_eq!(
        mainmenu_prompts(&document),
        vec!["Linux/kslim", "Second \"menu\""]
    );
    assert_eq!(config_symbols(&document), vec!["AFTER"]);
    assert_eq!(mainmenus[0].line(), 1);
    assert_eq!(mainmenus[0].end_line(), 1);
    assert_eq!(
        mainmenus[0].directive().text(),
        "mainmenu \"Linux/kslim\" # keep mainmenu note"
    );
    assert_eq!(
        mainmenus[1].directive().text(),
        "mainmenu\t\"Second \\\"menu\\\"\""
    );
    assert_eq!(mainmenus[1].line(), 4);
    assert_eq!(mainmenus[1].end_line(), 4);
}

#[test]
fn parse_kconfig_document_parses_help_blocks() {
    let document = parse_kconfig_document(concat!(
        "config LIVE\n",
        "\tbool \"Live\"\n",
        "\thelp\n",
        "\t  config NOT_A_SYMBOL\n",
        "\t    bool \"not syntax\"\n",
        "\tdefault y\n",
        "menuconfig NETDEVICES\n",
        "\tbool \"Network\"\n",
        "\t---help---\n",
        "\t  Menuconfig help\n",
        "choice PICK\n",
        "\tprompt \"Pick\"\n",
        "\thelp\n",
        "\t  Choice help\n",
        "endchoice\n",
    ))
    .unwrap();

    let help_blocks = document.help_blocks().collect::<Vec<_>>();
    let config = document.configs().next().unwrap();
    let menuconfig = document.menuconfigs().next().unwrap();
    let choice = document.choices().next().unwrap();

    assert_eq!(document.nodes().len(), 4);
    assert_eq!(config_symbols(&document), vec!["LIVE"]);
    assert_eq!(help_blocks.len(), 3);
    assert_eq!(config.help_blocks().len(), 1);
    assert_eq!(menuconfig.help_blocks().len(), 1);
    assert_eq!(choice.help_blocks().len(), 1);

    assert_eq!(help_blocks[0].line(), 3);
    assert_eq!(help_blocks[0].end_line(), 5);
    assert_eq!(help_blocks[0].directive().text(), "\thelp");
    assert_eq!(
        help_text(help_blocks[0]),
        vec!["\t  config NOT_A_SYMBOL", "\t    bool \"not syntax\""]
    );
    assert_eq!(config.end_line(), 6);
    assert_eq!(
        config
            .body()
            .iter()
            .map(KconfigRawLine::text)
            .collect::<Vec<_>>(),
        vec![
            "\tbool \"Live\"",
            "\thelp",
            "\t  config NOT_A_SYMBOL",
            "\t    bool \"not syntax\"",
            "\tdefault y",
        ]
    );

    assert_eq!(help_blocks[1].line(), 9);
    assert_eq!(help_blocks[1].end_line(), 10);
    assert_eq!(help_blocks[1].directive().text(), "\t---help---");
    assert_eq!(help_text(help_blocks[1]), vec!["\t  Menuconfig help"]);

    assert_eq!(help_blocks[2].line(), 13);
    assert_eq!(help_blocks[2].end_line(), 14);
    assert_eq!(help_blocks[2].directive().text(), "\thelp");
    assert_eq!(help_text(help_blocks[2]), vec!["\t  Choice help"]);
}
