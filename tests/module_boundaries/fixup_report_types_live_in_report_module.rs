use super::common::*;

#[test]
fn fixup_report_types_live_in_report_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixups = production_source(&root.join("src/fixups.rs"));
    let report = production_source(&root.join("src/fixups/report.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod report;",
        "pub use report::{AppliedFixup, FixupAttempt, FixupProof, FixupResult, SkippedFixup};",
    ] {
        assert!(
            fixups.contains(required),
            "src/fixups.rs should re-export fixup report type {required}"
        );
    }

    for required in [
        "pub enum FixupProof",
        "ClassifiedDiagnostic {",
        "pub(in crate::fixups) fn is_manifest_truth(&self) -> bool",
        "pub(in crate::fixups) fn matches_diagnostic(&self, diagnostic: &ClassifiedDiagnostic)",
        "pub struct FixupResult",
        "pub(in crate::fixups) fn new(edits: Vec<EditRecord>, proof_sources: Vec<FixupProof>)",
        "pub struct AppliedFixup",
        "pub struct SkippedFixup",
        "pub enum FixupAttempt",
    ] {
        assert!(
            report.contains(required),
            "src/fixups/report.rs should own fixup report/proof model detail {required}"
        );
    }

    for forbidden in [
        "\npub enum FixupProof",
        "\npub struct FixupResult",
        "\npub struct AppliedFixup",
        "\npub struct SkippedFixup",
        "\npub enum FixupAttempt",
    ] {
        assert!(
            !fixups.contains(forbidden),
            "src/fixups.rs should not keep fixup report/proof model body {forbidden}"
        );
    }

    for required in ["`src/fixups/report.rs`", "Fixup report"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document the fixup report split {required}"
        );
    }
}
