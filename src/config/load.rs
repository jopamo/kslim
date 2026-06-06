use anyhow::{Context, Result};
use std::path::Path;

use crate::model::ArchName;

use super::{
    validate_config, validate_profile, ConfigSourceMap, FeatureSafetyLevel, KslimConfig,
    LoadedKslimConfig, LoadedProfileConfig, ProfileConfig,
};

#[derive(Clone, Copy, Default)]
pub struct ProfileFeatureSelection<'a> {
    pub feature: Option<&'a str>,
    pub remove_feature: Option<&'a str>,
    pub preserve_feature: Option<&'a str>,
    pub arch: Option<&'a str>,
    pub primary_arch: Option<&'a str>,
    pub secondary_arch: Option<&'a str>,
    pub safety: Option<&'a str>,
}

impl<'a> ProfileFeatureSelection<'a> {
    pub fn new(
        feature: Option<&'a str>,
        remove_feature: Option<&'a str>,
        preserve_feature: Option<&'a str>,
        arch: Option<&'a str>,
        primary_arch: Option<&'a str>,
        secondary_arch: Option<&'a str>,
        safety: Option<&'a str>,
    ) -> Self {
        Self {
            feature,
            remove_feature,
            preserve_feature,
            arch,
            primary_arch,
            secondary_arch,
            safety,
        }
    }

    pub fn selected_feature_name(self) -> Result<Option<String>> {
        self.validate_exclusive_feature_selectors()?;
        if let Some(feature) = self.feature {
            return normalize_feature_name(feature).map(Some);
        }
        if let Some(feature) = self.remove_feature {
            return normalize_feature_name(feature).map(Some);
        }
        if let Some(feature) = self.preserve_feature {
            return normalize_feature_name(feature).map(Some);
        }
        Ok(None)
    }

    fn validate_exclusive_feature_selectors(self) -> Result<()> {
        if [self.feature, self.remove_feature, self.preserve_feature]
            .iter()
            .filter(|feature| feature.is_some())
            .count()
            > 1
        {
            anyhow::bail!(
                "--feature, --remove-feature, and --preserve-feature are mutually exclusive"
            );
        }
        if self.arch.is_some() && self.primary_arch.is_some() {
            anyhow::bail!("--arch and --primary-arch are mutually exclusive");
        }
        if self.arch.is_some() && self.secondary_arch.is_some() {
            anyhow::bail!("--arch and --secondary-arch are mutually exclusive");
        }
        Ok(())
    }

    fn selected_arches(self) -> Result<Vec<String>> {
        self.validate_exclusive_feature_selectors()?;
        let mut arches = Vec::new();
        if let Some(arch) = self.arch {
            arches.push(normalize_arch_name(arch)?);
        }
        if let Some(arch) = self.primary_arch {
            arches.push(normalize_arch_name(arch)?);
        }
        if let Some(arch) = self.secondary_arch {
            arches.push(normalize_arch_name(arch)?);
        }
        Ok(arches)
    }

    fn selected_safety(self) -> Result<Option<FeatureSafetyLevel>> {
        self.validate_exclusive_feature_selectors()?;
        self.safety.map(normalize_feature_safety_level).transpose()
    }
}

pub fn load_kslim_config(root: &Path) -> Result<KslimConfig> {
    Ok(load_kslim_config_with_source_map(root)?.config)
}

pub fn load_kslim_config_with_source_map(root: &Path) -> Result<LoadedKslimConfig> {
    let path = root.join("kslim.toml");
    load_kslim_config_file_with_source_map(&path)
}

pub fn load_kslim_config_file_with_source_map(path: &Path) -> Result<LoadedKslimConfig> {
    let root = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let mut config: KslimConfig =
        toml::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))?;
    let source_map =
        ConfigSourceMap::from_kslim_config_document(path.to_string_lossy(), &contents, &config)?;
    let upstream_path = Path::new(&config.upstream.url);
    if !upstream_path.is_absolute() && !config.upstream.url.trim().is_empty() {
        config.upstream.url = root.join(upstream_path).to_string_lossy().to_string();
    }
    validate_config(&config)?;
    Ok(LoadedKslimConfig { config, source_map })
}

pub fn load_profile(root: &Path, profile_name: &str) -> Result<ProfileConfig> {
    Ok(load_profile_with_source_map(root, profile_name)?.profile)
}

pub fn normalize_profile_name(profile_name: &str) -> Result<String> {
    let profile_name = profile_name.trim();
    if profile_name.is_empty() {
        anyhow::bail!("profile name must not be empty");
    }
    if profile_name == "." || profile_name == ".." {
        anyhow::bail!("profile name must not be '.' or '..'");
    }
    if profile_name.contains('/') || profile_name.contains('\\') {
        anyhow::bail!("profile name must not contain path separators: {profile_name}");
    }
    Ok(profile_name.to_string())
}

pub fn normalize_feature_name(feature_name: &str) -> Result<String> {
    let feature_name = feature_name.trim();
    if feature_name.is_empty() {
        anyhow::bail!("feature name must not be empty");
    }
    Ok(feature_name.to_string())
}

pub fn normalize_arch_name(arch: &str) -> Result<String> {
    let arch = arch.trim();
    if arch.is_empty() {
        anyhow::bail!("arch name must not be empty");
    }
    ArchName::new(arch).map_err(|err| anyhow::anyhow!("arch name is invalid: {:#}", err))?;
    Ok(arch.to_string())
}

pub fn normalize_feature_safety_level(safety: &str) -> Result<FeatureSafetyLevel> {
    let safety = safety.trim();
    if safety.is_empty() {
        anyhow::bail!("safety level must not be empty");
    }
    FeatureSafetyLevel::from_cli_name(safety).ok_or_else(|| {
        anyhow::anyhow!(
            "safety level is invalid: expected conservative, normal, aggressive, surgical, or unsafe"
        )
    })
}

pub fn select_profile_feature(profile: ProfileConfig, feature_name: &str) -> Result<ProfileConfig> {
    let feature_name = normalize_feature_name(feature_name)?;
    if !profile.has_named_feature(&feature_name) {
        anyhow::bail!(
            "feature '{}' is not declared in features.remove or features.preserve",
            feature_name
        );
    }
    Ok(profile.with_only_named_feature(&feature_name))
}

pub fn select_profile_remove_feature(
    profile: ProfileConfig,
    feature_name: &str,
) -> Result<ProfileConfig> {
    let feature_name = normalize_feature_name(feature_name)?;
    if !profile.has_named_remove_feature(&feature_name) {
        anyhow::bail!(
            "remove feature '{}' is not declared in features.remove",
            feature_name
        );
    }
    Ok(profile.with_only_named_remove_feature(&feature_name))
}

pub fn select_profile_preserve_feature(
    profile: ProfileConfig,
    feature_name: &str,
) -> Result<ProfileConfig> {
    let feature_name = normalize_feature_name(feature_name)?;
    if !profile.has_named_preserve_feature(&feature_name) {
        anyhow::bail!(
            "preserve feature '{}' is not declared in features.preserve",
            feature_name
        );
    }
    Ok(profile.with_only_named_preserve_feature(&feature_name))
}

pub fn select_profile_features(
    profile: ProfileConfig,
    selection: ProfileFeatureSelection<'_>,
) -> Result<ProfileConfig> {
    selection.validate_exclusive_feature_selectors()?;
    let mut profile = if let Some(feature) = selection.feature {
        select_profile_feature(profile, feature)?
    } else if let Some(feature) = selection.remove_feature {
        select_profile_remove_feature(profile, feature)?
    } else if let Some(feature) = selection.preserve_feature {
        select_profile_preserve_feature(profile, feature)?
    } else {
        profile
    };
    let arches = selection.selected_arches()?;
    if !arches.is_empty() {
        profile = profile.with_selected_feature_arches(&arches);
    }
    if let Some(safety) = selection.selected_safety()? {
        let mut applied = false;
        for intent in profile.features.remove.values_mut() {
            if intent.declares_removal_input() {
                intent.safety = Some(safety);
                applied = true;
            }
        }
        if !applied {
            anyhow::bail!("--safety requires active features.remove removal input");
        }
    }
    Ok(profile)
}

pub fn insert_profile_feature_selection_cli_overrides(
    map: &mut ConfigSourceMap,
    selection: ProfileFeatureSelection<'_>,
) {
    if selection.feature.is_some() {
        map.insert_cli_override("features.selected", "cli --feature");
    }
    if selection.remove_feature.is_some() {
        map.insert_cli_override("features.remove.selected", "cli --remove-feature");
    }
    if selection.preserve_feature.is_some() {
        map.insert_cli_override("features.preserve.selected", "cli --preserve-feature");
    }
    if selection.arch.is_some() {
        map.insert_cli_override("arch.selected", "cli --arch");
    }
    if selection.primary_arch.is_some() {
        map.insert_cli_override("arch.primary_arch", "cli --primary-arch");
    }
    if selection.secondary_arch.is_some() {
        map.insert_cli_override("arch.secondary_arches", "cli --secondary-arch");
    }
    if selection.safety.is_some() {
        map.insert_cli_override("features.remove.safety", "cli --safety");
    }
}

pub fn insert_profile_strictness_cli_overrides(map: &mut ConfigSourceMap, source: &'static str) {
    for key in [
        "reducer.report_unsupported_expressions",
        "reducer.fail_on_unknown_diagnostics",
        "reducer.reject_unproven_fixups",
        "reducer.reject_unreasoned_edits",
        "reducer.reject_speculative_fallout_edits",
    ] {
        map.insert_cli_override(key, source);
    }
}

pub fn load_profile_with_source_map(
    root: &Path,
    profile_name: &str,
) -> Result<LoadedProfileConfig> {
    let profile_name = normalize_profile_name(profile_name)?;
    let path = root.join("profiles").join(format!("{}.toml", profile_name));
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("profile '{}' not found at {}", profile_name, path.display()))?;
    let profile: ProfileConfig = toml::from_str(&contents)
        .with_context(|| format!("failed to parse profile {}", path.display()))?;
    let source_map =
        ConfigSourceMap::from_profile_config_document(path.to_string_lossy(), &contents, &profile)?;
    validate_profile(&profile)?;
    if profile.profile.name != profile_name {
        anyhow::bail!(
            "profile name mismatch: file declares '{}' but expected '{}'",
            profile.profile.name,
            profile_name
        );
    }
    Ok(LoadedProfileConfig {
        profile,
        source_map,
    })
}

pub fn list_profiles(root: &Path) -> Result<Vec<String>> {
    let dir = root.join("profiles");
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut profiles = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "toml") {
            if let Some(stem) = path.file_stem() {
                profiles.push(stem.to_string_lossy().to_string());
            }
        }
    }
    profiles.sort();
    Ok(profiles)
}

pub fn require_known_profile(root: &Path, profile_name: &str) -> Result<()> {
    let profile_name = normalize_profile_name(profile_name)?;
    let path = root.join("profiles").join(format!("{}.toml", profile_name));
    if !path.exists() {
        let known = list_profiles(root)?.join(", ");
        anyhow::bail!(
            "unknown profile '{}' (known profiles: [{}])",
            profile_name,
            known
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigSourceKind;

    #[test]
    fn load_kslim_config_with_source_map_records_project_config_values() {
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
branch_prefix = "reduced"
"#,
        )
        .unwrap();

        let loaded = load_kslim_config_with_source_map(root.path()).unwrap();

        assert_eq!(loaded.config.project.name, "demo");
        assert_eq!(
            loaded.config.upstream.url,
            root.path().join("linux.git").to_string_lossy().to_string()
        );
        assert_eq!(
            loaded
                .source_map
                .get("project.name")
                .map(|source| source.kind),
            Some(ConfigSourceKind::ConfigFile)
        );
        assert!(loaded
            .source_map
            .get("project.name")
            .unwrap()
            .source
            .ends_with("kslim.toml"));
        assert!(loaded.source_map.contains_value("upstream.url"));
        assert!(loaded.source_map.contains_value("output.branch_prefix"));
    }

    #[test]
    fn load_profile_with_source_map_records_profile_values() {
        let root = tempfile::tempdir().unwrap();
        let profiles = root.path().join("profiles");
        std::fs::create_dir_all(&profiles).unwrap();
        std::fs::write(
            profiles.join("default.toml"),
            r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
roots = ["net/bluetooth"]
report_only = true
require_clean_boot = true

[selftests]
commands = ["make test"]
"#,
        )
        .unwrap();

        let loaded = load_profile_with_source_map(root.path(), "default").unwrap();

        assert_eq!(loaded.profile.profile.name, "default");
        assert_eq!(
            loaded
                .source_map
                .get("profile.name")
                .map(|source| source.kind),
            Some(ConfigSourceKind::Profile)
        );
        assert!(loaded
            .source_map
            .get("profile.name")
            .unwrap()
            .source
            .ends_with("profiles/default.toml"));
        assert!(loaded
            .source_map
            .contains_value("features.remove.bluetooth.roots"));
        assert!(loaded
            .source_map
            .contains_value("features.remove.bluetooth.roots[0]"));
        assert!(loaded
            .source_map
            .contains_value("features.remove.bluetooth.report_only"));
        assert!(loaded
            .source_map
            .contains_value("features.remove.bluetooth.require_clean_boot"));
        assert!(loaded.source_map.contains_value("selftests.commands[0]"));
    }

    #[test]
    fn profile_names_are_trimmed_and_cannot_escape_profiles_dir() {
        assert_eq!(normalize_profile_name(" release ").unwrap(), "release");

        for invalid in [
            "",
            "   ",
            ".",
            "..",
            "../release",
            "release/debug",
            r"release\debug",
        ] {
            assert!(
                normalize_profile_name(invalid).is_err(),
                "profile name should be rejected: {invalid:?}"
            );
        }
    }

    #[test]
    fn feature_names_are_trimmed_and_selected_from_profile() {
        assert_eq!(normalize_feature_name(" bluetooth ").unwrap(), "bluetooth");
        assert!(normalize_feature_name(" ").is_err());

        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            "bluetooth".to_string(),
            crate::config::FeatureIntentConfig {
                roots: vec!["net/bluetooth".to_string()],
                ..crate::config::FeatureIntentConfig::default()
            },
        );
        profile.features.remove.insert(
            "wifi".to_string(),
            crate::config::FeatureIntentConfig {
                roots: vec!["drivers/net/wireless".to_string()],
                ..crate::config::FeatureIntentConfig::default()
            },
        );

        let selected = select_profile_feature(profile, " bluetooth ").unwrap();

        assert!(selected.features.remove.contains_key("bluetooth"));
        assert!(!selected.features.remove.contains_key("wifi"));
    }

    #[test]
    fn remove_feature_names_are_trimmed_and_selected_from_profile() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            "bluetooth".to_string(),
            crate::config::FeatureIntentConfig {
                roots: vec!["net/bluetooth".to_string()],
                ..crate::config::FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            "netfilter".to_string(),
            crate::config::FeatureIntentConfig {
                roots: vec!["net/netfilter".to_string()],
                ..crate::config::FeatureIntentConfig::default()
            },
        );

        let selected = select_profile_remove_feature(profile, " bluetooth ").unwrap();

        assert!(selected.features.remove.contains_key("bluetooth"));
        assert!(selected.features.preserve.is_empty());
        assert!(select_profile_remove_feature(selected, "netfilter").is_err());
    }

    #[test]
    fn preserve_feature_names_are_trimmed_and_selected_from_profile() {
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            "bluetooth".to_string(),
            crate::config::FeatureIntentConfig {
                roots: vec!["net/bluetooth".to_string()],
                ..crate::config::FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            "netfilter".to_string(),
            crate::config::FeatureIntentConfig {
                roots: vec!["net/netfilter".to_string()],
                ..crate::config::FeatureIntentConfig::default()
            },
        );

        let selected = select_profile_preserve_feature(profile, " netfilter ").unwrap();

        assert!(selected.features.remove.is_empty());
        assert!(selected.features.preserve.contains_key("netfilter"));
        assert!(select_profile_preserve_feature(selected, "bluetooth").is_err());
    }

    #[test]
    fn arch_names_are_trimmed_and_filter_feature_intent() {
        assert_eq!(normalize_arch_name(" x86 ").unwrap(), "x86");
        assert!(normalize_arch_name("x86/../../host").is_err());

        let mut profile = crate::config::default_profile_config("v1.0");
        profile.features.remove.insert(
            "bluetooth".to_string(),
            crate::config::FeatureIntentConfig {
                roots: vec!["net/bluetooth".to_string()],
                arch_scope: vec!["x86".to_string()],
                ..crate::config::FeatureIntentConfig::default()
            },
        );
        profile.features.remove.insert(
            "wifi".to_string(),
            crate::config::FeatureIntentConfig {
                roots: vec!["drivers/net/wireless".to_string()],
                arch_scope: vec!["arm64".to_string()],
                ..crate::config::FeatureIntentConfig::default()
            },
        );
        profile.features.preserve.insert(
            "netfilter".to_string(),
            crate::config::FeatureIntentConfig {
                roots: vec!["net/netfilter".to_string()],
                ..crate::config::FeatureIntentConfig::default()
            },
        );

        let selected = select_profile_features(
            profile,
            ProfileFeatureSelection::new(None, None, None, Some(" x86 "), None, None, None),
        )
        .unwrap();

        assert!(selected.features.remove.contains_key("bluetooth"));
        assert!(!selected.features.remove.contains_key("wifi"));
        assert!(selected.features.preserve.contains_key("netfilter"));
    }
}
