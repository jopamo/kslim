use super::common::*;

#[test]
fn generate_options_live_in_options_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let generate = production_source(&root.join("src/generate.rs"));
    let options = production_source(&root.join("src/generate/options.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod options;",
        "pub use options::GenerateOptions;",
    ] {
        assert!(
            generate.contains(required),
            "src/generate.rs should expose generate options through {required}"
        );
    }

    for required in [
        "pub struct GenerateOptions",
        "pub frozen_plan: Option<FrozenPlanInputs>",
        "pub(crate) fn normalized_base_ref_for_request(&self)",
        "pub(crate) fn normalized_feature_for_request(&self)",
        "pub(crate) fn normalized_matrix_for_request(&self)",
    ] {
        assert!(
            options.contains(required),
            "src/generate/options.rs should own generate option model and normalization through {required}"
        );
    }

    for forbidden in [
        "\npub struct GenerateOptions",
        "fn normalized_base_ref_for_request(&self)",
        "fn normalized_matrix_for_request(&self)",
    ] {
        assert!(
            !generate.contains(forbidden),
            "src/generate.rs should not retain extracted generate option implementation {forbidden}"
        );
    }

    for required in [
        "`src/generate/options.rs`",
        "Generate command option normalization",
    ] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document extracted generate options ownership through {required}"
        );
    }
}
