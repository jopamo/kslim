use super::common::*;

const LEGACY_TOP_LEVEL_MODULES_WITH_SUBSYSTEM_DIRS: &[&str] = &[
    "diagnostics",
    "edit_reason",
    "fixups",
    "generate",
    "output_repo",
    "prune",
    "removal_manifest",
];

#[test]
fn no_new_top_level_feature_modules_after_subsystem_dir_exists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = root.join("src");
    let legacy = LEGACY_TOP_LEVEL_MODULES_WITH_SUBSYSTEM_DIRS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();

    let mut violations = Vec::new();
    let mut entries = std::fs::read_dir(&src)
        .expect("failed to read src")
        .map(|entry| entry.expect("failed to read src entry").path())
        .collect::<Vec<_>>();
    entries.sort();

    for path in entries {
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if !src.join(stem).is_dir() || legacy.contains(stem) {
            continue;
        }
        violations.push(format!("src/{stem}.rs + src/{stem}/"));
    }

    assert!(
        violations.is_empty(),
        "new top-level src/*.rs feature modules are forbidden once a matching subsystem directory exists; use the subsystem directory instead. Violations: {violations:#?}"
    );

    let workflow = production_source(&root.join(".github/workflows/source-size.yml"));
    for required in [
        "top-level src/*.rs feature modules",
        "no_new_top_level_feature_modules_after_subsystem_dir_exists",
    ] {
        assert!(
            workflow.contains(required),
            "CI should run the top-level module-shape guard through {required}"
        );
    }

    let doc = production_source(&root.join("docs/file-size-policy.md"));
    for required in [
        "## Top-level feature-module guard",
        "top-level `src/*.rs`",
        "matching subsystem directory",
        "module-boundary test rejects it",
    ] {
        assert!(
            doc.contains(required),
            "docs/file-size-policy.md should document top-level module guard {required}"
        );
    }
}
