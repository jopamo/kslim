use super::common::*;

#[test]
fn command_execution_delegates_reducer_semantics() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands = commands_source(root);
    let reducer = production_source(&root.join("src/reducer/mod.rs"));
    let pipeline = production_source(&root.join("src/reducer/pipeline.rs"));

    assert!(
        reducer.contains("run_reducer_for_profile"),
        "reducer/mod.rs should expose a profile-level reducer command boundary"
    );
    assert!(
        pipeline.contains("pub fn run_reducer_for_profile(")
            && pipeline.contains("profile.effective_removal_input()")
            && pipeline.contains("profile.effective_preservation_input()")
            && pipeline.contains("profile.effective_abi_policy()")
            && pipeline.contains("super::ensure_supported_fallout(&result.stats, &profile.reducer)?"),
        "reducer/pipeline.rs should own profile-to-reducer semantic normalization and strict fallout checks"
    );

    let reduce_tree = function_slice(&commands, "fn cmd_reduce_tree", "fn cmd_matrix");
    assert!(
        reduce_tree.contains("reducer::run_reducer_for_profile(&kernel_root, &profile)?"),
        "cmd_reduce_tree should delegate reducer semantics to the reducer module"
    );

    for forbidden in [
        "run_reducer_with_abi_policy",
        "ensure_supported_fallout",
        "ReducerResult::default",
        "effective_removal_input",
        "effective_preservation_input",
        "effective_abi_policy",
        "RemovalManifest",
        "prune::",
        "kconfig::",
        "kbuild::",
        "cpp::",
        "includes::",
        "fixups::",
        "tree_index",
    ] {
        assert!(
            !reduce_tree.contains(forbidden),
            "cmd_reduce_tree should orchestrate command execution without owning reducer semantics; found {forbidden}"
        );
    }

    for forbidden_import in [
        "use crate::prune",
        "use crate::kconfig",
        "use crate::kbuild",
        "use crate::cpp",
        "use crate::includes",
        "use crate::fixups",
        "use crate::removal_manifest",
        "use crate::tree_index",
    ] {
        assert!(
            !commands.contains(forbidden_import),
            "src/commands/* should not import reducer pass or semantic implementation modules; found {forbidden_import}"
        );
    }
}

fn function_slice<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let start_index = source
        .find(start)
        .unwrap_or_else(|| panic!("source should contain {start}"));
    let after_start = &source[start_index..];
    let end_index = after_start
        .find(end)
        .unwrap_or_else(|| panic!("source should contain {end} after {start}"));
    &after_start[..end_index]
}
