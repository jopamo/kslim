use super::common::*;

#[test]
fn rust_source_files_over_2000_lines_are_explicitly_justified() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let justifications = RUST_FILE_SIZE_JUSTIFICATIONS
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    for (path, justification) in &justifications {
        assert!(
            justification.trim().len() >= 40,
            "{path} needs a concrete justification, not a token exception"
        );
    }

    let mut sources = Vec::new();
    collect_rust_sources(&root.join("src"), &mut sources);
    collect_rust_sources(&root.join("tests"), &mut sources);

    let mut oversized_paths = BTreeSet::new();
    let mut missing_justification = Vec::new();
    for source in sources {
        let content = std::fs::read_to_string(&source)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", source.display()));
        let line_count = content.lines().count();
        if line_count <= MAX_RUST_SOURCE_LINES {
            continue;
        }
        let relative = repo_relative_path(root, &source);
        oversized_paths.insert(relative.clone());
        if !justifications.contains_key(relative.as_str()) {
            missing_justification.push(format!(
                "{relative}: {line_count} lines exceeds {MAX_RUST_SOURCE_LINES}"
            ));
        }
    }

    let stale_justifications = justifications
        .keys()
        .filter(|path| !oversized_paths.contains(**path))
        .copied()
        .collect::<Vec<_>>();

    assert!(
        missing_justification.is_empty() && stale_justifications.is_empty(),
        "Rust files over {MAX_RUST_SOURCE_LINES} lines need explicit, current justification.\n\
         Missing justifications: {missing_justification:#?}\n\
         Stale justifications: {stale_justifications:#?}"
    );
}
