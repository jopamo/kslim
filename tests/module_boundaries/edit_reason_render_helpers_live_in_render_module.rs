use super::common::*;

#[test]
fn edit_reason_render_helpers_live_in_render_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let edit_reason = production_source(&root.join("src/edit_reason.rs"));
    let render = production_source(&root.join("src/edit_reason/render.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod render;",
        "pub use render::{",
        "grouped_edit_record_refs_by_reason",
        "sort_edit_records",
        "sorted_edit_record_refs",
        "pub(in crate::edit_reason) use render::{bounded_edit_content, payload_token};",
    ] {
        assert!(
            edit_reason.contains(required),
            "src/edit_reason.rs should expose edit render helpers through {required}"
        );
    }

    for required in [
        "const EDIT_PASS_ORDER: &[&str]",
        "pub fn sort_edit_records(edits: &mut Vec<EditRecord>)",
        "pub fn sorted_edit_record_refs(edits: &[EditRecord]) -> Vec<&EditRecord>",
        "pub fn grouped_edit_record_refs_by_reason",
        "fn compare_edit_records(",
        "fn edit_pass_rank(pass_name: &str) -> usize",
        "fn line_range_sort_key(line_range: Option<LineRange>) -> (usize, usize)",
        "pub(in crate::edit_reason) fn bounded_edit_content(content: String) -> String",
        "pub(in crate::edit_reason) fn payload_token(value: &str) -> String",
        "\"prune.remove_path\"",
        "\"fixups.remove_missing_kconfig_source\"",
        "\"<kslim: content elided len={}",
    ] {
        assert!(
            render.contains(required),
            "src/edit_reason/render.rs should own edit render helper detail {required}"
        );
    }

    for forbidden in [
        "\nconst EDIT_PASS_ORDER",
        "\npub fn sort_edit_records",
        "\npub fn sorted_edit_record_refs",
        "\npub fn grouped_edit_record_refs_by_reason",
        "\nfn compare_edit_records",
        "\nfn edit_pass_rank",
        "\nfn line_range_sort_key",
        "\nfn bounded_edit_content",
        "\nfn payload_token",
    ] {
        assert!(
            !edit_reason.contains(forbidden),
            "src/edit_reason.rs should not keep edit render helper body {forbidden}"
        );
    }

    for required in ["`src/edit_reason/render.rs`", "Edit reason render helpers"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document edit render helper ownership {required}"
        );
    }
}
