use super::common::*;

#[test]
fn publish_command_consumes_committed_output_metadata_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cli = cli_sources(root);
    let publish = production_source(&root.join("src/publish.rs"));
    let metadata = production_source(&root.join("src/output_repo/metadata.rs"));
    let publish_args = cli
        .split("pub struct PublishArgs")
        .nth(1)
        .and_then(|rest| rest.split("// ── Compare").next())
        .expect("src/cli/* should define PublishArgs");

    for forbidden_cli_override in ["profile", "base", "upstream", "candidate"] {
        assert!(
            !publish_args.contains(forbidden_cli_override),
            "publish CLI must not expose generate/candidate override {forbidden_cli_override}"
        );
    }

    for required in [
        "struct CommittedPublishState",
        "fn load(request: &PublishRequest)",
        "output_repo::load_authoritative_published_state",
        "crate::git::push(output_path, remote_name, &published.branch)",
        "crate::git::create_tag(output_path, &published.tag",
    ] {
        assert!(
            publish.contains(required),
            "publish.rs should consume committed authoritative state through {required}"
        );
    }

    for forbidden in [
        "let current_branch =",
        "read_candidate_metadata",
        "read_published_metadata",
        "candidate_metadata_dir",
        "published_metadata_dir",
        "load_committed_base_metadata",
        "load_committed_generated_metadata",
        "load_committed_published_snapshot_metadata",
        "write_verified_published_snapshot_metadata",
        "verify_candidate",
        "crate::upstream",
        "upstream::",
        "resolve_ref",
        "check_access",
        "ref_timestamp",
        "load_profile",
        "require_known_profile",
        "config::",
        "GenerateOptions",
        "GeneratePlan",
        "CandidateMetadataDir",
        "CandidateTreePath",
        "cli_overrides",
        "selected_profile",
        "base_ref",
        ".git/kslim",
        ".kslim/published.toml",
    ] {
        assert!(
            !publish.contains(forbidden),
            "publish.rs must not consume candidate/private/worktree metadata directly; found {forbidden}"
        );
    }

    let authoritative_loader = metadata
        .split("pub(crate) fn load_authoritative_published_state")
        .nth(1)
        .and_then(|rest| {
            rest.split("fn ensure_no_committed_published_metadata_without_lockfile")
                .next()
        })
        .expect("metadata.rs should define authoritative published-state loader");
    for required_committed_read in [
        "let current_commit = crate::git::head_commit(output_path)?;",
        "load_committed_base_metadata(output_repo, &current_commit)?",
        "load_committed_generated_metadata(output_repo, &current_commit)?",
        "load_committed_published_snapshot_metadata(output_repo, &current_commit)?",
    ] {
        assert!(
            authoritative_loader.contains(required_committed_read),
            "authoritative publish state must be derived from committed output HEAD metadata; missing {required_committed_read}"
        );
    }
    for forbidden_uncommitted_read in [
        "std::fs::read_to_string",
        "read_published_metadata",
        "published_metadata_dir",
        "read_candidate_metadata",
        "candidate_metadata_dir",
    ] {
        assert!(
            !authoritative_loader.contains(forbidden_uncommitted_read),
            "authoritative publish state must not read uncommitted output or candidate metadata; found {forbidden_uncommitted_read}"
        );
    }

    let committed_blob_reader = metadata
        .split("fn read_committed_metadata_blob")
        .nth(1)
        .and_then(|rest| rest.split("fn output_repo_path_str").next())
        .expect("metadata.rs should define committed metadata blob reader");
    for required_git_blob_read in [
        "crate::process::run_in_dir",
        "\"git\"",
        "\"show\"",
        "format!(\"{}:{}\", commit, metadata_ref)",
    ] {
        assert!(
            committed_blob_reader.contains(required_git_blob_read),
            "committed metadata must be loaded from git object storage, not the worktree; missing {required_git_blob_read}"
        );
    }
}
