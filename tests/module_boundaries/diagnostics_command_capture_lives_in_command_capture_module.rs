use super::common::*;

#[test]
fn diagnostics_command_capture_lives_in_command_capture_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let diagnostics = production_source(&root.join("src/diagnostics.rs"));
    let classifier = production_source(&root.join("src/diagnostics/classifier.rs"));
    let command_capture = production_source(&root.join("src/diagnostics/command_capture.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        diagnostics.contains("mod command_capture;"),
        "src/diagnostics.rs should register command-capture ownership"
    );

    for required in [
        "use crate::selftest::{CapturedCommandFailure, SelfTestFailure};",
        "pub(in crate::diagnostics) enum CapturedDiagnostic<'a>",
        "Command(CapturedCommandDiagnostic<'a>)",
        "BuiltIn(CapturedBuiltInDiagnostic<'a>)",
        "pub(in crate::diagnostics) struct CapturedCommandDiagnostic<'a>",
        "pub stderr: &'a str",
        "pub target: Option<&'a str>",
        "pub arch: Option<&'a str>",
        "pub config: Option<&'a str>",
        "pub(in crate::diagnostics) struct CapturedBuiltInDiagnostic<'a>",
        "pub check: &'a str",
        "pub message: &'a str",
        "pub(in crate::diagnostics) fn capture_selftest_failure(",
        "SelfTestFailure::KernelBuild { details, .. }",
        "SelfTestFailure::Command { details }",
        "SelfTestFailure::BuiltIn { check, message }",
        "fn capture_command_failure(details: &CapturedCommandFailure)",
        "details.target.as_deref()",
        "details.arch.as_deref()",
        "details.config.as_deref()",
    ] {
        assert!(
            command_capture.contains(required),
            "src/diagnostics/command_capture.rs should own command capture detail {required}"
        );
    }

    for forbidden in [
        "SelfTestFailure::KernelBuild { details, .. }",
        "SelfTestFailure::Command { details }",
        "SelfTestFailure::BuiltIn { check, message }",
        "CapturedCommandFailure",
    ] {
        assert!(
            !classifier.contains(forbidden),
            "src/diagnostics/classifier.rs should consume captured inputs instead of selftest command shapes {forbidden}"
        );
    }

    for required in ["`src/diagnostics/command_capture.rs`", "Diagnostic command capture"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document diagnostic command-capture ownership {required}"
        );
    }
}
