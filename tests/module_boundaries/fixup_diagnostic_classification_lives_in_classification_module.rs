use super::common::*;

#[test]
fn fixup_diagnostic_classification_lives_in_classification_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixups = production_source(&root.join("src/fixups.rs"));
    let classification = production_source(&root.join("src/fixups/classification.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod classification;",
        "pub(in crate::fixups) use classification::{",
        "classified_diagnostic_proof",
        "symbol_fallout_rejection_reason",
    ] {
        assert!(
            fixups.contains(required),
            "src/fixups.rs should expose diagnostic classification helper {required}"
        );
    }

    for required in [
        "pub(in crate::fixups) fn classified_diagnostic_proof(",
        "FixupProof::ClassifiedDiagnostic",
        "diagnostic.class()",
        "diagnostic.file().map(Path::to_path_buf)",
        "diagnostic.line()",
        "diagnostic.subject().map(str::to_string)",
        "pub(in crate::fixups) fn symbol_fallout_rejection_reason(",
        "ClassifiedDiagnostic::UndeclaredIdentifier",
        "ClassifiedDiagnostic::ImplicitDeclaration",
        "ClassifiedDiagnostic::UndefinedReference",
        "broad speculative edits are forbidden",
    ] {
        assert!(
            classification.contains(required),
            "src/fixups/classification.rs should own classification detail {required}"
        );
    }

    for forbidden in [
        "\nfn classified_diagnostic_proof(",
        "\nfn symbol_fallout_rejection_reason(",
    ] {
        assert!(
            !fixups.contains(forbidden),
            "src/fixups.rs should not keep diagnostic classification helper body {forbidden}"
        );
    }

    for required in ["`src/fixups/classification.rs`", "Fixup diagnostic classification"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document the fixup classification split {required}"
        );
    }
}
