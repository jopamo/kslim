use super::common::*;

#[test]
fn reducer_mod_is_module_facade_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let reducer_full = std::fs::read_to_string(root.join("src/reducer/mod.rs"))
        .expect("failed to read src/reducer/mod.rs");
    let reducer = production_source(&root.join("src/reducer/mod.rs"));
    let tests_root = production_source(&root.join("src/reducer/tests.rs"));
    let tests = production_sources(
        &root,
        &[
            "src/reducer/tests.rs",
            "src/reducer/tests_cpp_include.rs",
            "src/reducer/tests_fixups.rs",
            "src/reducer/tests_pipeline.rs",
            "src/reducer/tests_result_serialization.rs",
            "src/reducer/tests_syntax.rs",
        ],
    );
    let cpp_include_tests = production_source(&root.join("src/reducer/tests_cpp_include.rs"));
    let fixup_tests = production_source(&root.join("src/reducer/tests_fixups.rs"));
    let pipeline_tests = production_source(&root.join("src/reducer/tests_pipeline.rs"));
    let result_tests =
        production_source(&root.join("src/reducer/tests_result_serialization.rs"));
    let syntax_tests = production_source(&root.join("src/reducer/tests_syntax.rs"));

    for required in [
        "mod actions;",
        "mod context;",
        "mod diagnostics;",
        "mod engine;",
        "mod pipeline;",
        "mod report;",
        "mod result;",
        "mod stage;",
        "mod state;",
    ] {
        assert!(
            reducer.contains(required),
            "src/reducer/mod.rs should register reducer module {required}"
        );
    }

    for required in [
        "pub use actions::apply_selftest_fixup;",
        "pub use engine::run_fixed_point_loop;",
        "pub use pipeline::{",
        "pub use report::ensure_supported_fallout;",
        "pub use result::{",
        "pub(crate) use state::ReducerAttemptState;",
    ] {
        assert!(
            reducer.contains(required),
            "src/reducer/mod.rs should keep reducer facade re-export {required}"
        );
    }

    for forbidden in [
        "\nfn ",
        "\npub fn ",
        "\npub(crate) fn ",
        "\nstruct ",
        "\npub struct ",
        "\nenum ",
        "\npub enum ",
        "\nimpl ",
        "\ntrait ",
        "\npub trait ",
    ] {
        assert!(
            !reducer.contains(forbidden),
            "src/reducer/mod.rs must be a module/re-export facade only; found {forbidden:?}"
        );
    }

    assert!(
        reducer_full.contains("#[cfg(test)]\nmod tests;"),
        "src/reducer/mod.rs should register reducer unit tests as an external test module"
    );
    for required in [
        "#[path = \"tests_cpp_include.rs\"]\nmod cpp_include;",
        "#[path = \"tests_fixups.rs\"]\nmod fixups;",
        "#[path = \"tests_pipeline.rs\"]\nmod pipeline;",
        "#[path = \"tests_result_serialization.rs\"]\nmod result_serialization;",
        "#[path = \"tests_syntax.rs\"]\nmod syntax;",
    ] {
        assert!(
            tests_root.contains(required),
            "src/reducer/tests.rs should register behavior-focused reducer test module {required}"
        );
    }
    assert!(
        tests.contains("test_reducer_run_is_noop_without_slim_input")
            && tests.contains("test_run_reducer_entrypoint_executes_manifest_driven_pipeline")
            && tests.contains("test_reducer_run_rewrites_file_relative_quoted_private_header_include"),
        "src/reducer/tests.rs should own the former reducer root unit tests"
    );
    assert!(
        result_tests.contains("test_reducer_result_serializes_stable_public_shape")
            && result_tests.contains("test_reducer_result_committed_serialization_redacts_host_paths"),
        "src/reducer/tests_result_serialization.rs should own reducer result serialization tests"
    );
    assert!(
        pipeline_tests.contains("test_run_reducer_entrypoint_executes_manifest_driven_pipeline")
            && pipeline_tests.contains("test_reducer_rerun_on_already_reduced_tree_converges_to_zero_edits"),
        "src/reducer/tests_pipeline.rs should own reducer pipeline/convergence tests"
    );
    assert!(
        syntax_tests.contains("test_reducer_run_fails_closed_on_unsupported_kconfig_expression_by_default")
            && syntax_tests.contains("test_render_unsupported_expression_report_sorts_sites_by_stable_keys"),
        "src/reducer/tests_syntax.rs should own Kconfig syntax/report behavior tests"
    );
    assert!(
        cpp_include_tests.contains("test_reducer_run_fails_closed_on_unsupported_cpp_expression_by_default")
            && cpp_include_tests.contains("test_reducer_run_rewrites_file_relative_quoted_private_header_include"),
        "src/reducer/tests_cpp_include.rs should own CPP folding and include cleanup behavior tests"
    );
    assert!(
        fixup_tests.contains("test_strict_reducer_edit_provenance_rejects_unreasoned_edit")
            && fixup_tests.contains("test_apply_selftest_fixup_removes_proven_missing_kconfig_source"),
        "src/reducer/tests_fixups.rs should own edit-provenance and selftest-fixup behavior tests"
    );
    assert!(
        !reducer_full.contains("mod tests {"),
        "src/reducer/mod.rs must not keep an inline test module"
    );
}
