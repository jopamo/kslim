use super::common::*;

#[test]
fn kbuild_unit_tests_live_beside_owned_modules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kbuild = std::fs::read_to_string(root.join("src/kbuild/mod.rs"))
        .expect("failed to read src/kbuild/mod.rs");
    let tests = production_source(&root.join("src/kbuild/tests.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        kbuild.contains("#[cfg(test)]\nmod tests;"),
        "src/kbuild/mod.rs should register a sibling Kbuild tests module"
    );
    assert!(
        !kbuild.contains("mod tests {"),
        "src/kbuild/mod.rs should not keep inline Kbuild unit tests"
    );

    for required in [
        "use super::*;",
        "test_logical_lines_joins_backslash_continuations",
        "test_parse_kbuild_assignment_forms",
        "test_build_kbuild_index_collects_providers_refs_dirs_gates_and_include_flags",
        "test_rewrite_makefiles_removes_refs_to_removed_directories",
        "test_rewrite_makefiles_preserves_multiline_assignment_layout",
        "test_rewrite_makefiles_reports_ambiguous_live_ccflags_include_paths",
        "test_rewrite_makefiles_is_idempotent",
    ] {
        assert!(
            tests.contains(required),
            "src/kbuild/tests.rs should carry Kbuild unit test coverage through {required}"
        );
    }

    for required in ["`src/kbuild/tests.rs`", "Kbuild unit tests"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document Kbuild test module ownership through {required}"
        );
    }
}
