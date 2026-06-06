use super::common::*;

#[test]
fn edit_reason_model_lives_in_reason_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let edit_reason = production_source(&root.join("src/edit_reason.rs"));
    let reason = production_source(&root.join("src/edit_reason/reason.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in ["mod reason;", "pub use reason::{DiagnosticClass, EditReason};"] {
        assert!(
            edit_reason.contains(required),
            "src/edit_reason.rs should expose edit reason model through {required}"
        );
    }

    for required in [
        "pub enum DiagnosticClass",
        "MissingHeader",
        "UndefinedReference",
        "pub fn stable_name(&self) -> &'static str",
        "pub fn is_broad_speculative_fallout(&self) -> bool",
        "pub enum EditReason",
        "DeclaredPathPruned",
        "RemovedDeadKconfigSymbolDefinition { symbol: String }",
        "RemovedDeadBranchInclude { header: String, symbol: String }",
        "BuildDiagnostic { class: DiagnosticClass }",
        "pub fn proof_source_kind(&self) -> EditProofSourceKind",
        "pub fn json_key(&self) -> &'static str",
        "pub fn payload_label(&self) -> String",
        "pub(in crate::edit_reason) fn validate_reasoned_payload(&self) -> Result<()>",
        "pub(in crate::edit_reason) fn is_broad_speculative_fallout(&self) -> bool",
    ] {
        assert!(
            reason.contains(required),
            "src/edit_reason/reason.rs should own edit reason model detail {required}"
        );
    }

    for forbidden in ["\npub enum DiagnosticClass", "\npub enum EditReason"] {
        assert!(
            !edit_reason.contains(forbidden),
            "src/edit_reason.rs should not keep edit reason model body {forbidden}"
        );
    }

    for required in ["`src/edit_reason/reason.rs`", "Edit reason model"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document edit reason model ownership {required}"
        );
    }
}
