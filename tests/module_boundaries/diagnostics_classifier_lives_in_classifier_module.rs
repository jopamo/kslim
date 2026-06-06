use super::common::*;

#[test]
fn diagnostics_classifier_lives_in_classifier_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let diagnostics = production_source(&root.join("src/diagnostics.rs"));
    let classifier = production_source(&root.join("src/diagnostics/classifier.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in ["mod classifier;", "pub use classifier::classify_selftest_failure;"] {
        assert!(
            diagnostics.contains(required),
            "src/diagnostics.rs should expose the classifier through {required}"
        );
    }

    for required in [
        "pub fn classify_selftest_failure(root: &Path, failure: &SelfTestFailure)",
        "capture_selftest_failure(failure)",
        "CapturedDiagnostic::Command(details)",
        "CapturedDiagnostic::BuiltIn(details)",
        "pub(in crate::diagnostics) fn classify_builtin_failure(",
        "pub(in crate::diagnostics) fn classify_command_failure(",
        "pub(in crate::diagnostics) fn normalize_source_path(",
        "pub(in crate::diagnostics) fn parse_missing_header_line(",
        "pub(in crate::diagnostics) fn parse_make_missing_target_line(",
        "pub(in crate::diagnostics) fn parse_make_missing_directory_line(",
        "pub(in crate::diagnostics) fn parse_missing_kconfig_source_message(",
        "pub(in crate::diagnostics) fn parse_gcc_undeclared_identifier_line(",
        "pub(in crate::diagnostics) fn parse_clang_undeclared_identifier_line(",
        "pub(in crate::diagnostics) fn parse_gcc_implicit_declaration_line(",
        "pub(in crate::diagnostics) fn parse_clang_implicit_declaration_line(",
        "pub(in crate::diagnostics) fn parse_gcc_undefined_reference_line(",
        "ClassifiedDiagnostic::MissingHeader",
        "ClassifiedDiagnostic::UndefinedReference",
    ] {
        assert!(
            classifier.contains(required),
            "src/diagnostics/classifier.rs should own classifier detail {required}"
        );
    }

    for forbidden in [
        "\npub fn classify_selftest_failure(",
        "\nfn classify_command_failure(",
        "\nfn parse_missing_header_line(",
        "\nfn parse_gcc_undefined_reference_line(",
    ] {
        assert!(
            !diagnostics.contains(forbidden),
            "src/diagnostics.rs should not keep classifier helper body {forbidden}"
        );
    }

    for required in ["`src/diagnostics/classifier.rs`", "Diagnostic classifier"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document diagnostic classifier ownership {required}"
        );
    }
}
