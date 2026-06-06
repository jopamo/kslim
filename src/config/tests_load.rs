use super::*;

#[test]
fn test_load_kslim_config_resolves_relative_upstream_url_from_project_root() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("kslim.toml"),
        r#"
[project]
name = "demo"

[upstream]
name = "linux"
url = "linux.git"

[output]
path = "/tmp/output"
"#,
    )
    .unwrap();

    let config = load_kslim_config(root.path()).unwrap();

    assert_eq!(
        config.upstream.url,
        root.path().join("linux.git").to_string_lossy().to_string()
    );
    assert_eq!(config.output.branch_prefix, "kslim");
    assert!(config.publish.is_none());
}
#[test]
fn test_load_profile_reads_named_profile_from_profiles_dir() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("profiles")).unwrap();
    std::fs::write(
        root.path().join("profiles/default.toml"),
        r#"
[profile]
name = "default"

[base]
ref = "v1.0"
"#,
    )
    .unwrap();

    let profile = load_profile(root.path(), "default").unwrap();

    assert_eq!(profile.profile.name, "default");
    assert_eq!(profile.base.r#ref, "v1.0");
    assert!(profile.features.is_empty());
    assert!(profile.arch.is_default());
    assert!(profile.build_matrix.is_default());
    assert!(profile.runtime_matrix.is_default());
    assert!(profile.reports.is_default());
    assert!(profile.security.is_default());
    assert!(profile.performance.is_default());
    assert_eq!(profile.reducer, ReducerConfig::default());
}
#[test]
fn test_load_profile_rejects_file_name_mismatch() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("profiles")).unwrap();
    std::fs::write(
        root.path().join("profiles/default.toml"),
        r#"
[profile]
name = "other"

[base]
ref = "v1.0"
"#,
    )
    .unwrap();

    let err = load_profile(root.path(), "default")
        .unwrap_err()
        .to_string();

    assert!(err.contains("profile name mismatch"));
    assert!(err.contains("other"));
    assert!(err.contains("default"));
}
