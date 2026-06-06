use super::common::*;

#[test]
fn generate_orchestration_entrypoint_lives_in_orchestration_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let generate = production_source(&root.join("src/generate.rs"));
    let orchestration = production_source(&root.join("src/generate/orchestration.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod orchestration;",
        "pub use orchestration::generate;",
        "pub(crate) use orchestration::generate_with_source_maps;",
    ] {
        assert!(
            generate.contains(required),
            "src/generate.rs should expose generate orchestration through {required}"
        );
    }

    for required in [
        "pub fn generate(",
        "pub(crate) fn generate_with_source_maps(",
        "let mut failure = FailureReportContext",
        "RequestedGenerateState::from_inputs(",
        "generate_inner(",
        "write_authoritative_lockfile(",
        "rollback_output_repo_failure_atomic_state(",
        "record_generate_attempt_failure(",
        "failure::record_generate_failure(",
    ] {
        assert!(
            orchestration.contains(required),
            "src/generate/orchestration.rs should own generate entrypoint orchestration through {required}"
        );
    }

    for forbidden in [
        "\npub fn generate(",
        "\npub(crate) fn generate_with_source_maps(",
        "let mut failure = FailureReportContext",
    ] {
        assert!(
            !generate.contains(forbidden),
            "src/generate.rs should not retain extracted orchestration implementation {forbidden}"
        );
    }

    for required in [
        "`src/generate/orchestration.rs`",
        "Generate orchestration entrypoints",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document extracted generate orchestration ownership through {required}"
        );
    }
}
