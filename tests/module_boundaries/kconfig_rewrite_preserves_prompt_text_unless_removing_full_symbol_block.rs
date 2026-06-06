use super::common::*;

#[test]
fn kconfig_rewrite_preserves_prompt_text_unless_removing_full_symbol_block() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(&root, &["src/kconfig/mod.rs", "src/kconfig/rewrite.rs"]);
    let kconfig_with_tests = production_sources(
        &root,
        &[
            "src/kconfig/tests.rs",
            "src/kconfig/tests_report.rs",
            "src/kconfig/tests_rewrite.rs",
            "src/kconfig/tests_root_facade.rs",
            "src/kconfig/tests_solver.rs",
        ],
    );
    let architecture =
        std::fs::read_to_string(root.join("docs/architecture.md")).expect("failed to read docs");
    let kernel_build = kernel_build_iteration_docs(&root);

    for required in [
        "fn kconfig_prompt_text_line",
        "kconfig_prompt_text_line(directive_text)",
        "strip_kconfig_keyword(trimmed, \"prompt\")",
        "\"bool\", \"tristate\", \"int\", \"hex\", \"string\"",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig relation rewrites should preserve prompt text lines; missing {required}"
        );
    }

    for required in [
        "test_rewrite_kconfig_relations_preserves_prompt_text_unless_removing_full_symbol_block",
        "bool \\\"Live prompt\\\" if REMOVED || LIVE",
        "prompt \\\"Explicit prompt\\\" if REMOVED",
        "# kslim: removed config REMOVE_BLOCK",
    ] {
        assert!(
            kconfig_with_tests.contains(required),
            "unit tests should pin prompt text preservation; missing {required}"
        );
    }

    for docs in [architecture, kernel_build] {
        assert!(
            docs.contains(
                "Kconfig prompt text is preserved unless removing a full symbol block"
            ),
            "docs should describe prompt text preservation"
        );
    }
}
