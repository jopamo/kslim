//! Fixup report and proof models.
//!
//! This module owns the public result shapes emitted by deterministic fixups
//! and the proof records carried into reducer/output reports.

use std::path::PathBuf;

use crate::diagnostics::ClassifiedDiagnostic;
use crate::edit_reason::{DiagnosticClass, EditRecord};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FixupProof {
    ManifestPath {
        path: PathBuf,
    },
    TreeIndexIncludeSite {
        file: PathBuf,
        line: usize,
        target: String,
    },
    TreeIndexKbuildDirectoryRef {
        file: PathBuf,
        line: usize,
        assignment_lhs: String,
        directory: String,
        resolved_path: PathBuf,
    },
    TreeIndexKbuildObjectRef {
        file: PathBuf,
        line: usize,
        assignment_lhs: String,
        object: String,
        resolved_path: PathBuf,
    },
    TreeIndexKconfigSourceRef {
        file: PathBuf,
        line: usize,
        source: String,
        optional: bool,
        relative: bool,
    },
    ClassifiedDiagnostic {
        class: DiagnosticClass,
        file: Option<PathBuf>,
        line: Option<usize>,
        subject: Option<String>,
    },
}

impl FixupProof {
    pub(in crate::fixups) fn is_manifest_truth(&self) -> bool {
        matches!(self, Self::ManifestPath { .. })
    }

    pub(in crate::fixups) fn matches_diagnostic(&self, diagnostic: &ClassifiedDiagnostic) -> bool {
        match self {
            Self::ClassifiedDiagnostic {
                class,
                file,
                line,
                subject,
            } => {
                *class == diagnostic.class()
                    && file.as_deref() == diagnostic.file()
                    && *line == diagnostic.line()
                    && subject.as_deref() == diagnostic.subject()
            }
            Self::ManifestPath { .. }
            | Self::TreeIndexIncludeSite { .. }
            | Self::TreeIndexKbuildDirectoryRef { .. }
            | Self::TreeIndexKbuildObjectRef { .. }
            | Self::TreeIndexKconfigSourceRef { .. } => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixupResult {
    pub edits: Vec<EditRecord>,
    pub proof_sources: Vec<FixupProof>,
}

impl FixupResult {
    pub(in crate::fixups) fn new(edits: Vec<EditRecord>, proof_sources: Vec<FixupProof>) -> Self {
        let mut proof_sources = proof_sources;
        proof_sources.sort();
        proof_sources.dedup();
        Self {
            edits,
            proof_sources,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedFixup {
    pub fixer_name: &'static str,
    pub diagnostic: ClassifiedDiagnostic,
    pub edits: Vec<EditRecord>,
    pub proof_sources: Vec<FixupProof>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedFixup {
    pub fixer_name: Option<&'static str>,
    pub diagnostic: ClassifiedDiagnostic,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixupAttempt {
    Applied(AppliedFixup),
    Skipped(SkippedFixup),
}

