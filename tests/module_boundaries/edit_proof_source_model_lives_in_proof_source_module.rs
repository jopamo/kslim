use super::common::*;

#[test]
fn edit_proof_source_model_lives_in_proof_source_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let edit_reason = production_source(&root.join("src/edit_reason.rs"));
    let proof_source = production_source(&root.join("src/edit_reason/proof_source.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod proof_source;",
        "pub use proof_source::{",
        "DiagnosticId",
        "EditProofSource",
        "EditProofSourceKind",
        "IndexKind",
        "KconfigSolverKey",
        "ReferenceKind",
        "RemovalKey",
    ] {
        assert!(
            edit_reason.contains(required),
            "src/edit_reason.rs should expose proof source model through {required}"
        );
    }

    for required in [
        "pub enum EditProofSourceKind",
        "RemovalManifestEntry",
        "ClassifiedDiagnostic",
        "pub enum RemovalKey",
        "Header { header: String, path: PathBuf }",
        "pub enum IndexKind",
        "pub enum ReferenceKind",
        "pub enum KconfigSolverKey",
        "UnreachableSymbolDefinition",
        "pub struct DiagnosticId",
        "pub enum EditProofSource",
        "pub fn removal_manifest_path(path: PathBuf) -> Self",
        "pub fn kind(&self) -> EditProofSourceKind",
        "pub fn payload_label(&self) -> String",
        "pub fn matches_reason(&self, reason: &EditReason) -> bool",
        "pub(in crate::edit_reason) fn validate_reasoned_payload(&self) -> Result<()>",
        "pub(in crate::edit_reason) fn is_broad_speculative_fallout(&self) -> bool",
    ] {
        assert!(
            proof_source.contains(required),
            "src/edit_reason/proof_source.rs should own proof source model detail {required}"
        );
    }

    for forbidden in [
        "\npub enum EditProofSourceKind",
        "\npub enum RemovalKey",
        "\npub enum IndexKind",
        "\npub enum ReferenceKind",
        "\npub enum KconfigSolverKey",
        "\npub struct DiagnosticId",
        "\npub enum EditProofSource",
    ] {
        assert!(
            !edit_reason.contains(forbidden),
            "src/edit_reason.rs should not keep proof source model body {forbidden}"
        );
    }

    for required in ["`src/edit_reason/proof_source.rs`", "Edit proof source model"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document proof source model ownership {required}"
        );
    }
}
