mod common;
use common::*;

#[test]
fn test_manifest_is_sorted() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok, _, _) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok);

    let manifest_path = output_meta_path(&output_dir, "manifest.txt");
    let contents = std::fs::read_to_string(&manifest_path).unwrap();
    let paths: Vec<&str> = contents
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            // Format: "sha256  size  path" (two spaces between fields)
            let mut parts = l.split_whitespace();
            parts.next(); // sha256
            parts.next(); // size
            parts.next().unwrap_or("") // path
        })
        .collect();

    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted, "manifest paths should be sorted");
}

#[test]
fn test_manifest_uses_relative_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok, _, _) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok);

    let manifest_path = output_meta_path(&output_dir, "manifest.txt");
    let contents = std::fs::read_to_string(&manifest_path).unwrap();
    for line in contents.lines() {
        if line.is_empty() {
            continue;
        }
        let path = line.split_whitespace().nth(2).unwrap_or("");
        assert!(
            !path.starts_with('/'),
            "manifest path '{}' should be relative",
            path
        );
        assert!(
            !path.starts_with(".kslim/"),
            "manifest path '{}' should exclude kslim metadata",
            path
        );
        assert!(
            !path.starts_with(".git/"),
            "manifest path '{}' should exclude git metadata",
            path
        );
    }
}

#[test]
fn test_lockfile_records_commit_sha() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = create_fake_upstream(tmp.path(), "test", "1.0");
    let output_dir = tmp.path().join("output");
    let kslim_dir = create_kslim_project(
        tmp.path(),
        "test-linux",
        output_dir.to_str().unwrap(),
        &upstream,
    );

    let (ok, _, _) = kslim_in(&kslim_dir, &["generate"]);
    assert!(ok);

    let lockfile_path = kslim_dir.join("kslim.lock");
    assert!(lockfile_path.exists(), "lockfile should exist");
    let contents = std::fs::read_to_string(&lockfile_path).unwrap();
    assert!(
        contents.contains("commit = \""),
        "lockfile should contain commit: {}",
        contents
    );
    assert!(
        contents.contains("[published]"),
        "lockfile should contain authoritative published state: {}",
        contents
    );
    assert!(
        contents.contains("output_branch = \"kslim/v1.0/default\""),
        "lockfile should record published branch: {}",
        contents
    );
}
