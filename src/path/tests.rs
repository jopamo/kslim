//! Unit tests for lifecycle path wrappers.

use super::*;
use anyhow::Result;
use std::path::{Path, PathBuf};

fn assert_as_ref_path<T: AsRef<Path>>(path: &T, expected: &Path) {
    assert_eq!(path.as_ref(), expected);
}

fn assert_empty_path_error<T>(result: Result<T>, expected: &str) {
    match result {
        Ok(_) => panic!("expected empty path rejection containing {expected:?}"),
        Err(err) => {
            let err = err.to_string();
            assert!(
                err.contains(expected),
                "unexpected empty path error, expected {expected:?}: {err}"
            );
        }
    }
}

fn assert_normalized_path<T: AsRef<Path>>(result: Result<T>, expected: &Path) {
    let path = result.unwrap_or_else(|err| {
        panic!(
            "expected path to normalize to {}, got error: {err}",
            expected.display()
        )
    });
    assert_eq!(path.as_ref(), expected);
}

#[test]
fn test_lifecycle_path_wrappers_implement_as_ref_path() {
    assert_as_ref_path(
        &RequestedConfigPath::new("/tmp/project/kslim.toml").unwrap(),
        Path::new("/tmp/project/kslim.toml"),
    );
    assert_as_ref_path(
        &WorkspaceRoot::new("/tmp/kslim-workspace").unwrap(),
        Path::new("/tmp/kslim-workspace"),
    );
    assert_as_ref_path(
        &CandidateTreePath::new("/tmp/kslim-candidate").unwrap(),
        Path::new("/tmp/kslim-candidate"),
    );
    assert_as_ref_path(
        &KernelSourceRoot::new("/tmp/linux").unwrap(),
        Path::new("/tmp/linux"),
    );
    assert_as_ref_path(
        &KernelBuildDir::new("/tmp/linux/.kslim-selftest/build-1").unwrap(),
        Path::new("/tmp/linux/.kslim-selftest/build-1"),
    );
    assert_as_ref_path(
        &RelativeKernelPath::new("drivers/foo.c").unwrap(),
        Path::new("drivers/foo.c"),
    );
    assert_as_ref_path(
        &CandidateMetadataDir::new("/tmp/kslim-candidate/.kslim").unwrap(),
        Path::new("/tmp/kslim-candidate/.kslim"),
    );
    assert_as_ref_path(
        &AttemptMetadataDir::new("/tmp/project/.kslim/attempt").unwrap(),
        Path::new("/tmp/project/.kslim/attempt"),
    );
    assert_as_ref_path(
        &OutputRepoPath::new("/tmp/output").unwrap(),
        Path::new("/tmp/output"),
    );
    let output_candidate_repo = OutputRepoPath::new("/tmp/output-candidate").unwrap();
    assert_as_ref_path(
        &OutputCandidateArea::from_output_repo(&output_candidate_repo).unwrap(),
        Path::new("/tmp/output-candidate"),
    );
    assert_as_ref_path(
        &PublishedMetadataDir::new("/tmp/output/.git/kslim").unwrap(),
        Path::new("/tmp/output/.git/kslim"),
    );
    assert_as_ref_path(
        &LockfilePath::new("/tmp/project/kslim.lock").unwrap(),
        Path::new("/tmp/project/kslim.lock"),
    );
}

#[test]
fn test_validated_lifecycle_path_constructors_accept_expected_shapes() {
    let tmp = tempfile::tempdir().unwrap();
    let requested_config = tmp.path().join("kslim.toml");
    std::fs::write(&requested_config, "[project]\n").unwrap();

    let workspace_root = tempfile::tempdir().unwrap();
    let candidate_tree = tempfile::tempdir().unwrap();
    let kernel_source = tmp.path().join("linux");
    let absolute_kernel_build = tmp.path().join("build").join("tiny");
    let project = tmp.path().join("project");
    let attempt_dir = tmp.path().join(".kslim").join("attempt");
    let output = tmp.path().join("output");
    let tree_output = tmp.path().join("tree-output");
    std::fs::create_dir_all(&kernel_source).unwrap();
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(output.join(".git")).unwrap();

    let requested = RequestedConfigPath::new_existing_file(&requested_config).unwrap();
    let workspace = WorkspaceRoot::new_temp_workspace(workspace_root.path()).unwrap();
    let candidate = CandidateTreePath::new_temp_tree(candidate_tree.path()).unwrap();
    let kernel_root = KernelSourceRoot::new_existing_dir(&kernel_source).unwrap();
    let relative_kernel_build =
        KernelBuildDir::new_for_source_root(&kernel_root, ".kslim-selftest/tiny").unwrap();
    let absolute_kernel_build =
        KernelBuildDir::new_for_source_root(&kernel_root, &absolute_kernel_build).unwrap();
    let relative_kernel_path = RelativeKernelPath::new("drivers/foo.c").unwrap();
    let candidate_metadata = CandidateMetadataDir::new_in_candidate_tree(
        &candidate,
        candidate.as_path().join(".kslim"),
    )
    .unwrap();
    let attempt = AttemptMetadataDir::new(&attempt_dir).unwrap();
    let output_repo = OutputRepoPath::new_git_worktree(&output).unwrap();
    let tree_output_repo = OutputRepoPath::new(&tree_output).unwrap();
    let output_candidate_area = OutputCandidateArea::from_output_repo(&output_repo).unwrap();
    let git_metadata =
        PublishedMetadataDir::new_in_output_repo(&output_repo, output.join(".git/kslim"))
            .unwrap();
    let tree_metadata =
        PublishedMetadataDir::new_in_output_repo(&tree_output_repo, tree_output.join(".kslim"))
            .unwrap();
    let lockfile = LockfilePath::new_in_project_root(&project).unwrap();

    assert_eq!(
        requested.as_path(),
        requested_config.canonicalize().unwrap().as_path()
    );
    assert_eq!(
        workspace.as_path(),
        workspace_root.path().canonicalize().unwrap().as_path()
    );
    let expected_workspace_tree = workspace_root.path().canonicalize().unwrap().join("tree");
    assert_eq!(
        workspace.candidate_tree_path().as_path(),
        expected_workspace_tree.as_path()
    );
    assert_eq!(
        candidate.as_path(),
        candidate_tree.path().canonicalize().unwrap().as_path()
    );
    assert_eq!(
        kernel_root.as_path(),
        kernel_source.canonicalize().unwrap().as_path()
    );
    assert_eq!(
        relative_kernel_build.as_path(),
        kernel_source.join(".kslim-selftest/tiny").as_path()
    );
    assert_eq!(
        absolute_kernel_build.as_path(),
        tmp.path().join("build/tiny").as_path()
    );
    assert_eq!(relative_kernel_path.as_path(), Path::new("drivers/foo.c"));
    assert_eq!(
        candidate_metadata.as_path(),
        candidate_tree
            .path()
            .canonicalize()
            .unwrap()
            .join(".kslim")
            .as_path()
    );
    assert_eq!(attempt.as_path(), attempt_dir.as_path());
    assert_eq!(
        output_repo.as_path(),
        output.canonicalize().unwrap().as_path()
    );
    assert_eq!(output_candidate_area.as_path(), output.as_path());
    assert_eq!(git_metadata.as_path(), output.join(".git/kslim").as_path());
    assert_eq!(
        tree_metadata.as_path(),
        tree_output.join(".kslim").as_path()
    );
    assert_eq!(lockfile.as_path(), project.join("kslim.lock").as_path());
}

#[test]
fn test_validated_lifecycle_path_constructors_normalize_existing_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    let scratch = tmp.path().join("scratch");
    std::fs::create_dir_all(project.join("config")).unwrap();
    std::fs::create_dir_all(&scratch).unwrap();
    std::fs::create_dir_all(tmp.path().join("workspace")).unwrap();
    std::fs::create_dir_all(tmp.path().join("kernel")).unwrap();

    let requested_config = project.join("config/kslim.toml");
    std::fs::write(&requested_config, "[project]\n").unwrap();
    let requested =
        RequestedConfigPath::new_existing_file(scratch.join("../project/config/./kslim.toml"))
            .unwrap();
    assert_eq!(
        requested.as_path(),
        requested_config.canonicalize().unwrap().as_path()
    );

    let workspace = WorkspaceRoot::new_existing_dir(scratch.join("../workspace/./")).unwrap();
    assert_eq!(
        workspace.as_path(),
        tmp.path()
            .join("workspace")
            .canonicalize()
            .unwrap()
            .as_path()
    );

    let candidate_tree = tempfile::tempdir().unwrap();
    let candidate_child = candidate_tree.path().join("child");
    std::fs::create_dir_all(&candidate_child).unwrap();
    let candidate = CandidateTreePath::new_temp_tree(candidate_child.join("..")).unwrap();
    assert_eq!(
        candidate.as_path(),
        candidate_tree.path().canonicalize().unwrap().as_path()
    );

    let kernel = KernelSourceRoot::new_existing_dir(scratch.join("../kernel/./")).unwrap();
    assert_eq!(
        kernel.as_path(),
        tmp.path().join("kernel").canonicalize().unwrap().as_path()
    );

    let kernel_build = KernelBuildDir::new("./.kslim-selftest/./tiny").unwrap();
    assert_eq!(kernel_build.as_path(), Path::new(".kslim-selftest/tiny"));

    let relative_kernel_path = RelativeKernelPath::new("./drivers/./foo.c").unwrap();
    assert_eq!(relative_kernel_path.as_path(), Path::new("drivers/foo.c"));

    let broad_requested_config = RequestedConfigPath::new("./config/./kslim.toml").unwrap();
    assert_eq!(
        broad_requested_config.as_path(),
        Path::new("config/kslim.toml")
    );

    let broad_workspace = WorkspaceRoot::new("./workspace/./root").unwrap();
    assert_eq!(broad_workspace.as_path(), Path::new("workspace/root"));

    let broad_candidate_tree = CandidateTreePath::new("./candidate/./tree").unwrap();
    assert_eq!(broad_candidate_tree.as_path(), Path::new("candidate/tree"));

    let broad_candidate_metadata = CandidateMetadataDir::new("./candidate/./.kslim/.").unwrap();
    assert_eq!(
        broad_candidate_metadata.as_path(),
        Path::new("candidate/.kslim")
    );

    let broad_attempt_metadata =
        AttemptMetadataDir::new("./candidate/./.kslim/./attempt").unwrap();
    assert_eq!(
        broad_attempt_metadata.as_path(),
        Path::new("candidate/.kslim/attempt")
    );

    let broad_output_repo = OutputRepoPath::new("./output/./repo").unwrap();
    assert_eq!(broad_output_repo.as_path(), Path::new("output/repo"));

    let broad_published_metadata =
        PublishedMetadataDir::new("./output/./.git/./kslim").unwrap();
    assert_eq!(
        broad_published_metadata.as_path(),
        Path::new("output/.git/kslim")
    );

    let output = tmp.path().join("output");
    std::fs::create_dir_all(output.join(".git")).unwrap();
    let output_repo = OutputRepoPath::new_git_worktree(scratch.join("../output/.")).unwrap();
    assert_eq!(
        output_repo.as_path(),
        output.canonicalize().unwrap().as_path()
    );

    let output_candidate_area =
        OutputCandidateArea::new(scratch.join("../output/./staged/../candidate"))
            .unwrap_err()
            .to_string();
    assert!(output_candidate_area.contains("parent components"));

    let output_candidate_repo = OutputRepoPath::new("./output-candidate/./tree").unwrap();
    let output_candidate_area =
        OutputCandidateArea::from_output_repo(&output_candidate_repo).unwrap();
    assert_eq!(
        output_candidate_area.as_path(),
        Path::new("output-candidate/tree")
    );
}

#[test]
fn test_lifecycle_path_constructors_normalize_current_and_empty_components() {
    assert_normalized_path(
        RequestedConfigPath::new("./config//kslim.toml"),
        Path::new("config/kslim.toml"),
    );
    assert_normalized_path(
        WorkspaceRoot::new("./workspace//root/."),
        Path::new("workspace/root"),
    );
    assert_normalized_path(
        CandidateTreePath::new("./candidate//tree/."),
        Path::new("candidate/tree"),
    );
    assert_normalized_path(
        KernelSourceRoot::new("./linux//source/."),
        Path::new("linux/source"),
    );
    assert_normalized_path(
        KernelBuildDir::new("./.kslim-selftest//tiny/."),
        Path::new(".kslim-selftest/tiny"),
    );
    assert_normalized_path(
        RelativeKernelPath::new("./drivers//gpu/./foo.c"),
        Path::new("drivers/gpu/foo.c"),
    );
    assert_normalized_path(
        CandidateMetadataDir::new("./candidate//.kslim/."),
        Path::new("candidate/.kslim"),
    );
    assert_normalized_path(
        AttemptMetadataDir::new("./candidate//.kslim/./attempt"),
        Path::new("candidate/.kslim/attempt"),
    );

    let output_repo = OutputRepoPath::new("./output//repo/.").unwrap();
    assert_eq!(output_repo.as_path(), Path::new("output/repo"));
    assert_normalized_path(
        OutputCandidateArea::from_output_repo(&output_repo),
        Path::new("output/repo"),
    );
    assert_normalized_path(
        PublishedMetadataDir::new("./output//.git/./kslim/."),
        Path::new("output/.git/kslim"),
    );
    assert_normalized_path(
        LockfilePath::new("./project//kslim.lock"),
        Path::new("project/kslim.lock"),
    );
}

#[cfg(unix)]
#[test]
fn test_existing_lifecycle_path_constructors_canonicalize_symlink_aliases() {
    let tmp = tempfile::tempdir().unwrap();
    let real = tmp.path().join("real");
    let links = tmp.path().join("links");
    std::fs::create_dir_all(&links).unwrap();

    let config_dir = real.join("project/config");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("kslim.toml"), "[project]\n").unwrap();
    std::os::unix::fs::symlink(&config_dir, links.join("config")).unwrap();
    let requested =
        RequestedConfigPath::new_existing_file(links.join("config/kslim.toml")).unwrap();
    assert_eq!(
        requested.as_path(),
        config_dir
            .join("kslim.toml")
            .canonicalize()
            .unwrap()
            .as_path()
    );

    let workspace_dir = real.join("workspace");
    std::fs::create_dir_all(&workspace_dir).unwrap();
    std::os::unix::fs::symlink(&workspace_dir, links.join("workspace")).unwrap();
    let workspace = WorkspaceRoot::new_temp_workspace(links.join("workspace")).unwrap();
    assert_eq!(
        workspace.as_path(),
        workspace_dir.canonicalize().unwrap().as_path()
    );

    let candidate_dir = real.join("candidate");
    std::fs::create_dir_all(&candidate_dir).unwrap();
    std::os::unix::fs::symlink(&candidate_dir, links.join("candidate")).unwrap();
    let candidate = CandidateTreePath::new_temp_tree(links.join("candidate")).unwrap();
    assert_eq!(
        candidate.as_path(),
        candidate_dir.canonicalize().unwrap().as_path()
    );

    let kernel_dir = real.join("kernel");
    std::fs::create_dir_all(&kernel_dir).unwrap();
    std::os::unix::fs::symlink(&kernel_dir, links.join("kernel")).unwrap();
    let kernel = KernelSourceRoot::new_existing_dir(links.join("kernel")).unwrap();
    assert_eq!(
        kernel.as_path(),
        kernel_dir.canonicalize().unwrap().as_path()
    );

    let output_dir = real.join("output");
    std::fs::create_dir_all(output_dir.join(".git")).unwrap();
    std::os::unix::fs::symlink(&output_dir, links.join("output")).unwrap();
    let output = OutputRepoPath::new_git_worktree(links.join("output")).unwrap();
    assert_eq!(
        output.as_path(),
        output_dir.canonicalize().unwrap().as_path()
    );
}

#[test]
fn test_validated_lifecycle_path_constructors_reject_wrong_shapes() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");
    std::fs::create_dir_all(&output).unwrap();
    let output_repo = OutputRepoPath::new(&output).unwrap();

    let non_temp_candidate = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();

    let err = RequestedConfigPath::new_existing_file(tmp.path().join("missing.toml"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("not an existing file"));

    let err = WorkspaceRoot::new_existing_dir(tmp.path().join("missing-workspace"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("not an existing directory"));

    let err = WorkspaceRoot::new_temp_workspace(non_temp_candidate.path())
        .unwrap_err()
        .to_string();
    assert!(err.contains("outside temporary directory"));

    let err = CandidateTreePath::new_temp_tree(non_temp_candidate.path())
        .unwrap_err()
        .to_string();
    assert!(err.contains("outside temporary directory"));

    let err = CandidateMetadataDir::new(tmp.path().join("candidate/reports"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("candidate .kslim dir"));

    let err = KernelSourceRoot::new_existing_dir(tmp.path().join("missing-kernel"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("not an existing directory"));

    let err = KernelBuildDir::new("").unwrap_err().to_string();
    assert!(err.contains("kernel build dir path is empty"));

    let err = KernelBuildDir::new(".").unwrap_err().to_string();
    assert!(err.contains("must not be the source root"));

    let err = RelativeKernelPath::new("").unwrap_err().to_string();
    assert!(err.contains("relative kernel path is empty"));

    let err = RelativeKernelPath::new(".").unwrap_err().to_string();
    assert!(err.contains("kernel tree root"));

    let root_removal = RelativeKernelPath::new_for_explicit_unsafe_root_removal(".").unwrap();
    assert_eq!(root_removal.as_path(), Path::new("."));

    let err = RelativeKernelPath::new("/tmp/linux/drivers/foo.c")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must be relative"));

    let err = RelativeKernelPath::new("C:/linux/drivers/foo.c")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must be relative"));

    let err = RelativeKernelPath::new("file:///tmp/linux/drivers/foo.c")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must be relative"));

    let candidate = CandidateTreePath::new(tmp.path().join("candidate")).unwrap();
    let err = CandidateMetadataDir::new_in_candidate_tree(
        &candidate,
        tmp.path().join("candidate/reports"),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("candidate .kslim dir"));

    let err = CandidateMetadataDir::new_in_candidate_tree(
        &candidate,
        tmp.path().join("outside/.kslim"),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("candidate tree metadata dir"));

    let err = AttemptMetadataDir::new(tmp.path().join(".kslim/report.txt"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("not a non-authoritative attempt dir"));

    let err = OutputRepoPath::new_git_worktree(&output)
        .unwrap_err()
        .to_string();
    assert!(err.contains("not a git worktree"));

    let err = PublishedMetadataDir::new(tmp.path().join(".kslim/attempt"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("not a committed metadata dir"));

    let err = OutputCandidateArea::new(tmp.path().join("output/../candidate"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("parent components"));

    let err =
        PublishedMetadataDir::new_committed_metadata_dir(tmp.path().join(".kslim/attempt"))
            .unwrap_err()
            .to_string();
    assert!(err.contains("not a committed metadata dir"));

    let err = PublishedMetadataDir::new_in_output_repo(
        &output_repo,
        tmp.path().join("outside/.kslim"),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("output repo metadata dir"));

    let err = LockfilePath::new(tmp.path().join("output/../kslim.lock"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("parent components"));

    let err = LockfilePath::new(tmp.path().join("not-kslim.lock"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("must end in kslim.lock"));
}

#[test]
fn test_lifecycle_path_constructors_reject_empty_paths() {
    let candidate = CandidateTreePath::new("/tmp/kslim-candidate").unwrap();
    let output_repo = OutputRepoPath::new("/tmp/output").unwrap();

    for path in [PathBuf::new(), PathBuf::from(" ")] {
        assert_empty_path_error(
            RequestedConfigPath::new(path.clone()),
            "requested config path is empty",
        );
        assert_empty_path_error(
            RequestedConfigPath::new_existing_file(path.clone()),
            "requested config path is empty",
        );
        assert_empty_path_error(
            WorkspaceRoot::new(path.clone()),
            "workspace root path is empty",
        );
        assert_empty_path_error(
            WorkspaceRoot::new_existing_dir(path.clone()),
            "workspace root path is empty",
        );
        assert_empty_path_error(
            WorkspaceRoot::new_temp_workspace(path.clone()),
            "workspace root path is empty",
        );
        assert_empty_path_error(
            CandidateTreePath::new(path.clone()),
            "candidate tree path is empty",
        );
        assert_empty_path_error(
            CandidateTreePath::new_temp_tree(path.clone()),
            "candidate tree path is empty",
        );
        assert_empty_path_error(
            KernelSourceRoot::new(path.clone()),
            "kernel source root path is empty",
        );
        assert_empty_path_error(
            KernelSourceRoot::new_existing_dir(path.clone()),
            "kernel source root path is empty",
        );
        assert_empty_path_error(
            KernelBuildDir::new(path.clone()),
            "kernel build dir path is empty",
        );
        assert_empty_path_error(
            RelativeKernelPath::new(path.clone()),
            "relative kernel path is empty",
        );
        assert_empty_path_error(
            CandidateMetadataDir::new(path.clone()),
            "candidate metadata dir is empty",
        );
        assert_empty_path_error(
            CandidateMetadataDir::new_in_candidate_tree(&candidate, path.clone()),
            "candidate metadata dir is empty",
        );
        assert_empty_path_error(
            AttemptMetadataDir::new(path.clone()),
            "attempt metadata dir is empty",
        );
        assert_empty_path_error(
            OutputRepoPath::new(path.clone()),
            "published output repo path is empty",
        );
        assert_empty_path_error(
            OutputRepoPath::new_git_worktree(path.clone()),
            "published output repo path is empty",
        );
        assert_empty_path_error(
            OutputCandidateArea::new(path.clone()),
            "output candidate area path is empty",
        );
        assert_empty_path_error(
            PublishedMetadataDir::new(path.clone()),
            "published metadata dir is empty",
        );
        assert_empty_path_error(
            PublishedMetadataDir::new_committed_metadata_dir(path.clone()),
            "published metadata dir is empty",
        );
        assert_empty_path_error(
            PublishedMetadataDir::new_in_output_repo(&output_repo, path.clone()),
            "published metadata dir is empty",
        );
        assert_empty_path_error(
            LockfilePath::new(path.clone()),
            "authoritative lockfile path is empty",
        );
        assert_empty_path_error(
            LockfilePath::new_in_project_root(path),
            "authoritative lockfile project root is empty",
        );
    }
}

#[test]
fn test_validated_lifecycle_path_constructors_reject_root_escape_attempts() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("output");
    std::fs::create_dir_all(output.join(".git")).unwrap();
    let output_repo = OutputRepoPath::new_git_worktree(&output).unwrap();

    let err = RequestedConfigPath::new(tmp.path().join("config/../kslim.toml"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("parent components"));

    let err = WorkspaceRoot::new(tmp.path().join("workspace/../workspace"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("parent components"));

    let err = CandidateTreePath::new(tmp.path().join("candidate/../candidate"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("parent components"));

    let err = CandidateMetadataDir::new(tmp.path().join("candidate/../candidate/.kslim"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("parent components"));

    let err = AttemptMetadataDir::new(tmp.path().join("candidate/../candidate/.kslim/attempt"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("parent components"));

    let candidate = CandidateTreePath::new(tmp.path().join("candidate")).unwrap();
    let err = CandidateMetadataDir::new_in_candidate_tree(
        &candidate,
        tmp.path().join("candidate/../candidate/.kslim"),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("parent components"));

    let err = KernelSourceRoot::new(tmp.path().join("kernel/../kernel"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("parent components"));

    let kernel_root = KernelSourceRoot::new(tmp.path().join("kernel")).unwrap();
    let err =
        KernelBuildDir::new_for_source_root(&kernel_root, tmp.path().join("kernel/../build"))
            .unwrap_err()
            .to_string();
    assert!(err.contains("parent components"));

    let err = KernelBuildDir::new_for_source_root(&kernel_root, tmp.path().join("kernel"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("must not alias kernel source root"));

    let err = RelativeKernelPath::new("drivers/../foo.c")
        .unwrap_err()
        .to_string();
    assert!(err.contains("parent components"));

    let err = RelativeKernelPath::new(r"drivers\..\foo.c")
        .unwrap_err()
        .to_string();
    assert!(err.contains("parent components"));

    let err = OutputRepoPath::new(tmp.path().join("output/../published"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("parent components"));

    let err = PublishedMetadataDir::new(output.join(".git/../.git/kslim"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("parent components"));

    let err = PublishedMetadataDir::new_in_output_repo(
        &output_repo,
        output.join(".git/../.git/kslim"),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("parent components"));

    let err = LockfilePath::new_in_project_root(tmp.path().join("../project"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("project root must not contain parent components"));

    let err = LockfilePath::new(tmp.path().join("kslim.lock/child"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("must end in kslim.lock"));
}
