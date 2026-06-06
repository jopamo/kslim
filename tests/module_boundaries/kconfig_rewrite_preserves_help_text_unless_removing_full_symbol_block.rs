use super::common::*;

#[test]
fn kconfig_rewrite_preserves_help_text_unless_removing_full_symbol_block() {
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
        "fn kconfig_help_text_mask",
        "fn is_kconfig_help_directive",
        "let help_text = kconfig_help_text_mask(&lines);",
        "if help_text[idx]",
        "if !help_text[idx]",
        "is_kconfig_help_directive(trimmed)",
        "join_lines(&lines[proof.start_line - 1..proof.end_line])",
    ] {
        assert!(
            kconfig.contains(required),
            "Kconfig rewrites should preserve help text for line-level rewrites and remove it only with full blocks; missing {required}"
        );
    }

    for required in [
        "test_kconfig_rewrites_preserve_help_text_unless_removing_full_symbol_block",
        "depends on REMOVED || LIVE",
        "source \\\"drivers/missing/Kconfig\\\"",
        "# kslim: removed config REMOVE_BLOCK",
        "!rewritten.contains(\"remove help\")",
    ] {
        assert!(
            kconfig_with_tests.contains(required),
            "unit tests should pin help text preservation; missing {required}"
        );
    }

    for docs in [architecture, kernel_build] {
        assert!(
            docs.contains("Kconfig help text is preserved unless removing a full symbol block"),
            "docs should describe help text preservation"
        );
    }
}
