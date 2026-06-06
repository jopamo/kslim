use super::common::*;

#[test]
fn output_repo_naming_module_owns_stable_output_names() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output_repo = production_source(&root.join("src/output_repo.rs"));
    let naming = production_source(&root.join("src/output_repo/naming.rs"));

    assert!(
        output_repo.contains("mod naming;"),
        "output_repo.rs should register the stable output naming module"
    );

    for required in [
        "pub fn branch_name",
        "pub fn tag_name",
        "pub(crate) fn initial_branch",
        "pub fn snapshot_id",
        "pub fn commit_message",
        "COMMIT_SUBJECT_IMPORT_PREFIX",
        "COMMIT_SECTION_UPSTREAM",
        "COMMIT_SECTION_BASE_REF",
        "COMMIT_SECTION_BASE_COMMIT",
        "COMMIT_SECTION_PROFILE",
        "COMMIT_SECTION_MODE",
        "COMMIT_SECTION_PLAN_FINGERPRINT",
        "COMMIT_SECTION_REDUCER_SUMMARY",
        "COMMIT_SECTION_SELFTEST_SUMMARY",
        "COMMIT_MESSAGE_HOST_PATH_REDACTION",
        "pub(crate) fn sanitize_commit_message_value",
        "pub struct CommitMessageDetails",
        "STABLE_METADATA_FILE_NAMES",
        "metadata::BASE_METADATA_FILE",
        "report::REDUCER_REPORT_JSON",
    ] {
        assert!(
            naming.contains(required),
            "output_repo/naming.rs should own stable output naming item {required}"
        );
    }

    for moved_item in [
        "pub fn branch_name",
        "pub fn tag_name",
        "fn initial_branch",
        "pub fn commit_message",
    ] {
        assert!(
            !output_repo.contains(moved_item),
            "output_repo.rs should not retain stable output naming implementation {moved_item}"
        );
    }

    let forbidden_runtime_or_workspace_dependencies = [
        "std::fs::",
        "crate::git",
        "crate::process",
        "crate::generate",
        "tempfile",
        "CandidateTreePath",
        "OutputRepoPath",
        "PathBuf",
    ];

    for forbidden in forbidden_runtime_or_workspace_dependencies {
        assert!(
            !naming.contains(forbidden),
            "output_repo/naming.rs must stay limited to deterministic names; found forbidden token {forbidden}"
        );
    }
}
