//! Canonical ordering helpers for reducer JSON reports.

use std::collections::BTreeMap;

use crate::edit_reason::EditProofSourceKind;

use super::escaping::json_escape;

pub(super) fn sort_strings(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn sorted_edit_proof_source_kinds_by_report_key() -> [EditProofSourceKind; 5] {
    let mut kinds = [
        EditProofSourceKind::RemovalManifestEntry,
        EditProofSourceKind::TreeIndexEntry,
        EditProofSourceKind::KconfigSolverProof,
        EditProofSourceKind::StaleReference,
        EditProofSourceKind::ClassifiedDiagnostic,
    ];
    kinds.sort_by_key(|kind| kind.json_key());
    kinds
}

pub(super) fn render_edit_proof_source_count_entries(
    proof_counts: &BTreeMap<EditProofSourceKind, usize>,
    indent: &str,
) -> String {
    sorted_edit_proof_source_kinds_by_report_key()
        .into_iter()
        .map(|kind| {
            format!(
                "{}\"{}\": {}",
                indent,
                json_escape(kind.json_key()),
                proof_counts.get(&kind).copied().unwrap_or(0)
            )
        })
        .collect::<Vec<_>>()
        .join(",\n")
}
