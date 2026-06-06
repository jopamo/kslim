use super::common::*;

#[test]
fn edit_validation_lives_in_validation_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let edit_reason = production_source(&root.join("src/edit_reason.rs"));
    let validation = production_source(&root.join("src/edit_reason/validation.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod validation;",
        "pub use validation::{",
        "ensure_edit_records_for_mutation",
        "validate_edit_records",
        "validate_edit_records_with_policy",
        "write_verified_rewrite",
        "EditValidationPolicy",
        "pub(in crate::edit_reason) use validation::{",
        "validate_non_empty_payload",
        "validate_non_empty_payload_path",
        "validate_relative_edit_path",
    ] {
        assert!(
            edit_reason.contains(required),
            "src/edit_reason.rs should expose edit validation through {required}"
        );
    }

    for required in [
        "impl EditRecord {",
        "pub fn validate_no_competing_proof_sources(&self) -> Result<()>",
        "pub fn validate_required_fields(&self) -> Result<()>",
        "pub fn validate_reasoned(&self) -> Result<()>",
        "pub fn validate_not_speculative_fallout(&self) -> Result<()>",
        "fn validate_stable_audit_aliases(",
        "fn validate_span_for_edit_kind(",
        "fn validate_reason_for_pass(",
        "fn reason_allowed_for_pass(",
        "fn validate_audit_content(",
        "pub(in crate::edit_reason) fn validate_relative_edit_path(",
        "pub(in crate::edit_reason) fn validate_non_empty_payload(",
        "pub(in crate::edit_reason) fn validate_non_empty_payload_path(",
        "pub fn validate_edit_records(edits: &[EditRecord]) -> Result<()>",
        "pub fn validate_reasoned_edit_records(edits: &[EditRecord]) -> Result<()>",
        "pub fn validate_no_speculative_fallout_edit_records(edits: &[EditRecord]) -> Result<()>",
        "pub struct EditValidationPolicy",
        "pub fn validate_edit_records_with_policy(",
        "pub fn ensure_edit_records_for_mutation(",
        "pub fn write_verified_rewrite(",
        "fn verify_rewrite_is_fully_recorded(",
        "fn unique_text_replacements(",
        "fn split_lines_preserving_endings(",
    ] {
        assert!(
            validation.contains(required),
            "src/edit_reason/validation.rs should own edit validation detail {required}"
        );
    }

    for forbidden in [
        "\npub fn validate_edit_records",
        "\npub fn validate_edit_records_with_policy",
        "\npub fn ensure_edit_records_for_mutation",
        "\npub fn write_verified_rewrite",
        "\nfn validate_stable_audit_aliases",
        "\nfn validate_span_for_edit_kind",
        "\nfn validate_reason_for_pass",
        "\nfn reason_allowed_for_pass",
        "\nfn validate_audit_content",
        "\nfn verify_rewrite_is_fully_recorded",
        "\nfn split_lines_preserving_endings",
    ] {
        assert!(
            !edit_reason.contains(forbidden),
            "src/edit_reason.rs should not keep edit validation body {forbidden}"
        );
    }

    for required in ["`src/edit_reason/validation.rs`", "Edit validation"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document edit validation ownership {required}"
        );
    }
}
