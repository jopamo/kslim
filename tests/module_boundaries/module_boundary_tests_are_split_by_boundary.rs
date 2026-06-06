use super::common::*;

#[test]
fn module_boundary_tests_are_split_by_boundary() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let harness = production_source(&root.join("tests/module_boundaries.rs"));
    assert!(
        !harness.contains("#[test]"),
        "tests/module_boundaries.rs should only declare boundary modules"
    );

    let boundary_dir = root.join("tests/module_boundaries");
    let mut boundary_files = std::fs::read_dir(&boundary_dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", boundary_dir.display()))
        .map(|entry| entry.expect("failed to read boundary module entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rs"))
        .filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("common.rs"))
        .collect::<Vec<_>>();
    boundary_files.sort();

    assert!(
        boundary_files.len() >= 30,
        "boundary tests should remain split into focused files"
    );
    for path in boundary_files {
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let test_count = source
            .lines()
            .filter(|line| line.trim() == "#[test]")
            .count();
        assert_eq!(
            test_count,
            1,
            "{} should contain exactly one boundary test",
            path.display()
        );
        assert!(
            source.contains("use super::common::*;"),
            "{} should use shared boundary helpers",
            path.display()
        );
    }
}
