use super::common::*;

#[test]
fn cpp_folding_module_removes_only_proven_dead_branches() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cpp_source =
        std::fs::read_to_string(root.join("src/source_scan/cpp.rs")).expect("failed to read src/source_scan/cpp.rs");
    let cpp = cpp_source
        .split("#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(&cpp_source);

    for required in [
        "struct CppLineContext",
        "fn cpp_line_contexts",
        "fn parse_contextual_cpp_directive",
        "fn directive_structure_is_fully_understood_with_context",
        "TruthValue::Unknown",
        "condition.first_removed_symbol",
        "chain.elif_indices",
        "scan_unsupported_expressions",
        "EditProofSource::removal_manifest_config",
    ] {
        assert!(
            cpp.contains(required),
            "src/source_scan/cpp.rs should fold only manifest-proven dead branches through {required}"
        );
    }

    for forbidden in ["content.replace(", "line.replace(", "std::fs::write(&path"] {
        assert!(
            !cpp.contains(forbidden),
            "CPP folding must not use broad whole-file replacement patterns; found {forbidden}"
        );
    }
}
