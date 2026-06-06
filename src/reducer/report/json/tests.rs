//! Unit tests for split reducer JSON helpers.

use super::canonical::{render_edit_proof_source_count_entries, sort_strings};
use super::escaping::{bool_json, json_compact, json_escape};
use crate::edit_reason::EditProofSourceKind;
use std::collections::BTreeMap;

#[test]
fn test_json_escape_handles_control_characters() {
    assert_eq!(json_escape("quote\" slash\\ newline\n tab\t"), "quote\\\" slash\\\\ newline\\n tab\\t");
    assert_eq!(json_escape("control\u{0007}"), "control\\u0007");
}

#[test]
fn test_json_compact_and_bool_json_emit_stable_scalars() {
    assert_eq!(bool_json(true), "true");
    assert_eq!(bool_json(false), "false");
    assert_eq!(json_compact(&vec!["b", "a"]), "[\"b\",\"a\"]");
}

#[test]
fn test_canonical_ordering_dedups_strings_and_orders_proof_sources() {
    let mut values = vec![String::from("z"), String::from("a"), String::from("z")];
    sort_strings(&mut values);
    assert_eq!(values, ["a", "z"]);

    let mut counts = BTreeMap::new();
    counts.insert(EditProofSourceKind::ClassifiedDiagnostic, 2);
    counts.insert(EditProofSourceKind::RemovalManifestEntry, 1);

    let rendered = render_edit_proof_source_count_entries(&counts, "");
    let first = rendered.find("classified_build_diagnostic").unwrap();
    let manifest = rendered.find("removal_manifest_entry").unwrap();
    assert!(first < manifest, "proof source counts should be ordered by JSON key: {rendered}");
    assert!(rendered.contains("\"tree_index_entry\": 0"));
}
