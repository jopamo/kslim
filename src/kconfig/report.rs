use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use super::*;
use crate::edit_reason::EditRecord;
use super::expression::TristateLiteral;
use super::solver::{
    detect_kconfig_empty_menus, detect_kconfig_impossible_choices,
    detect_kconfig_orphaned_symbol_definitions,
    detect_kconfig_removed_symbols_forced_by_select,
    detect_kconfig_removed_symbols_weakly_enabled_by_imply,
    detect_kconfig_symbols_reenabled_by_defaults, parse_selected_profile_tristate_values,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct UnsupportedKconfigExpression {
    pub file: PathBuf,
    pub line: usize,
    pub directive: String,
    pub expression: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct KconfigRelationRewriteStats {
    pub rewrites: usize,
    pub edits: Vec<EditRecord>,
    pub unsupported: Vec<UnsupportedKconfigExpression>,
    pub report: KconfigReportCounts,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct KconfigReportCounts {
    pub dropped_selects: usize,
    pub dropped_implies: usize,
    pub simplified_depends: usize,
    pub simplified_visible_if: usize,
    pub simplified_defaults: usize,
    pub removed_sources: usize,
    pub removed_empty_menus: usize,
    pub skipped_expressions: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct KconfigSolverReport {
    pub(crate) files_analyzed: usize,
    pub(crate) removed_symbols: Vec<String>,
    pub(crate) default_reenabled_symbols: Vec<KconfigSolverDefaultReenabledSymbol>,
    pub(crate) forced_selects: Vec<KconfigSolverReverseDependency>,
    pub(crate) weak_implies: Vec<KconfigSolverReverseDependency>,
    pub(crate) impossible_choices: Vec<KconfigSolverImpossibleChoice>,
    pub(crate) empty_menus: Vec<KconfigSolverEmptyMenu>,
    pub(crate) orphaned_symbol_definitions: Vec<KconfigSolverOrphanedSymbolDefinition>,
    pub(crate) dead_symbol_definition_proofs: Vec<KconfigSolverDeadSymbolDefinitionProof>,
    pub(crate) skipped_files: Vec<KconfigSolverSkippedFile>,
}

impl KconfigSolverReport {
    fn normalize(&mut self) {
        self.removed_symbols.sort();
        self.removed_symbols.dedup();
        self.default_reenabled_symbols.sort();
        self.default_reenabled_symbols.dedup();
        self.forced_selects.sort();
        self.forced_selects.dedup();
        self.weak_implies.sort();
        self.weak_implies.dedup();
        for choice in &mut self.impossible_choices {
            choice.member_symbols.sort();
            choice.member_symbols.dedup();
        }
        self.impossible_choices.sort();
        self.impossible_choices.dedup();
        self.empty_menus.sort();
        self.empty_menus.dedup();
        self.orphaned_symbol_definitions.sort();
        self.orphaned_symbol_definitions.dedup();
        self.dead_symbol_definition_proofs.sort();
        self.dead_symbol_definition_proofs.dedup();
        self.skipped_files.sort();
        self.skipped_files.dedup();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct KconfigSolverDefaultReenabledSymbol {
    pub(crate) symbol: String,
    pub(crate) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct KconfigSolverReverseDependency {
    pub(crate) source_symbol: String,
    pub(crate) target_symbol: String,
    pub(crate) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct KconfigSolverImpossibleChoice {
    pub(crate) choice_symbol: Option<String>,
    pub(crate) line: usize,
    pub(crate) visibility: String,
    pub(crate) member_symbols: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct KconfigSolverEmptyMenu {
    pub(crate) prompt: String,
    pub(crate) line: usize,
    pub(crate) visibility: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct KconfigSolverOrphanedSymbolDefinition {
    pub(crate) symbol: String,
    pub(crate) definition_kind: String,
    pub(crate) line: usize,
    pub(crate) visibility: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct KconfigSolverDeadSymbolDefinitionProof {
    pub(crate) file: PathBuf,
    pub(crate) symbol: String,
    pub(crate) definition_kind: String,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct KconfigSolverSkippedFile {
    pub(crate) file: PathBuf,
    pub(crate) analysis: String,
    pub(crate) reason: String,
}

pub(crate) fn read_kconfig_selected_profile_values(
    root: &Path,
) -> Result<BTreeMap<String, String>> {
    let config_path = root.join(".config");
    let content = match std::fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(err) => {
            return Err(err).with_context(|| {
                format!(
                    "failed to read selected Kconfig profile values from '{}'",
                    config_path.display()
                )
            })
        }
    };

    let mut values = BTreeMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(symbol) = trimmed
            .strip_prefix("# CONFIG_")
            .and_then(|rest| rest.strip_suffix(" is not set"))
        {
            values.insert(symbol.to_string(), String::from("n"));
            continue;
        }

        let Some(rest) = trimmed.strip_prefix("CONFIG_") else {
            continue;
        };
        let Some((symbol, value)) = rest.split_once('=') else {
            continue;
        };
        let value = value.trim();
        if matches!(value, "y" | "m" | "n") {
            values.insert(symbol.to_string(), value.to_string());
        }
    }

    Ok(values)
}

#[allow(dead_code)]
pub(crate) fn kconfig_solver_report(
    root: &Path,
    selected_profile_values: &BTreeMap<String, String>,
    removed_configs: &[String],
) -> Result<KconfigSolverReport> {
    kconfig_solver_report_for_arch_policy(
        root,
        selected_profile_values,
        removed_configs,
        &crate::config::ArchPolicyConfig::default(),
    )
}

pub(crate) fn kconfig_solver_report_for_arch_policy(
    root: &Path,
    selected_profile_values: &BTreeMap<String, String>,
    removed_configs: &[String],
    arch_policy: &crate::config::ArchPolicyConfig,
) -> Result<KconfigSolverReport> {
    let selected_profile_values = parse_selected_profile_tristate_values(selected_profile_values)?;
    let removed_symbols: HashSet<&str> = removed_configs.iter().map(String::as_str).collect();
    let mut report = KconfigSolverReport {
        removed_symbols: removed_configs.to_vec(),
        ..KconfigSolverReport::default()
    };

    for path in kconfig_files(root) {
        let relative = relative_to_root_path(root, &path);
        let content = std::fs::read_to_string(&path).with_context(|| {
            format!(
                "failed to read Kconfig solver input '{}'",
                relative.display()
            )
        })?;
        let document = match parse_kconfig_document(&content) {
            Ok(document) => document,
            Err(err) => {
                push_skipped_solver_analysis(
                    &mut report,
                    relative,
                    "parse_kconfig_document",
                    err.to_string(),
                );
                continue;
            }
        };
        report.files_analyzed += 1;
        collect_document_solver_report(
            &mut report,
            &relative,
            &document,
            &selected_profile_values,
            &removed_symbols,
        );
    }

    let selected_profile_values_for_proofs =
        selected_profile_values_as_strings(&selected_profile_values);
    match prove_dead_kconfig_symbol_definitions_for_arch_policy(
        root,
        &selected_profile_values_for_proofs,
        removed_configs,
        arch_policy,
    ) {
        Ok(proofs) => {
            report.dead_symbol_definition_proofs.extend(
                proofs
                    .into_iter()
                    .map(|proof| KconfigSolverDeadSymbolDefinitionProof {
                        file: proof.file,
                        symbol: proof.symbol,
                        definition_kind: kconfig_symbol_definition_kind_keyword(
                            proof.definition_kind,
                        )
                        .to_string(),
                        start_line: proof.start_line,
                        end_line: proof.end_line,
                    }),
            );
        }
        Err(err) => push_skipped_solver_analysis(
            &mut report,
            PathBuf::from("Kconfig"),
            "dead_symbol_definition_proofs",
            err.to_string(),
        ),
    }

    report.normalize();
    Ok(report)
}

fn collect_document_solver_report(
    report: &mut KconfigSolverReport,
    file: &Path,
    document: &KconfigDocument,
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
    removed_symbols: &HashSet<&str>,
) {
    match detect_kconfig_symbols_reenabled_by_defaults(
        document,
        selected_profile_values,
        removed_symbols,
    ) {
        Some(items) => report.default_reenabled_symbols.extend(items.into_iter().map(|item| {
            KconfigSolverDefaultReenabledSymbol {
                symbol: item.symbol().to_string(),
                value: tristate_report_value(item.value()).to_string(),
            }
        })),
        None => push_skipped_solver_analysis(
            report,
            file.to_path_buf(),
            "default_reenabled_symbols",
            "unsupported or unknown Kconfig expression prevented complete default analysis",
        ),
    }

    match detect_kconfig_removed_symbols_forced_by_select(
        document,
        selected_profile_values,
        removed_symbols,
    ) {
        Some(items) => report.forced_selects.extend(items.into_iter().map(|item| {
            KconfigSolverReverseDependency {
                source_symbol: item.source_symbol().to_string(),
                target_symbol: item.target_symbol().to_string(),
                value: tristate_report_value(item.value()).to_string(),
            }
        })),
        None => push_skipped_solver_analysis(
            report,
            file.to_path_buf(),
            "forced_selects",
            "unsupported or unknown Kconfig expression prevented complete select analysis",
        ),
    }

    match detect_kconfig_removed_symbols_weakly_enabled_by_imply(
        document,
        selected_profile_values,
        removed_symbols,
    ) {
        Some(items) => report.weak_implies.extend(items.into_iter().map(|item| {
            KconfigSolverReverseDependency {
                source_symbol: item.source_symbol().to_string(),
                target_symbol: item.target_symbol().to_string(),
                value: tristate_report_value(item.value()).to_string(),
            }
        })),
        None => push_skipped_solver_analysis(
            report,
            file.to_path_buf(),
            "weak_implies",
            "unsupported or unknown Kconfig expression prevented complete imply analysis",
        ),
    }

    match detect_kconfig_impossible_choices(document, selected_profile_values, removed_symbols) {
        Some(items) => report.impossible_choices.extend(items.into_iter().map(|item| {
            KconfigSolverImpossibleChoice {
                choice_symbol: item.choice_symbol().map(ToString::to_string),
                line: item.line(),
                visibility: tristate_report_value(item.visibility()).to_string(),
                member_symbols: item.member_symbols().to_vec(),
            }
        })),
        None => push_skipped_solver_analysis(
            report,
            file.to_path_buf(),
            "impossible_choices",
            "unsupported or unknown Kconfig expression prevented complete choice analysis",
        ),
    }

    match detect_kconfig_empty_menus(document, selected_profile_values, removed_symbols) {
        Some(items) => report.empty_menus.extend(items.into_iter().map(|item| {
            KconfigSolverEmptyMenu {
                prompt: item.prompt().to_string(),
                line: item.line(),
                visibility: tristate_report_value(item.visibility()).to_string(),
            }
        })),
        None => push_skipped_solver_analysis(
            report,
            file.to_path_buf(),
            "empty_menus",
            "unsupported or unknown Kconfig expression prevented complete menu analysis",
        ),
    }

    match detect_kconfig_orphaned_symbol_definitions(
        document,
        selected_profile_values,
        removed_symbols,
    ) {
        Some(items) => report.orphaned_symbol_definitions.extend(items.into_iter().map(|item| {
            KconfigSolverOrphanedSymbolDefinition {
                symbol: item.symbol().to_string(),
                definition_kind: kconfig_symbol_definition_kind_keyword(item.definition_kind())
                    .to_string(),
                line: item.line(),
                visibility: tristate_report_value(item.visibility()).to_string(),
            }
        })),
        None => push_skipped_solver_analysis(
            report,
            file.to_path_buf(),
            "orphaned_symbol_definitions",
            "unsupported or unknown Kconfig expression prevented complete orphan analysis",
        ),
    }
}

fn selected_profile_values_as_strings(
    selected_profile_values: &BTreeMap<String, TristateLiteral>,
) -> BTreeMap<String, String> {
    selected_profile_values
        .iter()
        .map(|(symbol, value)| (symbol.clone(), tristate_report_value(*value).to_string()))
        .collect()
}

fn push_skipped_solver_analysis(
    report: &mut KconfigSolverReport,
    file: PathBuf,
    analysis: impl Into<String>,
    reason: impl Into<String>,
) {
    report.skipped_files.push(KconfigSolverSkippedFile {
        file,
        analysis: analysis.into(),
        reason: reason.into(),
    });
}

fn tristate_report_value(value: TristateLiteral) -> &'static str {
    match value {
        TristateLiteral::Y => "y",
        TristateLiteral::M => "m",
        TristateLiteral::N => "n",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_selected_profile_tristate_values_from_dot_config() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(".config"),
            concat!(
                "CONFIG_LIVE=y\n",
                "CONFIG_MOD=m\n",
                "# CONFIG_OFF is not set\n",
                "CONFIG_STRING=\"value\"\n",
            ),
        )
        .unwrap();

        let values = read_kconfig_selected_profile_values(tmp.path()).unwrap();

        assert_eq!(values.get("LIVE").map(String::as_str), Some("y"));
        assert_eq!(values.get("MOD").map(String::as_str), Some("m"));
        assert_eq!(values.get("OFF").map(String::as_str), Some("n"));
        assert!(!values.contains_key("STRING"));
    }

    #[test]
    fn emits_kconfig_solver_report_for_removed_symbol_fallout() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("Kconfig"),
            concat!(
                "config REMOVED_DEFAULT\n",
                "\ttristate \"Removed default\"\n",
                "\tdefault y if DEFAULT_GATE\n",
                "config SELECT_SOURCE\n",
                "\tbool \"Select source\"\n",
                "\tselect REMOVED_SELECTED\n",
                "config IMPLY_SOURCE\n",
                "\ttristate \"Imply source\"\n",
                "\timply REMOVED_IMPLIED\n",
                "choice BROKEN_CHOICE\n",
                "\tbool \"Broken\"\n",
                "config REMOVED_MEMBER\n",
                "\tbool \"Removed member\"\n",
                "endchoice\n",
                "menu \"Empty menu\"\n",
                "\tdepends on LIVE\n",
                "config REMOVED_MENU_MEMBER\n",
                "\tbool \"Removed menu member\"\n",
                "endmenu\n",
                "config ORPHANED_HIDDEN\n",
                "\tbool \"Orphaned hidden\" if OFF\n",
            ),
        )
        .unwrap();
        let selected_profile_values = BTreeMap::from([
            (String::from("BROKEN_CHOICE"), String::from("n")),
            (String::from("DEFAULT_GATE"), String::from("y")),
            (String::from("IMPLY_SOURCE"), String::from("m")),
            (String::from("LIVE"), String::from("y")),
            (String::from("OFF"), String::from("n")),
            (String::from("ORPHANED_HIDDEN"), String::from("n")),
            (String::from("SELECT_SOURCE"), String::from("y")),
        ]);
        let removed = vec![
            String::from("REMOVED_DEFAULT"),
            String::from("REMOVED_IMPLIED"),
            String::from("REMOVED_MEMBER"),
            String::from("REMOVED_MENU_MEMBER"),
            String::from("REMOVED_SELECTED"),
        ];

        let report = kconfig_solver_report(tmp.path(), &selected_profile_values, &removed).unwrap();

        assert_eq!(report.files_analyzed, 1);
        assert_eq!(report.default_reenabled_symbols.len(), 1);
        assert_eq!(report.default_reenabled_symbols[0].symbol, "REMOVED_DEFAULT");
        assert_eq!(report.default_reenabled_symbols[0].value, "y");
        assert_eq!(report.forced_selects.len(), 1);
        assert_eq!(report.forced_selects[0].target_symbol, "REMOVED_SELECTED");
        assert_eq!(report.weak_implies.len(), 1);
        assert_eq!(report.weak_implies[0].target_symbol, "REMOVED_IMPLIED");
        assert_eq!(report.weak_implies[0].value, "m");
        assert_eq!(report.impossible_choices.len(), 1);
        assert_eq!(report.impossible_choices[0].choice_symbol.as_deref(), Some("BROKEN_CHOICE"));
        assert_eq!(report.empty_menus.len(), 1);
        assert_eq!(report.empty_menus[0].prompt, "Empty menu");
        assert_eq!(report.orphaned_symbol_definitions.len(), 1);
        assert_eq!(report.orphaned_symbol_definitions[0].symbol, "ORPHANED_HIDDEN");
        assert!(report.skipped_files.is_empty());
    }
}
