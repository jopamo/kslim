pub use std::collections::{BTreeMap, BTreeSet};
pub use std::path::{Path, PathBuf};

pub const MAX_RUST_SOURCE_LINES: usize = 2000;
pub const RUST_FILE_SIZE_JUSTIFICATIONS: &[(&str, &str)] = &[
    (
        "src/source_scan/cpp.rs",
        "CPP folding still combines parser, proof scan, rewrite, and tests until split by responsibility.",
    ),
    (
        "src/generate.rs",
        "Generate orchestration is still the composition point until stage-specific orchestration is extracted.",
    ),
    (
        "src/generate/failure.rs",
        "Generate failure reporting, rollback restoration, and legacy module-local tests remain together until failure tests split.",
    ),
    (
        "src/plan/mod.rs",
        "Immutable generate plan identity, resolution, and legacy module-local tests remain coupled until plan tests split.",
    ),
    (
        "src/prune.rs",
        "Prune orchestration and legacy module-local tests remain together until tests move beside owned modules.",
    ),
];

pub fn production_source(path: &Path) -> String {
    let source = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    source
        .split_once("\n#[cfg(test)]\nmod tests")
        .map(|(production, _)| production.to_string())
        .unwrap_or(source)
}

pub fn production_sources(root: &Path, relative_paths: &[&str]) -> String {
    relative_paths
        .iter()
        .map(|relative| production_source(&root.join(relative)))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn kernel_build_iteration_docs(root: &Path) -> String {
    production_sources(
        root,
        &[
            "docs/kernel-build-iteration.md",
            "docs/reference/profile-field-reference.md",
        ],
    )
}

pub fn cli_sources(root: &Path) -> String {
    production_sources(
        root,
        &[
            "src/cli/mod.rs",
            "src/cli/entrypoint.rs",
            "src/cli/command.rs",
            "src/cli/args.rs",
        ],
    )
}

pub fn commands_source(root: &Path) -> String {
    production_source(&root.join("src/commands/mod.rs"))
}

pub fn state_source(root: &Path) -> String {
    production_source(&root.join("src/state/mod.rs"))
}

pub fn plan_source(root: &Path) -> String {
    production_source(&root.join("src/plan/mod.rs"))
}

pub fn index_source(root: &Path) -> String {
    production_source(&root.join("src/index/mod.rs"))
}

pub fn collect_rust_sources(dir: &Path, sources: &mut Vec<PathBuf>) {
    let mut entries = std::fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("failed to read source directory {}: {err}", dir.display()))
        .map(|entry| entry.expect("failed to read source directory entry").path())
        .collect::<Vec<_>>();
    entries.sort();

    for entry in entries {
        if entry.is_dir() {
            collect_rust_sources(&entry, sources);
        } else if entry.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            sources.push(entry);
        }
    }
}

pub fn repo_relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or_else(|err| {
            panic!(
                "failed to make {} relative to {}: {err}",
                path.display(),
                root.display()
            )
        })
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
