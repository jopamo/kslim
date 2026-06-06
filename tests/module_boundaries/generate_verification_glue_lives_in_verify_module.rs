use super::common::*;

#[test]
fn generate_verification_glue_lives_in_verify_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let generate = production_source(&root.join("src/generate.rs"));
    let verify = production_source(&root.join("src/generate/verify.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "verify::verify_generated_output(",
        "verify::write_candidate_metadata_and_verify(",
    ] {
        assert!(
            generate.contains(required),
            "src/generate.rs should delegate verification glue through {required}"
        );
    }

    for required in [
        "pub(crate) struct VerifiedGeneratedOutput",
        "pub(super) fn verify_generated_output(",
        "upstream::validate_tree(tree_path)?",
        "verify_required_metadata(std::path::Path::new(tree_path))?",
        "KernelSourceRoot::new(tree_path)?",
        "reducer::run_selftests_with_fixups(",
        "pub(super) fn write_candidate_metadata_and_verify(",
        "candidate::write_candidate_metadata_for_verified_generate(",
        "verify_candidate(plan, &candidate)",
    ] {
        assert!(
            verify.contains(required),
            "src/generate/verify.rs should own legacy verification glue item {required}"
        );
    }

    for forbidden in [
        "\npub(crate) struct VerifiedGeneratedOutput",
        "\nfn verify_generated_output(",
        "\nfn verify_required_metadata(",
        "\nfn write_candidate_metadata_and_verify(",
        "upstream::validate_tree(tree_path)?",
        "reducer::run_selftests_with_fixups(",
    ] {
        assert!(
            !generate.contains(forbidden),
            "src/generate.rs should not retain extracted verification glue {forbidden}"
        );
    }

    for required in [
        "`src/generate/verify.rs`",
        "Generate verification glue",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document extracted generate verification ownership through {required}"
        );
    }
}
