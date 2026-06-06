use super::common::*;

#[test]
fn diagnostics_model_lives_in_model_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let diagnostics = production_source(&root.join("src/diagnostics.rs"));
    let model = production_source(&root.join("src/diagnostics/model.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in ["mod model;", "pub use model::ClassifiedDiagnostic;"] {
        assert!(
            diagnostics.contains(required),
            "src/diagnostics.rs should expose the diagnostic model through {required}"
        );
    }

    for required in [
        "pub enum ClassifiedDiagnostic",
        "MissingHeader {",
        "MissingKconfigSource {",
        "MissingMakeDirectory {",
        "MissingMakeTarget {",
        "UndeclaredIdentifier {",
        "ImplicitDeclaration {",
        "UndefinedReference {",
        "Unknown",
        "pub fn class(&self) -> DiagnosticClass",
        "pub fn is_unknown_class(&self) -> bool",
        "pub fn file(&self) -> Option<&Path>",
        "pub fn line(&self) -> Option<usize>",
        "pub fn build_target(&self) -> Option<&str>",
        "pub fn arch(&self) -> Option<&str>",
        "pub fn config(&self) -> Option<&str>",
        "pub fn subject(&self) -> Option<&str>",
    ] {
        assert!(
            model.contains(required),
            "src/diagnostics/model.rs should own diagnostic model detail {required}"
        );
    }

    for forbidden in ["\npub enum ClassifiedDiagnostic", "\nimpl ClassifiedDiagnostic"] {
        assert!(
            !diagnostics.contains(forbidden),
            "src/diagnostics.rs should not keep diagnostic model body {forbidden}"
        );
    }

    for required in ["`src/diagnostics/model.rs`", "Diagnostic model"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document diagnostic model ownership {required}"
        );
    }
}
