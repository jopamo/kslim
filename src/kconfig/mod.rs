//! Kconfig parsing and tristate-aware simplification support.
//!
//! The current reducer-owned Kconfig surface is still narrow:
//! - remove declared config blocks
//! - rewrite explicit config defaults
//! - simplify proven Kconfig graph edges that reference removed symbols
//! - drop stale `source` edges whose targets were removed

use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Component, Path, PathBuf};

#[cfg(test)]
use crate::edit_reason::{EditProofSource, EditReason, LineRange};

mod ast;
mod expression;
mod parser;
mod rewrite;
mod solver;
mod report;
#[allow(unused_imports)]
pub(crate) use ast::{
    parse_kconfig_document, KconfigBlankLine, KconfigChoiceEntry, KconfigCommentEntry,
    KconfigConfigEntry, KconfigDefaultDefinition, KconfigDependencyDefinition,
    KconfigDefinitionSourceLocation, KconfigDocument, KconfigEndchoiceEntry,
    KconfigEndifEntry, KconfigEndmenuEntry, KconfigHelpBlock, KconfigIfEntry,
    KconfigImplyDefinition, KconfigLineComment, KconfigMainmenuEntry, KconfigMenuEntry,
    KconfigMenuconfigEntry, KconfigModulesDefinition, KconfigNode, KconfigOptionDefinition,
    KconfigOrsourceEntry, KconfigOsourceEntry, KconfigPromptConsistencyDefinition,
    KconfigPromptConsistencyViolation,
    KconfigPromptDefinition, KconfigRangeDefinition, KconfigRawLine, KconfigRsourceEntry,
    KconfigSelectDefinition, KconfigSkippedSite, KconfigSourceEntry, KconfigSymbolType,
    KconfigSymbolDefinition, KconfigSymbolDefinitionGroup, KconfigSymbolDefinitionKind,
    KconfigTypeConsistencyDefinition, KconfigTypeConsistencyViolation, KconfigTypeDefinition,
};
#[allow(unused_imports)]
pub(crate) use report::{
    kconfig_solver_report, kconfig_solver_report_for_arch_policy,
    read_kconfig_selected_profile_values,
    KconfigRelationRewriteStats, KconfigReportCounts, KconfigSolverDeadSymbolDefinitionProof,
    KconfigSolverDefaultReenabledSymbol, KconfigSolverEmptyMenu, KconfigSolverImpossibleChoice,
    KconfigSolverOrphanedSymbolDefinition, KconfigSolverReport, KconfigSolverReverseDependency,
    KconfigSolverSkippedFile, UnsupportedKconfigExpression,
};
#[allow(unused_imports)]
pub(crate) use rewrite::{
    prune_configs, rewrite_dead_kconfig_symbol_definitions, rewrite_empty_kconfig_menus,
    rewrite_kconfig_defaults, rewrite_kconfig_relations, rewrite_kconfig_sources,
};

use expression::{parse_kconfig_expr, KconfigExpr};
#[cfg(test)]
use expression::{
    equivalent_kconfig_expr_simplification,
    evaluate_kconfig_expr, evaluate_kconfig_expr_after_removed_symbols,
    kconfig_expr_rewrite_is_tristate_equivalent, render_kconfig_expr, simplify_kconfig_expr,
    TristateLiteral,
};
use parser::{
    parse_kconfig_directive, split_kconfig_if_clause, split_kconfig_trailing_comment,
    KconfigDirective,
};
#[cfg(test)]
use parser::KconfigEntryKind;
pub(crate) use parser::parse_kconfig_source;
use solver::{
    detect_kconfig_empty_menus, detect_kconfig_orphaned_symbol_definitions,
    parse_selected_profile_tristate_values,
};
#[cfg(test)]
use solver::{
    detect_kconfig_impossible_choices, detect_kconfig_removed_symbols_forced_by_select,
    detect_kconfig_removed_symbols_weakly_enabled_by_imply,
    detect_kconfig_symbols_reenabled_by_defaults, evaluate_kconfig_defaults,
    evaluate_kconfig_defaults_after_removed_symbols,
    evaluate_kconfig_reachability_after_removed_symbols,
    evaluate_kconfig_reachability_under_selected_profile,
    evaluate_kconfig_symbol_dependency_upper_bound_after_removed_symbols,
    evaluate_kconfig_visibility, evaluate_kconfig_visibility_after_removed_symbols,
};
#[cfg(test)]
use rewrite::KCONFIG_UNKNOWN_REMOVED_TARGET_CONDITION_REASON;

#[allow(dead_code)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct KconfigLiveArchSymbolUsage {
    symbols: BTreeSet<String>,
    unknown_expressions: bool,
}

impl KconfigLiveArchSymbolUsage {
    fn preserves_symbol(&self, symbol: &str) -> bool {
        self.unknown_expressions || self.symbols.contains(symbol)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigSource {
    pub path: String,
    pub optional: bool,
    pub relative: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct KconfigSourceRemovalProof {
    pub file: PathBuf,
    pub line: usize,
    pub source: String,
    pub optional: bool,
    pub relative: bool,
    pub removed_target: PathBuf,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KconfigDeadSymbolDefinitionProof {
    pub file: PathBuf,
    pub symbol: String,
    pub definition_kind: KconfigSymbolDefinitionKind,
    pub start_line: usize,
    pub end_line: usize,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct KconfigEmptyMenuRemovalProof {
    pub file: PathBuf,
    pub prompt: String,
    pub start_line: usize,
    pub end_line: usize,
}


#[allow(dead_code)]
pub(crate) fn prove_dead_kconfig_symbol_definitions(
    root: &Path,
    selected_profile_values: &BTreeMap<String, String>,
    removed_configs: &[String],
) -> Result<Vec<KconfigDeadSymbolDefinitionProof>> {
    prove_dead_kconfig_symbol_definitions_for_arch_policy(
        root,
        selected_profile_values,
        removed_configs,
        &crate::config::ArchPolicyConfig::default(),
    )
}

#[allow(dead_code)]
pub(crate) fn prove_dead_kconfig_symbol_definitions_for_arch_policy(
    root: &Path,
    selected_profile_values: &BTreeMap<String, String>,
    removed_configs: &[String],
    arch_policy: &crate::config::ArchPolicyConfig,
) -> Result<Vec<KconfigDeadSymbolDefinitionProof>> {
    let selected_profile_values = parse_selected_profile_tristate_values(selected_profile_values)?;
    let removed_symbols: HashSet<&str> = removed_configs.iter().map(String::as_str).collect();
    let live_arch_symbol_usage = kconfig_live_arch_symbol_usage(root, arch_policy)?;
    let mut proofs = Vec::new();

    for path in kconfig_files(root) {
        let content = std::fs::read_to_string(&path)?;
        let document = parse_kconfig_document(&content)?;
        let relative = relative_to_root_path(root, &path);
        let orphaned = detect_kconfig_orphaned_symbol_definitions(
            &document,
            &selected_profile_values,
            &removed_symbols,
        )
        .unwrap_or_default();

        for definition in orphaned {
            if live_arch_symbol_usage.preserves_symbol(definition.symbol()) {
                continue;
            }
            if !kconfig_dead_symbol_definition_kind_is_rewrite_supported(
                definition.definition_kind(),
            ) {
                continue;
            }
            let Some(end_line) = kconfig_symbol_definition_end_line(
                &document,
                definition.symbol(),
                definition.definition_kind(),
                definition.line(),
            ) else {
                continue;
            };
            proofs.push(KconfigDeadSymbolDefinitionProof {
                file: relative.clone(),
                symbol: definition.symbol().to_string(),
                definition_kind: definition.definition_kind(),
                start_line: definition.line(),
                end_line,
            });
        }
    }

    sort_dead_symbol_definition_proofs(&mut proofs);
    Ok(proofs)
}

#[allow(dead_code)]
pub(crate) fn prove_empty_kconfig_menus(
    root: &Path,
    selected_profile_values: &BTreeMap<String, String>,
    removed_configs: &[String],
) -> Result<Vec<KconfigEmptyMenuRemovalProof>> {
    let selected_profile_values = parse_selected_profile_tristate_values(selected_profile_values)?;
    let removed_symbols: HashSet<&str> = removed_configs.iter().map(String::as_str).collect();
    let mut proofs = Vec::new();

    for path in kconfig_files(root) {
        let content = std::fs::read_to_string(&path)?;
        let document = parse_kconfig_document(&content)?;
        let relative = relative_to_root_path(root, &path);
        let empty_menus = detect_kconfig_empty_menus(
            &document,
            &selected_profile_values,
            &removed_symbols,
        )
        .unwrap_or_default();

        for menu in empty_menus {
            proofs.push(KconfigEmptyMenuRemovalProof {
                file: relative.clone(),
                prompt: menu.prompt().to_string(),
                start_line: menu.line(),
                end_line: menu.end_line(),
            });
        }
    }

    sort_empty_menu_removal_proofs(&mut proofs);
    Ok(proofs)
}


fn kconfig_live_arch_symbol_usage(
    root: &Path,
    arch_policy: &crate::config::ArchPolicyConfig,
) -> Result<KconfigLiveArchSymbolUsage> {
    let live_arches = kconfig_live_arch_names(root, arch_policy)?;
    let mut usage = KconfigLiveArchSymbolUsage::default();
    for path in kconfig_files(root) {
        let relative = relative_to_root_path(root, &path);
        if !kconfig_path_is_live_arch_kconfig(&relative, live_arches.as_ref()) {
            continue;
        }

        let content = std::fs::read_to_string(&path)?;
        let document = parse_kconfig_document(&content)?;
        kconfig_collect_live_arch_symbol_usage(&document, &mut usage);
    }
    Ok(usage)
}

fn kconfig_live_arch_names(
    root: &Path,
    arch_policy: &crate::config::ArchPolicyConfig,
) -> Result<Option<BTreeSet<String>>> {
    let primary = arch_policy
        .primary_arch
        .as_deref()
        .map(crate::config::normalize_arch_name)
        .transpose()?;
    let mut explicit_live = arch_policy
        .secondary_arches
        .iter()
        .map(|arch| crate::config::normalize_arch_name(arch))
        .collect::<Result<BTreeSet<_>>>()?;
    let disabled = arch_policy
        .disabled_arches
        .iter()
        .map(|arch| crate::config::normalize_arch_name(arch))
        .collect::<Result<BTreeSet<_>>>()?;

    if let Some(primary) = primary {
        explicit_live.insert(primary);
    }
    if !explicit_live.is_empty() {
        ensure_live_arches_exist(root, &explicit_live)?;
        return Ok(Some(explicit_live));
    }
    if disabled.is_empty() {
        return Ok(None);
    }

    let live_arches = kernel_tree_arch_names(root)?
        .into_iter()
        .filter(|arch| !disabled.contains(arch))
        .collect::<BTreeSet<_>>();
    if live_arches.is_empty() {
        anyhow::bail!(
            "arch policy disabled every architecture under '{}'; declare at least one live arch",
            root.join("arch").display()
        );
    }
    Ok(Some(live_arches))
}

fn ensure_live_arches_exist(root: &Path, live_arches: &BTreeSet<String>) -> Result<()> {
    let tree_arches = kernel_tree_arch_names(root)?;
    for arch in live_arches {
        if tree_arches.contains(arch) {
            continue;
        }
        if matches!(arch.as_str(), "amd64" | "x86_64") && tree_arches.contains("x86") {
            anyhow::bail!(
                "arch policy selects '{}' but the kernel source tree uses arch/x86 for amd64/x86_64 builds; use 'x86'",
                arch
            );
        }
        anyhow::bail!(
            "arch policy selects '{}' but '{}' does not exist",
            arch,
            root.join("arch").join(arch).display()
        );
    }
    Ok(())
}

fn kernel_tree_arch_names(root: &Path) -> Result<BTreeSet<String>> {
    let arch_root = root.join("arch");
    let mut arches = BTreeSet::new();
    if !arch_root.is_dir() {
        return Ok(arches);
    }
    for entry in std::fs::read_dir(&arch_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        if let Some(name) = entry.file_name().to_str() {
            arches.insert(name.to_string());
        }
    }
    Ok(arches)
}

fn kconfig_path_is_live_arch_kconfig(
    relative: &Path,
    live_arches: Option<&BTreeSet<String>>,
) -> bool {
    let mut components = relative.components();
    matches!(
        (components.next(), components.next()),
        (Some(Component::Normal(root)), Some(Component::Normal(arch)))
            if root == "arch"
                && live_arches
                    .map(|arches| arches.contains(arch.to_string_lossy().as_ref()))
                    .unwrap_or(true)
    )
}

fn kconfig_collect_live_arch_symbol_usage(
    document: &KconfigDocument,
    usage: &mut KconfigLiveArchSymbolUsage,
) {
    for node in document.nodes() {
        match node {
            KconfigNode::Config(config) => {
                usage.symbols.insert(config.symbol().as_str().to_string());
                kconfig_collect_live_arch_symbol_definition_usage(
                    config.prompt_definitions(),
                    config.default_definitions(),
                    config.range_definitions(),
                    config.dependency_definitions(),
                    config.select_definitions(),
                    config.imply_definitions(),
                    usage,
                );
            }
            KconfigNode::Menuconfig(menuconfig) => {
                usage
                    .symbols
                    .insert(menuconfig.symbol().as_str().to_string());
                kconfig_collect_live_arch_symbol_definition_usage(
                    menuconfig.prompt_definitions(),
                    menuconfig.default_definitions(),
                    menuconfig.range_definitions(),
                    menuconfig.dependency_definitions(),
                    menuconfig.select_definitions(),
                    menuconfig.imply_definitions(),
                    usage,
                );
            }
            KconfigNode::Choice(choice) => {
                if let Some(symbol) = choice.symbol() {
                    usage.symbols.insert(symbol.as_str().to_string());
                }
                kconfig_collect_live_arch_symbol_definition_usage(
                    choice.prompt_definitions(),
                    choice.default_definitions(),
                    choice.range_definitions(),
                    choice.dependency_definitions(),
                    choice.select_definitions(),
                    choice.imply_definitions(),
                    usage,
                );
            }
            KconfigNode::If(if_entry) => {
                kconfig_collect_live_arch_expr_usage(if_entry.condition(), usage);
                kconfig_collect_live_arch_raw_body_usage(if_entry.body(), usage);
            }
            KconfigNode::Menu(menu) => {
                kconfig_collect_live_arch_raw_body_usage(menu.body(), usage);
            }
            KconfigNode::Comment(comment) => {
                kconfig_collect_live_arch_raw_body_usage(comment.body(), usage);
            }
            KconfigNode::Endchoice(_)
            | KconfigNode::Endmenu(_)
            | KconfigNode::Endif(_)
            | KconfigNode::Source(_)
            | KconfigNode::Rsource(_)
            | KconfigNode::Osource(_)
            | KconfigNode::Orsource(_)
            | KconfigNode::Mainmenu(_)
            | KconfigNode::LineComment(_)
            | KconfigNode::BlankLine(_)
            | KconfigNode::SkippedSite(_) => {}
        }
    }
}

fn kconfig_collect_live_arch_symbol_definition_usage(
    prompt_definitions: &[KconfigPromptDefinition],
    default_definitions: &[KconfigDefaultDefinition],
    range_definitions: &[KconfigRangeDefinition],
    dependency_definitions: &[KconfigDependencyDefinition],
    select_definitions: &[KconfigSelectDefinition],
    imply_definitions: &[KconfigImplyDefinition],
    usage: &mut KconfigLiveArchSymbolUsage,
) {
    for prompt in prompt_definitions {
        if let Some(condition) = prompt.condition() {
            kconfig_collect_live_arch_expr_usage(condition, usage);
        }
    }
    for default in default_definitions {
        kconfig_collect_live_arch_value_usage(default.value(), usage);
        if let Some(condition) = default.condition() {
            kconfig_collect_live_arch_expr_usage(condition, usage);
        }
    }
    for range in range_definitions {
        kconfig_collect_live_arch_value_usage(range.minimum(), usage);
        kconfig_collect_live_arch_value_usage(range.maximum(), usage);
        if let Some(condition) = range.condition() {
            kconfig_collect_live_arch_expr_usage(condition, usage);
        }
    }
    for dependency in dependency_definitions {
        kconfig_collect_live_arch_expr_usage(dependency.expression(), usage);
    }
    for select in select_definitions {
        usage.symbols.insert(select.target().as_str().to_string());
        if let Some(condition) = select.condition() {
            kconfig_collect_live_arch_expr_usage(condition, usage);
        }
    }
    for imply in imply_definitions {
        usage.symbols.insert(imply.target().as_str().to_string());
        if let Some(condition) = imply.condition() {
            kconfig_collect_live_arch_expr_usage(condition, usage);
        }
    }
}

fn kconfig_collect_live_arch_raw_body_usage(
    body: &[KconfigRawLine],
    usage: &mut KconfigLiveArchSymbolUsage,
) {
    for line in body {
        match parse_kconfig_directive(line.text()) {
            Some(KconfigDirective::Entry { symbol, .. }) => {
                usage.symbols.insert(symbol);
            }
            Some(KconfigDirective::DependsOn { expr })
            | Some(KconfigDirective::VisibleIf { expr })
            | Some(KconfigDirective::If { expr }) => {
                kconfig_collect_live_arch_expr_usage(&expr, usage);
            }
            Some(KconfigDirective::Default { value, condition }) => {
                kconfig_collect_live_arch_value_usage(&value, usage);
                if let Some(condition) = condition {
                    kconfig_collect_live_arch_expr_usage(&condition, usage);
                }
            }
            Some(KconfigDirective::Select { symbol, condition })
            | Some(KconfigDirective::Imply { symbol, condition }) => {
                usage.symbols.insert(symbol);
                if let Some(condition) = condition {
                    kconfig_collect_live_arch_expr_usage(&condition, usage);
                }
            }
            Some(KconfigDirective::Source { .. }) | None => {}
        }
    }
}

fn kconfig_collect_live_arch_value_usage(value: &str, usage: &mut KconfigLiveArchSymbolUsage) {
    if value.trim_start().starts_with('"') {
        return;
    }
    kconfig_collect_live_arch_expr_usage(value, usage);
}

fn kconfig_collect_live_arch_expr_usage(
    expr_src: &str,
    usage: &mut KconfigLiveArchSymbolUsage,
) {
    let Some(expr) = parse_kconfig_expr(expr_src) else {
        usage.unknown_expressions = true;
        return;
    };
    kconfig_collect_expr_symbol_references(&expr, &mut usage.symbols);
}

fn kconfig_collect_expr_symbol_references(expr: &KconfigExpr, symbols: &mut BTreeSet<String>) {
    match expr {
        KconfigExpr::Symbol(symbol) => {
            symbols.insert(symbol.clone());
        }
        KconfigExpr::Literal(_) | KconfigExpr::StringLiteral(_) => {}
        KconfigExpr::Not(inner) => kconfig_collect_expr_symbol_references(inner, symbols),
        KconfigExpr::And(lhs, rhs)
        | KconfigExpr::Or(lhs, rhs)
        | KconfigExpr::Eq(lhs, rhs)
        | KconfigExpr::Ne(lhs, rhs) => {
            kconfig_collect_expr_symbol_references(lhs, symbols);
            kconfig_collect_expr_symbol_references(rhs, symbols);
        }
    }
}

#[allow(dead_code)]
fn kconfig_dead_symbol_definition_kind_is_rewrite_supported(
    definition_kind: KconfigSymbolDefinitionKind,
) -> bool {
    matches!(
        definition_kind,
        KconfigSymbolDefinitionKind::Config | KconfigSymbolDefinitionKind::Menuconfig
    )
}

#[allow(dead_code)]
fn kconfig_symbol_definition_end_line(
    document: &KconfigDocument,
    symbol: &str,
    definition_kind: KconfigSymbolDefinitionKind,
    line: usize,
) -> Option<usize> {
    for node in document.nodes() {
        match node {
            KconfigNode::Config(config)
                if definition_kind == KconfigSymbolDefinitionKind::Config
                    && config.symbol().as_str() == symbol
                    && config.line() == line =>
            {
                return Some(config.end_line());
            }
            KconfigNode::Menuconfig(menuconfig)
                if definition_kind == KconfigSymbolDefinitionKind::Menuconfig
                    && menuconfig.symbol().as_str() == symbol
                    && menuconfig.line() == line =>
            {
                return Some(menuconfig.end_line());
            }
            KconfigNode::Choice(choice)
                if definition_kind == KconfigSymbolDefinitionKind::Choice
                    && choice.symbol().is_some_and(|choice_symbol| choice_symbol.as_str() == symbol)
                    && choice.line() == line =>
            {
                return Some(choice.end_line());
            }
            _ => {}
        }
    }
    None
}


fn kconfig_menu_block_end_line_from_nodes(
    nodes: &[KconfigNode],
    menu_index: usize,
) -> Option<usize> {
    let mut nested_menu_depth = 0usize;
    let mut idx = menu_index + 1;

    while idx < nodes.len() {
        match &nodes[idx] {
            KconfigNode::Menu(_) => {
                nested_menu_depth += 1;
            }
            KconfigNode::Endmenu(endmenu) if nested_menu_depth == 0 => {
                return Some(endmenu.line());
            }
            KconfigNode::Endmenu(_) => {
                nested_menu_depth -= 1;
            }
            _ => {}
        }
        idx += 1;
    }

    None
}

#[allow(dead_code)]
fn kconfig_node_line_range(node: &KconfigNode) -> Option<(usize, usize)> {
    match node {
        KconfigNode::Config(config) => Some((config.line(), config.end_line())),
        KconfigNode::Menuconfig(menuconfig) => {
            Some((menuconfig.line(), menuconfig.end_line()))
        }
        KconfigNode::Choice(choice) => Some((choice.line(), choice.end_line())),
        KconfigNode::Endchoice(endchoice) => Some((endchoice.line(), endchoice.end_line())),
        KconfigNode::Menu(menu) => Some((menu.line(), menu.end_line())),
        KconfigNode::Endmenu(endmenu) => Some((endmenu.line(), endmenu.end_line())),
        KconfigNode::If(if_entry) => Some((if_entry.line(), if_entry.end_line())),
        KconfigNode::Endif(endif) => Some((endif.line(), endif.end_line())),
        KconfigNode::Source(source) => Some((source.line(), source.end_line())),
        KconfigNode::Rsource(rsource) => Some((rsource.line(), rsource.end_line())),
        KconfigNode::Osource(osource) => Some((osource.line(), osource.end_line())),
        KconfigNode::Orsource(orsource) => Some((orsource.line(), orsource.end_line())),
        KconfigNode::Mainmenu(mainmenu) => Some((mainmenu.line(), mainmenu.end_line())),
        KconfigNode::Comment(comment) => Some((comment.line(), comment.end_line())),
        KconfigNode::LineComment(comment) => Some((comment.line(), comment.end_line())),
        KconfigNode::BlankLine(blank) => Some((blank.line(), blank.end_line())),
        KconfigNode::SkippedSite(site) => Some((site.line(), site.end_line())),
    }
}

#[allow(dead_code)]
fn kconfig_symbol_definition_kind_keyword(
    definition_kind: KconfigSymbolDefinitionKind,
) -> &'static str {
    match definition_kind {
        KconfigSymbolDefinitionKind::Config => "config",
        KconfigSymbolDefinitionKind::Menuconfig => "menuconfig",
        KconfigSymbolDefinitionKind::Choice => "choice",
    }
}

#[allow(dead_code)]
fn sort_dead_symbol_definition_proofs(proofs: &mut Vec<KconfigDeadSymbolDefinitionProof>) {
    proofs.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then(left.start_line.cmp(&right.start_line))
            .then(left.end_line.cmp(&right.end_line))
            .then(left.symbol.cmp(&right.symbol))
            .then(
                kconfig_symbol_definition_kind_keyword(left.definition_kind)
                    .cmp(kconfig_symbol_definition_kind_keyword(right.definition_kind)),
            )
    });
    proofs.dedup();
}

#[allow(dead_code)]
fn sort_empty_menu_removal_proofs(proofs: &mut Vec<KconfigEmptyMenuRemovalProof>) {
    proofs.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then(left.start_line.cmp(&right.start_line))
            .then(left.end_line.cmp(&right.end_line))
            .then(left.prompt.cmp(&right.prompt))
    });
    proofs.dedup();
}


pub(crate) fn resolve_kconfig_source(
    root: &Path,
    current_dir: &Path,
    source: &KconfigSource,
) -> Option<PathBuf> {
    let primary = if source.relative {
        current_dir.join(&source.path)
    } else {
        root.join(&source.path)
    };
    let fallback = if source.relative {
        root.join(&source.path)
    } else {
        current_dir.join(&source.path)
    };

    if primary.exists() {
        Some(normalize_relative(&primary))
    } else if fallback.exists() {
        Some(normalize_relative(&fallback))
    } else {
        None
    }
}

pub(crate) fn kconfig_files(root: &Path) -> Vec<PathBuf> {
    walk_named(root, |name| name == "Kconfig" || name.starts_with("Kconfig."))
}

pub(crate) fn defined_symbols_in_file(path: &Path) -> Result<Vec<String>> {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return Ok(Vec::new());
    };
    if !name.starts_with("Kconfig") {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();
    let help_text = kconfig_help_text_mask(&lines);
    let mut symbols = Vec::new();
    for (idx, line) in lines.iter().copied().enumerate() {
        if help_text[idx] {
            continue;
        }
        let Some(symbol) = parse_config_symbol(line) else {
            continue;
        };
        if !symbols.contains(&symbol) {
            symbols.push(symbol);
        }
    }
    symbols.sort();
    symbols.dedup();
    Ok(symbols)
}

pub(crate) fn defined_symbols_in_tree(root: &Path) -> Result<HashSet<String>> {
    let mut symbols = HashSet::new();
    for path in kconfig_files(root) {
        for symbol in defined_symbols_in_file(&path)? {
            symbols.insert(symbol);
        }
    }
    Ok(symbols)
}


fn parse_config_symbol(line: &str) -> Option<String> {
    match parse_kconfig_directive(line)? {
        KconfigDirective::Entry { symbol, .. } => Some(symbol),
        _ => None,
    }
}


fn line_indentation_prefix(line: &str) -> &str {
    &line[..line.len() - line.trim_start().len()]
}

fn kconfig_help_text_mask(lines: &[&str]) -> Vec<bool> {
    let mut mask = vec![false; lines.len()];
    let mut in_help = false;
    let mut help_indent = 0usize;

    for (idx, line) in lines.iter().enumerate() {
        if in_help {
            if line.trim().is_empty() || indentation(line) > help_indent {
                mask[idx] = true;
                continue;
            }
            in_help = false;
        }

        if is_kconfig_help_directive(line.trim_start()) {
            in_help = true;
            help_indent = indentation(line);
        }
    }

    mask
}

fn is_kconfig_help_directive(trimmed: &str) -> bool {
    trimmed == "help" || trimmed.starts_with("help ") || trimmed == "---help---"
}



fn join_lines(lines: &[&str]) -> String {
    let mut out = String::new();
    for line in lines {
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn indentation(line: &str) -> usize {
    line.len() - line.trim_start_matches([' ', '\t']).len()
}

fn is_kconfig_boundary(trimmed: &str) -> bool {
    matches!(
        trimmed.split_whitespace().next(),
        Some(
            "config"
                | "menuconfig"
                | "menu"
                | "endmenu"
                | "if"
                | "endif"
                | "choice"
                | "endchoice"
                | "comment"
                | "source"
                | "rsource"
                | "osource"
                | "orsource"
                | "mainmenu"
        )
    )
}

fn relative_to_root_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn walk_named(root: &Path, matches: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let relative = relative_to_root_path(root, entry.path());
        if relative
            .components()
            .any(|component| component.as_os_str() == "templates")
        {
            continue;
        }
        if entry
            .file_name()
            .to_str()
            .is_some_and(&matches)
        {
            out.push(entry.into_path());
        }
    }
    out.sort();
    out
}

fn normalize_relative(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(seg) => out.push(seg),
            Component::RootDir | Component::Prefix(_) => {
                out = PathBuf::from(component.as_os_str());
            }
        }
    }

    out
}

#[cfg(test)]
mod expression_tests;

#[cfg(test)]
mod tests;
