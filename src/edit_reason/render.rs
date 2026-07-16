//! Edit reason render helpers.
//!
//! This module owns deterministic edit ordering/grouping, audit-content
//! elision, and stable payload-token rendering consumed by reason and proof
//! labels before reducer reports render accepted edit records.

use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::BTreeMap;

use super::{EditRecord, LineRange, MAX_EDIT_CONTENT_BYTES};

const EDIT_PASS_ORDER: &[&str] = &[
    "prune.remove_path",
    "prune.cleanup_empty_parents",
    "prune.prune_configs",
    "prune.rewrite_kconfig_defaults",
    "kconfig.rewrite_relations",
    "kconfig.rewrite_dead_symbol_definitions",
    "kconfig.rewrite_empty_menus",
    "prune.rewrite_kconfig_sources",
    "prune.rewrite_removed_kconfig_helpers",
    "prune.rewrite_makefiles",
    "cpp.fold_removed_config_branches",
    "includes.rewrite_removed_headers",
    "fixups.remove_missing_header_include",
    "fixups.remove_stale_kbuild_directory_ref",
    "fixups.remove_stale_kbuild_object_ref",
    "fixups.remove_missing_kconfig_source",
];

pub fn sort_edit_records(edits: &mut Vec<EditRecord>) {
    edits.sort_by(compare_edit_records);
    edits.dedup();
}

pub fn sorted_edit_record_refs(edits: &[EditRecord]) -> Vec<&EditRecord> {
    let mut refs = edits.iter().collect::<Vec<_>>();
    refs.sort_by(|left, right| compare_edit_records(left, right));
    refs.dedup_by(|left, right| **left == **right);
    refs
}

pub fn grouped_edit_record_refs_by_reason<'a>(
    edits: &'a [EditRecord],
) -> BTreeMap<&'static str, Vec<&'a EditRecord>> {
    let mut groups: BTreeMap<&'static str, Vec<&'a EditRecord>> = BTreeMap::new();
    for edit in sorted_edit_record_refs(edits) {
        groups.entry(edit.reason.json_key()).or_default().push(edit);
    }
    groups
}

fn compare_edit_records(left: &EditRecord, right: &EditRecord) -> Ordering {
    edit_pass_rank(left.pass_name)
        .cmp(&edit_pass_rank(right.pass_name))
        .then(left.pass_name.cmp(right.pass_name))
        .then(left.file.cmp(&right.file))
        .then(line_range_sort_key(left.line_range).cmp(&line_range_sort_key(right.line_range)))
        .then(left.edit_kind.cmp(&right.edit_kind))
        .then(left.reason.cmp(&right.reason))
        .then(left.proof_source.cmp(&right.proof_source))
        .then(left.before.cmp(&right.before))
        .then(left.after.cmp(&right.after))
        .then(left.idempotence_marker.cmp(&right.idempotence_marker))
}

fn edit_pass_rank(pass_name: &str) -> usize {
    EDIT_PASS_ORDER
        .iter()
        .position(|known| *known == pass_name)
        .unwrap_or(EDIT_PASS_ORDER.len())
}

fn line_range_sort_key(line_range: Option<LineRange>) -> (usize, usize) {
    line_range
        .map(|range| (range.start, range.end))
        .unwrap_or((0, 0))
}

pub(in crate::edit_reason) fn bounded_edit_content(content: String) -> String {
    if content.len() <= MAX_EDIT_CONTENT_BYTES {
        return content;
    }

    format!(
        "<kslim: content elided len={} sha256={}>\n",
        content.len(),
        hex::encode(Sha256::digest(content.as_bytes()))
    )
}

pub(in crate::edit_reason) fn payload_token(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/' | b':') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}
