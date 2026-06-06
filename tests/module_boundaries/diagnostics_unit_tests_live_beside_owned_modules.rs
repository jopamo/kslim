use super::common::*;

#[test]
fn diagnostics_unit_tests_live_beside_owned_modules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let diagnostics = std::fs::read_to_string(root.join("src/diagnostics.rs"))
        .expect("failed to read src/diagnostics.rs");
    let tests = production_source(&root.join("src/diagnostics/tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        diagnostics.contains("#[cfg(test)]\nmod tests;"),
        "src/diagnostics.rs should register a sibling diagnostics tests module"
    );
    assert!(
        !diagnostics.contains("mod tests {"),
        "src/diagnostics.rs should not keep inline diagnostics unit tests"
    );

    for required in [
        "use super::*;",
        "use super::classifier::*;",
        "test_parse_missing_header_line_accepts_gcc_missing_header_shape",
        "test_parse_make_missing_target_line_accepts_make_target_shape_but_not_directory_shape",
        "test_parse_missing_kconfig_source_message_accepts_selftest_shape",
        "test_classified_diagnostic_file_returns_primary_path_context",
        "test_classified_diagnostic_subject_returns_primary_symbol_header_or_object",
        "test_parse_gcc_undefined_reference_line_accepts_direct_and_ld_prefixed_shapes",
        "test_classify_selftest_failure_recognizes_gcc_missing_header",
        "test_classify_selftest_failure_normalizes_absolute_gcc_undefined_reference_path",
    ] {
        assert!(
            tests.contains(required),
            "src/diagnostics/tests.rs should carry diagnostics unit test coverage through {required}"
        );
    }

    for required in ["`src/diagnostics/tests.rs`", "Diagnostics unit tests"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document diagnostics test module ownership through {required}"
        );
    }
}
