use std::collections::BTreeMap;

use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KconfigSymbolDefinitionKind {
    Config,
    Menuconfig,
    Choice,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KconfigSymbolDefinitionEntry<'a> {
    Config(&'a KconfigConfigEntry),
    Menuconfig(&'a KconfigMenuconfigEntry),
    Choice(&'a KconfigChoiceEntry),
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KconfigSymbolDefinition<'a> {
    entry: KconfigSymbolDefinitionEntry<'a>,
}

#[allow(dead_code)]
impl<'a> KconfigSymbolDefinition<'a> {
    fn config(config: &'a KconfigConfigEntry) -> Self {
        Self {
            entry: KconfigSymbolDefinitionEntry::Config(config),
        }
    }

    fn menuconfig(menuconfig: &'a KconfigMenuconfigEntry) -> Self {
        Self {
            entry: KconfigSymbolDefinitionEntry::Menuconfig(menuconfig),
        }
    }

    fn choice(choice: &'a KconfigChoiceEntry) -> Self {
        Self {
            entry: KconfigSymbolDefinitionEntry::Choice(choice),
        }
    }

    pub(crate) fn kind(&self) -> KconfigSymbolDefinitionKind {
        match self.entry {
            KconfigSymbolDefinitionEntry::Config(_) => KconfigSymbolDefinitionKind::Config,
            KconfigSymbolDefinitionEntry::Menuconfig(_) => {
                KconfigSymbolDefinitionKind::Menuconfig
            }
            KconfigSymbolDefinitionEntry::Choice(_) => KconfigSymbolDefinitionKind::Choice,
        }
    }

    pub(crate) fn symbol(&self) -> &'a KconfigSymbol {
        match self.entry {
            KconfigSymbolDefinitionEntry::Config(config) => config.symbol(),
            KconfigSymbolDefinitionEntry::Menuconfig(menuconfig) => menuconfig.symbol(),
            KconfigSymbolDefinitionEntry::Choice(choice) => choice
                .symbol()
                .expect("symbol definition choices should be named"),
        }
    }

    pub(crate) fn line(&self) -> usize {
        match self.entry {
            KconfigSymbolDefinitionEntry::Config(config) => config.line(),
            KconfigSymbolDefinitionEntry::Menuconfig(menuconfig) => menuconfig.line(),
            KconfigSymbolDefinitionEntry::Choice(choice) => choice.line(),
        }
    }

    pub(crate) fn end_line(&self) -> usize {
        match self.entry {
            KconfigSymbolDefinitionEntry::Config(config) => config.end_line(),
            KconfigSymbolDefinitionEntry::Menuconfig(menuconfig) => menuconfig.end_line(),
            KconfigSymbolDefinitionEntry::Choice(choice) => choice.end_line(),
        }
    }

    pub(crate) fn directive(&self) -> &'a KconfigRawLine {
        match self.entry {
            KconfigSymbolDefinitionEntry::Config(config) => config.directive(),
            KconfigSymbolDefinitionEntry::Menuconfig(menuconfig) => menuconfig.directive(),
            KconfigSymbolDefinitionEntry::Choice(choice) => choice.directive(),
        }
    }

    pub(crate) fn source_location(&self) -> KconfigDefinitionSourceLocation<'a> {
        KconfigDefinitionSourceLocation {
            symbol: self.symbol(),
            kind: self.kind(),
            line: self.line(),
            end_line: self.end_line(),
            directive: self.directive(),
        }
    }

    pub(crate) fn type_definitions(&self) -> &'a [KconfigTypeDefinition] {
        match self.entry {
            KconfigSymbolDefinitionEntry::Config(config) => config.type_definitions(),
            KconfigSymbolDefinitionEntry::Menuconfig(menuconfig) => {
                menuconfig.type_definitions()
            }
            KconfigSymbolDefinitionEntry::Choice(choice) => choice.type_definitions(),
        }
    }

    pub(crate) fn prompt_definitions(&self) -> &'a [KconfigPromptDefinition] {
        match self.entry {
            KconfigSymbolDefinitionEntry::Config(config) => config.prompt_definitions(),
            KconfigSymbolDefinitionEntry::Menuconfig(menuconfig) => {
                menuconfig.prompt_definitions()
            }
            KconfigSymbolDefinitionEntry::Choice(choice) => choice.prompt_definitions(),
        }
    }

    pub(crate) fn prompt_consistency_violation(
        &self,
    ) -> Option<KconfigPromptConsistencyViolation> {
        let prompt_definitions = self.prompt_definitions();
        if prompt_definitions.len() <= 1 {
            return None;
        }

        Some(KconfigPromptConsistencyViolation {
            symbol: self.symbol().clone(),
            prompts: prompt_definitions
                .iter()
                .map(|prompt_definition| KconfigPromptConsistencyDefinition {
                    prompt: prompt_definition.prompt().to_string(),
                    condition: prompt_definition.condition().map(str::to_string),
                    symbol_definition_kind: self.kind(),
                    definition_line: self.line(),
                    prompt_line: prompt_definition.line(),
                    directive: prompt_definition.directive().clone(),
                })
                .collect(),
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigSymbolDefinitionGroup<'a> {
    symbol: &'a KconfigSymbol,
    definitions: Vec<KconfigSymbolDefinition<'a>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KconfigDefinitionSourceLocation<'a> {
    symbol: &'a KconfigSymbol,
    kind: KconfigSymbolDefinitionKind,
    line: usize,
    end_line: usize,
    directive: &'a KconfigRawLine,
}

#[allow(dead_code)]
impl<'a> KconfigDefinitionSourceLocation<'a> {
    pub(crate) fn symbol(&self) -> &'a KconfigSymbol {
        self.symbol
    }

    pub(crate) fn kind(&self) -> KconfigSymbolDefinitionKind {
        self.kind
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(crate) fn directive(&self) -> &'a KconfigRawLine {
        self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigTypeConsistencyDefinition {
    kind: KconfigSymbolType,
    symbol_definition_kind: KconfigSymbolDefinitionKind,
    definition_line: usize,
    type_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigTypeConsistencyDefinition {
    pub(crate) fn kind(&self) -> KconfigSymbolType {
        self.kind
    }

    pub(crate) fn symbol_definition_kind(&self) -> KconfigSymbolDefinitionKind {
        self.symbol_definition_kind
    }

    pub(crate) fn definition_line(&self) -> usize {
        self.definition_line
    }

    pub(crate) fn type_line(&self) -> usize {
        self.type_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigTypeConsistencyViolation {
    symbol: KconfigSymbol,
    definitions: Vec<KconfigTypeConsistencyDefinition>,
}

#[allow(dead_code)]
impl KconfigTypeConsistencyViolation {
    pub(crate) fn symbol(&self) -> &KconfigSymbol {
        &self.symbol
    }

    pub(crate) fn definitions(&self) -> &[KconfigTypeConsistencyDefinition] {
        &self.definitions
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigPromptConsistencyDefinition {
    prompt: String,
    condition: Option<String>,
    symbol_definition_kind: KconfigSymbolDefinitionKind,
    definition_line: usize,
    prompt_line: usize,
    directive: KconfigRawLine,
}

#[allow(dead_code)]
impl KconfigPromptConsistencyDefinition {
    pub(crate) fn prompt(&self) -> &str {
        &self.prompt
    }

    pub(crate) fn condition(&self) -> Option<&str> {
        self.condition.as_deref()
    }

    pub(crate) fn symbol_definition_kind(&self) -> KconfigSymbolDefinitionKind {
        self.symbol_definition_kind
    }

    pub(crate) fn definition_line(&self) -> usize {
        self.definition_line
    }

    pub(crate) fn prompt_line(&self) -> usize {
        self.prompt_line
    }

    pub(crate) fn directive(&self) -> &KconfigRawLine {
        &self.directive
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigPromptConsistencyViolation {
    symbol: KconfigSymbol,
    prompts: Vec<KconfigPromptConsistencyDefinition>,
}

#[allow(dead_code)]
impl KconfigPromptConsistencyViolation {
    pub(crate) fn symbol(&self) -> &KconfigSymbol {
        &self.symbol
    }

    pub(crate) fn prompts(&self) -> &[KconfigPromptConsistencyDefinition] {
        &self.prompts
    }
}

#[allow(dead_code)]
impl<'a> KconfigSymbolDefinitionGroup<'a> {
    pub(crate) fn symbol(&self) -> &'a KconfigSymbol {
        self.symbol
    }

    pub(crate) fn definitions(&self) -> &[KconfigSymbolDefinition<'a>] {
        &self.definitions
    }

    pub(crate) fn is_multiple(&self) -> bool {
        self.definitions.len() > 1
    }

    pub(crate) fn source_locations(&self) -> Vec<KconfigDefinitionSourceLocation<'a>> {
        self.definitions
            .iter()
            .map(|definition| definition.source_location())
            .collect()
    }

    pub(crate) fn type_consistency_violation(
        &self,
    ) -> Option<KconfigTypeConsistencyViolation> {
        let definitions = self
            .definitions
            .iter()
            .flat_map(|definition| {
                definition.type_definitions().iter().map(|type_definition| {
                    KconfigTypeConsistencyDefinition {
                        kind: type_definition.kind(),
                        symbol_definition_kind: definition.kind(),
                        definition_line: definition.line(),
                        type_line: type_definition.line(),
                        directive: type_definition.directive().clone(),
                    }
                })
            })
            .collect::<Vec<_>>();

        let expected = definitions.first()?.kind();
        if definitions
            .iter()
            .all(|definition| definition.kind() == expected)
        {
            return None;
        }

        Some(KconfigTypeConsistencyViolation {
            symbol: self.symbol.clone(),
            definitions,
        })
    }
}

#[allow(dead_code)]
impl KconfigDocument {
    pub(crate) fn symbol_definitions(
        &self,
    ) -> impl Iterator<Item = KconfigSymbolDefinition<'_>> {
        self.nodes.iter().filter_map(|node| match node {
            KconfigNode::Config(config) => Some(KconfigSymbolDefinition::config(config)),
            KconfigNode::Menuconfig(menuconfig) => {
                Some(KconfigSymbolDefinition::menuconfig(menuconfig))
            }
            KconfigNode::Choice(choice) if choice.symbol().is_some() => {
                Some(KconfigSymbolDefinition::choice(choice))
            }
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

    pub(crate) fn symbol_definition_groups(&self) -> Vec<KconfigSymbolDefinitionGroup<'_>> {
        let mut groups = BTreeMap::new();
        for definition in self.symbol_definitions() {
            groups
                .entry(definition.symbol())
                .or_insert_with(Vec::new)
                .push(definition);
        }
        groups
            .into_iter()
            .map(|(symbol, definitions)| KconfigSymbolDefinitionGroup {
                symbol,
                definitions,
            })
            .collect()
    }

    pub(crate) fn multiple_symbol_definition_groups(
        &self,
    ) -> Vec<KconfigSymbolDefinitionGroup<'_>> {
        self.symbol_definition_groups()
            .into_iter()
            .filter(KconfigSymbolDefinitionGroup::is_multiple)
            .collect()
    }

    pub(crate) fn symbol_definition_source_locations(
        &self,
    ) -> impl Iterator<Item = KconfigDefinitionSourceLocation<'_>> {
        self.symbol_definitions()
            .map(|definition| definition.source_location())
    }

    pub(crate) fn type_consistency_violations(
        &self,
    ) -> Vec<KconfigTypeConsistencyViolation> {
        self.symbol_definition_groups()
            .into_iter()
            .filter_map(|group| group.type_consistency_violation())
            .collect()
    }

    pub(crate) fn prompt_consistency_violations(
        &self,
    ) -> Vec<KconfigPromptConsistencyViolation> {
        self.symbol_definitions()
            .filter_map(|definition| definition.prompt_consistency_violation())
            .collect()
    }

    pub(crate) fn type_definitions(&self) -> impl Iterator<Item = &KconfigTypeDefinition> {
        self.nodes.iter().flat_map(|node| {
            let type_definitions: &[KconfigTypeDefinition] = match node {
                KconfigNode::Config(config) => config.type_definitions(),
                KconfigNode::Menuconfig(menuconfig) => menuconfig.type_definitions(),
                KconfigNode::Choice(choice) => choice.type_definitions(),
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
            type_definitions.iter()
        })
    }

    pub(crate) fn prompt_definitions(&self) -> impl Iterator<Item = &KconfigPromptDefinition> {
        self.nodes.iter().flat_map(|node| {
            let prompt_definitions: &[KconfigPromptDefinition] = match node {
                KconfigNode::Config(config) => config.prompt_definitions(),
                KconfigNode::Menuconfig(menuconfig) => menuconfig.prompt_definitions(),
                KconfigNode::Choice(choice) => choice.prompt_definitions(),
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
            prompt_definitions.iter()
        })
    }

    pub(crate) fn default_definitions(&self) -> impl Iterator<Item = &KconfigDefaultDefinition> {
        self.nodes.iter().flat_map(|node| {
            let default_definitions: &[KconfigDefaultDefinition] = match node {
                KconfigNode::Config(config) => config.default_definitions(),
                KconfigNode::Menuconfig(menuconfig) => menuconfig.default_definitions(),
                KconfigNode::Choice(choice) => choice.default_definitions(),
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
            default_definitions.iter()
        })
    }

    pub(crate) fn range_definitions(&self) -> impl Iterator<Item = &KconfigRangeDefinition> {
        self.nodes.iter().flat_map(|node| {
            let range_definitions: &[KconfigRangeDefinition] = match node {
                KconfigNode::Config(config) => config.range_definitions(),
                KconfigNode::Menuconfig(menuconfig) => menuconfig.range_definitions(),
                KconfigNode::Choice(choice) => choice.range_definitions(),
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
            range_definitions.iter()
        })
    }

    pub(crate) fn dependency_definitions(
        &self,
    ) -> impl Iterator<Item = &KconfigDependencyDefinition> {
        self.nodes.iter().flat_map(|node| {
            let dependency_definitions: &[KconfigDependencyDefinition] = match node {
                KconfigNode::Config(config) => config.dependency_definitions(),
                KconfigNode::Menuconfig(menuconfig) => menuconfig.dependency_definitions(),
                KconfigNode::Choice(choice) => choice.dependency_definitions(),
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
            dependency_definitions.iter()
        })
    }

    pub(crate) fn select_definitions(&self) -> impl Iterator<Item = &KconfigSelectDefinition> {
        self.nodes.iter().flat_map(|node| {
            let select_definitions: &[KconfigSelectDefinition] = match node {
                KconfigNode::Config(config) => config.select_definitions(),
                KconfigNode::Menuconfig(menuconfig) => menuconfig.select_definitions(),
                KconfigNode::Choice(choice) => choice.select_definitions(),
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
            select_definitions.iter()
        })
    }

    pub(crate) fn imply_definitions(&self) -> impl Iterator<Item = &KconfigImplyDefinition> {
        self.nodes.iter().flat_map(|node| {
            let imply_definitions: &[KconfigImplyDefinition] = match node {
                KconfigNode::Config(config) => config.imply_definitions(),
                KconfigNode::Menuconfig(menuconfig) => menuconfig.imply_definitions(),
                KconfigNode::Choice(choice) => choice.imply_definitions(),
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
            imply_definitions.iter()
        })
    }

    pub(crate) fn option_definitions(&self) -> impl Iterator<Item = &KconfigOptionDefinition> {
        self.nodes.iter().flat_map(|node| {
            let option_definitions: &[KconfigOptionDefinition] = match node {
                KconfigNode::Config(config) => config.option_definitions(),
                KconfigNode::Menuconfig(menuconfig) => menuconfig.option_definitions(),
                KconfigNode::Choice(choice) => choice.option_definitions(),
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
            option_definitions.iter()
        })
    }

    pub(crate) fn modules_definitions(&self) -> impl Iterator<Item = &KconfigModulesDefinition> {
        self.nodes.iter().flat_map(|node| {
            let modules_definitions: &[KconfigModulesDefinition] = match node {
                KconfigNode::Config(config) => config.modules_definitions(),
                KconfigNode::Menuconfig(menuconfig) => menuconfig.modules_definitions(),
                KconfigNode::Choice(choice) => choice.modules_definitions(),
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
            modules_definitions.iter()
        })
    }
}
