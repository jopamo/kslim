//! Semantic prune rewrites.
//!
//! This module owns Kconfig-symbol pruning semantics: ABI guard preservation,
//! selected-profile solver inputs, config/default/relation rewrites, empty-menu
//! cleanup, and semantic stage accounting. Stale reference pruning stays in the
//! parent module until that slice is split.

use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::edit_reason::EditRecord;
use crate::kconfig::{
    KconfigReportCounts, KconfigSolverReport, UnsupportedKconfigExpression,
};
use crate::removal_manifest::RemovalManifest;

use super::{normalize_and_sort_symbols, normalize_relative_path, rewrite_kconfig_sources};

#[derive(Debug, Clone, Default)]
pub(crate) struct KconfigPruneStageResult {
    pub removed_config_symbols: Vec<String>,
    pub configs_disabled: usize,
    pub defaults_overridden: usize,
    pub kconfig_report: KconfigReportCounts,
    pub kconfig_solver_report: KconfigSolverReport,
    pub unsupported_kconfig_expressions: Vec<UnsupportedKconfigExpression>,
    pub edits: Vec<EditRecord>,
}

pub(crate) fn rewrite_kconfig_stage(
    root: &Path,
    manifest: &RemovalManifest,
) -> Result<KconfigPruneStageResult> {
    let mut edits = Vec::new();
    let removed_config_symbols = effective_removed_config_symbols_for_abi_policy(root, manifest)?;
    let selected_profile_values = crate::kconfig::read_kconfig_selected_profile_values(root)?;
    let kconfig_solver_report = if removed_config_symbols.is_empty() {
        KconfigSolverReport::default()
    } else {
        crate::kconfig::kconfig_solver_report(
            root,
            &selected_profile_values,
            &removed_config_symbols,
        )?
    };

    let (configs_disabled, config_edits) = if !removed_config_symbols.is_empty() {
        prune_configs(root, &removed_config_symbols)?
    } else {
        (0, Vec::new())
    };
    edits.extend(config_edits);

    let (defaults_overridden, default_edits) = if !manifest.default_overrides().is_empty() {
        rewrite_kconfig_defaults(root, manifest.default_overrides())?
    } else {
        (0, Vec::new())
    };
    edits.extend(default_edits);

    let kconfig_relation_stats = if !removed_config_symbols.is_empty() {
        rewrite_kconfig_relations(root, &removed_config_symbols)?
    } else {
        crate::kconfig::KconfigRelationRewriteStats::default()
    };
    edits.extend(kconfig_relation_stats.edits.clone());
    let mut kconfig_report = kconfig_relation_stats.report;

    let (kconfig_refs_removed, kconfig_source_edits) = rewrite_kconfig_sources(root, manifest)?;
    edits.extend(kconfig_source_edits);
    kconfig_report.removed_sources = kconfig_refs_removed;

    let (empty_menus_removed, empty_menu_edits) = if !removed_config_symbols.is_empty() {
        rewrite_empty_kconfig_menus(root, &selected_profile_values, &removed_config_symbols)?
    } else {
        (0, Vec::new())
    };
    edits.extend(empty_menu_edits);
    kconfig_report.removed_empty_menus = empty_menus_removed;

    Ok(KconfigPruneStageResult {
        removed_config_symbols,
        configs_disabled,
        defaults_overridden,
        kconfig_report,
        kconfig_solver_report,
        unsupported_kconfig_expressions: kconfig_relation_stats.unsupported,
        edits,
    })
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct AbiGuardSymbolUse {
    public_header: bool,
    uapi_header: bool,
}

impl AbiGuardSymbolUse {
    fn record_path(&mut self, path: &Path) {
        if crate::abi::is_uapi_header_path(path) {
            self.uapi_header = true;
        } else if path_has_header_extension(path)
            && crate::abi::is_public_header_path(path)
        {
            self.public_header = true;
        }
    }

    fn removal_allowed_by(self, policy: &crate::abi::AbiPolicyConfig) -> bool {
        (!self.public_header || policy.allow_public_header_removal)
            && (!self.uapi_header || policy.allow_uapi_header_removal)
    }
}

pub(in crate::prune) fn effective_removed_config_symbols_for_abi_policy(
    root: &Path,
    manifest: &RemovalManifest,
) -> Result<Vec<String>> {
    let removed_config_symbols = manifest.removed_config_symbols_vec();
    if removed_config_symbols.is_empty() {
        return Ok(removed_config_symbols);
    }

    let guard_uses = abi_guard_symbol_uses(root, &removed_config_symbols)?;
    let mut effective = Vec::new();
    for symbol in removed_config_symbols {
        match guard_uses.get(&symbol) {
            Some(guard_use) if !guard_use.removal_allowed_by(&manifest.abi_policy) => {
                log::warn!(
                    concat!(
                        "prune: preserving ABI guard config symbol '{}' because ABI policy ",
                        "does not allow removing its public/UAPI header surface"
                    ),
                    symbol
                );
            }
            _ => effective.push(symbol),
        }
    }
    normalize_and_sort_symbols(&mut effective);
    Ok(effective)
}

fn abi_guard_symbol_uses(
    root: &Path,
    removed_config_symbols: &[String],
) -> Result<BTreeMap<String, AbiGuardSymbolUse>> {
    let removed_config_symbols = removed_config_symbols
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut guard_uses = BTreeMap::new();

    for entry in walkdir::WalkDir::new(root) {
        let entry = entry.with_context(|| {
            format!(
                "failed to scan ABI guard config symbols under '{}'",
                root.display()
            )
        })?;
        if !entry.file_type().is_file() {
            continue;
        }

        let relative = entry
            .path()
            .strip_prefix(root)
            .map(normalize_relative_path)
            .unwrap_or_else(|_| normalize_relative_path(entry.path()));
        if !abi_guard_header_path(&relative) {
            continue;
        }

        let content = std::fs::read_to_string(entry.path()).with_context(|| {
            format!(
                "failed to scan ABI guard config symbols in '{}'",
                relative.display()
            )
        })?;
        for symbol in config_symbols_in_text(&content) {
            if removed_config_symbols.contains(&symbol) {
                guard_uses
                    .entry(symbol)
                    .or_insert_with(AbiGuardSymbolUse::default)
                    .record_path(&relative);
            }
        }
    }

    Ok(guard_uses)
}

fn abi_guard_header_path(path: &Path) -> bool {
    crate::abi::is_uapi_header_path(path)
        || (path_has_header_extension(path) && crate::abi::is_public_header_path(path))
}

fn path_has_header_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext == "h")
}

fn config_symbols_in_text(content: &str) -> BTreeSet<String> {
    let mut symbols = BTreeSet::new();
    let mut offset = 0usize;

    while let Some(pos) = content[offset..].find("CONFIG_") {
        let start = offset + pos + "CONFIG_".len();
        let mut end = start;
        for ch in content[start..].chars() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                end += ch.len_utf8();
            } else {
                break;
            }
        }
        if end > start {
            symbols.insert(content[start..end].to_string());
        }
        offset = if end > start { end } else { start };
    }

    symbols
}

fn prune_configs(root: &Path, configs: &[String]) -> Result<(usize, Vec<EditRecord>)> {
    crate::kconfig::prune_configs(root, configs)
}

fn rewrite_kconfig_defaults(
    root: &Path,
    overrides: &BTreeMap<String, String>,
) -> Result<(usize, Vec<EditRecord>)> {
    crate::kconfig::rewrite_kconfig_defaults(root, overrides)
}

fn rewrite_kconfig_relations(
    root: &Path,
    removed_configs: &[String],
) -> Result<crate::kconfig::KconfigRelationRewriteStats> {
    crate::kconfig::rewrite_kconfig_relations(root, removed_configs)
}

fn rewrite_empty_kconfig_menus(
    root: &Path,
    selected_profile_values: &BTreeMap<String, String>,
    removed_configs: &[String],
) -> Result<(usize, Vec<EditRecord>)> {
    let proofs =
        crate::kconfig::prove_empty_kconfig_menus(root, selected_profile_values, removed_configs)?;
    crate::kconfig::rewrite_empty_kconfig_menus(root, &proofs)
}
