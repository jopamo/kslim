use super::*;

#[test]
fn parse_kconfig_document_models_bool_type_definitions() {
    let document = parse_kconfig_document(concat!(
        "config BOOL_ONLY\n",
        "\tbool\n",
        "\thelp\n",
        "\t  bool \"not a type\"\n",
        "menuconfig BOOL_PROMPT\n",
        "\tbool \"Prompt\" if EXPERT # keep type note\n",
        "choice PICK\n",
        "\tbool \"Pick\"\n",
        "endchoice\n",
    ))
    .unwrap();

    let type_definitions = document.type_definitions().collect::<Vec<_>>();
    let config = document.configs().next().unwrap();
    let menuconfig = document.menuconfigs().next().unwrap();
    let choice = document.choices().next().unwrap();

    assert_eq!(document.nodes().len(), 4);
    assert_eq!(
        type_kinds(&document),
        vec![
            KconfigSymbolType::Bool,
            KconfigSymbolType::Bool,
            KconfigSymbolType::Bool
        ]
    );
    assert_eq!(type_definitions.len(), 3);
    assert_eq!(config.type_definitions().len(), 1);
    assert_eq!(menuconfig.type_definitions().len(), 1);
    assert_eq!(choice.type_definitions().len(), 1);

    assert_eq!(type_definitions[0].kind(), KconfigSymbolType::Bool);
    assert_eq!(type_definitions[0].line(), 2);
    assert_eq!(type_definitions[0].end_line(), 2);
    assert_eq!(type_definitions[0].directive().text(), "\tbool");
    assert_eq!(config.help_blocks().len(), 1);

    assert_eq!(type_definitions[1].kind(), KconfigSymbolType::Bool);
    assert_eq!(type_definitions[1].line(), 6);
    assert_eq!(type_definitions[1].end_line(), 6);
    assert_eq!(
        type_definitions[1].directive().text(),
        "\tbool \"Prompt\" if EXPERT # keep type note"
    );

    assert_eq!(type_definitions[2].kind(), KconfigSymbolType::Bool);
    assert_eq!(type_definitions[2].line(), 8);
    assert_eq!(type_definitions[2].end_line(), 8);
    assert_eq!(type_definitions[2].directive().text(), "\tbool \"Pick\"");
}
#[test]
fn parse_kconfig_document_models_tristate_type_definitions() {
    let document = parse_kconfig_document(concat!(
        "config TRI_ONLY\n",
        "\ttristate\n",
        "\thelp\n",
        "\t  tristate \"not a type\"\n",
        "menuconfig TRI_PROMPT\n",
        "\ttristate \"Prompt\" if MODULES # keep type note\n",
        "choice PICK\n",
        "\ttristate \"Pick\"\n",
        "endchoice\n",
    ))
    .unwrap();

    let type_definitions = document.type_definitions().collect::<Vec<_>>();
    let config = document.configs().next().unwrap();
    let menuconfig = document.menuconfigs().next().unwrap();
    let choice = document.choices().next().unwrap();

    assert_eq!(
        type_kinds(&document),
        vec![
            KconfigSymbolType::Tristate,
            KconfigSymbolType::Tristate,
            KconfigSymbolType::Tristate
        ]
    );
    assert_eq!(type_definitions.len(), 3);
    assert_eq!(config.type_definitions().len(), 1);
    assert_eq!(menuconfig.type_definitions().len(), 1);
    assert_eq!(choice.type_definitions().len(), 1);

    assert_eq!(type_definitions[0].kind(), KconfigSymbolType::Tristate);
    assert_eq!(type_definitions[0].line(), 2);
    assert_eq!(type_definitions[0].end_line(), 2);
    assert_eq!(type_definitions[0].directive().text(), "\ttristate");
    assert_eq!(config.help_blocks().len(), 1);

    assert_eq!(type_definitions[1].kind(), KconfigSymbolType::Tristate);
    assert_eq!(type_definitions[1].line(), 6);
    assert_eq!(type_definitions[1].end_line(), 6);
    assert_eq!(
        type_definitions[1].directive().text(),
        "\ttristate \"Prompt\" if MODULES # keep type note"
    );

    assert_eq!(type_definitions[2].kind(), KconfigSymbolType::Tristate);
    assert_eq!(type_definitions[2].line(), 8);
    assert_eq!(type_definitions[2].end_line(), 8);
    assert_eq!(
        type_definitions[2].directive().text(),
        "\ttristate \"Pick\""
    );
}
#[test]
fn parse_kconfig_document_models_string_type_definitions() {
    let document = parse_kconfig_document(concat!(
        "config STR_ONLY\n",
        "\tstring\n",
        "\thelp\n",
        "\t  string \"not a type\"\n",
        "menuconfig STR_PROMPT\n",
        "\tstring \"Prompt\" if EXPERT # keep type note\n",
        "choice PICK\n",
        "\tstring \"Pick\"\n",
        "endchoice\n",
    ))
    .unwrap();

    let type_definitions = document.type_definitions().collect::<Vec<_>>();
    let config = document.configs().next().unwrap();
    let menuconfig = document.menuconfigs().next().unwrap();
    let choice = document.choices().next().unwrap();

    assert_eq!(
        type_kinds(&document),
        vec![
            KconfigSymbolType::String,
            KconfigSymbolType::String,
            KconfigSymbolType::String
        ]
    );
    assert_eq!(type_definitions.len(), 3);
    assert_eq!(config.type_definitions().len(), 1);
    assert_eq!(menuconfig.type_definitions().len(), 1);
    assert_eq!(choice.type_definitions().len(), 1);

    assert_eq!(type_definitions[0].kind(), KconfigSymbolType::String);
    assert_eq!(type_definitions[0].line(), 2);
    assert_eq!(type_definitions[0].end_line(), 2);
    assert_eq!(type_definitions[0].directive().text(), "\tstring");
    assert_eq!(config.help_blocks().len(), 1);

    assert_eq!(type_definitions[1].kind(), KconfigSymbolType::String);
    assert_eq!(type_definitions[1].line(), 6);
    assert_eq!(type_definitions[1].end_line(), 6);
    assert_eq!(
        type_definitions[1].directive().text(),
        "\tstring \"Prompt\" if EXPERT # keep type note"
    );

    assert_eq!(type_definitions[2].kind(), KconfigSymbolType::String);
    assert_eq!(type_definitions[2].line(), 8);
    assert_eq!(type_definitions[2].end_line(), 8);
    assert_eq!(
        type_definitions[2].directive().text(),
        "\tstring \"Pick\""
    );
}
#[test]
fn parse_kconfig_document_models_int_type_definitions() {
    let document = parse_kconfig_document(concat!(
        "config INT_ONLY\n",
        "\tint\n",
        "\thelp\n",
        "\t  int \"not a type\"\n",
        "menuconfig INT_PROMPT\n",
        "\tint \"Prompt\" if EXPERT # keep type note\n",
        "choice PICK\n",
        "\tint \"Pick\"\n",
        "endchoice\n",
    ))
    .unwrap();

    let type_definitions = document.type_definitions().collect::<Vec<_>>();
    let config = document.configs().next().unwrap();
    let menuconfig = document.menuconfigs().next().unwrap();
    let choice = document.choices().next().unwrap();

    assert_eq!(
        type_kinds(&document),
        vec![
            KconfigSymbolType::Int,
            KconfigSymbolType::Int,
            KconfigSymbolType::Int
        ]
    );
    assert_eq!(type_definitions.len(), 3);
    assert_eq!(config.type_definitions().len(), 1);
    assert_eq!(menuconfig.type_definitions().len(), 1);
    assert_eq!(choice.type_definitions().len(), 1);

    assert_eq!(type_definitions[0].kind(), KconfigSymbolType::Int);
    assert_eq!(type_definitions[0].line(), 2);
    assert_eq!(type_definitions[0].end_line(), 2);
    assert_eq!(type_definitions[0].directive().text(), "\tint");
    assert_eq!(config.help_blocks().len(), 1);

    assert_eq!(type_definitions[1].kind(), KconfigSymbolType::Int);
    assert_eq!(type_definitions[1].line(), 6);
    assert_eq!(type_definitions[1].end_line(), 6);
    assert_eq!(
        type_definitions[1].directive().text(),
        "\tint \"Prompt\" if EXPERT # keep type note"
    );

    assert_eq!(type_definitions[2].kind(), KconfigSymbolType::Int);
    assert_eq!(type_definitions[2].line(), 8);
    assert_eq!(type_definitions[2].end_line(), 8);
    assert_eq!(type_definitions[2].directive().text(), "\tint \"Pick\"");
}
#[test]
fn parse_kconfig_document_models_hex_type_definitions() {
    let document = parse_kconfig_document(concat!(
        "config HEX_ONLY\n",
        "\thex\n",
        "\thelp\n",
        "\t  hex \"not a type\"\n",
        "menuconfig HEX_PROMPT\n",
        "\thex \"Prompt\" if EXPERT # keep type note\n",
        "choice PICK\n",
        "\thex \"Pick\"\n",
        "endchoice\n",
    ))
    .unwrap();

    let type_definitions = document.type_definitions().collect::<Vec<_>>();
    let config = document.configs().next().unwrap();
    let menuconfig = document.menuconfigs().next().unwrap();
    let choice = document.choices().next().unwrap();

    assert_eq!(
        type_kinds(&document),
        vec![
            KconfigSymbolType::Hex,
            KconfigSymbolType::Hex,
            KconfigSymbolType::Hex
        ]
    );
    assert_eq!(type_definitions.len(), 3);
    assert_eq!(config.type_definitions().len(), 1);
    assert_eq!(menuconfig.type_definitions().len(), 1);
    assert_eq!(choice.type_definitions().len(), 1);

    assert_eq!(type_definitions[0].kind(), KconfigSymbolType::Hex);
    assert_eq!(type_definitions[0].line(), 2);
    assert_eq!(type_definitions[0].end_line(), 2);
    assert_eq!(type_definitions[0].directive().text(), "\thex");
    assert_eq!(config.help_blocks().len(), 1);

    assert_eq!(type_definitions[1].kind(), KconfigSymbolType::Hex);
    assert_eq!(type_definitions[1].line(), 6);
    assert_eq!(type_definitions[1].end_line(), 6);
    assert_eq!(
        type_definitions[1].directive().text(),
        "\thex \"Prompt\" if EXPERT # keep type note"
    );

    assert_eq!(type_definitions[2].kind(), KconfigSymbolType::Hex);
    assert_eq!(type_definitions[2].line(), 8);
    assert_eq!(type_definitions[2].end_line(), 8);
    assert_eq!(type_definitions[2].directive().text(), "\thex \"Pick\"");
}
#[test]
fn parse_kconfig_document_models_prompt_visibility() {
    let document = parse_kconfig_document(concat!(
        "config TYPE_PROMPT\n",
        "\tbool \"Typed Prompt\" if EXPERT # keep prompt note\n",
        "\thelp\n",
        "\t  prompt \"not a prompt\" if BROKEN\n",
        "menuconfig EXPLICIT_PROMPT\n",
        "\ttristate\n",
        "\tprompt \"Explicit Prompt\" if MODULES\n",
        "choice PICK\n",
        "\tprompt \"Choose one\"\n",
        "\tbool \"Choice typed prompt\" if CHOICE_VISIBLE\n",
        "endchoice\n",
    ))
    .unwrap();

    let prompt_definitions = document.prompt_definitions().collect::<Vec<_>>();
    let config = document.configs().next().unwrap();
    let menuconfig = document.menuconfigs().next().unwrap();
    let choice = document.choices().next().unwrap();

    assert_eq!(
        prompt_texts(&document),
        vec![
            "Typed Prompt",
            "Explicit Prompt",
            "Choose one",
            "Choice typed prompt"
        ]
    );
    assert_eq!(
        prompt_conditions(&document),
        vec![Some("EXPERT"), Some("MODULES"), None, Some("CHOICE_VISIBLE")]
    );
    assert_eq!(prompt_definitions.len(), 4);
    assert_eq!(config.prompt_definitions().len(), 1);
    assert_eq!(menuconfig.prompt_definitions().len(), 1);
    assert_eq!(choice.prompt_definitions().len(), 2);

    assert_eq!(prompt_definitions[0].line(), 2);
    assert_eq!(prompt_definitions[0].end_line(), 2);
    assert_eq!(
        prompt_definitions[0].directive().text(),
        "\tbool \"Typed Prompt\" if EXPERT # keep prompt note"
    );
    assert_eq!(config.help_blocks().len(), 1);

    assert_eq!(prompt_definitions[1].line(), 7);
    assert_eq!(
        prompt_definitions[1].directive().text(),
        "\tprompt \"Explicit Prompt\" if MODULES"
    );
    assert_eq!(prompt_definitions[2].condition(), None);
    assert_eq!(prompt_definitions[2].directive().text(), "\tprompt \"Choose one\"");
    assert_eq!(prompt_definitions[3].condition(), Some("CHOICE_VISIBLE"));
}
#[test]
fn parse_kconfig_document_models_defaults() {
    let document = parse_kconfig_document(concat!(
        "config BOOL_DEFAULT\n",
        "\tbool \"Bool\"\n",
        "\tdefault y if EXPERT # keep default note\n",
        "\thelp\n",
        "\t  default n if BROKEN\n",
        "menuconfig STRING_DEFAULT\n",
        "\tstring\n",
        "\tdefault \"hello world\"\n",
        "choice PICK\n",
        "\tdefault FIRST if CHOICE_VISIBLE\n",
        "endchoice\n",
    ))
    .unwrap();

    let default_definitions = document.default_definitions().collect::<Vec<_>>();
    let config = document.configs().next().unwrap();
    let menuconfig = document.menuconfigs().next().unwrap();
    let choice = document.choices().next().unwrap();

    assert_eq!(
        default_values(&document),
        vec!["y", "\"hello world\"", "FIRST"]
    );
    assert_eq!(
        default_conditions(&document),
        vec![Some("EXPERT"), None, Some("CHOICE_VISIBLE")]
    );
    assert_eq!(default_definitions.len(), 3);
    assert_eq!(config.default_definitions().len(), 1);
    assert_eq!(menuconfig.default_definitions().len(), 1);
    assert_eq!(choice.default_definitions().len(), 1);

    assert_eq!(default_definitions[0].line(), 3);
    assert_eq!(default_definitions[0].end_line(), 3);
    assert_eq!(
        default_definitions[0].directive().text(),
        "\tdefault y if EXPERT # keep default note"
    );
    assert_eq!(config.help_blocks().len(), 1);

    assert_eq!(default_definitions[1].line(), 8);
    assert_eq!(default_definitions[1].condition(), None);
    assert_eq!(default_definitions[1].directive().text(), "\tdefault \"hello world\"");
    assert_eq!(default_definitions[2].value(), "FIRST");
    assert_eq!(default_definitions[2].condition(), Some("CHOICE_VISIBLE"));
}
#[test]
fn parse_kconfig_document_models_ranges() {
    let document = parse_kconfig_document(concat!(
        "config INT_RANGE\n",
        "\tint \"Integer\"\n",
        "\trange 0 255 if EXPERT # keep range note\n",
        "\thelp\n",
        "\t  range 1 2 if BROKEN\n",
        "menuconfig HEX_RANGE\n",
        "\thex\n",
        "\trange 0x10 0xff\n",
        "choice PICK_RANGE\n",
        "\trange FIRST LAST if CHOICE_VISIBLE\n",
        "endchoice\n",
    ))
    .unwrap();

    let range_definitions = document.range_definitions().collect::<Vec<_>>();
    let config = document.configs().next().unwrap();
    let menuconfig = document.menuconfigs().next().unwrap();
    let choice = document.choices().next().unwrap();

    assert_eq!(range_minimums(&document), vec!["0", "0x10", "FIRST"]);
    assert_eq!(range_maximums(&document), vec!["255", "0xff", "LAST"]);
    assert_eq!(
        range_conditions(&document),
        vec![Some("EXPERT"), None, Some("CHOICE_VISIBLE")]
    );
    assert_eq!(range_definitions.len(), 3);
    assert_eq!(config.range_definitions().len(), 1);
    assert_eq!(menuconfig.range_definitions().len(), 1);
    assert_eq!(choice.range_definitions().len(), 1);

    assert_eq!(range_definitions[0].line(), 3);
    assert_eq!(range_definitions[0].end_line(), 3);
    assert_eq!(
        range_definitions[0].directive().text(),
        "\trange 0 255 if EXPERT # keep range note"
    );
    assert_eq!(config.help_blocks().len(), 1);

    assert_eq!(range_definitions[1].line(), 8);
    assert_eq!(range_definitions[1].condition(), None);
    assert_eq!(range_definitions[1].directive().text(), "\trange 0x10 0xff");
    assert_eq!(range_definitions[2].minimum(), "FIRST");
    assert_eq!(range_definitions[2].maximum(), "LAST");
    assert_eq!(range_definitions[2].condition(), Some("CHOICE_VISIBLE"));
}
#[test]
fn parse_kconfig_document_models_dependencies() {
    let document = parse_kconfig_document(concat!(
        "config DEP_CONFIG\n",
        "\tbool \"Dependent\"\n",
        "\tdepends on NET && (PCI || USB) # keep dependency note\n",
        "\thelp\n",
        "\t  depends on BROKEN\n",
        "menuconfig DEP_MENUCONFIG\n",
        "\tbool\n",
        "\tdepends on MODULES\n",
        "choice DEP_CHOICE\n",
        "\tdepends on CHOICE_VISIBLE || EXPERT\n",
        "endchoice\n",
    ))
    .unwrap();

    let dependency_definitions = document.dependency_definitions().collect::<Vec<_>>();
    let config = document.configs().next().unwrap();
    let menuconfig = document.menuconfigs().next().unwrap();
    let choice = document.choices().next().unwrap();

    assert_eq!(
        dependency_expressions(&document),
        vec![
            "NET && (PCI || USB)",
            "MODULES",
            "CHOICE_VISIBLE || EXPERT"
        ]
    );
    assert_eq!(dependency_definitions.len(), 3);
    assert_eq!(config.dependency_definitions().len(), 1);
    assert_eq!(menuconfig.dependency_definitions().len(), 1);
    assert_eq!(choice.dependency_definitions().len(), 1);

    assert_eq!(dependency_definitions[0].line(), 3);
    assert_eq!(dependency_definitions[0].end_line(), 3);
    assert_eq!(
        dependency_definitions[0].directive().text(),
        "\tdepends on NET && (PCI || USB) # keep dependency note"
    );
    assert_eq!(config.help_blocks().len(), 1);

    assert_eq!(dependency_definitions[1].line(), 8);
    assert_eq!(dependency_definitions[1].expression(), "MODULES");
    assert_eq!(
        dependency_definitions[1].directive().text(),
        "\tdepends on MODULES"
    );
    assert_eq!(
        dependency_definitions[2].expression(),
        "CHOICE_VISIBLE || EXPERT"
    );
}
#[test]
fn parse_kconfig_document_models_reverse_dependencies_through_select() {
    let document = parse_kconfig_document(concat!(
        "config SELECT_CONFIG\n",
        "\tbool \"Selector\"\n",
        "\tselect NET_CORE if NET # keep select note\n",
        "\thelp\n",
        "\t  select BROKEN if HELP_TEXT\n",
        "menuconfig SELECT_MENUCONFIG\n",
        "\tbool\n",
        "\tselect RFKILL\n",
        "choice SELECT_CHOICE\n",
        "\tselect CHOICE_TARGET if CHOICE_VISIBLE\n",
        "endchoice\n",
    ))
    .unwrap();

    let select_definitions = document.select_definitions().collect::<Vec<_>>();
    let config = document.configs().next().unwrap();
    let menuconfig = document.menuconfigs().next().unwrap();
    let choice = document.choices().next().unwrap();

    assert_eq!(select_targets(&document), vec!["NET_CORE", "RFKILL", "CHOICE_TARGET"]);
    assert_eq!(
        select_conditions(&document),
        vec![Some("NET"), None, Some("CHOICE_VISIBLE")]
    );
    assert_eq!(select_definitions.len(), 3);
    assert_eq!(config.select_definitions().len(), 1);
    assert_eq!(menuconfig.select_definitions().len(), 1);
    assert_eq!(choice.select_definitions().len(), 1);

    assert_eq!(select_definitions[0].line(), 3);
    assert_eq!(select_definitions[0].end_line(), 3);
    assert_eq!(
        select_definitions[0].directive().text(),
        "\tselect NET_CORE if NET # keep select note"
    );
    assert_eq!(config.help_blocks().len(), 1);

    assert_eq!(select_definitions[1].line(), 8);
    assert_eq!(select_definitions[1].condition(), None);
    assert_eq!(select_definitions[1].directive().text(), "\tselect RFKILL");
    assert_eq!(select_definitions[2].target().as_str(), "CHOICE_TARGET");
    assert_eq!(select_definitions[2].condition(), Some("CHOICE_VISIBLE"));
}
#[test]
fn parse_kconfig_document_models_weak_reverse_dependencies_through_imply() {
    let document = parse_kconfig_document(concat!(
        "config IMPLY_CONFIG\n",
        "\tbool \"Implying\"\n",
        "\timply NET_CORE if NET # keep imply note\n",
        "\thelp\n",
        "\t  imply BROKEN if HELP_TEXT\n",
        "menuconfig IMPLY_MENUCONFIG\n",
        "\tbool\n",
        "\timply RFKILL\n",
        "choice IMPLY_CHOICE\n",
        "\timply CHOICE_TARGET if CHOICE_VISIBLE\n",
        "endchoice\n",
    ))
    .unwrap();

    let imply_definitions = document.imply_definitions().collect::<Vec<_>>();
    let config = document.configs().next().unwrap();
    let menuconfig = document.menuconfigs().next().unwrap();
    let choice = document.choices().next().unwrap();

    assert_eq!(imply_targets(&document), vec!["NET_CORE", "RFKILL", "CHOICE_TARGET"]);
    assert_eq!(
        imply_conditions(&document),
        vec![Some("NET"), None, Some("CHOICE_VISIBLE")]
    );
    assert_eq!(imply_definitions.len(), 3);
    assert_eq!(config.imply_definitions().len(), 1);
    assert_eq!(menuconfig.imply_definitions().len(), 1);
    assert_eq!(choice.imply_definitions().len(), 1);

    assert_eq!(imply_definitions[0].line(), 3);
    assert_eq!(imply_definitions[0].end_line(), 3);
    assert_eq!(
        imply_definitions[0].directive().text(),
        "\timply NET_CORE if NET # keep imply note"
    );
    assert_eq!(config.help_blocks().len(), 1);

    assert_eq!(imply_definitions[1].line(), 8);
    assert_eq!(imply_definitions[1].condition(), None);
    assert_eq!(imply_definitions[1].directive().text(), "\timply RFKILL");
    assert_eq!(imply_definitions[2].target().as_str(), "CHOICE_TARGET");
    assert_eq!(imply_definitions[2].condition(), Some("CHOICE_VISIBLE"));
}
#[test]
fn parse_kconfig_document_models_option() {
    let document = parse_kconfig_document(concat!(
        "config OPTION_CONFIG\n",
        "\tstring \"Optioned\"\n",
        "\toption env=\"CONFIG_OPTION_ENV\" # keep option note\n",
        "\thelp\n",
        "\t  option ignored=HELP_TEXT\n",
        "menuconfig OPTION_MENUCONFIG\n",
        "\tbool\n",
        "\toption allnoconfig_y\n",
        "choice OPTION_CHOICE\n",
        "\toption defconfig_list\n",
        "endchoice\n",
    ))
    .unwrap();

    let option_definitions = document.option_definitions().collect::<Vec<_>>();
    let config = document.configs().next().unwrap();
    let menuconfig = document.menuconfigs().next().unwrap();
    let choice = document.choices().next().unwrap();

    assert_eq!(
        option_names(&document),
        vec!["env", "allnoconfig_y", "defconfig_list"]
    );
    assert_eq!(
        option_values(&document),
        vec![Some("\"CONFIG_OPTION_ENV\""), None, None]
    );
    assert_eq!(option_definitions.len(), 3);
    assert_eq!(config.option_definitions().len(), 1);
    assert_eq!(menuconfig.option_definitions().len(), 1);
    assert_eq!(choice.option_definitions().len(), 1);

    assert_eq!(option_definitions[0].line(), 3);
    assert_eq!(option_definitions[0].end_line(), 3);
    assert_eq!(
        option_definitions[0].directive().text(),
        "\toption env=\"CONFIG_OPTION_ENV\" # keep option note"
    );
    assert_eq!(config.help_blocks().len(), 1);

    assert_eq!(option_definitions[1].line(), 8);
    assert_eq!(option_definitions[1].value(), None);
    assert_eq!(option_definitions[1].directive().text(), "\toption allnoconfig_y");
    assert_eq!(option_definitions[2].name(), "defconfig_list");
    assert_eq!(option_definitions[2].value(), None);
}
#[test]
fn parse_kconfig_document_models_modules() {
    let document = parse_kconfig_document(concat!(
        "config MODULES_CONFIG\n",
        "\tbool \"Modules\"\n",
        "\tmodules # keep modules note\n",
        "\thelp\n",
        "\t  modules\n",
        "menuconfig MODULES_MENUCONFIG\n",
        "\tbool\n",
        "\tmodules\n",
        "choice MODULES_CHOICE\n",
        "\tmodules\n",
        "endchoice\n",
    ))
    .unwrap();

    let modules_definitions = document.modules_definitions().collect::<Vec<_>>();
    let config = document.configs().next().unwrap();
    let menuconfig = document.menuconfigs().next().unwrap();
    let choice = document.choices().next().unwrap();

    assert_eq!(modules_lines(&document), vec![3, 8, 10]);
    assert_eq!(modules_definitions.len(), 3);
    assert_eq!(config.modules_definitions().len(), 1);
    assert_eq!(menuconfig.modules_definitions().len(), 1);
    assert_eq!(choice.modules_definitions().len(), 1);

    assert_eq!(modules_definitions[0].line(), 3);
    assert_eq!(modules_definitions[0].end_line(), 3);
    assert_eq!(
        modules_definitions[0].directive().text(),
        "\tmodules # keep modules note"
    );
    assert_eq!(config.help_blocks().len(), 1);

    assert_eq!(modules_definitions[1].line(), 8);
    assert_eq!(modules_definitions[1].directive().text(), "\tmodules");
    assert_eq!(modules_definitions[2].line(), 10);
    assert_eq!(modules_definitions[2].directive().text(), "\tmodules");
}
#[test]
fn parse_kconfig_document_models_multiple_symbol_definitions() {
    let document = parse_kconfig_document(concat!(
        "config DUP_SYMBOL\n",
        "\tbool \"First definition\"\n",
        "menuconfig UNIQUE_MENU\n",
        "\tbool \"Unique menu\"\n",
        "menuconfig DUP_SYMBOL\n",
        "\ttristate \"Second definition\"\n",
        "choice CHOICE_SYMBOL\n",
        "\tprompt \"Named choice\"\n",
        "endchoice\n",
        "choice\n",
        "\tprompt \"Anonymous choice\"\n",
        "endchoice\n",
        "config UNIQUE_CONFIG\n",
        "\tbool \"Unique config\"\n",
    ))
    .unwrap();

    let definitions = document.symbol_definitions().collect::<Vec<_>>();
    let groups = document.symbol_definition_groups();
    let multiple_groups = document.multiple_symbol_definition_groups();
    let duplicate_group = multiple_groups.first().unwrap();

    assert_eq!(
        symbol_definition_symbols(&document),
        vec![
            "DUP_SYMBOL",
            "UNIQUE_MENU",
            "DUP_SYMBOL",
            "CHOICE_SYMBOL",
            "UNIQUE_CONFIG"
        ]
    );
    assert_eq!(
        symbol_definition_kinds(&document),
        vec![
            KconfigSymbolDefinitionKind::Config,
            KconfigSymbolDefinitionKind::Menuconfig,
            KconfigSymbolDefinitionKind::Menuconfig,
            KconfigSymbolDefinitionKind::Choice,
            KconfigSymbolDefinitionKind::Config,
        ]
    );
    assert_eq!(
        symbol_definition_group_symbols(&document),
        vec![
            "CHOICE_SYMBOL",
            "DUP_SYMBOL",
            "UNIQUE_CONFIG",
            "UNIQUE_MENU"
        ]
    );
    assert_eq!(
        multiple_symbol_definition_group_symbols(&document),
        vec!["DUP_SYMBOL"]
    );
    assert_eq!(definitions.len(), 5);
    assert_eq!(groups.len(), 4);
    assert_eq!(multiple_groups.len(), 1);
    assert_eq!(duplicate_group.definitions().len(), 2);
    assert!(duplicate_group.is_multiple());

    assert_eq!(definitions[0].line(), 1);
    assert_eq!(definitions[0].end_line(), 2);
    assert_eq!(definitions[0].directive().text(), "config DUP_SYMBOL");
    assert_eq!(definitions[0].type_definitions().len(), 1);
    assert_eq!(definitions[0].prompt_definitions().len(), 1);

    assert_eq!(definitions[2].line(), 5);
    assert_eq!(definitions[2].kind(), KconfigSymbolDefinitionKind::Menuconfig);
    assert_eq!(definitions[2].symbol().as_str(), "DUP_SYMBOL");
    assert_eq!(definitions[3].kind(), KconfigSymbolDefinitionKind::Choice);
    assert_eq!(definitions[3].symbol().as_str(), "CHOICE_SYMBOL");

    assert_eq!(duplicate_group.definitions()[0].line(), 1);
    assert_eq!(duplicate_group.definitions()[1].line(), 5);
}
#[test]
fn parse_kconfig_document_validates_type_consistency_across_definitions() {
    let document = parse_kconfig_document(concat!(
        "config SAME_TYPE\n",
        "\tbool \"First same\"\n",
        "menuconfig SAME_TYPE\n",
        "\tbool \"Second same\"\n",
        "config TYPE_CONFLICT\n",
        "\tbool \"Bool definition\"\n",
        "menuconfig TYPE_CONFLICT\n",
        "\ttristate \"Tristate definition\"\n",
        "choice CHOICE_CONFLICT\n",
        "\tbool \"Bool choice\"\n",
        "endchoice\n",
        "choice CHOICE_CONFLICT\n",
        "\ttristate \"Tristate choice\"\n",
        "endchoice\n",
    ))
    .unwrap();

    let violations = document.type_consistency_violations();
    let symbols = violations
        .iter()
        .map(|violation| violation.symbol().as_str())
        .collect::<Vec<_>>();

    assert_eq!(symbols, vec!["CHOICE_CONFLICT", "TYPE_CONFLICT"]);
    assert_eq!(
        violations[1]
            .definitions()
            .iter()
            .map(KconfigTypeConsistencyDefinition::kind)
            .collect::<Vec<_>>(),
        vec![KconfigSymbolType::Bool, KconfigSymbolType::Tristate]
    );
    assert_eq!(
        violations[1]
            .definitions()
            .iter()
            .map(KconfigTypeConsistencyDefinition::symbol_definition_kind)
            .collect::<Vec<_>>(),
        vec![
            KconfigSymbolDefinitionKind::Config,
            KconfigSymbolDefinitionKind::Menuconfig
        ]
    );
    assert_eq!(violations[1].definitions()[0].definition_line(), 5);
    assert_eq!(violations[1].definitions()[0].type_line(), 6);
    assert_eq!(
        violations[1].definitions()[0].directive().text(),
        "\tbool \"Bool definition\""
    );
    assert_eq!(violations[1].definitions()[1].definition_line(), 7);
    assert_eq!(violations[1].definitions()[1].type_line(), 8);
}
#[test]
fn parse_kconfig_document_validates_prompt_consistency_policy() {
    let document = parse_kconfig_document(concat!(
        "config SINGLE_PROMPT\n",
        "\tbool \"One\"\n",
        "config CONFIG_PROMPT_CONFLICT\n",
        "\tbool \"Typed\"\n",
        "\tprompt \"Second\" if EXPERT\n",
        "menuconfig MENU_PROMPT_CONFLICT\n",
        "\ttristate\n",
        "\tprompt \"First\"\n",
        "\tprompt \"Second\" if MODULES\n",
        "choice CHOICE_PROMPT_CONFLICT\n",
        "\tbool \"Typed choice\"\n",
        "\tprompt \"Explicit choice\"\n",
        "endchoice\n",
    ))
    .unwrap();

    let violations = document.prompt_consistency_violations();
    let symbols = violations
        .iter()
        .map(|violation| violation.symbol().as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        symbols,
        vec![
            "CONFIG_PROMPT_CONFLICT",
            "MENU_PROMPT_CONFLICT",
            "CHOICE_PROMPT_CONFLICT"
        ]
    );
    assert_eq!(
        violations[0]
            .prompts()
            .iter()
            .map(KconfigPromptConsistencyDefinition::prompt)
            .collect::<Vec<_>>(),
        vec!["Typed", "Second"]
    );
    assert_eq!(violations[0].prompts()[1].condition(), Some("EXPERT"));
    assert_eq!(
        violations[0].prompts()[0].symbol_definition_kind(),
        KconfigSymbolDefinitionKind::Config
    );
    assert_eq!(violations[0].prompts()[0].definition_line(), 3);
    assert_eq!(violations[0].prompts()[0].prompt_line(), 4);
    assert_eq!(
        violations[0].prompts()[0].directive().text(),
        "\tbool \"Typed\""
    );
    assert_eq!(violations[1].prompts()[0].prompt_line(), 8);
    assert_eq!(
        violations[1].prompts()[0].symbol_definition_kind(),
        KconfigSymbolDefinitionKind::Menuconfig
    );
    assert_eq!(
        violations[2].prompts()[0].symbol_definition_kind(),
        KconfigSymbolDefinitionKind::Choice
    );
}
