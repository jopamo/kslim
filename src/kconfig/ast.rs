use anyhow::{Context, Result};

use crate::model::KconfigSymbol;

use super::{
    indentation, is_kconfig_boundary, kconfig_help_text_mask, split_kconfig_trailing_comment,
};

mod document_model;
mod symbol_model;
pub(crate) use document_model::{
    KconfigDefinitionSourceLocation, KconfigPromptConsistencyDefinition,
    KconfigPromptConsistencyViolation, KconfigSymbolDefinition, KconfigSymbolDefinitionGroup,
    KconfigSymbolDefinitionKind, KconfigTypeConsistencyDefinition,
    KconfigTypeConsistencyViolation,
};
pub(crate) use symbol_model::{
    KconfigDefaultDefinition, KconfigDependencyDefinition, KconfigImplyDefinition,
    KconfigModulesDefinition, KconfigOptionDefinition, KconfigPromptDefinition,
    KconfigRangeDefinition, KconfigSelectDefinition, KconfigSymbolType, KconfigTypeDefinition,
};
use symbol_model::{
    parse_default_definitions, parse_dependency_definitions, parse_imply_definitions,
    parse_modules_definitions, parse_option_definitions, parse_prompt_definitions,
    parse_range_definitions, parse_select_definitions, parse_type_definitions,
};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct KconfigDocument {
    nodes: Vec<KconfigNode>,
}

#[allow(dead_code)]
impl KconfigDocument {
    pub(crate) fn nodes(&self) -> &[KconfigNode] {
        &self.nodes
    }

    pub(crate) fn configs(&self) -> impl Iterator<Item = &KconfigConfigEntry> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(config) => Some(config),
            KconfigNode::Menuconfig(_)
            | KconfigNode::Choice(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::Menu(_)
            | KconfigNode::If(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::Comment(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::BlankLine(_)
            | KconfigNode::SkippedSite(_) => None,
        })
    }

    pub(crate) fn menuconfigs(&self) -> impl Iterator<Item = &KconfigMenuconfigEntry> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(_) => None,
            KconfigNode::Menuconfig(menuconfig) => Some(menuconfig),
            KconfigNode::Choice(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::Menu(_)
            | KconfigNode::If(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::Comment(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::BlankLine(_)
            | KconfigNode::SkippedSite(_) => None,
        })
    }

    pub(crate) fn choices(&self) -> impl Iterator<Item = &KconfigChoiceEntry> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(_)
            | KconfigNode::Menuconfig(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::Menu(_)
            | KconfigNode::If(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::Comment(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::BlankLine(_)
            | KconfigNode::SkippedSite(_) => None,
            KconfigNode::Choice(choice) => Some(choice),
        })
    }

    pub(crate) fn endchoices(&self) -> impl Iterator<Item = &KconfigEndchoiceEntry> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(_)
            | KconfigNode::Menuconfig(_)
            | KconfigNode::Choice(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::Menu(_)
            | KconfigNode::If(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::Comment(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::BlankLine(_)
            | KconfigNode::SkippedSite(_) => None,
            KconfigNode::Endchoice(endchoice) => Some(endchoice),
        })
    }

    pub(crate) fn menus(&self) -> impl Iterator<Item = &KconfigMenuEntry> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(_)
            | KconfigNode::Menuconfig(_)
            | KconfigNode::Choice(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::If(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::Comment(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::BlankLine(_)
            | KconfigNode::SkippedSite(_) => None,
            KconfigNode::Menu(menu) => Some(menu),
        })
    }

    pub(crate) fn endmenus(&self) -> impl Iterator<Item = &KconfigEndmenuEntry> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(_)
            | KconfigNode::Menuconfig(_)
            | KconfigNode::Choice(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Menu(_)
            | KconfigNode::If(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::Comment(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::BlankLine(_)
            | KconfigNode::SkippedSite(_) => None,
            KconfigNode::Endmenu(endmenu) => Some(endmenu),
        })
    }

    pub(crate) fn ifs(&self) -> impl Iterator<Item = &KconfigIfEntry> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(_)
            | KconfigNode::Menuconfig(_)
            | KconfigNode::Choice(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Menu(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::Comment(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::BlankLine(_)
            | KconfigNode::SkippedSite(_) => None,
            KconfigNode::If(if_entry) => Some(if_entry),
        })
    }

    pub(crate) fn endifs(&self) -> impl Iterator<Item = &KconfigEndifEntry> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(_)
            | KconfigNode::Menuconfig(_)
            | KconfigNode::Choice(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Menu(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::If(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::Comment(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::BlankLine(_)
            | KconfigNode::SkippedSite(_) => None,
            KconfigNode::Endif(endif) => Some(endif),
        })
    }

    pub(crate) fn sources(&self) -> impl Iterator<Item = &KconfigSourceEntry> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(_)
            | KconfigNode::Menuconfig(_)
            | KconfigNode::Choice(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Menu(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::If(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::Comment(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::BlankLine(_)
            | KconfigNode::SkippedSite(_) => None,
            KconfigNode::Source(source) => Some(source),
        })
    }

    pub(crate) fn rsources(&self) -> impl Iterator<Item = &KconfigRsourceEntry> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(_)
            | KconfigNode::Menuconfig(_)
            | KconfigNode::Choice(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Menu(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::If(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Source(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::Comment(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::BlankLine(_)
            | KconfigNode::SkippedSite(_) => None,
            KconfigNode::Rsource(rsource) => Some(rsource),
        })
    }

    pub(crate) fn osources(&self) -> impl Iterator<Item = &KconfigOsourceEntry> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(_)
            | KconfigNode::Menuconfig(_)
            | KconfigNode::Choice(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Menu(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::If(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::Comment(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::BlankLine(_)
            | KconfigNode::SkippedSite(_) => None,
            KconfigNode::Osource(osource) => Some(osource),
            KconfigNode::Orsource(_) => None,
        })
    }

    pub(crate) fn orsources(&self) -> impl Iterator<Item = &KconfigOrsourceEntry> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(_)
            | KconfigNode::Menuconfig(_)
            | KconfigNode::Choice(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Menu(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::If(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::Comment(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::BlankLine(_)
            | KconfigNode::SkippedSite(_) => None,
            KconfigNode::Orsource(orsource) => Some(orsource),
        })
    }

    pub(crate) fn mainmenus(&self) -> impl Iterator<Item = &KconfigMainmenuEntry> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(_)
            | KconfigNode::Menuconfig(_)
            | KconfigNode::Choice(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Menu(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::If(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Comment(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::BlankLine(_)
            | KconfigNode::SkippedSite(_) => None,
            KconfigNode::Mainmenu(mainmenu) => Some(mainmenu),
        })
    }

    pub(crate) fn comments(&self) -> impl Iterator<Item = &KconfigCommentEntry> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(_)
            | KconfigNode::Menuconfig(_)
            | KconfigNode::Choice(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Menu(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::If(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::BlankLine(_)
            | KconfigNode::SkippedSite(_) => None,
            KconfigNode::Comment(comment) => Some(comment),
        })
    }

    pub(crate) fn line_comments(&self) -> impl Iterator<Item = &KconfigLineComment> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(_)
            | KconfigNode::Menuconfig(_)
            | KconfigNode::Choice(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Menu(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::If(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::Comment(_)
            | KconfigNode::BlankLine(_)
            | KconfigNode::SkippedSite(_) => None,
            KconfigNode::LineComment(line_comment) => Some(line_comment),
        })
    }

    pub(crate) fn blank_lines(&self) -> impl Iterator<Item = &KconfigBlankLine> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(_)
            | KconfigNode::Menuconfig(_)
            | KconfigNode::Choice(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Menu(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::If(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::Comment(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::SkippedSite(_) => None,
            KconfigNode::BlankLine(blank_line) => Some(blank_line),
        })
    }

    pub(crate) fn skipped_sites(&self) -> impl Iterator<Item = &KconfigSkippedSite> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(_)
            | KconfigNode::Menuconfig(_)
            | KconfigNode::Choice(_)
            | KconfigNode::Endchoice(_)
            | KconfigNode::Menu(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::If(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::Comment(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::BlankLine(_) => None,
            KconfigNode::SkippedSite(skipped_site) => Some(skipped_site),
        })
    }

    pub(crate) fn help_blocks(&self) -> impl Iterator<Item = &KconfigHelpBlock> {
        self.nodes.iter().flat_map(|node| {
            let help_blocks: &[KconfigHelpBlock] = match node {
                KconfigNode::Config(config) => config.help_blocks(),
                KconfigNode::Menuconfig(menuconfig) => menuconfig.help_blocks(),
                KconfigNode::Choice(choice) => choice.help_blocks(),
                KconfigNode::Endchoice(_)
                | KconfigNode::Endmenu(_)
                | KconfigNode::Menu(_)
                | KconfigNode::If(_)
                | KconfigNode::Endif(_)
                | KconfigNode::Source(_)
                | KconfigNode::Rsource(_)
                | KconfigNode::Osource(_)
                | KconfigNode::Orsource(_)
                | KconfigNode::Mainmenu(_)
                | KconfigNode::Comment(_)
                | KconfigNode::LineComment(_)
                | KconfigNode::BlankLine(_)
                | KconfigNode::SkippedSite(_) => &[],
            };
            help_blocks.iter()
        })
    }

}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KconfigNode {
    Config(KconfigConfigEntry),
    Menuconfig(KconfigMenuconfigEntry),
    Choice(KconfigChoiceEntry),
    Endchoice(KconfigEndchoiceEntry),
    Menu(KconfigMenuEntry),
    Endmenu(KconfigEndmenuEntry),
    If(KconfigIfEntry),
    Endif(KconfigEndifEntry),
    Source(KconfigSourceEntry),
    Rsource(KconfigRsourceEntry),
    Osource(KconfigOsourceEntry),
    Orsource(KconfigOrsourceEntry),
    Mainmenu(KconfigMainmenuEntry),
    Comment(KconfigCommentEntry),
    LineComment(KconfigLineComment),
    BlankLine(KconfigBlankLine),
    SkippedSite(KconfigSkippedSite),
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigConfigEntry {
    symbol: KconfigSymbol,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
    body: Vec<KconfigRawLine>,
    help_blocks: Vec<KconfigHelpBlock>,
    type_definitions: Vec<KconfigTypeDefinition>,
    prompt_definitions: Vec<KconfigPromptDefinition>,
    default_definitions: Vec<KconfigDefaultDefinition>,
    range_definitions: Vec<KconfigRangeDefinition>,
    dependency_definitions: Vec<KconfigDependencyDefinition>,
    select_definitions: Vec<KconfigSelectDefinition>,
    imply_definitions: Vec<KconfigImplyDefinition>,
    option_definitions: Vec<KconfigOptionDefinition>,
    modules_definitions: Vec<KconfigModulesDefinition>,
}

#[allow(dead_code)]
impl KconfigConfigEntry {
    pub(crate) fn symbol(&self) -> &KconfigSymbol {
        &self.symbol
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }

    pub(crate) fn body(&self) -> &[KconfigRawLine] {
        &self.body
    }

    pub(crate) fn help_blocks(&self) -> &[KconfigHelpBlock] {
        &self.help_blocks
    }

    pub(crate) fn type_definitions(&self) -> &[KconfigTypeDefinition] {
        &self.type_definitions
    }

    pub(crate) fn prompt_definitions(&self) -> &[KconfigPromptDefinition] {
        &self.prompt_definitions
    }

    pub(crate) fn default_definitions(&self) -> &[KconfigDefaultDefinition] {
        &self.default_definitions
    }

    pub(crate) fn range_definitions(&self) -> &[KconfigRangeDefinition] {
        &self.range_definitions
    }

    pub(crate) fn dependency_definitions(&self) -> &[KconfigDependencyDefinition] {
        &self.dependency_definitions
    }

    pub(crate) fn select_definitions(&self) -> &[KconfigSelectDefinition] {
        &self.select_definitions
    }

    pub(crate) fn imply_definitions(&self) -> &[KconfigImplyDefinition] {
        &self.imply_definitions
    }

    pub(crate) fn option_definitions(&self) -> &[KconfigOptionDefinition] {
        &self.option_definitions
    }

    pub(crate) fn modules_definitions(&self) -> &[KconfigModulesDefinition] {
        &self.modules_definitions
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigMenuconfigEntry {
    symbol: KconfigSymbol,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
    body: Vec<KconfigRawLine>,
    help_blocks: Vec<KconfigHelpBlock>,
    type_definitions: Vec<KconfigTypeDefinition>,
    prompt_definitions: Vec<KconfigPromptDefinition>,
    default_definitions: Vec<KconfigDefaultDefinition>,
    range_definitions: Vec<KconfigRangeDefinition>,
    dependency_definitions: Vec<KconfigDependencyDefinition>,
    select_definitions: Vec<KconfigSelectDefinition>,
    imply_definitions: Vec<KconfigImplyDefinition>,
    option_definitions: Vec<KconfigOptionDefinition>,
    modules_definitions: Vec<KconfigModulesDefinition>,
}

#[allow(dead_code)]
impl KconfigMenuconfigEntry {
    pub(crate) fn symbol(&self) -> &KconfigSymbol {
        &self.symbol
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }

    pub(crate) fn body(&self) -> &[KconfigRawLine] {
        &self.body
    }

    pub(crate) fn help_blocks(&self) -> &[KconfigHelpBlock] {
        &self.help_blocks
    }

    pub(crate) fn type_definitions(&self) -> &[KconfigTypeDefinition] {
        &self.type_definitions
    }

    pub(crate) fn prompt_definitions(&self) -> &[KconfigPromptDefinition] {
        &self.prompt_definitions
    }

    pub(crate) fn default_definitions(&self) -> &[KconfigDefaultDefinition] {
        &self.default_definitions
    }

    pub(crate) fn range_definitions(&self) -> &[KconfigRangeDefinition] {
        &self.range_definitions
    }

    pub(crate) fn dependency_definitions(&self) -> &[KconfigDependencyDefinition] {
        &self.dependency_definitions
    }

    pub(crate) fn select_definitions(&self) -> &[KconfigSelectDefinition] {
        &self.select_definitions
    }

    pub(crate) fn imply_definitions(&self) -> &[KconfigImplyDefinition] {
        &self.imply_definitions
    }

    pub(crate) fn option_definitions(&self) -> &[KconfigOptionDefinition] {
        &self.option_definitions
    }

    pub(crate) fn modules_definitions(&self) -> &[KconfigModulesDefinition] {
        &self.modules_definitions
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigChoiceEntry {
    symbol: Option<KconfigSymbol>,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
    body: Vec<KconfigRawLine>,
    help_blocks: Vec<KconfigHelpBlock>,
    type_definitions: Vec<KconfigTypeDefinition>,
    prompt_definitions: Vec<KconfigPromptDefinition>,
    default_definitions: Vec<KconfigDefaultDefinition>,
    range_definitions: Vec<KconfigRangeDefinition>,
    dependency_definitions: Vec<KconfigDependencyDefinition>,
    select_definitions: Vec<KconfigSelectDefinition>,
    imply_definitions: Vec<KconfigImplyDefinition>,
    option_definitions: Vec<KconfigOptionDefinition>,
    modules_definitions: Vec<KconfigModulesDefinition>,
}

#[allow(dead_code)]
impl KconfigChoiceEntry {
    pub(crate) fn symbol(&self) -> Option<&KconfigSymbol> {
        self.symbol.as_ref()
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }

    pub(crate) fn body(&self) -> &[KconfigRawLine] {
        &self.body
    }

    pub(crate) fn help_blocks(&self) -> &[KconfigHelpBlock] {
        &self.help_blocks
    }

    pub(crate) fn type_definitions(&self) -> &[KconfigTypeDefinition] {
        &self.type_definitions
    }

    pub(crate) fn prompt_definitions(&self) -> &[KconfigPromptDefinition] {
        &self.prompt_definitions
    }

    pub(crate) fn default_definitions(&self) -> &[KconfigDefaultDefinition] {
        &self.default_definitions
    }

    pub(crate) fn range_definitions(&self) -> &[KconfigRangeDefinition] {
        &self.range_definitions
    }

    pub(crate) fn dependency_definitions(&self) -> &[KconfigDependencyDefinition] {
        &self.dependency_definitions
    }

    pub(crate) fn select_definitions(&self) -> &[KconfigSelectDefinition] {
        &self.select_definitions
    }

    pub(crate) fn imply_definitions(&self) -> &[KconfigImplyDefinition] {
        &self.imply_definitions
    }

    pub(crate) fn option_definitions(&self) -> &[KconfigOptionDefinition] {
        &self.option_definitions
    }

    pub(crate) fn modules_definitions(&self) -> &[KconfigModulesDefinition] {
        &self.modules_definitions
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigEndchoiceEntry {
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigEndchoiceEntry {
    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigEndmenuEntry {
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigEndmenuEntry {
    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigMenuEntry {
    prompt: String,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
    body: Vec<KconfigRawLine>,
}

#[allow(dead_code)]
impl KconfigMenuEntry {
    pub(crate) fn prompt(&self) -> &str {
        &self.prompt
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }

    pub(crate) fn body(&self) -> &[KconfigRawLine] {
        &self.body
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigIfEntry {
    condition: String,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
    body: Vec<KconfigRawLine>,
}

#[allow(dead_code)]
impl KconfigIfEntry {
    pub(crate) fn condition(&self) -> &str {
        &self.condition
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }

    pub(crate) fn body(&self) -> &[KconfigRawLine] {
        &self.body
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigEndifEntry {
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigEndifEntry {
    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigSourceEntry {
    path: String,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigSourceEntry {
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigRsourceEntry {
    path: String,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigRsourceEntry {
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigOsourceEntry {
    path: String,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigOsourceEntry {
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigOrsourceEntry {
    path: String,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigOrsourceEntry {
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigMainmenuEntry {
    prompt: String,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigMainmenuEntry {
    pub(crate) fn prompt(&self) -> &str {
        &self.prompt
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigCommentEntry {
    prompt: String,
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
    body: Vec<KconfigRawLine>,
}

#[allow(dead_code)]
impl KconfigCommentEntry {
    pub(crate) fn prompt(&self) -> &str {
        &self.prompt
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }

    pub(crate) fn body(&self) -> &[KconfigRawLine] {
        &self.body
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigLineComment {
    line: usize,
    end_line: usize,
    raw: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigLineComment {
    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn raw(&self) -> &KconfigRawLine {
        &self.raw
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigBlankLine {
    line: usize,
    end_line: usize,
    raw: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigBlankLine {
    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn raw(&self) -> &KconfigRawLine {
        &self.raw
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigSkippedSite {
    line: usize,
    end_line: usize,
    raw: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigSkippedSite {
    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn raw(&self) -> &KconfigRawLine {
        &self.raw
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigHelpBlock {
    line: usize,
    end_line: usize,
    directive: KconfigRawLine,
    text: Vec<KconfigRawLine>,
}

#[allow(dead_code)]
impl KconfigHelpBlock {
    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }

    pub(crate) fn text(&self) -> &[KconfigRawLine] {
        &self.text
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigRawLine {
    line: usize,
    text: String,
}

#[allow(dead_code)]
impl KconfigRawLine {
    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }
}

#[allow(dead_code)]
pub(crate) fn parse_kconfig_document(source: &str) -> Result<KconfigDocument> {
    let lines = source.lines().collect::<Vec<_>>();
    let help_text = kconfig_help_text_mask(&lines);
    let mut nodes = Vec::new();
    let mut idx = 0usize;

    while idx < lines.len() {
        if help_text[idx] {
            idx += 1;
            continue;
        }

        if is_kconfig_blank_line(lines[idx]) {
            let line = idx + 1;
            nodes.push(KconfigNode::BlankLine(KconfigBlankLine {
                line,
                end_line: line,
                raw: raw_line(line, lines[idx]),
            }));
            idx += 1;
            continue;
        }

        if is_kconfig_line_comment(lines[idx]) {
            let line = idx + 1;
            nodes.push(KconfigNode::LineComment(KconfigLineComment {
                line,
                end_line: line,
                raw: raw_line(line, lines[idx]),
            }));
            idx += 1;
            continue;
        }

        let line = idx + 1;
        let Some(header) = parse_entry_header(lines[idx], line)? else {
            nodes.push(KconfigNode::SkippedSite(KconfigSkippedSite {
                line,
                end_line: line,
                raw: raw_line(line, lines[idx]),
            }));
            idx += 1;
            continue;
        };

        let start_line = idx + 1;
        let directive = raw_line(start_line, lines[idx]);
        match header.kind {
            KconfigEntryHeaderKind::Endchoice => {
                nodes.push(KconfigNode::Endchoice(KconfigEndchoiceEntry {
                    line: start_line,
                    end_line: start_line,
                    directive,
                }));
                idx += 1;
                continue;
            }
            KconfigEntryHeaderKind::Endmenu => {
                nodes.push(KconfigNode::Endmenu(KconfigEndmenuEntry {
                    line: start_line,
                    end_line: start_line,
                    directive,
                }));
                idx += 1;
                continue;
            }
            KconfigEntryHeaderKind::Endif => {
                nodes.push(KconfigNode::Endif(KconfigEndifEntry {
                    line: start_line,
                    end_line: start_line,
                    directive,
                }));
                idx += 1;
                continue;
            }
            KconfigEntryHeaderKind::Source => {
                nodes.push(KconfigNode::Source(KconfigSourceEntry {
                    path: header
                        .path
                        .expect("source parser should require a path"),
                    line: start_line,
                    end_line: start_line,
                    directive,
                }));
                idx += 1;
                continue;
            }
            KconfigEntryHeaderKind::Rsource => {
                nodes.push(KconfigNode::Rsource(KconfigRsourceEntry {
                    path: header
                        .path
                        .expect("rsource parser should require a path"),
                    line: start_line,
                    end_line: start_line,
                    directive,
                }));
                idx += 1;
                continue;
            }
            KconfigEntryHeaderKind::Osource => {
                nodes.push(KconfigNode::Osource(KconfigOsourceEntry {
                    path: header
                        .path
                        .expect("osource parser should require a path"),
                    line: start_line,
                    end_line: start_line,
                    directive,
                }));
                idx += 1;
                continue;
            }
            KconfigEntryHeaderKind::Orsource => {
                nodes.push(KconfigNode::Orsource(KconfigOrsourceEntry {
                    path: header
                        .path
                        .expect("orsource parser should require a path"),
                    line: start_line,
                    end_line: start_line,
                    directive,
                }));
                idx += 1;
                continue;
            }
            KconfigEntryHeaderKind::Mainmenu => {
                nodes.push(KconfigNode::Mainmenu(KconfigMainmenuEntry {
                    prompt: header
                        .prompt
                        .expect("mainmenu parser should require a prompt"),
                    line: start_line,
                    end_line: start_line,
                    directive,
                }));
                idx += 1;
                continue;
            }
            KconfigEntryHeaderKind::Config
            | KconfigEntryHeaderKind::Menuconfig
            | KconfigEntryHeaderKind::Choice
            | KconfigEntryHeaderKind::Comment
            | KconfigEntryHeaderKind::Menu
            | KconfigEntryHeaderKind::If => {}
        }

        let base_indent = indentation(lines[idx]);
        idx += 1;

        let mut body = Vec::new();
        while idx < lines.len() {
            let line = lines[idx];
            if !help_text[idx] {
                let trimmed = line.trim_start();
                if !trimmed.is_empty()
                    && indentation(line) <= base_indent
                    && is_kconfig_boundary(trimmed)
                {
                    break;
                }
            }
            body.push(raw_line(idx + 1, line));
            idx += 1;
        }

        let end_line = body.last().map(KconfigRawLine::line).unwrap_or(start_line);
        match header.kind {
            KconfigEntryHeaderKind::Config => {
                let help_blocks = parse_help_blocks(&body);
                let type_definitions = parse_type_definitions(&body)?;
                let prompt_definitions = parse_prompt_definitions(&body)?;
                let default_definitions = parse_default_definitions(&body)?;
                let range_definitions = parse_range_definitions(&body)?;
                let dependency_definitions = parse_dependency_definitions(&body)?;
                let select_definitions = parse_select_definitions(&body)?;
                let imply_definitions = parse_imply_definitions(&body)?;
                let option_definitions = parse_option_definitions(&body)?;
                let modules_definitions = parse_modules_definitions(&body)?;
                nodes.push(KconfigNode::Config(KconfigConfigEntry {
                    symbol: header
                        .symbol
                        .expect("config parser should require a symbol"),
                    line: start_line,
                    end_line,
                    directive,
                    body,
                    help_blocks,
                    type_definitions,
                    prompt_definitions,
                    default_definitions,
                    range_definitions,
                    dependency_definitions,
                    select_definitions,
                    imply_definitions,
                    option_definitions,
                    modules_definitions,
                }))
            }
            KconfigEntryHeaderKind::Menuconfig => {
                let help_blocks = parse_help_blocks(&body);
                let type_definitions = parse_type_definitions(&body)?;
                let prompt_definitions = parse_prompt_definitions(&body)?;
                let default_definitions = parse_default_definitions(&body)?;
                let range_definitions = parse_range_definitions(&body)?;
                let dependency_definitions = parse_dependency_definitions(&body)?;
                let select_definitions = parse_select_definitions(&body)?;
                let imply_definitions = parse_imply_definitions(&body)?;
                let option_definitions = parse_option_definitions(&body)?;
                let modules_definitions = parse_modules_definitions(&body)?;
                nodes.push(KconfigNode::Menuconfig(KconfigMenuconfigEntry {
                    symbol: header
                        .symbol
                        .expect("menuconfig parser should require a symbol"),
                    line: start_line,
                    end_line,
                    directive,
                    body,
                    help_blocks,
                    type_definitions,
                    prompt_definitions,
                    default_definitions,
                    range_definitions,
                    dependency_definitions,
                    select_definitions,
                    imply_definitions,
                    option_definitions,
                    modules_definitions,
                }))
            }
            KconfigEntryHeaderKind::Choice => {
                let help_blocks = parse_help_blocks(&body);
                let type_definitions = parse_type_definitions(&body)?;
                let prompt_definitions = parse_prompt_definitions(&body)?;
                let default_definitions = parse_default_definitions(&body)?;
                let range_definitions = parse_range_definitions(&body)?;
                let dependency_definitions = parse_dependency_definitions(&body)?;
                let select_definitions = parse_select_definitions(&body)?;
                let imply_definitions = parse_imply_definitions(&body)?;
                let option_definitions = parse_option_definitions(&body)?;
                let modules_definitions = parse_modules_definitions(&body)?;
                nodes.push(KconfigNode::Choice(KconfigChoiceEntry {
                    symbol: header.symbol,
                    line: start_line,
                    end_line,
                    directive,
                    body,
                    help_blocks,
                    type_definitions,
                    prompt_definitions,
                    default_definitions,
                    range_definitions,
                    dependency_definitions,
                    select_definitions,
                    imply_definitions,
                    option_definitions,
                    modules_definitions,
                }))
            }
            KconfigEntryHeaderKind::Endchoice => unreachable!("endchoice is handled as a marker"),
            KconfigEntryHeaderKind::Endmenu => unreachable!("endmenu is handled as a marker"),
            KconfigEntryHeaderKind::Endif => unreachable!("endif is handled as a marker"),
            KconfigEntryHeaderKind::Source => unreachable!("source is handled as a marker"),
            KconfigEntryHeaderKind::Rsource => unreachable!("rsource is handled as a marker"),
            KconfigEntryHeaderKind::Osource => unreachable!("osource is handled as a marker"),
            KconfigEntryHeaderKind::Orsource => unreachable!("orsource is handled as a marker"),
            KconfigEntryHeaderKind::Mainmenu => unreachable!("mainmenu is handled as a marker"),
            KconfigEntryHeaderKind::Comment => nodes.push(KconfigNode::Comment(
                KconfigCommentEntry {
                    prompt: header
                        .prompt
                        .expect("comment parser should require a prompt"),
                    line: start_line,
                    end_line,
                    directive,
                    body,
                },
            )),
            KconfigEntryHeaderKind::Menu => nodes.push(KconfigNode::Menu(KconfigMenuEntry {
                prompt: header.prompt.expect("menu parser should require a prompt"),
                line: start_line,
                end_line,
                directive,
                body,
            })),
            KconfigEntryHeaderKind::If => nodes.push(KconfigNode::If(KconfigIfEntry {
                condition: header
                    .condition
                    .expect("if parser should require a condition"),
                line: start_line,
                end_line,
                directive,
                body,
            })),
        }
    }

    Ok(KconfigDocument { nodes })
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KconfigEntryHeaderKind {
    Config,
    Menuconfig,
    Choice,
    Endchoice,
    Menu,
    Endmenu,
    If,
    Endif,
    Source,
    Rsource,
    Osource,
    Orsource,
    Mainmenu,
    Comment,
}

#[allow(dead_code)]
struct KconfigParsedEntryHeader {
    kind: KconfigEntryHeaderKind,
    symbol: Option<KconfigSymbol>,
    prompt: Option<String>,
    condition: Option<String>,
    path: Option<String>,
}

#[allow(dead_code)]
fn parse_entry_header(line: &str, line_number: usize) -> Result<Option<KconfigParsedEntryHeader>> {
    let (directive_text, _) = split_kconfig_trailing_comment(line);
    let trimmed = directive_text.trim_start();
    if let Some(symbol) = parse_symbol_entry_header(trimmed, "config", line_number)? {
        return Ok(Some(KconfigParsedEntryHeader {
            kind: KconfigEntryHeaderKind::Config,
            symbol: Some(symbol),
            prompt: None,
            condition: None,
            path: None,
        }));
    }
    if let Some(symbol) = parse_symbol_entry_header(trimmed, "menuconfig", line_number)? {
        return Ok(Some(KconfigParsedEntryHeader {
            kind: KconfigEntryHeaderKind::Menuconfig,
            symbol: Some(symbol),
            prompt: None,
            condition: None,
            path: None,
        }));
    }
    if let Some(symbol) = parse_optional_symbol_entry_header(trimmed, "choice", line_number)? {
        return Ok(Some(KconfigParsedEntryHeader {
            kind: KconfigEntryHeaderKind::Choice,
            symbol,
            prompt: None,
            condition: None,
            path: None,
        }));
    }
    if parse_keyword_only_entry_header(trimmed, "endchoice", line_number)? {
        return Ok(Some(KconfigParsedEntryHeader {
            kind: KconfigEntryHeaderKind::Endchoice,
            symbol: None,
            prompt: None,
            condition: None,
            path: None,
        }));
    }
    if parse_keyword_only_entry_header(trimmed, "endmenu", line_number)? {
        return Ok(Some(KconfigParsedEntryHeader {
            kind: KconfigEntryHeaderKind::Endmenu,
            symbol: None,
            prompt: None,
            condition: None,
            path: None,
        }));
    }
    if parse_keyword_only_entry_header(trimmed, "endif", line_number)? {
        return Ok(Some(KconfigParsedEntryHeader {
            kind: KconfigEntryHeaderKind::Endif,
            symbol: None,
            prompt: None,
            condition: None,
            path: None,
        }));
    }
    if let Some(path) = parse_quoted_path_entry_header(trimmed, "source", line_number)? {
        return Ok(Some(KconfigParsedEntryHeader {
            kind: KconfigEntryHeaderKind::Source,
            symbol: None,
            prompt: None,
            condition: None,
            path: Some(path),
        }));
    }
    if let Some(path) = parse_quoted_path_entry_header(trimmed, "rsource", line_number)? {
        return Ok(Some(KconfigParsedEntryHeader {
            kind: KconfigEntryHeaderKind::Rsource,
            symbol: None,
            prompt: None,
            condition: None,
            path: Some(path),
        }));
    }
    if let Some(path) = parse_quoted_path_entry_header(trimmed, "osource", line_number)? {
        return Ok(Some(KconfigParsedEntryHeader {
            kind: KconfigEntryHeaderKind::Osource,
            symbol: None,
            prompt: None,
            condition: None,
            path: Some(path),
        }));
    }
    if let Some(path) = parse_quoted_path_entry_header(trimmed, "orsource", line_number)? {
        return Ok(Some(KconfigParsedEntryHeader {
            kind: KconfigEntryHeaderKind::Orsource,
            symbol: None,
            prompt: None,
            condition: None,
            path: Some(path),
        }));
    }
    if let Some(prompt) = parse_quoted_string_entry_header(trimmed, "mainmenu", line_number)? {
        return Ok(Some(KconfigParsedEntryHeader {
            kind: KconfigEntryHeaderKind::Mainmenu,
            symbol: None,
            prompt: Some(prompt),
            condition: None,
            path: None,
        }));
    }
    if let Some(prompt) = parse_quoted_string_entry_header(trimmed, "comment", line_number)? {
        return Ok(Some(KconfigParsedEntryHeader {
            kind: KconfigEntryHeaderKind::Comment,
            symbol: None,
            prompt: Some(prompt),
            condition: None,
            path: None,
        }));
    }
    if let Some(prompt) = parse_quoted_string_entry_header(trimmed, "menu", line_number)? {
        return Ok(Some(KconfigParsedEntryHeader {
            kind: KconfigEntryHeaderKind::Menu,
            symbol: None,
            prompt: Some(prompt),
            condition: None,
            path: None,
        }));
    }
    if let Some(condition) = parse_condition_entry_header(trimmed, "if", line_number)? {
        return Ok(Some(KconfigParsedEntryHeader {
            kind: KconfigEntryHeaderKind::If,
            symbol: None,
            prompt: None,
            condition: Some(condition),
            path: None,
        }));
    }
    Ok(None)
}

fn parse_symbol_entry_header(
    trimmed: &str,
    keyword: &str,
    line_number: usize,
) -> Result<Option<KconfigSymbol>> {
    let Some(rest) = trimmed.strip_prefix(keyword) else {
        return Ok(None);
    };
    if rest.is_empty() {
        anyhow::bail!("Kconfig {keyword} directive on line {line_number} is missing a symbol");
    }
    if !rest.starts_with(char::is_whitespace) {
        return Ok(None);
    }

    let mut parts = rest.split_whitespace();
    let Some(symbol) = parts.next() else {
        anyhow::bail!("Kconfig {keyword} directive on line {line_number} is missing a symbol");
    };
    if parts.next().is_some() {
        anyhow::bail!(
            "Kconfig {keyword} directive on line {line_number} has unexpected trailing tokens"
        );
    }

    KconfigSymbol::new(symbol)
        .with_context(|| format!("invalid Kconfig {keyword} symbol on line {line_number}"))
        .map(Some)
}

fn parse_optional_symbol_entry_header(
    trimmed: &str,
    keyword: &str,
    line_number: usize,
) -> Result<Option<Option<KconfigSymbol>>> {
    let Some(rest) = trimmed.strip_prefix(keyword) else {
        return Ok(None);
    };
    if rest.is_empty() {
        return Ok(Some(None));
    }
    if !rest.starts_with(char::is_whitespace) {
        return Ok(None);
    }

    let mut parts = rest.split_whitespace();
    let Some(symbol) = parts.next() else {
        return Ok(Some(None));
    };
    if parts.next().is_some() {
        anyhow::bail!(
            "Kconfig {keyword} directive on line {line_number} has unexpected trailing tokens"
        );
    }

    KconfigSymbol::new(symbol)
        .with_context(|| format!("invalid Kconfig {keyword} symbol on line {line_number}"))
        .map(Some)
        .map(Some)
}

fn parse_keyword_only_entry_header(
    trimmed: &str,
    keyword: &str,
    line_number: usize,
) -> Result<bool> {
    let Some(rest) = trimmed.strip_prefix(keyword) else {
        return Ok(false);
    };
    if rest.is_empty() {
        return Ok(true);
    }
    if !rest.starts_with(char::is_whitespace) {
        return Ok(false);
    }
    if rest.split_whitespace().next().is_some() {
        anyhow::bail!(
            "Kconfig {keyword} directive on line {line_number} has unexpected trailing tokens"
        );
    }
    Ok(true)
}

fn parse_condition_entry_header(
    trimmed: &str,
    keyword: &str,
    line_number: usize,
) -> Result<Option<String>> {
    let Some(rest) = trimmed.strip_prefix(keyword) else {
        return Ok(None);
    };
    if rest.is_empty() {
        anyhow::bail!("Kconfig {keyword} directive on line {line_number} is missing a condition");
    }
    if !rest.starts_with(char::is_whitespace) {
        return Ok(None);
    }

    let condition = rest.trim();
    if condition.is_empty() {
        anyhow::bail!("Kconfig {keyword} directive on line {line_number} is missing a condition");
    }

    Ok(Some(condition.to_string()))
}

fn parse_quoted_path_entry_header(
    trimmed: &str,
    keyword: &str,
    line_number: usize,
) -> Result<Option<String>> {
    let Some(rest) = trimmed.strip_prefix(keyword) else {
        return Ok(None);
    };
    if rest.is_empty() {
        anyhow::bail!("Kconfig {keyword} directive on line {line_number} is missing a path");
    }
    if !rest.starts_with(char::is_whitespace) {
        return Ok(None);
    }

    let rest = rest.trim_start();
    if rest.is_empty() {
        anyhow::bail!("Kconfig {keyword} directive on line {line_number} is missing a path");
    }

    let Some((path, trailing)) =
        parse_quoted_string_literal(rest, keyword, "path", line_number)?
    else {
        anyhow::bail!(
            "Kconfig {keyword} directive on line {line_number} is missing a quoted path"
        );
    };
    if path.is_empty() {
        anyhow::bail!("Kconfig {keyword} directive on line {line_number} is missing a path");
    }
    if trailing.split_whitespace().next().is_some() {
        anyhow::bail!(
            "Kconfig {keyword} directive on line {line_number} has unexpected trailing tokens"
        );
    }

    Ok(Some(path))
}

fn parse_quoted_string_entry_header(
    trimmed: &str,
    keyword: &str,
    line_number: usize,
) -> Result<Option<String>> {
    let Some(rest) = trimmed.strip_prefix(keyword) else {
        return Ok(None);
    };
    if rest.is_empty() {
        anyhow::bail!("Kconfig {keyword} directive on line {line_number} is missing a prompt");
    }
    if !rest.starts_with(char::is_whitespace) {
        return Ok(None);
    }

    let rest = rest.trim_start();
    if rest.is_empty() {
        anyhow::bail!("Kconfig {keyword} directive on line {line_number} is missing a prompt");
    }

    let Some((prompt, trailing)) =
        parse_quoted_string_literal(rest, keyword, "prompt", line_number)?
    else {
        anyhow::bail!(
            "Kconfig {keyword} directive on line {line_number} is missing a quoted prompt"
        );
    };
    if trailing.split_whitespace().next().is_some() {
        anyhow::bail!(
            "Kconfig {keyword} directive on line {line_number} has unexpected trailing tokens"
        );
    }

    Ok(Some(prompt))
}

fn parse_quoted_string_literal<'a>(
    input: &'a str,
    keyword: &str,
    value_name: &str,
    line_number: usize,
) -> Result<Option<(String, &'a str)>> {
    if !input.starts_with('"') {
        return Ok(None);
    }

    let mut prompt = String::new();
    let mut escape = false;
    for (offset, ch) in input[1..].char_indices() {
        if escape {
            prompt.push(ch);
            escape = false;
            continue;
        }

        match ch {
            '\\' => escape = true,
            '"' => {
                let trailing_start = 1 + offset + ch.len_utf8();
                return Ok(Some((prompt, &input[trailing_start..])));
            }
            _ => prompt.push(ch),
        }
    }

    match value_name {
        "prompt" => anyhow::bail!(
            "Kconfig {keyword} directive on line {line_number} has an unterminated quoted prompt"
        ),
        "path" => anyhow::bail!(
            "Kconfig {keyword} directive on line {line_number} has an unterminated quoted path"
        ),
        _ => anyhow::bail!(
            "Kconfig {keyword} directive on line {line_number} has an unterminated quoted value"
        ),
    }
}

fn parse_help_blocks(body: &[KconfigRawLine]) -> Vec<KconfigHelpBlock> {
    let mut blocks = Vec::new();
    let mut idx = 0usize;

    while idx < body.len() {
        let directive = &body[idx];
        let trimmed = directive.text().trim_start();
        if !is_kconfig_help_block_directive(trimmed) {
            idx += 1;
            continue;
        }

        let help_indent = indentation(directive.text());
        let directive = directive.clone();
        idx += 1;

        let mut text = Vec::new();
        while idx < body.len() {
            let line = &body[idx];
            if line.text().trim().is_empty() || indentation(line.text()) > help_indent {
                text.push(line.clone());
                idx += 1;
                continue;
            }
            break;
        }

        let end_line = text
            .last()
            .map(KconfigRawLine::line)
            .unwrap_or_else(|| directive.line());
        blocks.push(KconfigHelpBlock {
            line: directive.line(),
            end_line,
            directive,
            text,
        });
    }

    blocks
}

fn is_kconfig_help_block_directive(trimmed: &str) -> bool {
    trimmed == "help" || trimmed.starts_with("help ") || trimmed == "---help---"
}

fn is_kconfig_line_comment(line: &str) -> bool {
    line.trim_start().starts_with('#')
}

fn is_kconfig_blank_line(line: &str) -> bool {
    line.trim().is_empty()
}

#[allow(dead_code)]
fn raw_line(line: usize, text: &str) -> KconfigRawLine {
    KconfigRawLine {
        line,
        text: text.to_string(),
    }
}

#[cfg(test)]
mod source_location_tests;
#[cfg(test)]
mod tests;
