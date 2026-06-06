use super::common::*;

#[test]
fn authoritative_lockfile_update_is_final_publication_step() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let generate = production_sources(
        &root,
        &["src/generate.rs", "src/generate/orchestration.rs"],
    );
    let publish = production_source(&root.join("src/generate/publish.rs"));

    let success_cleanup = generate
        .find("clear_project_failure_artifacts(project_root.as_path())")
        .expect("generate success should clear attempt metadata before final lockfile update");
    let generate_lockfile_call = generate
        .find("if let Err(err) = write_authoritative_lockfile(")
        .expect("generate success should write authoritative lockfile");
    assert!(
        success_cleanup < generate_lockfile_call,
        "non-authoritative success cleanup must happen before final kslim.lock update"
    );

    let generate_failure_rollback = generate
        .find("if result.is_err()")
        .expect("generate should check failure rollback after success finalization");
    let after_generate_lockfile = &generate[generate_lockfile_call..generate_failure_rollback];
    for forbidden_after_lockfile in [
        "clear_project_failure_artifacts",
        "write_project_last_attempt",
        "write_project_reducer_failure_report",
        "record_generate_failure",
        "std::fs::write",
        "commit_output_repo_state(",
        "write_output_metadata_report_and_manifest",
        "write_verified_published_snapshot_metadata",
        "write_verified_committed_published_snapshot_metadata",
        "write_reducer_metadata_at_dir_with_context",
    ] {
        assert!(
            !after_generate_lockfile.contains(forbidden_after_lockfile),
            "generate must not perform publication or cleanup writes after kslim.lock finalization; found {forbidden_after_lockfile}"
        );
    }

    let publish_commit = publish
        .find("crate::git::commit_if_changed")
        .expect("publish should commit output before lockfile");
    let published_metadata_read = publish
        .find("output_repo::load_committed_published_snapshot_metadata")
        .expect("publish should consume committed published metadata before lockfile");
    let publish_lockfile = publish
        .find("lockfile::write_published_lockfile(&lockfile_path, &update)")
        .expect("publish should write authoritative lockfile");
    assert!(
        publish_commit < published_metadata_read && published_metadata_read < publish_lockfile,
        "publish must commit output and consume committed metadata before final kslim.lock update"
    );

    let publish_lockfile_function_end = publish
        .find("\nfn ensure_output_repo_matches_plan")
        .expect("publish lockfile function should end before output-repo matching helper");
    let after_publish_lockfile = &publish[publish_lockfile..publish_lockfile_function_end];
    for forbidden_after_lockfile in [
        "crate::git::commit_if_changed",
        "crate::git::create_branch",
        "crate::git::head_commit",
        "output_repo::sync_working_tree",
        "output_repo::sync_candidate_metadata_dir",
        "output_repo::sync_candidate_committed_metadata_dir",
        "write_verified_published_snapshot_metadata",
        "write_verified_committed_published_snapshot_metadata",
    ] {
        assert!(
            !after_publish_lockfile.contains(forbidden_after_lockfile),
            "publish must not perform publication writes after kslim.lock finalization; found {forbidden_after_lockfile}"
        );
    }
}
