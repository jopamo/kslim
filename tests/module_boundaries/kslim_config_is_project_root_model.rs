use super::common::*;

fn section_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let (_, rest) = source
        .split_once(start)
        .unwrap_or_else(|| panic!("missing section start marker {start:?}"));
    let (section, _) = rest
        .split_once(end)
        .unwrap_or_else(|| panic!("missing section end marker {end:?}"));
    section
}

#[test]
fn kslim_config_is_project_root_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_model = production_source(&root.join("src/config/model.rs"));
    let config_mod = production_source(&root.join("src/config/mod.rs"));
    let config_load = production_source(&root.join("src/config/load.rs"));
    let config_templates = production_source(&root.join("src/config/templates.rs"));
    let config_validate = production_source(&root.join("src/config/validate.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let architecture_flat = architecture.replace("\n  ", " ");

    assert!(
        config_model.contains("pub struct KslimConfig {"),
        "config/model.rs should define the project-root KslimConfig model"
    );
    let kslim_config = section_between(
        &config_model,
        "pub struct KslimConfig",
        "pub struct ProjectConfig",
    );
    let kslim_fields = kslim_config
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("pub "))
        .collect::<Vec<_>>();
    assert_eq!(
        kslim_fields,
        vec![
            "pub project: ProjectConfig,",
            "pub upstream: UpstreamConfig,",
            "pub output: OutputConfig,",
            "pub git: GitConfig,",
            "pub publish: Option<PublishConfig>,",
        ],
        "KslimConfig should contain only project-root kslim.toml fields"
    );

    for required in [
        "pub struct ProjectConfig",
        "pub struct UpstreamConfig",
        "pub struct OutputConfig",
        "pub struct GitConfig",
        "pub struct PublishConfig",
    ] {
        assert!(
            config_model.contains(required),
            "config/model.rs should define KslimConfig child model {required}"
        );
    }

    for forbidden in [
        "ProfileConfig",
        "SlimConfig",
        "ReducerConfig",
        "SelfTestConfig",
        "AbiPolicyConfig",
        "ArchPolicyConfig",
        "PatchConfig",
        "IntegrationsConfig",
        "BuildMatrixConfig",
        "RuntimeMatrixConfig",
        "ReportConfig",
        "SecurityConfig",
        "PerformanceConfig",
    ] {
        assert!(
            !kslim_config.contains(forbidden),
            "KslimConfig must not absorb profile/removal/reducer/test policy {forbidden}"
        );
    }

    assert!(
        config_templates
            .contains("pub fn default_kslim_config(name: &str, output_path: &str) -> KslimConfig")
            && config_templates.contains("project: ProjectConfig")
            && config_templates.contains("upstream: UpstreamConfig")
            && config_templates.contains("output: OutputConfig")
            && config_templates.contains("git: GitConfig::default()")
            && config_templates.contains("publish: None"),
        "config/templates.rs should construct a complete default KslimConfig"
    );
    assert!(
        config_load.contains("pub fn load_kslim_config(root: &Path) -> Result<KslimConfig>")
            && config_load.contains("root.join(\"kslim.toml\")")
            && config_load.contains("toml::from_str(&contents)")
            && config_load.contains("validate_config(&config)?"),
        "config/load.rs should load and validate KslimConfig from project-root kslim.toml"
    );
    assert!(
        config_validate.contains("pub fn validate_config(config: &KslimConfig) -> Result<()>")
            && config_validate.contains("project.name must not be empty")
            && config_validate.contains("upstream.name must not be empty")
            && config_validate.contains("upstream.url must not be empty")
            && config_validate.contains("output.path must not be empty")
            && config_validate.contains("output.branch_prefix must not be empty")
            && config_validate.contains("git.user_email must not be empty")
            && config_validate.contains("git.user_name must not be empty")
            && config_validate.contains("git.remote_name must not be empty"),
        "config/validate.rs should own KslimConfig validation"
    );
    assert!(
        config_mod.contains("mod model;")
            && config_mod.contains("pub use model::*;")
            && config_mod.contains("load_kslim_config")
            && config_mod.contains("default_kslim_config")
            && config_mod.contains("validate_config"),
        "config/mod.rs should re-export KslimConfig model, defaults, loading, and validation"
    );
    assert!(
        architecture_flat.contains("`KslimConfig` is the project-root `kslim.toml` model")
            && architecture_flat
                .contains("project, upstream, output, git, and optional publish settings")
            && architecture_flat.contains("not in `KslimConfig`"),
        "architecture docs should describe KslimConfig ownership"
    );
}
