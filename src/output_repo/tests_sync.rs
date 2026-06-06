use super::*;

#[test]
fn test_sync_working_tree_is_incremental_for_unchanged_files() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");
    let snapshot = tmp.path().join("snapshot");

    std::fs::create_dir_all(output.join(".git")).unwrap();
    std::fs::create_dir_all(output.join("include")).unwrap();
    std::fs::create_dir_all(snapshot.join("include")).unwrap();

    std::fs::write(output.join("Makefile"), "old makefile\n").unwrap();
    std::fs::write(snapshot.join("Makefile"), "new makefile\n").unwrap();
    std::fs::write(output.join("include/Kconfig"), "# same\n").unwrap();
    std::fs::write(snapshot.join("include/Kconfig"), "# same\n").unwrap();

    let before = std::fs::metadata(output.join("include/Kconfig"))
        .unwrap()
        .modified()
        .unwrap();
    std::thread::sleep(Duration::from_secs(1));

    sync_working_tree(&output_repo_path(&output), &candidate_tree_path(&snapshot)).unwrap();

    let after = std::fs::metadata(output.join("include/Kconfig"))
        .unwrap()
        .modified()
        .unwrap();

    assert_eq!(after, before, "unchanged file should not be rewritten");
    assert_eq!(
        std::fs::read_to_string(output.join("Makefile")).unwrap(),
        "new makefile\n"
    );
}

#[test]
fn test_sync_working_tree_removes_deleted_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");
    let snapshot = tmp.path().join("snapshot");

    std::fs::create_dir_all(output.join(".git")).unwrap();
    std::fs::create_dir_all(output.join("drivers/old")).unwrap();
    std::fs::create_dir_all(snapshot.join("drivers")).unwrap();

    std::fs::write(output.join("drivers/old/stale.txt"), "stale\n").unwrap();
    std::fs::write(output.join("drivers/keep.txt"), "old\n").unwrap();
    std::fs::write(snapshot.join("drivers/keep.txt"), "new\n").unwrap();

    sync_working_tree(&output_repo_path(&output), &candidate_tree_path(&snapshot)).unwrap();

    assert!(!output.join("drivers/old").exists());
    assert_eq!(
        std::fs::read_to_string(output.join("drivers/keep.txt")).unwrap(),
        "new\n"
    );
}

#[test]
fn test_sync_working_tree_preserves_top_level_git_and_kslim() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");
    let snapshot = tmp.path().join("snapshot");

    std::fs::create_dir_all(output.join(".git")).unwrap();
    std::fs::create_dir_all(output.join(".kslim")).unwrap();
    std::fs::create_dir_all(snapshot.join(".kslim")).unwrap();
    std::fs::write(output.join(".git/config"), "existing git config\n").unwrap();
    std::fs::write(output.join(".kslim/local.txt"), "keep me\n").unwrap();
    std::fs::write(snapshot.join(".kslim/generated.txt"), "do not copy\n").unwrap();
    std::fs::write(snapshot.join("Makefile"), "new makefile\n").unwrap();

    sync_working_tree(&output_repo_path(&output), &candidate_tree_path(&snapshot)).unwrap();

    assert_eq!(
        std::fs::read_to_string(output.join(".git/config")).unwrap(),
        "existing git config\n"
    );
    assert_eq!(
        std::fs::read_to_string(output.join(".kslim/local.txt")).unwrap(),
        "keep me\n"
    );
    assert!(!output.join(".kslim/generated.txt").exists());
    assert_eq!(
        std::fs::read_to_string(output.join("Makefile")).unwrap(),
        "new makefile\n"
    );
}

#[test]
fn test_candidate_metadata_sync_does_not_write_published_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");
    let candidate = tmp.path().join("candidate");
    std::fs::create_dir_all(output.join(".git/kslim")).unwrap();
    std::fs::write(
        output.join(format!(".git/kslim/{}", PUBLISHED_SNAPSHOT_FILE)),
        "branch = \"existing-published\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(candidate.join(".kslim")).unwrap();
    std::fs::write(candidate.join(".kslim/base.toml"), "base_ref = \"v1.0\"\n").unwrap();
    std::fs::write(
        candidate.join(format!(".kslim/{}", PUBLISHED_SNAPSHOT_FILE)),
        "branch = \"candidate-must-not-publish\"\n",
    )
    .unwrap();

    sync_candidate_metadata_dir(&output_repo_path(&output), &candidate_tree_path(&candidate))
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(output.join(".git/kslim/base.toml")).unwrap(),
        "base_ref = \"v1.0\"\n"
    );
    assert_eq!(
        std::fs::read_to_string(output.join(format!(".git/kslim/{}", PUBLISHED_SNAPSHOT_FILE)))
            .unwrap(),
        "branch = \"existing-published\"\n"
    );
}
