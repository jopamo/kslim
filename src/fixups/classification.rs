//! Diagnostic classification helpers for deterministic fixups.
//!
//! This module owns the conversion from an already-classified diagnostic into
//! fixup proof truth, plus fail-closed rejection of broad symbol fallout that
//! has no deterministic, proof-backed fixup.

use std::path::Path;

use crate::diagnostics::ClassifiedDiagnostic;

use super::FixupProof;

pub(in crate::fixups) fn classified_diagnostic_proof(
    diagnostic: &ClassifiedDiagnostic,
) -> FixupProof {
    FixupProof::ClassifiedDiagnostic {
        class: diagnostic.class(),
        file: diagnostic.file().map(Path::to_path_buf),
        line: diagnostic.line(),
        subject: diagnostic.subject().map(str::to_string),
    }
}

pub(in crate::fixups) fn symbol_fallout_rejection_reason(
    diagnostic: &ClassifiedDiagnostic,
) -> Option<String> {
    match diagnostic {
        ClassifiedDiagnostic::UndeclaredIdentifier { symbol, .. }
        | ClassifiedDiagnostic::ImplicitDeclaration { symbol, .. }
        | ClassifiedDiagnostic::UndefinedReference { symbol, .. } => Some(format!(
            "symbol fallout for '{}' has no deterministic fixer; broad speculative edits are forbidden",
            symbol
        )),
        _ => None,
    }
}
