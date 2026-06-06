use super::*;

fn config_symbols(document: &KconfigDocument) -> Vec<&str> {
    document
        .configs()
        .map(|config| config.symbol().as_str())
        .collect()
}

fn menuconfig_symbols(document: &KconfigDocument) -> Vec<&str> {
    document
        .menuconfigs()
        .map(|menuconfig| menuconfig.symbol().as_str())
        .collect()
}

fn choice_symbols(document: &KconfigDocument) -> Vec<Option<&str>> {
    document
        .choices()
        .map(|choice| choice.symbol().map(KconfigSymbol::as_str))
        .collect()
}

fn endchoice_lines(document: &KconfigDocument) -> Vec<usize> {
    document
        .endchoices()
        .map(KconfigEndchoiceEntry::line)
        .collect()
}

fn endmenu_lines(document: &KconfigDocument) -> Vec<usize> {
    document.endmenus().map(KconfigEndmenuEntry::line).collect()
}

fn endif_lines(document: &KconfigDocument) -> Vec<usize> {
    document.endifs().map(KconfigEndifEntry::line).collect()
}

fn menu_prompts(document: &KconfigDocument) -> Vec<&str> {
    document.menus().map(KconfigMenuEntry::prompt).collect()
}

fn comment_prompts(document: &KconfigDocument) -> Vec<&str> {
    document.comments().map(KconfigCommentEntry::prompt).collect()
}

fn mainmenu_prompts(document: &KconfigDocument) -> Vec<&str> {
    document
        .mainmenus()
        .map(KconfigMainmenuEntry::prompt)
        .collect()
}

fn if_conditions(document: &KconfigDocument) -> Vec<&str> {
    document.ifs().map(KconfigIfEntry::condition).collect()
}

fn source_paths(document: &KconfigDocument) -> Vec<&str> {
    document.sources().map(KconfigSourceEntry::path).collect()
}

fn rsource_paths(document: &KconfigDocument) -> Vec<&str> {
    document.rsources().map(KconfigRsourceEntry::path).collect()
}

fn osource_paths(document: &KconfigDocument) -> Vec<&str> {
    document.osources().map(KconfigOsourceEntry::path).collect()
}

fn orsource_paths(document: &KconfigDocument) -> Vec<&str> {
    document
        .orsources()
        .map(KconfigOrsourceEntry::path)
        .collect()
}

fn help_text(block: &KconfigHelpBlock) -> Vec<&str> {
    block.text().iter().map(KconfigRawLine::text).collect()
}

fn line_comment_texts(document: &KconfigDocument) -> Vec<&str> {
    document
        .line_comments()
        .map(|line_comment| line_comment.raw().text())
        .collect()
}

fn blank_line_texts(document: &KconfigDocument) -> Vec<&str> {
    document
        .blank_lines()
        .map(|blank_line| blank_line.raw().text())
        .collect()
}

fn skipped_site_texts(document: &KconfigDocument) -> Vec<&str> {
    document
        .skipped_sites()
        .map(|skipped_site| skipped_site.raw().text())
        .collect()
}

fn type_kinds(document: &KconfigDocument) -> Vec<KconfigSymbolType> {
    document
        .type_definitions()
        .map(KconfigTypeDefinition::kind)
        .collect()
}

fn prompt_texts(document: &KconfigDocument) -> Vec<&str> {
    document
        .prompt_definitions()
        .map(KconfigPromptDefinition::prompt)
        .collect()
}

fn prompt_conditions(document: &KconfigDocument) -> Vec<Option<&str>> {
    document
        .prompt_definitions()
        .map(KconfigPromptDefinition::condition)
        .collect()
}

fn default_values(document: &KconfigDocument) -> Vec<&str> {
    document
        .default_definitions()
        .map(KconfigDefaultDefinition::value)
        .collect()
}

fn default_conditions(document: &KconfigDocument) -> Vec<Option<&str>> {
    document
        .default_definitions()
        .map(KconfigDefaultDefinition::condition)
        .collect()
}

fn range_minimums(document: &KconfigDocument) -> Vec<&str> {
    document
        .range_definitions()
        .map(KconfigRangeDefinition::minimum)
        .collect()
}

fn range_maximums(document: &KconfigDocument) -> Vec<&str> {
    document
        .range_definitions()
        .map(KconfigRangeDefinition::maximum)
        .collect()
}

fn range_conditions(document: &KconfigDocument) -> Vec<Option<&str>> {
    document
        .range_definitions()
        .map(KconfigRangeDefinition::condition)
        .collect()
}

fn dependency_expressions(document: &KconfigDocument) -> Vec<&str> {
    document
        .dependency_definitions()
        .map(KconfigDependencyDefinition::expression)
        .collect()
}

fn select_targets(document: &KconfigDocument) -> Vec<&str> {
    document
        .select_definitions()
        .map(|select| select.target().as_str())
        .collect()
}

fn select_conditions(document: &KconfigDocument) -> Vec<Option<&str>> {
    document
        .select_definitions()
        .map(KconfigSelectDefinition::condition)
        .collect()
}

fn imply_targets(document: &KconfigDocument) -> Vec<&str> {
    document
        .imply_definitions()
        .map(|imply| imply.target().as_str())
        .collect()
}

fn imply_conditions(document: &KconfigDocument) -> Vec<Option<&str>> {
    document
        .imply_definitions()
        .map(KconfigImplyDefinition::condition)
        .collect()
}

fn option_names(document: &KconfigDocument) -> Vec<&str> {
    document
        .option_definitions()
        .map(KconfigOptionDefinition::name)
        .collect()
}

fn option_values(document: &KconfigDocument) -> Vec<Option<&str>> {
    document
        .option_definitions()
        .map(KconfigOptionDefinition::value)
        .collect()
}

fn modules_lines(document: &KconfigDocument) -> Vec<usize> {
    document
        .modules_definitions()
        .map(KconfigModulesDefinition::line)
        .collect()
}

fn symbol_definition_symbols(document: &KconfigDocument) -> Vec<&str> {
    document
        .symbol_definitions()
        .map(|definition| definition.symbol().as_str())
        .collect()
}

fn symbol_definition_kinds(document: &KconfigDocument) -> Vec<KconfigSymbolDefinitionKind> {
    document
        .symbol_definitions()
        .map(|definition| definition.kind())
        .collect()
}

fn symbol_definition_group_symbols(document: &KconfigDocument) -> Vec<&str> {
    document
        .symbol_definition_groups()
        .iter()
        .map(|group| group.symbol().as_str())
        .collect()
}

fn multiple_symbol_definition_group_symbols(document: &KconfigDocument) -> Vec<&str> {
    document
        .multiple_symbol_definition_groups()
        .iter()
        .map(|group| group.symbol().as_str())
        .collect()
}

#[path = "tests_directives.rs"]
mod directives;
#[path = "tests_malformed.rs"]
mod malformed;
#[path = "tests_preservation.rs"]
mod preservation;
#[path = "tests_symbol_model.rs"]
mod symbol_model;
