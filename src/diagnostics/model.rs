//! Diagnostic classification model.
//!
//! This module owns the stable diagnostic shapes and their primary context
//! accessors. Classifier parsing and command capture live outside this model.

use std::path::{Path, PathBuf};

use crate::edit_reason::DiagnosticClass;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassifiedDiagnostic {
    MissingHeader {
        source_file: PathBuf,
        line: usize,
        header: String,
        build_target: Option<String>,
        arch: Option<String>,
        config: Option<String>,
    },
    MissingKconfigSource {
        kconfig_file: PathBuf,
        line: usize,
        source: String,
    },
    MissingMakeDirectory {
        path: String,
        build_target: Option<String>,
        arch: Option<String>,
        config: Option<String>,
    },
    MissingMakeTarget {
        target: String,
        build_target: Option<String>,
        arch: Option<String>,
        config: Option<String>,
    },
    UndeclaredIdentifier {
        source_file: PathBuf,
        line: usize,
        symbol: String,
        build_target: Option<String>,
        arch: Option<String>,
        config: Option<String>,
    },
    ImplicitDeclaration {
        source_file: PathBuf,
        line: usize,
        symbol: String,
        build_target: Option<String>,
        arch: Option<String>,
        config: Option<String>,
    },
    UndefinedReference {
        source_file: PathBuf,
        symbol: String,
        build_target: Option<String>,
        arch: Option<String>,
        config: Option<String>,
    },
    Unknown,
}

impl ClassifiedDiagnostic {
    #[allow(dead_code)]
    pub fn class(&self) -> DiagnosticClass {
        match self {
            Self::MissingHeader { .. } => DiagnosticClass::MissingHeader,
            Self::MissingKconfigSource { .. } => DiagnosticClass::MissingKconfigSource,
            Self::MissingMakeDirectory { .. } => DiagnosticClass::StaleKbuildDirectoryRef,
            Self::MissingMakeTarget { .. } => DiagnosticClass::StaleKbuildObjectRef,
            Self::UndeclaredIdentifier { .. } => DiagnosticClass::Unknown,
            Self::ImplicitDeclaration { .. } => DiagnosticClass::Unknown,
            Self::UndefinedReference { .. } => DiagnosticClass::UndefinedReference,
            Self::Unknown => DiagnosticClass::Unknown,
        }
    }

    pub fn is_unknown_class(&self) -> bool {
        self.class() == DiagnosticClass::Unknown
    }

    #[allow(dead_code)]
    pub fn file(&self) -> Option<&Path> {
        match self {
            Self::MissingHeader { source_file, .. } => Some(source_file.as_path()),
            Self::MissingKconfigSource { kconfig_file, .. } => Some(kconfig_file.as_path()),
            Self::MissingMakeDirectory { path, .. } => Some(Path::new(path)),
            Self::MissingMakeTarget { target, .. } => Some(Path::new(target)),
            Self::UndeclaredIdentifier { source_file, .. } => Some(source_file.as_path()),
            Self::ImplicitDeclaration { source_file, .. } => Some(source_file.as_path()),
            Self::UndefinedReference { source_file, .. } => Some(source_file.as_path()),
            Self::Unknown => None,
        }
    }

    #[allow(dead_code)]
    pub fn line(&self) -> Option<usize> {
        match self {
            Self::MissingHeader { line, .. } => Some(*line),
            Self::MissingKconfigSource { line, .. } => Some(*line),
            Self::UndeclaredIdentifier { line, .. } => Some(*line),
            Self::ImplicitDeclaration { line, .. } => Some(*line),
            Self::MissingMakeDirectory { .. }
            | Self::MissingMakeTarget { .. }
            | Self::UndefinedReference { .. }
            | Self::Unknown => None,
        }
    }

    #[allow(dead_code)]
    pub fn build_target(&self) -> Option<&str> {
        match self {
            Self::MissingHeader { build_target, .. }
            | Self::MissingMakeDirectory { build_target, .. }
            | Self::MissingMakeTarget { build_target, .. }
            | Self::UndeclaredIdentifier { build_target, .. }
            | Self::ImplicitDeclaration { build_target, .. }
            | Self::UndefinedReference { build_target, .. } => build_target.as_deref(),
            Self::MissingKconfigSource { .. } | Self::Unknown => None,
        }
    }

    #[allow(dead_code)]
    pub fn arch(&self) -> Option<&str> {
        match self {
            Self::MissingHeader { arch, .. }
            | Self::MissingMakeDirectory { arch, .. }
            | Self::MissingMakeTarget { arch, .. }
            | Self::UndeclaredIdentifier { arch, .. }
            | Self::ImplicitDeclaration { arch, .. }
            | Self::UndefinedReference { arch, .. } => arch.as_deref(),
            Self::MissingKconfigSource { .. } | Self::Unknown => None,
        }
    }

    #[allow(dead_code)]
    pub fn config(&self) -> Option<&str> {
        match self {
            Self::MissingHeader { config, .. }
            | Self::MissingMakeDirectory { config, .. }
            | Self::MissingMakeTarget { config, .. }
            | Self::UndeclaredIdentifier { config, .. }
            | Self::ImplicitDeclaration { config, .. }
            | Self::UndefinedReference { config, .. } => config.as_deref(),
            Self::MissingKconfigSource { .. } | Self::Unknown => None,
        }
    }

    #[allow(dead_code)]
    pub fn subject(&self) -> Option<&str> {
        match self {
            Self::MissingHeader { header, .. } => Some(header),
            Self::MissingKconfigSource { source, .. } => Some(source),
            Self::MissingMakeDirectory { path, .. } => Some(path),
            Self::MissingMakeTarget { target, .. } => Some(target),
            Self::UndeclaredIdentifier { symbol, .. } => Some(symbol),
            Self::ImplicitDeclaration { symbol, .. } => Some(symbol),
            Self::UndefinedReference { symbol, .. } => Some(symbol),
            Self::Unknown => None,
        }
    }
}

