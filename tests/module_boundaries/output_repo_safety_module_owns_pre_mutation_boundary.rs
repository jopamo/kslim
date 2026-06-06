use super::common::*;

#[test]
fn output_repo_safety_module_owns_pre_mutation_boundary() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output_repo = production_source(&root.join("src/output_repo.rs"));
    let safety = production_source(&root.join("src/output_repo/safety.rs"));

    for required in [
        "pub(crate) struct OutputRepoSafety",
        "pub(crate) fn check_output_repo_safety",
        "repo: &OutputRepoPath",
        "expected: &OutputPlan",
        "branch: OutputBranchName",
        "head: Option<GitCommitId>",
        "clean: bool",
        "metadata_consistent: bool",
        "lockfile_consistent: bool",
        "repo_exists: bool",
        "git_dir_exists: bool",
        "is_git_worktree: bool",
        "current_branch: String",
        "current_head: Option<String>",
        "branch_matches_expected: bool",
        "tracked_tree_clean: bool",
        "untracked_paths: Vec<PathBuf>",
        "metadata_dir_sane: bool",
        "published_metadata_present: bool",
        "published_metadata_commit: Option<String>",
        "published_metadata_consistent: bool",
        "lockfile_path: Option<LockfilePath>",
        "lockfile_present: bool",
        "lockfile_published_snapshot_present: bool",
    ] {
        assert!(
            safety.contains(required),
            "output_repo/safety.rs should own output pre-mutation safety item {required}"
        );
    }

    assert!(
        output_repo.contains("mod safety;"),
        "output_repo.rs should register the output repo safety module"
    );
    assert!(
        !output_repo.contains("fn check_output_repo_safety"),
        "output_repo.rs should not retain output safety implementation"
    );

    let forbidden_mutation_publication_or_resolution = [
        "std::fs::write",
        "std::fs::remove",
        "std::fs::rename",
        "std::fs::copy",
        "create_dir",
        "sync_working_tree",
        "sync_candidate_metadata_dir",
        "publish_output_candidate",
        "write_base_metadata",
        "write_generated_metadata",
        "write_published_snapshot_metadata",
        "write_resolved_base_lockfile",
        "write_published_lockfile",
        "crate::upstream",
        "resolve_ref",
        "KslimConfig",
        "ProfileConfig",
    ];

    for forbidden in forbidden_mutation_publication_or_resolution {
        assert!(
            !safety.contains(forbidden),
            "output_repo/safety.rs must stay read-only and policy-free; found forbidden token {forbidden}"
        );
    }

    let deferred_override_modes = ["force", "repair", "allow_dirty", "ignore_dirty"];
    for deferred in deferred_override_modes {
        assert!(
            !safety.contains(deferred),
            "output_repo/safety.rs must not grow implicit {deferred} bypasses; add explicit force/repair mode only later"
        );
    }
}
