use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet, HashSet};

use super::expression::{
    evaluate_kconfig_expr, evaluate_kconfig_expr_after_removed_symbols, parse_kconfig_expr,
    tristate_and, tristate_or, KconfigExpr, TristateLiteral,
};
use super::parser::{
    parse_kconfig_directive, split_kconfig_trailing_comment, KconfigDirective,
};
use super::{
    kconfig_menu_block_end_line_from_nodes, KconfigChoiceEntry, KconfigDefaultDefinition,
    KconfigDependencyDefinition, KconfigDocument, KconfigImplyDefinition, KconfigNode,
    KconfigPromptDefinition, KconfigRawLine, KconfigSelectDefinition,
    KconfigSymbolDefinitionKind,
};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KconfigDefaultReenabledSymbol {
    symbol: String,
    value: TristateLiteral,
}

#[allow(dead_code)]
impl KconfigDefaultReenabledSymbol {
    pub(super) fn symbol(&self) -> &str {
        &self.symbol
    }

    pub(super) fn value(&self) -> TristateLiteral {
        self.value
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KconfigSelectForcedRemovedSymbol {
    source_symbol: String,
    target_symbol: String,
    value: TristateLiteral,
}

#[allow(dead_code)]
impl KconfigSelectForcedRemovedSymbol {
    pub(super) fn source_symbol(&self) -> &str {
        &self.source_symbol
    }

    pub(super) fn target_symbol(&self) -> &str {
        &self.target_symbol
    }

    pub(super) fn value(&self) -> TristateLiteral {
        self.value
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KconfigImplyWeaklyEnabledRemovedSymbol {
    source_symbol: String,
    target_symbol: String,
    value: TristateLiteral,
}

#[allow(dead_code)]
impl KconfigImplyWeaklyEnabledRemovedSymbol {
    pub(super) fn source_symbol(&self) -> &str {
        &self.source_symbol
    }

    pub(super) fn target_symbol(&self) -> &str {
        &self.target_symbol
    }

    pub(super) fn value(&self) -> TristateLiteral {
        self.value
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KconfigImpossibleChoice {
    choice_symbol: Option<String>,
    line: usize,
    visibility: TristateLiteral,
    member_symbols: Vec<String>,
}

#[allow(dead_code)]
impl KconfigImpossibleChoice {
    pub(super) fn choice_symbol(&self) -> Option<&str> {
        self.choice_symbol.as_deref()
    }

    pub(super) fn line(&self) -> usize {
        self.line
    }

    pub(super) fn visibility(&self) -> TristateLiteral {
        self.visibility
    }

    pub(super) fn member_symbols(&self) -> &[String] {
        &self.member_symbols
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KconfigEmptyMenu {
    prompt: String,
    line: usize,
    end_line: usize,
    visibility: TristateLiteral,
}

#[allow(dead_code)]
impl KconfigEmptyMenu {
    pub(super) fn prompt(&self) -> &str {
        &self.prompt
    }

    pub(super) fn line(&self) -> usize {
        self.line
    }

    pub(super) fn end_line(&self) -> usize {
        self.end_line
    }

    pub(super) fn visibility(&self) -> TristateLiteral {
        self.visibility
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KconfigOrphanedSymbolDefinition {
    symbol: String,
    definition_kind: KconfigSymbolDefinitionKind,
    line: usize,
    visibility: TristateLiteral,
}

#[allow(dead_code)]
impl KconfigOrphanedSymbolDefinition {
    pub(super) fn symbol(&self) -> &str {
        &self.symbol
    }

    pub(super) fn definition_kind(&self) -> KconfigSymbolDefinitionKind {
        self.definition_kind
    }

    pub(super) fn line(&self) -> usize {
        self.line
    }

    pub(super) fn visibility(&self) -> TristateLiteral {
        self.visibility
    }
}

#[allow(dead_code)]
pub(super) fn parse_selected_profile_tristate_values(
    selected_profile_values: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, TristateLiteral>> {
    let mut parsed = BTreeMap::new();
    for (symbol, value) in selected_profile_values {
        let value = match value.trim() {
            "y" => TristateLiteral::Y,
            "m" => TristateLiteral::M,
            "n" => TristateLiteral::N,
            other => {
                anyhow::bail!(
                    "selected profile value for Kconfig symbol '{}' is not tristate: {}",
                    symbol,
                    other
                );
            }
        };
        parsed.insert(symbol.clone(), value);
    }
    Ok(parsed)
}
#[allow(dead_code)]
pub(super) fn evaluate_kconfig_visibility(
    prompt_definitions: &[KconfigPromptDefinition],
    dependency_definitions: &[KconfigDependencyDefinition],
    symbol_values: &BTreeMap<String, TristateLiteral>,
) -> Option<TristateLiteral> {
    evaluate_kconfig_visibility_with(
        prompt_definitions,
        dependency_definitions,
        |expr| evaluate_kconfig_expr(expr, symbol_values),
    )
}

#[allow(dead_code)]
fn evaluate_kconfig_visibility_with(
    prompt_definitions: &[KconfigPromptDefinition],
    dependency_definitions: &[KconfigDependencyDefinition],
    mut evaluate_expr: impl FnMut(&KconfigExpr) -> Option<TristateLiteral>,
) -> Option<TristateLiteral> {
    if prompt_definitions.is_empty() {
        return Some(TristateLiteral::N);
    }

    let mut prompt_visibility = TristateLiteral::N;
    for prompt in prompt_definitions {
        let visibility = match prompt.condition() {
            Some(condition) => evaluate_expr(&parse_kconfig_expr(condition)?)?,
            None => TristateLiteral::Y,
        };
        prompt_visibility = tristate_or(prompt_visibility, visibility);
    }

    let dependency_visibility =
        evaluate_kconfig_dependency_visibility_with(dependency_definitions, &mut evaluate_expr)?;

    Some(tristate_and(prompt_visibility, dependency_visibility))
}

#[allow(dead_code)]
fn evaluate_kconfig_dependency_visibility_with(
    dependency_definitions: &[KconfigDependencyDefinition],
    mut evaluate_expr: impl FnMut(&KconfigExpr) -> Option<TristateLiteral>,
) -> Option<TristateLiteral> {
    let mut dependency_visibility = TristateLiteral::Y;
    for dependency in dependency_definitions {
        let expr = parse_kconfig_expr(dependency.expression())?;
        dependency_visibility = tristate_and(dependency_visibility, evaluate_expr(&expr)?);
    }

    Some(dependency_visibility)
}

#[allow(dead_code)]
pub(super) fn evaluate_kconfig_reachability_under_selected_profile(
    prompt_definitions: &[KconfigPromptDefinition],
    dependency_definitions: &[KconfigDependencyDefinition],
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
) -> Option<bool> {
    evaluate_kconfig_visibility(
        prompt_definitions,
        dependency_definitions,
        selected_profile_values,
    )
    .map(|visibility| visibility != TristateLiteral::N)
}

#[allow(dead_code)]
fn evaluate_kconfig_dependency_visibility_after_removed_symbols(
    dependency_definitions: &[KconfigDependencyDefinition],
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<TristateLiteral> {
    evaluate_kconfig_dependency_visibility_with(dependency_definitions, |expr| {
        evaluate_kconfig_expr_after_removed_symbols(
            expr,
            selected_profile_values,
            removed_symbols,
        )
    })
}

#[allow(dead_code)]
pub(super) fn evaluate_kconfig_visibility_after_removed_symbols(
    prompt_definitions: &[KconfigPromptDefinition],
    dependency_definitions: &[KconfigDependencyDefinition],
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<TristateLiteral> {
    evaluate_kconfig_visibility_with(
        prompt_definitions,
        dependency_definitions,
        |expr| {
            evaluate_kconfig_expr_after_removed_symbols(
                expr,
                selected_profile_values,
                removed_symbols,
            )
        },
    )
}

#[allow(dead_code)]
pub(super) fn evaluate_kconfig_reachability_after_removed_symbols(
    prompt_definitions: &[KconfigPromptDefinition],
    dependency_definitions: &[KconfigDependencyDefinition],
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<bool> {
    evaluate_kconfig_visibility_after_removed_symbols(
        prompt_definitions,
        dependency_definitions,
        selected_profile_values,
        removed_symbols,
    )
    .map(|visibility| visibility != TristateLiteral::N)
}

#[allow(dead_code)]
pub(super) fn evaluate_kconfig_defaults(
    default_definitions: &[KconfigDefaultDefinition],
    symbol_values: &BTreeMap<String, TristateLiteral>,
) -> Option<TristateLiteral> {
    evaluate_kconfig_defaults_with(default_definitions, |expr| {
        evaluate_kconfig_expr(expr, symbol_values)
    })
}

#[allow(dead_code)]
fn evaluate_kconfig_defaults_with(
    default_definitions: &[KconfigDefaultDefinition],
    evaluate_expr: impl FnMut(&KconfigExpr) -> Option<TristateLiteral>,
) -> Option<TristateLiteral> {
    evaluate_first_active_kconfig_default(default_definitions.iter(), evaluate_expr)
        .map(|value| value.unwrap_or(TristateLiteral::N))
}

#[allow(dead_code)]
fn evaluate_first_active_kconfig_default<'a>(
    default_definitions: impl IntoIterator<Item = &'a KconfigDefaultDefinition>,
    mut evaluate_expr: impl FnMut(&KconfigExpr) -> Option<TristateLiteral>,
) -> Option<Option<TristateLiteral>> {
    for default in default_definitions {
        let condition = match default.condition() {
            Some(condition) => evaluate_expr(&parse_kconfig_expr(condition)?)?,
            None => TristateLiteral::Y,
        };
        if condition == TristateLiteral::N {
            continue;
        }

        return Some(Some(evaluate_expr(&parse_kconfig_expr(default.value())?)?));
    }

    Some(None)
}

#[allow(dead_code)]
pub(super) fn detect_kconfig_symbols_reenabled_by_defaults(
    document: &KconfigDocument,
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<Vec<KconfigDefaultReenabledSymbol>> {
    let mut default_definitions_by_symbol: BTreeMap<&str, Vec<&KconfigDefaultDefinition>> =
        BTreeMap::new();
    for node in document.nodes() {
        let Some((symbol, default_definitions)) = kconfig_node_symbol_defaults(node) else {
            continue;
        };
        if removed_symbols.contains(symbol) {
            default_definitions_by_symbol
                .entry(symbol)
                .or_default()
                .extend(default_definitions.iter());
        }
    }

    let mut reenabled = Vec::new();
    for (symbol, default_definitions) in default_definitions_by_symbol {
        let value = evaluate_first_active_kconfig_default(
            default_definitions.into_iter(),
            |expr| {
                evaluate_kconfig_expr_after_removed_symbols(
                    expr,
                    selected_profile_values,
                    removed_symbols,
                )
            },
        )?
        .unwrap_or(TristateLiteral::N);
        if value != TristateLiteral::N {
            reenabled.push(KconfigDefaultReenabledSymbol {
                symbol: symbol.to_string(),
                value,
            });
        }
    }

    Some(reenabled)
}

#[allow(dead_code)]
fn kconfig_node_symbol_defaults(
    node: &KconfigNode,
) -> Option<(&str, &[KconfigDefaultDefinition])> {
    match node {
        KconfigNode::Config(config) => {
            Some((config.symbol().as_str(), config.default_definitions()))
        }
        KconfigNode::Menuconfig(menuconfig) => {
            Some((menuconfig.symbol().as_str(), menuconfig.default_definitions()))
        }
        KconfigNode::Choice(choice) => choice
            .symbol()
            .map(|symbol| (symbol.as_str(), choice.default_definitions())),
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
        | KconfigNode::SkippedSite(_) => None,
    }
}

#[allow(dead_code)]
pub(super) fn detect_kconfig_removed_symbols_forced_by_select(
    document: &KconfigDocument,
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<Vec<KconfigSelectForcedRemovedSymbol>> {
    let mut forced = Vec::new();
    for node in document.nodes() {
        let Some((source_symbol, select_definitions)) = kconfig_node_symbol_selects(node) else {
            continue;
        };
        let source_value = evaluate_kconfig_symbol_after_removed_symbols(
            source_symbol,
            selected_profile_values,
            removed_symbols,
        )?;
        if source_value == TristateLiteral::N {
            continue;
        }

        for select in select_definitions {
            let target_symbol = select.target().as_str();
            if !removed_symbols.contains(target_symbol) {
                continue;
            }

            // Kconfig `select` bypasses the target symbol's own dependencies;
            // only the live source value and the select condition determine
            // whether a removed target is forced back on.
            let condition = match select.condition() {
                Some(condition) => evaluate_kconfig_expr_after_removed_symbols(
                    &parse_kconfig_expr(condition)?,
                    selected_profile_values,
                    removed_symbols,
                )?,
                None => TristateLiteral::Y,
            };
            let value = tristate_and(source_value, condition);
            if value != TristateLiteral::N {
                forced.push(KconfigSelectForcedRemovedSymbol {
                    source_symbol: source_symbol.to_string(),
                    target_symbol: target_symbol.to_string(),
                    value,
                });
            }
        }
    }

    Some(forced)
}

#[allow(dead_code)]
fn evaluate_kconfig_symbol_after_removed_symbols(
    symbol: &str,
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<TristateLiteral> {
    if removed_symbols.contains(symbol) {
        return Some(TristateLiteral::N);
    }

    selected_profile_values.get(symbol).copied()
}

#[allow(dead_code)]
fn kconfig_node_symbol_selects(
    node: &KconfigNode,
) -> Option<(&str, &[KconfigSelectDefinition])> {
    match node {
        KconfigNode::Config(config) => {
            Some((config.symbol().as_str(), config.select_definitions()))
        }
        KconfigNode::Menuconfig(menuconfig) => {
            Some((menuconfig.symbol().as_str(), menuconfig.select_definitions()))
        }
        KconfigNode::Choice(choice) => choice
            .symbol()
            .map(|symbol| (symbol.as_str(), choice.select_definitions())),
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
        | KconfigNode::SkippedSite(_) => None,
    }
}

#[allow(dead_code)]
pub(super) fn detect_kconfig_removed_symbols_weakly_enabled_by_imply(
    document: &KconfigDocument,
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<Vec<KconfigImplyWeaklyEnabledRemovedSymbol>> {
    let mut weakly_enabled = Vec::new();
    for node in document.nodes() {
        let Some((source_symbol, imply_definitions)) = kconfig_node_symbol_implies(node) else {
            continue;
        };
        let source_value = evaluate_kconfig_symbol_after_removed_symbols(
            source_symbol,
            selected_profile_values,
            removed_symbols,
        )?;
        if source_value == TristateLiteral::N {
            continue;
        }

        for imply in imply_definitions {
            let target_symbol = imply.target().as_str();
            if !removed_symbols.contains(target_symbol) {
                continue;
            }

            let condition = match imply.condition() {
                Some(condition) => evaluate_kconfig_expr_after_removed_symbols(
                    &parse_kconfig_expr(condition)?,
                    selected_profile_values,
                    removed_symbols,
                )?,
                None => TristateLiteral::Y,
            };
            let implied_value = tristate_and(source_value, condition);
            let target_dependency_upper_bound =
                evaluate_kconfig_symbol_dependency_upper_bound_after_removed_symbols(
                    document,
                    target_symbol,
                    selected_profile_values,
                    removed_symbols,
                )?;
            let value = tristate_and(implied_value, target_dependency_upper_bound);
            if value != TristateLiteral::N {
                weakly_enabled.push(KconfigImplyWeaklyEnabledRemovedSymbol {
                    source_symbol: source_symbol.to_string(),
                    target_symbol: target_symbol.to_string(),
                    value,
                });
            }
        }
    }

    Some(weakly_enabled)
}

#[allow(dead_code)]
pub(super) fn evaluate_kconfig_symbol_dependency_upper_bound_after_removed_symbols(
    document: &KconfigDocument,
    target_symbol: &str,
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<TristateLiteral> {
    let mut saw_definition = false;
    let mut upper_bound = TristateLiteral::N;

    for node in document.nodes() {
        let Some((symbol, dependency_definitions)) = kconfig_node_symbol_dependencies(node) else {
            continue;
        };
        if symbol != target_symbol {
            continue;
        }

        saw_definition = true;
        let definition_upper_bound = evaluate_kconfig_dependency_visibility_after_removed_symbols(
            dependency_definitions,
            selected_profile_values,
            removed_symbols,
        )?;
        upper_bound = tristate_or(upper_bound, definition_upper_bound);
    }

    if saw_definition {
        Some(upper_bound)
    } else {
        Some(TristateLiteral::Y)
    }
}

#[allow(dead_code)]
fn kconfig_node_symbol_dependencies(
    node: &KconfigNode,
) -> Option<(&str, &[KconfigDependencyDefinition])> {
    match node {
        KconfigNode::Config(config) => {
            Some((config.symbol().as_str(), config.dependency_definitions()))
        }
        KconfigNode::Menuconfig(menuconfig) => Some((
            menuconfig.symbol().as_str(),
            menuconfig.dependency_definitions(),
        )),
        KconfigNode::Choice(choice) => choice
            .symbol()
            .map(|symbol| (symbol.as_str(), choice.dependency_definitions())),
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
        | KconfigNode::SkippedSite(_) => None,
    }
}

#[allow(dead_code)]
fn kconfig_node_symbol_implies(
    node: &KconfigNode,
) -> Option<(&str, &[KconfigImplyDefinition])> {
    match node {
        KconfigNode::Config(config) => {
            Some((config.symbol().as_str(), config.imply_definitions()))
        }
        KconfigNode::Menuconfig(menuconfig) => {
            Some((menuconfig.symbol().as_str(), menuconfig.imply_definitions()))
        }
        KconfigNode::Choice(choice) => choice
            .symbol()
            .map(|symbol| (symbol.as_str(), choice.imply_definitions())),
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
        | KconfigNode::SkippedSite(_) => None,
    }
}

#[allow(dead_code)]
pub(super) fn detect_kconfig_impossible_choices(
    document: &KconfigDocument,
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<Vec<KconfigImpossibleChoice>> {
    let mut impossible = Vec::new();
    let nodes = document.nodes();
    let mut idx = 0usize;

    while idx < nodes.len() {
        let KconfigNode::Choice(choice) = &nodes[idx] else {
            idx += 1;
            continue;
        };
        let members = kconfig_choice_members_after_removed_symbols(
            nodes,
            idx,
            selected_profile_values,
            removed_symbols,
        )?;
        let next_index = members.next_index;
        let visibility = evaluate_kconfig_visibility_after_removed_symbols(
            choice.prompt_definitions(),
            choice.dependency_definitions(),
            selected_profile_values,
            removed_symbols,
        )?;
        if visibility != TristateLiteral::N
            && !kconfig_choice_is_optional(choice)
            && !members.has_reachable_member
        {
            impossible.push(KconfigImpossibleChoice {
                choice_symbol: choice.symbol().map(|symbol| symbol.as_str().to_string()),
                line: choice.line(),
                visibility,
                member_symbols: members.member_symbols,
            });
        }

        idx = next_index;
    }

    Some(impossible)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KconfigChoiceMembersAfterRemoval {
    member_symbols: Vec<String>,
    has_reachable_member: bool,
    next_index: usize,
}

fn kconfig_choice_members_after_removed_symbols(
    nodes: &[KconfigNode],
    choice_index: usize,
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<KconfigChoiceMembersAfterRemoval> {
    let mut member_symbols = Vec::new();
    let mut has_reachable_member = false;
    let mut nested_choice_depth = 0usize;
    let mut idx = choice_index + 1;

    while idx < nodes.len() {
        match &nodes[idx] {
            KconfigNode::Choice(_) => {
                nested_choice_depth += 1;
            }
            KconfigNode::Endchoice(_) if nested_choice_depth == 0 => {
                return Some(KconfigChoiceMembersAfterRemoval {
                    member_symbols,
                    has_reachable_member,
                    next_index: idx + 1,
                });
            }
            KconfigNode::Endchoice(_) => {
                nested_choice_depth -= 1;
            }
            node if nested_choice_depth == 0 => {
                if let Some((symbol, prompts, dependencies)) =
                    kconfig_node_config_like_visibility(node)
                {
                    member_symbols.push(symbol.to_string());
                    if !removed_symbols.contains(symbol) {
                        let visibility = evaluate_kconfig_visibility_after_removed_symbols(
                            prompts,
                            dependencies,
                            selected_profile_values,
                            removed_symbols,
                        )?;
                        if visibility != TristateLiteral::N {
                            has_reachable_member = true;
                        }
                    }
                }
            }
            _ => {}
        }

        idx += 1;
    }

    Some(KconfigChoiceMembersAfterRemoval {
        member_symbols,
        has_reachable_member,
        next_index: idx,
    })
}

fn kconfig_node_config_like_visibility(
    node: &KconfigNode,
) -> Option<(
    &str,
    &[KconfigPromptDefinition],
    &[KconfigDependencyDefinition],
)> {
    match node {
        KconfigNode::Config(config) => Some((
            config.symbol().as_str(),
            config.prompt_definitions(),
            config.dependency_definitions(),
        )),
        KconfigNode::Menuconfig(menuconfig) => Some((
            menuconfig.symbol().as_str(),
            menuconfig.prompt_definitions(),
            menuconfig.dependency_definitions(),
        )),
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
    }
}

fn kconfig_choice_is_optional(choice: &KconfigChoiceEntry) -> bool {
    choice.option_definitions().iter().any(|option| {
        option.name() == "optional" && option.value().is_none()
    }) || choice.body().iter().any(|line| {
        let (directive, _) = split_kconfig_trailing_comment(line.text());
        directive.trim_start() == "optional"
    })
}

#[allow(dead_code)]
pub(super) fn detect_kconfig_empty_menus(
    document: &KconfigDocument,
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<Vec<KconfigEmptyMenu>> {
    let mut empty = Vec::new();
    let nodes = document.nodes();
    for (idx, node) in nodes.iter().enumerate() {
        let KconfigNode::Menu(menu) = node else {
            continue;
        };

        let visibility = evaluate_kconfig_body_visibility_after_removed_symbols(
            menu.body(),
            selected_profile_values,
            removed_symbols,
        )?;
        let end_line = kconfig_menu_block_end_line_from_nodes(nodes, idx)?;
        if visibility != TristateLiteral::N
            && !kconfig_menu_has_reachable_content_after_removed_symbols(
                nodes,
                idx,
                selected_profile_values,
                removed_symbols,
            )?
        {
            empty.push(KconfigEmptyMenu {
                prompt: menu.prompt().to_string(),
                line: menu.line(),
                end_line,
                visibility,
            });
        }
    }

    Some(empty)
}

fn kconfig_menu_has_reachable_content_after_removed_symbols(
    nodes: &[KconfigNode],
    menu_index: usize,
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<bool> {
    let mut nested_menu_depth = 0usize;
    let mut idx = menu_index + 1;

    while idx < nodes.len() {
        match &nodes[idx] {
            KconfigNode::Menu(_) => {
                nested_menu_depth += 1;
            }
            KconfigNode::Endmenu(_) if nested_menu_depth == 0 => {
                return Some(false);
            }
            KconfigNode::Endmenu(_) => {
                nested_menu_depth -= 1;
            }
            KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_) => {
                return Some(true);
            }
            KconfigNode::Comment(comment) => {
                if evaluate_kconfig_body_visibility_after_removed_symbols(
                    comment.body(),
                    selected_profile_values,
                    removed_symbols,
                )? != TristateLiteral::N
                {
                    return Some(true);
                }
            }
            node => {
                if let Some((symbol, prompts, dependencies)) =
                    kconfig_node_config_like_visibility(node)
                {
                    if !removed_symbols.contains(symbol) {
                        let visibility = evaluate_kconfig_visibility_after_removed_symbols(
                            prompts,
                            dependencies,
                            selected_profile_values,
                            removed_symbols,
                        )?;
                        if visibility != TristateLiteral::N {
                            return Some(true);
                        }
                    }
                }
            }
        }

        idx += 1;
    }

    Some(false)
}

fn evaluate_kconfig_body_visibility_after_removed_symbols(
    body: &[KconfigRawLine],
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<TristateLiteral> {
    let mut visibility = TristateLiteral::Y;
    for line in body {
        let expr = match parse_kconfig_directive(line.text()) {
            Some(KconfigDirective::DependsOn { expr }) => expr,
            Some(KconfigDirective::VisibleIf { expr }) => expr,
            Some(
                KconfigDirective::Entry { .. }
                | KconfigDirective::Select { .. }
                | KconfigDirective::Imply { .. }
                | KconfigDirective::If { .. }
                | KconfigDirective::Default { .. }
                | KconfigDirective::Source { .. },
            )
            | None => continue,
        };
        visibility = tristate_and(
            visibility,
            evaluate_kconfig_expr_after_removed_symbols(
                &parse_kconfig_expr(&expr)?,
                selected_profile_values,
                removed_symbols,
            )?,
        );
    }

    Some(visibility)
}

#[allow(dead_code)]
pub(super) fn detect_kconfig_orphaned_symbol_definitions(
    document: &KconfigDocument,
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<Vec<KconfigOrphanedSymbolDefinition>> {
    let live_reverse_dependencies = kconfig_symbols_with_live_reverse_dependencies(
        document,
        selected_profile_values,
        removed_symbols,
    )?;
    let mut orphaned = Vec::new();

    for node in document.nodes() {
        let Some(definition) = kconfig_node_symbol_definition_solver_inputs(node) else {
            continue;
        };
        if removed_symbols.contains(definition.symbol) {
            continue;
        }

        let selected_value = selected_kconfig_symbol_value_after_removed_symbols(
            definition.symbol,
            selected_profile_values,
            removed_symbols,
        );
        if selected_value != TristateLiteral::N
            || live_reverse_dependencies.contains(definition.symbol)
        {
            continue;
        }

        let visibility = evaluate_kconfig_visibility_after_removed_symbols(
            definition.prompt_definitions,
            definition.dependency_definitions,
            selected_profile_values,
            removed_symbols,
        )?;
        if visibility != TristateLiteral::N {
            continue;
        }

        let default_value = evaluate_kconfig_defaults_after_removed_symbols(
            definition.default_definitions,
            selected_profile_values,
            removed_symbols,
        )?;
        if default_value != TristateLiteral::N {
            continue;
        }

        orphaned.push(KconfigOrphanedSymbolDefinition {
            symbol: definition.symbol.to_string(),
            definition_kind: definition.definition_kind,
            line: definition.line,
            visibility,
        });
    }

    Some(orphaned)
}

struct KconfigSymbolDefinitionSolverInputs<'a> {
    symbol: &'a str,
    definition_kind: KconfigSymbolDefinitionKind,
    line: usize,
    prompt_definitions: &'a [KconfigPromptDefinition],
    dependency_definitions: &'a [KconfigDependencyDefinition],
    default_definitions: &'a [KconfigDefaultDefinition],
}

fn kconfig_node_symbol_definition_solver_inputs(
    node: &KconfigNode,
) -> Option<KconfigSymbolDefinitionSolverInputs<'_>> {
    match node {
        KconfigNode::Config(config) => Some(KconfigSymbolDefinitionSolverInputs {
            symbol: config.symbol().as_str(),
            definition_kind: KconfigSymbolDefinitionKind::Config,
            line: config.line(),
            prompt_definitions: config.prompt_definitions(),
            dependency_definitions: config.dependency_definitions(),
            default_definitions: config.default_definitions(),
        }),
        KconfigNode::Menuconfig(menuconfig) => Some(KconfigSymbolDefinitionSolverInputs {
            symbol: menuconfig.symbol().as_str(),
            definition_kind: KconfigSymbolDefinitionKind::Menuconfig,
            line: menuconfig.line(),
            prompt_definitions: menuconfig.prompt_definitions(),
            dependency_definitions: menuconfig.dependency_definitions(),
            default_definitions: menuconfig.default_definitions(),
        }),
        KconfigNode::Choice(choice) => {
            let symbol = choice.symbol()?;
            Some(KconfigSymbolDefinitionSolverInputs {
                symbol: symbol.as_str(),
                definition_kind: KconfigSymbolDefinitionKind::Choice,
                line: choice.line(),
                prompt_definitions: choice.prompt_definitions(),
                dependency_definitions: choice.dependency_definitions(),
                default_definitions: choice.default_definitions(),
            })
        }
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
        | KconfigNode::SkippedSite(_) => None,
    }
}

fn kconfig_symbols_with_live_reverse_dependencies(
    document: &KconfigDocument,
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<BTreeSet<String>> {
    let mut symbols = BTreeSet::new();
    for node in document.nodes() {
        if let Some((source_symbol, select_definitions)) = kconfig_node_symbol_selects(node) {
            let source_value = selected_kconfig_symbol_value_after_removed_symbols(
                source_symbol,
                selected_profile_values,
                removed_symbols,
            );
            if source_value != TristateLiteral::N {
                for select in select_definitions {
                    let condition = match select.condition() {
                        Some(condition) => evaluate_kconfig_expr_after_removed_symbols(
                            &parse_kconfig_expr(condition)?,
                            selected_profile_values,
                            removed_symbols,
                        )?,
                        None => TristateLiteral::Y,
                    };
                    if tristate_and(source_value, condition) != TristateLiteral::N {
                        symbols.insert(select.target().as_str().to_string());
                    }
                }
            }
        }

        if let Some((source_symbol, imply_definitions)) = kconfig_node_symbol_implies(node) {
            let source_value = selected_kconfig_symbol_value_after_removed_symbols(
                source_symbol,
                selected_profile_values,
                removed_symbols,
            );
            if source_value != TristateLiteral::N {
                for imply in imply_definitions {
                    let condition = match imply.condition() {
                        Some(condition) => evaluate_kconfig_expr_after_removed_symbols(
                            &parse_kconfig_expr(condition)?,
                            selected_profile_values,
                            removed_symbols,
                        )?,
                        None => TristateLiteral::Y,
                    };
                    if tristate_and(source_value, condition) != TristateLiteral::N {
                        symbols.insert(imply.target().as_str().to_string());
                    }
                }
            }
        }
    }

    Some(symbols)
}

fn selected_kconfig_symbol_value_after_removed_symbols(
    symbol: &str,
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> TristateLiteral {
    if removed_symbols.contains(symbol) {
        return TristateLiteral::N;
    }

    selected_profile_values
        .get(symbol)
        .copied()
        .unwrap_or(TristateLiteral::N)
}

#[allow(dead_code)]
pub(super) fn evaluate_kconfig_defaults_after_removed_symbols(
    default_definitions: &[KconfigDefaultDefinition],
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) -> Option<TristateLiteral> {
    evaluate_kconfig_defaults_with(default_definitions, |expr| {
        evaluate_kconfig_expr_after_removed_symbols(
            expr,
            selected_profile_values,
            removed_symbols,
        )
    })
}
