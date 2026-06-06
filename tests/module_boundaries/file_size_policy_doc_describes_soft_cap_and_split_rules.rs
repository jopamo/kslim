use super::common::*;

#[test]
fn file_size_policy_doc_describes_soft_cap_and_split_rules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let doc = std::fs::read_to_string(root.join("docs/file-size-policy.md"))
        .expect("failed to read docs/file-size-policy.md");

    for required in [
        "# Rust file-size policy",
        "2000-line soft cap",
        "Around 1500 lines",
        "`src/**/*.rs` and `tests/**/*.rs`",
        "cargo test --test module_boundaries",
        "rust_source_files_over_2000_lines_are_explicitly_justified",
        "`RUST_FILE_SIZE_JUSTIFICATIONS`",
        "Token exceptions are rejected",
        "stale justifications",
        "## Split rules",
        "Prefer splits by lifecycle or responsibility",
        "one test file per behavior or boundary being enforced",
        "forwarding wrappers",
        "moving cold policy into hot rewrite/index loops",
        "dependency direction honest",
    ] {
        assert!(
            doc.contains(required),
            "docs/file-size-policy.md should document file-size rule {required}"
        );
    }
}
