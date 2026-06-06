use super::common::*;

#[test]
fn ci_runs_rust_source_size_policy_check() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow = std::fs::read_to_string(root.join(".github/workflows/source-size.yml"))
        .expect("failed to read .github/workflows/source-size.yml");

    for required in [
        "Rust source-size policy",
        "src/**/*.rs and tests/**/*.rs",
        "cargo test --test module_boundaries",
        "rust_source_files_over_2000_lines_are_explicitly_justified",
    ] {
        assert!(
            workflow.contains(required),
            "source-size CI workflow should wire the Rust source-size policy through {required}"
        );
    }
}
