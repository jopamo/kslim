//! Edit reason model.
//!
//! This module owns stable edit-reason taxonomy, diagnostic class names,
//! reason-to-proof-kind mapping, and reason payload labels.

use anyhow::Result;
use std::path::PathBuf;

use super::{
    payload_token, validate_non_empty_payload, validate_non_empty_payload_path,
    EditProofSourceKind,
};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticClass {
    MissingHeader,
    MissingKconfigSource,
    StaleKbuildDirectoryRef,
    StaleKbuildObjectRef,
    DeadConfigGatedCodePath,
    RemovedConfigSymbolUse,
    RemovedHeaderSymbolUse,
    UndefinedReference,
    Unknown,
}

impl DiagnosticClass {
    pub fn stable_name(&self) -> &'static str {
        match self {
            Self::MissingHeader => "MissingHeader",
            Self::MissingKconfigSource => "MissingKconfigSource",
            Self::StaleKbuildDirectoryRef => "StaleKbuildDirectoryRef",
            Self::StaleKbuildObjectRef => "StaleKbuildObjectRef",
            Self::DeadConfigGatedCodePath => "DeadConfigGatedCodePath",
            Self::RemovedConfigSymbolUse => "RemovedConfigSymbolUse",
            Self::RemovedHeaderSymbolUse => "RemovedHeaderSymbolUse",
            Self::UndefinedReference => "UndefinedReference",
            Self::Unknown => "Unknown",
        }
    }

    pub fn is_broad_speculative_fallout(&self) -> bool {
        matches!(self, Self::UndefinedReference)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EditReason {
    DeclaredPathPruned,
    RemovedKconfigSource,
    RemovedKconfigSymbolEdge,
    RemovedDeadKconfigSymbolDefinition { symbol: String },
    RemovedEmptyKconfigMenu { prompt: String },
    SimplifiedKconfigExpression,
    RemovedKbuildDirectoryRef,
    RemovedKbuildObjectRef,
    RemovedKbuildConfigGatedRef,
    RemovedKbuildIncludePath,
    FoldedDeadPreprocessorBranch,
    RemovedManifestBackedInclude,
    RemovedDeadBranchInclude { header: String, symbol: String },
    ReportedLiveMissingInclude,
    DiagnosticMissingHeaderFixup,
    DiagnosticStaleKbuildDirFixup,
    DiagnosticStaleKbuildObjectFixup,
    DiagnosticMissingKconfigSourceFixup,
    DiagnosticPreprocessorRefoldFixup,
    ManifestPath { path: PathBuf },
    ManifestConfig { symbol: String },
    RemovedKbuildRef { reference: String },
    RemovedHeader { header: String },
    SimplifiedTristateExpr { symbol: String },
    BuildDiagnostic { class: DiagnosticClass },
}

impl EditReason {
    pub fn proof_source_kind(&self) -> EditProofSourceKind {
        match self {
            Self::DeclaredPathPruned
            | Self::SimplifiedKconfigExpression
            | Self::FoldedDeadPreprocessorBranch
            | Self::RemovedManifestBackedInclude
            | Self::RemovedDeadBranchInclude { .. }
            | Self::ManifestPath { .. }
            | Self::ManifestConfig { .. }
            | Self::RemovedKconfigSource
            | Self::RemovedHeader { .. }
            | Self::SimplifiedTristateExpr { .. } => EditProofSourceKind::RemovalManifestEntry,
            Self::ReportedLiveMissingInclude => EditProofSourceKind::TreeIndexEntry,
            Self::RemovedDeadKconfigSymbolDefinition { .. }
            | Self::RemovedEmptyKconfigMenu { .. } => {
                EditProofSourceKind::KconfigSolverProof
            }
            Self::RemovedKconfigSymbolEdge
            | Self::RemovedKbuildDirectoryRef
            | Self::RemovedKbuildObjectRef
            | Self::RemovedKbuildConfigGatedRef
            | Self::RemovedKbuildIncludePath
            | Self::RemovedKbuildRef { .. } => EditProofSourceKind::StaleReference,
            Self::DiagnosticMissingHeaderFixup
            | Self::DiagnosticStaleKbuildDirFixup
            | Self::DiagnosticStaleKbuildObjectFixup
            | Self::DiagnosticMissingKconfigSourceFixup
            | Self::DiagnosticPreprocessorRefoldFixup
            | Self::BuildDiagnostic { .. } => EditProofSourceKind::ClassifiedDiagnostic,
        }
    }

    pub fn json_key(&self) -> &'static str {
        match self {
            Self::DeclaredPathPruned => "declared_path_pruned",
            Self::RemovedKconfigSource => "removed_kconfig_source",
            Self::RemovedKconfigSymbolEdge => "removed_kconfig_symbol_edge",
            Self::RemovedDeadKconfigSymbolDefinition { .. } => {
                "removed_dead_kconfig_symbol_definition"
            }
            Self::RemovedEmptyKconfigMenu { .. } => "removed_empty_kconfig_menu",
            Self::SimplifiedKconfigExpression => "simplified_kconfig_expression",
            Self::RemovedKbuildDirectoryRef => "removed_kbuild_directory_ref",
            Self::RemovedKbuildObjectRef => "removed_kbuild_object_ref",
            Self::RemovedKbuildConfigGatedRef => "removed_kbuild_config_gated_ref",
            Self::RemovedKbuildIncludePath => "removed_kbuild_include_path",
            Self::FoldedDeadPreprocessorBranch => "folded_dead_preprocessor_branch",
            Self::RemovedManifestBackedInclude => "removed_manifest_backed_include",
            Self::RemovedDeadBranchInclude { .. } => "removed_dead_branch_include",
            Self::ReportedLiveMissingInclude => "reported_live_missing_include",
            Self::DiagnosticMissingHeaderFixup => "diagnostic_missing_header_fixup",
            Self::DiagnosticStaleKbuildDirFixup => "diagnostic_stale_kbuild_dir_fixup",
            Self::DiagnosticStaleKbuildObjectFixup => "diagnostic_stale_kbuild_object_fixup",
            Self::DiagnosticMissingKconfigSourceFixup => "diagnostic_missing_kconfig_source_fixup",
            Self::DiagnosticPreprocessorRefoldFixup => "diagnostic_preprocessor_refold_fixup",
            Self::ManifestPath { .. } => "manifest_path",
            Self::ManifestConfig { .. } => "manifest_config",
            Self::RemovedKbuildRef { .. } => "removed_kbuild_ref",
            Self::RemovedHeader { .. } => "removed_header",
            Self::SimplifiedTristateExpr { .. } => "simplified_tristate_expr",
            Self::BuildDiagnostic { .. } => "build_diagnostic",
        }
    }

    pub fn payload_label(&self) -> String {
        match self {
            Self::DeclaredPathPruned => String::from("reason=declared_path_pruned"),
            Self::RemovedKconfigSource => String::from("reason=removed_kconfig_source"),
            Self::RemovedKconfigSymbolEdge => String::from("reason=removed_kconfig_symbol_edge"),
            Self::RemovedDeadKconfigSymbolDefinition { symbol } => format!("symbol={symbol}"),
            Self::RemovedEmptyKconfigMenu { prompt } => {
                format!("prompt={}", payload_token(prompt))
            }
            Self::SimplifiedKconfigExpression => {
                String::from("reason=simplified_kconfig_expression")
            }
            Self::RemovedKbuildDirectoryRef => String::from("reason=removed_kbuild_directory_ref"),
            Self::RemovedKbuildObjectRef => String::from("reason=removed_kbuild_object_ref"),
            Self::RemovedKbuildConfigGatedRef => {
                String::from("reason=removed_kbuild_config_gated_ref")
            }
            Self::RemovedKbuildIncludePath => String::from("reason=removed_kbuild_include_path"),
            Self::FoldedDeadPreprocessorBranch => {
                String::from("reason=folded_dead_preprocessor_branch")
            }
            Self::RemovedManifestBackedInclude => {
                String::from("reason=removed_manifest_backed_include")
            }
            Self::RemovedDeadBranchInclude { header, symbol } => {
                format!("header={header} symbol={symbol}")
            }
            Self::ReportedLiveMissingInclude => {
                String::from("reason=reported_live_missing_include")
            }
            Self::DiagnosticMissingHeaderFixup => {
                String::from("reason=diagnostic_missing_header_fixup")
            }
            Self::DiagnosticStaleKbuildDirFixup => {
                String::from("reason=diagnostic_stale_kbuild_dir_fixup")
            }
            Self::DiagnosticStaleKbuildObjectFixup => {
                String::from("reason=diagnostic_stale_kbuild_object_fixup")
            }
            Self::DiagnosticMissingKconfigSourceFixup => {
                String::from("reason=diagnostic_missing_kconfig_source_fixup")
            }
            Self::DiagnosticPreprocessorRefoldFixup => {
                String::from("reason=diagnostic_preprocessor_refold_fixup")
            }
            Self::ManifestPath { path } => format!("path={}", path.display()),
            Self::ManifestConfig { symbol } => format!("symbol={symbol}"),
            Self::RemovedKbuildRef { reference } => format!("reference={reference}"),
            Self::RemovedHeader { header } => format!("header={header}"),
            Self::SimplifiedTristateExpr { symbol } => format!("symbol={symbol}"),
            Self::BuildDiagnostic { class } => format!("class={}", class.stable_name()),
        }
    }

    pub(in crate::edit_reason) fn validate_reasoned_payload(&self) -> Result<()> {
        match self {
            Self::DeclaredPathPruned
            | Self::RemovedKconfigSource
            | Self::RemovedKconfigSymbolEdge
            | Self::SimplifiedKconfigExpression
            | Self::RemovedKbuildDirectoryRef
            | Self::RemovedKbuildObjectRef
            | Self::RemovedKbuildConfigGatedRef
            | Self::RemovedKbuildIncludePath
            | Self::FoldedDeadPreprocessorBranch
            | Self::RemovedManifestBackedInclude
            | Self::ReportedLiveMissingInclude
            | Self::DiagnosticMissingHeaderFixup
            | Self::DiagnosticStaleKbuildDirFixup
            | Self::DiagnosticStaleKbuildObjectFixup
            | Self::DiagnosticMissingKconfigSourceFixup
            | Self::DiagnosticPreprocessorRefoldFixup => Ok(()),
            Self::RemovedDeadKconfigSymbolDefinition { symbol } => {
                validate_non_empty_payload("removed dead Kconfig symbol definition", symbol)
            }
            Self::RemovedEmptyKconfigMenu { prompt } => {
                validate_non_empty_payload("removed empty Kconfig menu", prompt)
            }
            Self::ManifestPath { path } => validate_non_empty_payload_path("manifest path", path),
            Self::ManifestConfig { symbol } => {
                validate_non_empty_payload("manifest symbol", symbol)
            }
            Self::RemovedKbuildRef { reference } => {
                validate_non_empty_payload("removed kbuild reference", reference)
            }
            Self::RemovedHeader { header } => validate_non_empty_payload("removed header", header),
            Self::RemovedDeadBranchInclude { header, symbol } => {
                validate_non_empty_payload("removed dead-branch include header", header)?;
                validate_non_empty_payload("removed dead-branch include symbol", symbol)
            }
            Self::SimplifiedTristateExpr { symbol } => {
                validate_non_empty_payload("simplified tristate symbol", symbol)
            }
            Self::BuildDiagnostic { class } if *class == DiagnosticClass::Unknown => {
                anyhow::bail!("classified diagnostic edit reason is Unknown")
            }
            Self::BuildDiagnostic { .. } => Ok(()),
        }
    }

    pub(in crate::edit_reason) fn is_broad_speculative_fallout(&self) -> bool {
        match self {
            Self::BuildDiagnostic { class } => class.is_broad_speculative_fallout(),
            _ => false,
        }
    }
}

