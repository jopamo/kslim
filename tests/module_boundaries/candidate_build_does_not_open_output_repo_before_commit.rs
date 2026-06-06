use super::common::*;

#[test]
fn candidate_build_does_not_open_output_repo_before_commit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = [
        production_source(&root.join("src/generate/candidate.rs")),
        production_source(&root.join("src/generate/candidate/errors.rs")),
        production_source(&root.join("src/generate/candidate/metadata.rs")),
        production_source(&root.join("src/generate/candidate/model.rs")),
        production_source(&root.join("src/generate/candidate/write.rs")),
    ]
    .join("\n");
    let candidate_model = production_source(&root.join("src/generate/candidate/model.rs"));

    for required in [
        "fn ensure_candidate_mutation_target(",
        "fn path_aliases_across_lifecycle(",
        "fn normalize_candidate_boundary_path(",
    ] {
        assert!(
            candidate_model.contains(required),
            "generate/candidate/model.rs should guard candidate/output path aliases without opening output; missing {required}"
        );
    }

    let forbidden_output_repo_access = [
        "init_output_repo",
        "require_managed",
        "require_clean",
        "require_not_detached",
        "sync_working_tree",
        "publish_output_candidate",
        "validate_output_candidate",
        "OutputRepoPath",
        "PublishedMetadataDir",
        "write_base_metadata",
        "write_generated_metadata",
        "write_patch_metadata",
        "write_report(",
        "write_published_snapshot_metadata",
        "load_authoritative_published_state",
        "load_committed_base_metadata",
        "load_committed_generated_metadata",
        "load_committed_published_snapshot_metadata",
        "crate::git",
        "git::",
        "commit_if_changed",
        "create_branch",
        "head_commit(",
        "current_branch(",
        "branch_exists(",
        "rev_parse(",
    ];

    for forbidden in forbidden_output_repo_access {
        assert!(
            !candidate.contains(forbidden),
            "generate/candidate.rs must not open output repo before commit; found forbidden token {forbidden}"
        );
    }
}
