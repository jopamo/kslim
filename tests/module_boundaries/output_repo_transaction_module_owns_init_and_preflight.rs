use super::common::*;

#[test]
fn output_repo_transaction_module_owns_init_and_preflight() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output_repo = production_source(&root.join("src/output_repo.rs"));
    let transaction = production_source(&root.join("src/output_repo/transaction.rs"));
    let generate_publish = production_source(&root.join("src/generate/publish.rs"));
    let publish = production_source(&root.join("src/publish.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod transaction;",
        "pub use transaction::{",
        "init_output_repo",
        "is_kslim_managed",
        "require_clean",
        "require_managed",
        "require_not_detached",
        "sync_repo_git_config",
        "write_managed_marker",
    ] {
        assert!(
            output_repo.contains(required),
            "src/output_repo.rs should expose transaction helpers through {required}"
        );
    }

    for required in [
        "const MANAGED_FILE: &str = \".kslim/managed.toml\"",
        "const GIT_MANAGED_FILE: &str = \".git/kslim/managed.toml\"",
        "pub fn is_kslim_managed(output_path: &str) -> bool",
        "pub fn require_managed(output_path: &str) -> Result<()>",
        "KslimError::NotManaged",
        "pub fn require_clean(output_path: &str, force: bool) -> Result<()>",
        "pub fn require_not_detached(output_path: &str, force: bool) -> Result<()>",
        "pub fn init_output_repo(config: &KslimConfig, _profile: &ProfileConfig) -> Result<()>",
        "crate::git::init_repo(output_path)?",
        "crate::git::create_branch(output_path, &initial_branch)?",
        "write_managed_marker(output_path, &config.project.name)?",
        "pub fn sync_repo_git_config(config: &KslimConfig, branch: Option<&str>) -> Result<()>",
        "crate::git::config_set(output_path, \"user.email\"",
        "crate::git::remote_add(output_path, remote_name, &publish.remote)?",
        "pub fn write_managed_marker(output_path: &str, project_name: &str) -> Result<()>",
        "fn published_metadata_dir_path(output_path: &Path) -> Result<PathBuf>",
        "metadata::published_metadata_dir(&output_repo)?",
    ] {
        assert!(
            transaction.contains(required),
            "src/output_repo/transaction.rs should own output init/preflight detail {required}"
        );
    }

    for moved_item in [
        "\nconst MANAGED_FILE",
        "\nconst GIT_MANAGED_FILE",
        "\npub fn is_kslim_managed",
        "\npub fn require_managed",
        "\npub fn require_clean",
        "\npub fn require_not_detached",
        "\npub fn init_output_repo",
        "\npub fn sync_repo_git_config",
        "\npub fn write_managed_marker",
    ] {
        assert!(
            !output_repo.contains(moved_item),
            "src/output_repo.rs should not retain output transaction implementation {moved_item}"
        );
    }

    for required in [
        "output_repo::init_output_repo(config, profile)?",
        "output_repo::require_managed(output_path)?",
        "output_repo::require_clean(output_path, output_repo.force)?",
        "output_repo::sync_repo_git_config(config, Some(branch))?",
        "output_repo::write_managed_marker(&output_candidate_path, &config.project.name)?",
    ] {
        assert!(
            generate_publish.contains(required),
            "generate/publish.rs should keep command flow through output transaction facade {required}"
        );
    }
    for required in [
        "output_repo::require_managed(output_path)?",
        "output_repo::require_clean(output_path, opts.force)?",
        "output_repo::require_not_detached(output_path, opts.force)?",
    ] {
        assert!(
            publish.contains(required),
            "publish.rs should keep publish preflight through output transaction facade {required}"
        );
    }

    for required in ["`src/output_repo/transaction.rs`", "Output repo transaction"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document output transaction ownership {required}"
        );
    }
}
