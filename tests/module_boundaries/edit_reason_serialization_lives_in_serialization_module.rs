use super::common::*;

#[test]
fn edit_reason_serialization_lives_in_serialization_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let edit_reason = production_source(&root.join("src/edit_reason.rs"));
    let serialization = production_source(&root.join("src/edit_reason/serialization.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod serialization;",
        "pub use serialization::{",
        "proof_source_kind_for_reason_key",
        "validate_reported_no_speculative_fallout_edit",
        "validate_reported_proof_source_payload_for_reason",
    ] {
        assert!(
            edit_reason.contains(required),
            "src/edit_reason.rs should expose edit serialization through {required}"
        );
    }

    for required in [
        "pub fn proof_source_kind_for_reason_key(reason_key: &str) -> Option<EditProofSourceKind>",
        "pub fn validate_reported_proof_source_payload_for_reason(",
        "pub fn validate_reported_no_speculative_fallout_edit(",
        "fn reported_truth_is_broad_speculative_fallout(",
        "fn validate_reported_payload_values_match(",
        "fn validate_reported_payload_value(",
        "fn validate_reported_payload_value_in(",
        "fn require_reported_payload_value",
        "fn reported_payload_value",
        "\"removed_dead_kconfig_symbol_definition\"",
        "\"classified_build_diagnostic\"",
    ] {
        assert!(
            serialization.contains(required),
            "src/edit_reason/serialization.rs should own edit serialization detail {required}"
        );
    }

    for forbidden in [
        "\npub fn proof_source_kind_for_reason_key",
        "\npub fn validate_reported_proof_source_payload_for_reason",
        "\npub fn validate_reported_no_speculative_fallout_edit",
        "\nfn reported_truth_is_broad_speculative_fallout",
        "\nfn reported_payload_value",
    ] {
        assert!(
            !edit_reason.contains(forbidden),
            "src/edit_reason.rs should not keep edit serialization body {forbidden}"
        );
    }

    for required in [
        "`src/edit_reason/serialization.rs`",
        "Edit reason serialization",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document edit serialization ownership {required}"
        );
    }
}
