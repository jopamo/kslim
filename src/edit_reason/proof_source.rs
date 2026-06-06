//! Edit proof source model.
//!
//! This module owns proof source taxonomy, proof keys, payload labels,
//! matching between proof sources and edit reasons, and proof-source payload
//! validation.

use anyhow::Result;
use std::path::PathBuf;

use super::{
    payload_token, validate_non_empty_payload, validate_non_empty_payload_path,
    validate_relative_edit_path, DiagnosticClass, EditReason,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EditProofSourceKind {
    RemovalManifestEntry,
    TreeIndexEntry,
    KconfigSolverProof,
    StaleReference,
    ClassifiedDiagnostic,
}

impl EditProofSourceKind {
    pub fn json_key(self) -> &'static str {
        match self {
            Self::RemovalManifestEntry => "removal_manifest_entry",
            Self::TreeIndexEntry => "tree_index_entry",
            Self::KconfigSolverProof => "kconfig_solver_proof",
            Self::StaleReference => "stale_reference",
            Self::ClassifiedDiagnostic => "classified_build_diagnostic",
        }
    }

    pub fn report_label(self) -> &'static str {
        match self {
            Self::RemovalManifestEntry => "Removal manifest entry",
            Self::TreeIndexEntry => "Tree index entry",
            Self::KconfigSolverProof => "Kconfig solver proof",
            Self::StaleReference => "Kconfig/kbuild/index-derived stale reference",
            Self::ClassifiedDiagnostic => "Classified compiler/build diagnostic",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RemovalKey {
    Path(PathBuf),
    Config(String),
    KconfigSource(PathBuf),
    Header { header: String, path: PathBuf },
}

impl RemovalKey {
    fn payload_label(&self) -> String {
        match self {
            Self::Path(path) => format!("path={}", path.display()),
            Self::Config(symbol) => format!("symbol={symbol}"),
            Self::KconfigSource(path) => {
                format!("kconfig_source={}", path.display())
            }
            Self::Header { header, path } => {
                format!("header={header} path={}", path.display())
            }
        }
    }

    fn validate_reasoned_payload(&self) -> Result<()> {
        match self {
            Self::Path(path) => {
                validate_non_empty_payload_path("removal manifest path proof", path)
            }
            Self::Config(symbol) => {
                validate_non_empty_payload("removal manifest config proof", symbol)
            }
            Self::KconfigSource(path) => {
                validate_non_empty_payload_path("removal manifest Kconfig source proof", path)
            }
            Self::Header { header, path } => {
                validate_non_empty_payload("removal manifest header proof", header)?;
                validate_non_empty_payload_path("removal manifest header path proof", path)
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndexKind {
    File,
    Header,
    IncludeSite,
    KconfigFile,
    KconfigDefinition,
    KconfigReference,
    KconfigSource,
    KbuildFile,
    KbuildObjectProvider,
    KbuildObjectReference,
    KbuildDirectoryReference,
    CppGate,
}

impl IndexKind {
    pub fn json_key(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Header => "header",
            Self::IncludeSite => "include_site",
            Self::KconfigFile => "kconfig_file",
            Self::KconfigDefinition => "kconfig_definition",
            Self::KconfigReference => "kconfig_reference",
            Self::KconfigSource => "kconfig_source",
            Self::KbuildFile => "kbuild_file",
            Self::KbuildObjectProvider => "kbuild_object_provider",
            Self::KbuildObjectReference => "kbuild_object_reference",
            Self::KbuildDirectoryReference => "kbuild_directory_reference",
            Self::CppGate => "cpp_gate",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReferenceKind {
    KconfigSource,
    KconfigSymbolEdge,
    KbuildDirectoryRef,
    KbuildObjectRef,
    KbuildConfigGatedRef,
    KbuildIncludePath,
}

impl ReferenceKind {
    pub fn json_key(self) -> &'static str {
        match self {
            Self::KconfigSource => "kconfig_source",
            Self::KconfigSymbolEdge => "kconfig_symbol_edge",
            Self::KbuildDirectoryRef => "kbuild_directory_ref",
            Self::KbuildObjectRef => "kbuild_object_ref",
            Self::KbuildConfigGatedRef => "kbuild_config_gated_ref",
            Self::KbuildIncludePath => "kbuild_include_path",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KconfigSolverKey {
    UnreachableSymbolDefinition {
        symbol: String,
        file: PathBuf,
        line: usize,
    },
    EmptyMenu {
        prompt: String,
        file: PathBuf,
        line: usize,
    },
}

impl KconfigSolverKey {
    fn payload_label(&self) -> String {
        match self {
            Self::UnreachableSymbolDefinition { symbol, file, line } => {
                format!(
                    "solver=unreachable_symbol_definition symbol={symbol} file={} line={line}",
                    file.display()
                )
            }
            Self::EmptyMenu { prompt, file, line } => {
                format!(
                    "solver=empty_menu prompt={} file={} line={line}",
                    payload_token(prompt),
                    file.display()
                )
            }
        }
    }

    fn validate_reasoned_payload(&self) -> Result<()> {
        match self {
            Self::UnreachableSymbolDefinition { symbol, file, line } => {
                validate_non_empty_payload("Kconfig solver symbol proof", symbol)?;
                validate_relative_edit_path(file)?;
                if *line == 0 {
                    anyhow::bail!("Kconfig solver proof line must be non-zero");
                }
                Ok(())
            }
            Self::EmptyMenu { prompt, file, line } => {
                validate_non_empty_payload("Kconfig solver empty menu prompt", prompt)?;
                validate_relative_edit_path(file)?;
                if *line == 0 {
                    anyhow::bail!("Kconfig solver proof line must be non-zero");
                }
                Ok(())
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiagnosticId {
    pub class: DiagnosticClass,
    pub key: String,
}

impl DiagnosticId {
    pub fn from_class(class: DiagnosticClass) -> Self {
        let key = class.stable_name().to_string();
        Self { key, class }
    }

    pub(in crate::edit_reason) fn payload_label(&self) -> String {
        format!("class={} key={}", self.class.stable_name(), self.key)
    }

    fn validate_reasoned_payload(&self) -> Result<()> {
        if self.class == DiagnosticClass::Unknown {
            anyhow::bail!("classified diagnostic proof source is Unknown");
        }
        validate_non_empty_payload("classified diagnostic id", &self.key)
    }
}

impl From<DiagnosticClass> for DiagnosticId {
    fn from(class: DiagnosticClass) -> Self {
        Self::from_class(class)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EditProofSource {
    RemovalManifest {
        key: RemovalKey,
    },
    TreeIndex {
        index_kind: IndexKind,
        key: String,
    },
    KconfigSolver {
        key: KconfigSolverKey,
    },
    StaleReference {
        reference_kind: ReferenceKind,
        key: String,
    },
    ClassifiedDiagnostic {
        diagnostic_id: DiagnosticId,
    },
}

impl EditProofSource {
    pub fn removal_manifest_path(path: PathBuf) -> Self {
        Self::RemovalManifest {
            key: RemovalKey::Path(path),
        }
    }

    pub fn removal_manifest_config(symbol: String) -> Self {
        Self::RemovalManifest {
            key: RemovalKey::Config(symbol),
        }
    }

    pub fn removal_manifest_kconfig_source(source: PathBuf) -> Self {
        Self::RemovalManifest {
            key: RemovalKey::KconfigSource(source),
        }
    }

    pub fn removal_manifest_header(header: String, path: PathBuf) -> Self {
        Self::RemovalManifest {
            key: RemovalKey::Header { header, path },
        }
    }

    #[allow(dead_code)]
    pub fn kconfig_solver_unreachable_symbol_definition(
        symbol: String,
        file: PathBuf,
        line: usize,
    ) -> Self {
        Self::KconfigSolver {
            key: KconfigSolverKey::UnreachableSymbolDefinition { symbol, file, line },
        }
    }

    #[allow(dead_code)]
    pub fn kconfig_solver_empty_menu(prompt: String, file: PathBuf, line: usize) -> Self {
        Self::KconfigSolver {
            key: KconfigSolverKey::EmptyMenu { prompt, file, line },
        }
    }

    pub fn stale_kbuild_reference(reference: String) -> Self {
        Self::StaleReference {
            reference_kind: ReferenceKind::KbuildObjectRef,
            key: reference,
        }
    }

    pub fn kind(&self) -> EditProofSourceKind {
        match self {
            Self::RemovalManifest { .. } => EditProofSourceKind::RemovalManifestEntry,
            Self::TreeIndex { .. } => EditProofSourceKind::TreeIndexEntry,
            Self::KconfigSolver { .. } => EditProofSourceKind::KconfigSolverProof,
            Self::StaleReference { .. } => EditProofSourceKind::StaleReference,
            Self::ClassifiedDiagnostic { .. } => EditProofSourceKind::ClassifiedDiagnostic,
        }
    }

    pub fn payload_label(&self) -> String {
        match self {
            Self::RemovalManifest { key } => key.payload_label(),
            Self::TreeIndex { index_kind, key } => {
                format!("index_kind={} key={key}", index_kind.json_key())
            }
            Self::KconfigSolver { key } => key.payload_label(),
            Self::StaleReference {
                reference_kind,
                key,
            } => format!("reference_kind={} key={key}", reference_kind.json_key()),
            Self::ClassifiedDiagnostic { diagnostic_id } => diagnostic_id.payload_label(),
        }
    }

    pub fn matches_reason(&self, reason: &EditReason) -> bool {
        match (self, reason) {
            (
                Self::RemovalManifest {
                    key: RemovalKey::Path(_),
                },
                EditReason::DeclaredPathPruned,
            ) => true,
            (
                Self::RemovalManifest {
                    key: RemovalKey::Path(proof_path),
                },
                EditReason::ManifestPath { path: reason_path },
            ) => proof_path == reason_path,
            (
                Self::RemovalManifest {
                    key: RemovalKey::Config(_),
                },
                EditReason::RemovedKconfigSymbolEdge
                | EditReason::SimplifiedKconfigExpression
                | EditReason::FoldedDeadPreprocessorBranch,
            ) => true,
            (
                Self::RemovalManifest {
                    key: RemovalKey::Config(proof_symbol),
                },
                EditReason::ManifestConfig {
                    symbol: reason_symbol,
                }
                | EditReason::SimplifiedTristateExpr {
                    symbol: reason_symbol,
                },
            ) => proof_symbol == reason_symbol,
            (
                Self::RemovalManifest {
                    key: RemovalKey::Config(proof_symbol),
                },
                EditReason::RemovedDeadBranchInclude {
                    symbol: reason_symbol,
                    ..
                },
            ) => proof_symbol == reason_symbol,
            (
                Self::RemovalManifest {
                    key: RemovalKey::Header { .. },
                },
                EditReason::RemovedManifestBackedInclude,
            ) => true,
            (
                Self::RemovalManifest {
                    key: RemovalKey::KconfigSource(_),
                },
                EditReason::RemovedKconfigSource,
            ) => true,
            (
                Self::RemovalManifest {
                    key:
                        RemovalKey::Header {
                            header: proof_header,
                            ..
                        },
                },
                EditReason::RemovedHeader {
                    header: reason_header,
                },
            ) => proof_header == reason_header,
            (Self::TreeIndex { .. }, EditReason::ReportedLiveMissingInclude) => true,
            (
                Self::KconfigSolver {
                    key:
                        KconfigSolverKey::UnreachableSymbolDefinition {
                            symbol: proof_symbol,
                            ..
                        },
                },
                EditReason::RemovedDeadKconfigSymbolDefinition {
                    symbol: reason_symbol,
                },
            ) => proof_symbol == reason_symbol,
            (
                Self::KconfigSolver {
                    key:
                        KconfigSolverKey::EmptyMenu {
                            prompt: proof_prompt,
                            ..
                        },
                },
                EditReason::RemovedEmptyKconfigMenu {
                    prompt: reason_prompt,
                },
            ) => proof_prompt == reason_prompt,
            (
                Self::StaleReference {
                    reference_kind: ReferenceKind::KconfigSymbolEdge,
                    ..
                },
                EditReason::RemovedKconfigSymbolEdge,
            ) => true,
            (
                Self::StaleReference {
                    reference_kind: ReferenceKind::KbuildDirectoryRef,
                    ..
                },
                EditReason::RemovedKbuildDirectoryRef,
            ) => true,
            (
                Self::StaleReference {
                    reference_kind: ReferenceKind::KbuildObjectRef,
                    ..
                },
                EditReason::RemovedKbuildObjectRef,
            ) => true,
            (
                Self::StaleReference {
                    reference_kind: ReferenceKind::KbuildConfigGatedRef,
                    ..
                },
                EditReason::RemovedKbuildConfigGatedRef,
            ) => true,
            (
                Self::StaleReference {
                    reference_kind: ReferenceKind::KbuildIncludePath,
                    ..
                },
                EditReason::RemovedKbuildIncludePath,
            ) => true,
            (
                Self::StaleReference {
                    key: proof_reference,
                    ..
                },
                EditReason::RemovedKbuildRef {
                    reference: reason_reference,
                },
            ) => proof_reference == reason_reference,
            (
                Self::ClassifiedDiagnostic {
                    diagnostic_id:
                        DiagnosticId {
                            class: proof_class, ..
                        },
                },
                EditReason::BuildDiagnostic {
                    class: reason_class,
                },
            ) => proof_class == reason_class,
            (
                Self::ClassifiedDiagnostic {
                    diagnostic_id:
                        DiagnosticId {
                            class: DiagnosticClass::MissingHeader,
                            ..
                        },
                },
                EditReason::DiagnosticMissingHeaderFixup,
            ) => true,
            (
                Self::ClassifiedDiagnostic {
                    diagnostic_id:
                        DiagnosticId {
                            class: DiagnosticClass::StaleKbuildDirectoryRef,
                            ..
                        },
                },
                EditReason::DiagnosticStaleKbuildDirFixup,
            ) => true,
            (
                Self::ClassifiedDiagnostic {
                    diagnostic_id:
                        DiagnosticId {
                            class: DiagnosticClass::StaleKbuildObjectRef,
                            ..
                        },
                },
                EditReason::DiagnosticStaleKbuildObjectFixup,
            ) => true,
            (
                Self::ClassifiedDiagnostic {
                    diagnostic_id:
                        DiagnosticId {
                            class: DiagnosticClass::MissingKconfigSource,
                            ..
                        },
                },
                EditReason::DiagnosticMissingKconfigSourceFixup,
            ) => true,
            (
                Self::ClassifiedDiagnostic {
                    diagnostic_id:
                        DiagnosticId {
                            class:
                                DiagnosticClass::DeadConfigGatedCodePath
                                | DiagnosticClass::RemovedConfigSymbolUse
                                | DiagnosticClass::RemovedHeaderSymbolUse,
                            ..
                        },
                },
                EditReason::DiagnosticPreprocessorRefoldFixup,
            ) => true,
            _ => false,
        }
    }

    pub(in crate::edit_reason) fn validate_reasoned_payload(&self) -> Result<()> {
        match self {
            Self::RemovalManifest { key } => key.validate_reasoned_payload(),
            Self::TreeIndex { index_kind: _, key } => {
                validate_non_empty_payload("tree index proof key", key)
            }
            Self::KconfigSolver { key } => key.validate_reasoned_payload(),
            Self::StaleReference {
                reference_kind: _,
                key,
            } => validate_non_empty_payload("stale reference proof key", key),
            Self::ClassifiedDiagnostic { diagnostic_id } => {
                diagnostic_id.validate_reasoned_payload()
            }
        }
    }

    pub(in crate::edit_reason) fn is_broad_speculative_fallout(&self) -> bool {
        match self {
            Self::ClassifiedDiagnostic { diagnostic_id } => {
                diagnostic_id.class.is_broad_speculative_fallout()
            }
            _ => false,
        }
    }
}
