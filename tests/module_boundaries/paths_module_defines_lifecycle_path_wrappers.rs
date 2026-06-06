use super::common::*;

#[test]
fn paths_module_defines_lifecycle_path_wrappers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let paths_mod = std::fs::read_to_string(root.join("src/paths/mod.rs"))
        .expect("failed to read src/paths/mod.rs");
    let path_mod = std::fs::read_to_string(root.join("src/path/mod.rs"))
        .expect("failed to read src/path/mod.rs");
    let typed = production_source(&root.join("src/path/typed.rs"));
    let normalization = production_source(&root.join("src/path/normalization.rs"));
    let traversal = production_source(&root.join("src/path/traversal.rs"));
    let display = production_source(&root.join("src/path/display.rs"));
    let tests = production_source(&root.join("src/path/tests.rs"));
    let state = state_source(root);

    assert!(
        main.contains("mod path;") && main.contains("mod paths;"),
        "main.rs should register the owned path module and legacy paths facade"
    );
    assert!(
        !root.join("src/path.rs").exists(),
        "src/path ownership should live in src/path/mod.rs, not a duplicate src/path.rs root"
    );
    for module in [
        "mod display;",
        "mod normalization;",
        "mod traversal;",
        "mod typed;",
        "#[cfg(test)]\nmod tests;",
    ] {
        assert!(
            path_mod.contains(module),
            "src/path/mod.rs should register path submodule {module}"
        );
    }
    assert!(
        paths_mod.contains("pub(crate) use crate::path::{")
            && !paths_mod.contains("mod typed;")
            && !paths_mod.contains("#[cfg(test)]\nmod tests;"),
        "src/paths/mod.rs should remain a compatibility facade over src/path/* ownership"
    );

    for wrapper in [
        "RequestedConfigPath",
        "WorkspaceRoot",
        "CandidateTreePath",
        "CandidateMetadataDir",
        "AttemptMetadataDir",
        "OutputRepoPath",
        "OutputCandidateArea",
        "PublishedMetadataDir",
        "LockfilePath",
        "KernelSourceRoot",
        "KernelBuildDir",
        "RelativeKernelPath",
    ] {
        assert!(
            typed.contains(&format!("pub(crate) struct {wrapper}(PathBuf);")),
            "src/path/typed.rs should define typed lifecycle path wrapper {wrapper}"
        );
        assert!(
            path_mod.contains(wrapper) && paths_mod.contains(wrapper),
            "src/path/mod.rs and src/paths/mod.rs should re-export typed lifecycle path wrapper {wrapper}"
        );
    }

    for forbidden in [
        "impl From<PathBuf>",
        "impl From<std::path::PathBuf>",
        "impl From<&Path>",
        "impl From<&std::path::Path>",
    ] {
        assert!(
            !typed.contains(forbidden),
            "src/path/typed.rs must not expose broad path conversion {forbidden}; use validated constructors instead"
        );
    }

    let output_candidate_area_impl = typed
        .split("impl OutputCandidateArea")
        .nth(1)
        .and_then(|rest| {
            rest.split("#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]")
                .next()
        })
        .expect("src/path/typed.rs should define OutputCandidateArea implementation");
    assert!(
        output_candidate_area_impl.contains("pub(crate) fn from_output_repo"),
        "OutputCandidateArea should be publicly constructible only from an OutputRepoPath boundary"
    );
    assert!(
        !output_candidate_area_impl.contains("pub(crate) fn new(path"),
        "OutputCandidateArea must not expose a raw path constructor"
    );

    let published_metadata_dir_impl = typed
        .split("impl PublishedMetadataDir")
        .nth(1)
        .and_then(|rest| {
            rest.split("#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]")
                .next()
        })
        .expect("src/path/typed.rs should define PublishedMetadataDir implementation");
    assert!(
        published_metadata_dir_impl.contains("pub(crate) fn new_in_output_repo"),
        "PublishedMetadataDir should be publicly constructible only from an OutputRepoPath boundary"
    );
    assert!(
        published_metadata_dir_impl.contains("pub(crate) fn new_committed_tree_in_output_repo"),
        "PublishedMetadataDir committed tree metadata should also require an OutputRepoPath boundary"
    );
    for forbidden_raw_constructor in [
        "pub(crate) fn new(path",
        "pub(crate) fn new_committed_metadata_dir",
    ] {
        assert!(
            !published_metadata_dir_impl.contains(forbidden_raw_constructor),
            "PublishedMetadataDir must not expose raw path constructors; found {forbidden_raw_constructor}"
        );
    }

    for required in [
        "pub(crate) fn normalize_path_without_parent_components",
        "pub(crate) fn canonicalize_existing_path",
        "pub(crate) fn reject_empty_path",
        "Component::CurDir",
        "PathBuf::from(\".\")",
    ] {
        assert!(
            normalization.contains(required),
            "src/path/normalization.rs should own normalization helper {required}"
        );
    }
    assert!(
        typed.contains("use super::normalization::{"),
        "typed lifecycle path constructors should consume normalization helpers from src/path/normalization.rs"
    );

    for required in [
        "pub(crate) fn reject_parent_traversal",
        "pub(crate) fn reject_absolute_like_relative_kernel_path",
        "path_contains_parent_traversal",
        "path_is_absolute_like",
    ] {
        assert!(
            traversal.contains(required),
            "src/path/traversal.rs should own traversal/host-root rejection helper {required}"
        );
    }
    assert!(
        typed.contains("use super::traversal::{")
            && normalization.contains("use super::traversal::{"),
        "typed path construction and normalization should call traversal rejection helpers through src/path/traversal.rs"
    );

    assert!(
        display.contains("pub(crate) fn path_to_config_string")
            && typed.contains("path_to_config_string(self.as_path())"),
        "src/path/display.rs should own display normalization used by RelativeKernelPath"
    );
    assert!(
        tests.contains("test_lifecycle_path_wrappers_implement_as_ref_path")
            && tests.contains("test_validated_lifecycle_path_constructors_reject_root_escape_attempts"),
        "src/path/tests.rs should keep lifecycle path unit tests beside the split path modules"
    );
    assert!(
        !typed.contains("#[cfg(test)]"),
        "src/path/typed.rs should not retain the old module-local test block"
    );

    assert!(
        state.contains("use crate::paths::{"),
        "generate/state.rs should consume lifecycle path wrappers from the paths module"
    );
    assert!(
        state.contains("pub(crate) output_path: OutputRepoPath"),
        "generate/state.rs should store resolved output targets as OutputRepoPath"
    );
    assert!(
        state.contains("pub(crate) lockfile_path: Option<LockfilePath>"),
        "generate/state.rs should store authoritative lockfile paths as LockfilePath"
    );
    assert!(
        state.contains("pub(crate) output_dir: Option<KernelBuildDir>"),
        "generate/state.rs should store kernel build output dirs as KernelBuildDir"
    );
    assert!(
        state.contains("pub(crate) remove_paths: Vec<RelativeKernelPath>"),
        "generate/state.rs should store resolved prune paths as RelativeKernelPath"
    );
    let reducer_pipeline = production_source(&root.join("src/reducer/pipeline.rs"));
    assert!(
        reducer_pipeline.contains("root: &KernelSourceRoot"),
        "reducer pipeline should accept kernel source roots through KernelSourceRoot"
    );

    let path_sources = [path_mod, typed, normalization, traversal, display].join("\n");
    for forbidden in [
        "crate::commands",
        "crate::reducer",
        "crate::generate",
        "crate::kconfig",
        "crate::kbuild",
        "crate::output_repo",
        "crate::tree_index",
    ] {
        assert!(
            !path_sources.contains(forbidden),
            "src/path/* should not depend on command, reducer, output, or kernel-domain modules; found {forbidden}"
        );
    }
}
