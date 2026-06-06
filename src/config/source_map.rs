use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{KslimConfig, PatchConfig, PatchSourceConfig, ProfileConfig};

const DEFAULT_SOURCE: &str = "built-in default";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ConfigSourceKind {
    Default,
    ConfigFile,
    Profile,
    IncludeFile,
    Environment,
    Cli,
}

#[allow(dead_code)]
impl ConfigSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::ConfigFile => "config_file",
            Self::Profile => "profile",
            Self::IncludeFile => "include_file",
            Self::Environment => "environment",
            Self::Cli => "cli",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigValueSource {
    pub kind: ConfigSourceKind,
    pub source: String,
}

impl ConfigValueSource {
    pub fn new(kind: ConfigSourceKind, source: impl Into<String>) -> Self {
        Self {
            kind,
            source: source.into(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigSourceMap {
    values: BTreeMap<String, ConfigValueSource>,
}

#[allow(dead_code)]
impl ConfigSourceMap {
    pub fn from_kslim_config_document(
        source: impl Into<String>,
        contents: &str,
        config: &KslimConfig,
    ) -> Result<Self> {
        let mut map = Self::from_toml_document(ConfigSourceKind::ConfigFile, source, contents)?;
        map.record_kslim_config_defaults(config);
        Ok(map)
    }

    pub fn from_profile_config_document(
        source: impl Into<String>,
        contents: &str,
        profile: &ProfileConfig,
    ) -> Result<Self> {
        let mut map = Self::from_toml_document(ConfigSourceKind::Profile, source, contents)?;
        map.record_profile_config_defaults(profile);
        Ok(map)
    }

    pub fn from_include_file_document(source: impl Into<String>, contents: &str) -> Result<Self> {
        Self::from_toml_document(ConfigSourceKind::IncludeFile, source, contents)
    }

    pub fn from_toml_document(
        kind: ConfigSourceKind,
        source: impl Into<String>,
        contents: &str,
    ) -> Result<Self> {
        let source = source.into();
        let parsed: toml::Value =
            toml::from_str(contents).with_context(|| format!("failed to parse {}", source))?;
        let mut map = Self::default();
        map.record_toml_value(kind, &source, String::new(), &parsed);
        Ok(map)
    }

    pub fn insert(
        &mut self,
        path: impl Into<String>,
        kind: ConfigSourceKind,
        source: impl Into<String>,
    ) {
        self.values
            .insert(path.into(), ConfigValueSource::new(kind, source));
    }

    pub fn insert_cli_override(&mut self, path: impl Into<String>, source: impl Into<String>) {
        self.insert(path, ConfigSourceKind::Cli, source);
    }

    pub fn insert_environment_override(
        &mut self,
        path: impl Into<String>,
        source: impl Into<String>,
    ) {
        self.insert(path, ConfigSourceKind::Environment, source);
    }

    pub fn insert_include_file_value(
        &mut self,
        path: impl Into<String>,
        source: impl Into<String>,
    ) {
        self.insert(path, ConfigSourceKind::IncludeFile, source);
    }

    pub fn get(&self, path: &str) -> Option<&ConfigValueSource> {
        self.values.get(path)
    }

    pub fn contains_value(&self, path: &str) -> bool {
        self.values.contains_key(path)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &ConfigValueSource)> {
        self.values
            .iter()
            .map(|(path, source)| (path.as_str(), source))
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn record_kslim_config_defaults(&mut self, _config: &KslimConfig) {
        self.insert_default("upstream.mode");
        self.insert_default("upstream.cache");
        self.insert_default("output.branch_prefix");
        self.insert_default("output.branch");
        self.insert_default("git.user_email");
        self.insert_default("git.user_name");
        self.insert_default("git.remote_name");
        self.insert_default_container("publish");
    }

    pub fn record_profile_config_defaults(&mut self, profile: &ProfileConfig) {
        self.insert_default("profile.description");
        self.insert_default("profile.inherits");
        if let Some(slim) = &profile.slim {
            self.record_slim_defaults("slim", slim);
        } else {
            self.insert_default_container("slim");
        }
        self.record_feature_defaults(profile);
        self.insert_default("abi.allow_public_header_removal");
        self.insert_default("abi.allow_uapi_header_removal");
        self.insert_default("arch.primary_arch");
        self.insert_default_container("arch.secondary_arches");
        self.insert_default_container("arch.disabled_arches");
        self.insert_default("arch.allow_arch_local_removal");
        self.insert_default("arch.preserve_arch_shared");
        self.record_build_matrix_defaults();
        self.record_runtime_matrix_defaults();
        self.record_report_defaults();
        self.record_security_defaults();
        self.record_performance_defaults();
        if let Some(patches) = &profile.patches {
            self.record_patch_defaults("patches", patches);
        } else {
            self.insert_default_container("patches");
        }
        self.record_integration_defaults(profile);
        self.insert_default("reducer.max_fixup_passes");
        self.insert_default("reducer.report_unsupported_expressions");
        self.insert_default("reducer.fail_on_unknown_diagnostics");
        self.insert_default("reducer.reject_unproven_fixups");
        self.insert_default("reducer.reject_unreasoned_edits");
        self.insert_default("reducer.reject_speculative_fallout_edits");
        self.insert_default("reducer.fail_on_missing_prune_paths");
        self.insert_default("reducer.ignore_unsupported_special_removals");
        self.record_selftest_defaults(profile);
    }

    fn record_toml_value(
        &mut self,
        kind: ConfigSourceKind,
        source: &str,
        path: String,
        value: &toml::Value,
    ) {
        match value {
            toml::Value::Table(table) => {
                if table.is_empty() && !path.is_empty() {
                    self.insert(path, kind, source);
                    return;
                }
                for (key, value) in table {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    self.record_toml_value(kind, source, child_path, value);
                }
            }
            toml::Value::Array(values) => {
                if !path.is_empty() {
                    self.insert(path.clone(), kind, source);
                }
                for (idx, value) in values.iter().enumerate() {
                    self.record_toml_value(kind, source, format!("{path}[{idx}]"), value);
                }
            }
            _ => {
                if !path.is_empty() {
                    self.insert(path, kind, source);
                }
            }
        }
    }

    fn record_slim_defaults(&mut self, prefix: &str, _slim: &super::SlimConfig) {
        self.insert_default_container(format!("{prefix}.remove_paths"));
        self.insert_default_container(format!("{prefix}.remove_configs"));
        self.insert_default_container(format!("{prefix}.set_defaults"));
        self.insert_default(format!("{prefix}.unsafe_allow_root_path_removal"));
    }

    fn record_feature_defaults(&mut self, profile: &ProfileConfig) {
        self.insert_default_container("features.remove");
        self.insert_default_container("features.preserve");
        for name in profile.features.remove.keys() {
            self.record_feature_intent_defaults(&format!("features.remove.{name}"));
        }
        for name in profile.features.preserve.keys() {
            self.record_feature_intent_defaults(&format!("features.preserve.{name}"));
        }
    }

    fn record_feature_intent_defaults(&mut self, prefix: &str) {
        self.insert_default_container(format!("{prefix}.roots"));
        self.insert_default_container(format!("{prefix}.remove_paths"));
        self.insert_default_container(format!("{prefix}.configs"));
        self.insert_default_container(format!("{prefix}.remove_configs"));
        self.insert_default_container(format!("{prefix}.exported_symbols"));
        self.insert_default_container(format!("{prefix}.remove_exported_symbols"));
        self.insert_default_container(format!("{prefix}.module_names"));
        self.insert_default_container(format!("{prefix}.remove_module_names"));
        self.insert_default_container(format!("{prefix}.module_aliases"));
        self.insert_default_container(format!("{prefix}.remove_module_aliases"));
        self.insert_default_container(format!("{prefix}.device_compatibles"));
        self.insert_default_container(format!("{prefix}.remove_device_compatibles"));
        self.insert_default_container(format!("{prefix}.acpi_ids"));
        self.insert_default_container(format!("{prefix}.remove_acpi_ids"));
        self.insert_default_container(format!("{prefix}.pci_ids"));
        self.insert_default_container(format!("{prefix}.remove_pci_ids"));
        self.insert_default_container(format!("{prefix}.usb_ids"));
        self.insert_default_container(format!("{prefix}.remove_usb_ids"));
        self.insert_default_container(format!("{prefix}.firmware_paths"));
        self.insert_default_container(format!("{prefix}.remove_firmware_paths"));
        self.insert_default_container(format!("{prefix}.initcalls"));
        self.insert_default_container(format!("{prefix}.remove_initcalls"));
        self.insert_default_container(format!("{prefix}.runtime_registrations"));
        self.insert_default_container(format!("{prefix}.remove_runtime_registrations"));
        self.insert_default_container(format!("{prefix}.docs"));
        self.insert_default_container(format!("{prefix}.remove_docs"));
        self.insert_default_container(format!("{prefix}.tools"));
        self.insert_default_container(format!("{prefix}.remove_tools"));
        self.insert_default_container(format!("{prefix}.samples"));
        self.insert_default_container(format!("{prefix}.remove_samples"));
        self.insert_default_container(format!("{prefix}.kunit_suites"));
        self.insert_default_container(format!("{prefix}.remove_kunit_suites"));
        self.insert_default_container(format!("{prefix}.kselftest_targets"));
        self.insert_default_container(format!("{prefix}.remove_kselftest_targets"));
        self.insert_default(format!("{prefix}.allow_public_header_removal"));
        self.insert_default(format!("{prefix}.allow_uapi_header_removal"));
        self.insert_default_container(format!("{prefix}.arch_scope"));
        self.insert_default(format!("{prefix}.kind"));
        self.insert_default(format!("{prefix}.safety"));
        self.insert_default(format!("{prefix}.preserve_uapi"));
        self.insert_default(format!("{prefix}.preserve_module_aliases"));
        self.insert_default(format!("{prefix}.require_clean_boot"));
        self.insert_default(format!("{prefix}.report_only"));
    }

    fn record_build_matrix_defaults(&mut self) {
        self.insert_default("build_matrix.enabled");
        self.insert_default_container("build_matrix.presets");
        self.insert_default_container("build_matrix.arches");
        self.insert_default_container("build_matrix.config_targets");
        self.insert_default_container("build_matrix.targets");
        self.insert_default("build_matrix.randconfig_seed");
        self.insert_default("build_matrix.jobs");
        self.insert_default("build_matrix.fail_on_error");
    }

    fn record_runtime_matrix_defaults(&mut self) {
        self.insert_default("runtime_matrix.enabled");
        self.insert_default_container("runtime_matrix.boot_arches");
        self.insert_default_container("runtime_matrix.qemu_machines");
        self.insert_default_container("runtime_matrix.kunit_suites");
        self.insert_default_container("runtime_matrix.kselftest_targets");
        self.insert_default("runtime_matrix.module_smoke");
        self.insert_default("runtime_matrix.require_clean_dmesg");
        self.insert_default("runtime_matrix.boot_timeout_seconds");
        self.insert_default("runtime_matrix.fail_on_error");
    }

    fn record_report_defaults(&mut self) {
        self.insert_default_array("reports.formats", 3);
        self.insert_default("reports.include_edit_records");
        self.insert_default("reports.include_diagnostics");
        self.insert_default("reports.include_source_map");
        self.insert_default("reports.redact_host_paths");
        self.insert_default("reports.include_raw_logs");
        self.insert_default("reports.fail_on_error");
    }

    fn record_security_defaults(&mut self) {
        self.insert_default("security.allow_network");
        self.insert_default("security.require_local_upstream");
        self.insert_default("security.reject_host_paths_in_committed_metadata");
        self.insert_default("security.reject_temp_paths_in_committed_metadata");
        self.insert_default("security.reject_raw_logs_in_committed_metadata");
        self.insert_default("security.require_reproducible_timestamps");
        self.insert_default("security.require_phase_typed_metadata");
        self.insert_default("security.compatibility_mode");
        self.insert_default("security.fail_on_policy_violation");
    }

    fn record_performance_defaults(&mut self) {
        self.insert_default("performance.enabled");
        self.insert_default("performance.max_worker_threads");
        self.insert_default("performance.max_io_threads");
        self.insert_default("performance.cache_tree_index");
        self.insert_default("performance.incremental_reindex");
        self.insert_default("performance.collect_timing_metrics");
        self.insert_default("performance.profile_hot_paths");
        self.insert_default("performance.fail_on_regression");
    }

    fn record_patch_defaults(&mut self, prefix: &str, patches: &PatchConfig) {
        match patches {
            PatchConfig::Single(source) => self.record_patch_source_defaults(prefix, source),
            PatchConfig::Multi(config) => {
                self.insert_default_container(format!("{prefix}.sources"));
                for (idx, source) in config.sources.iter().enumerate() {
                    self.record_patch_source_defaults(&format!("{prefix}.sources[{idx}]"), source);
                }
            }
        }
    }

    fn record_patch_source_defaults(&mut self, prefix: &str, _source: &PatchSourceConfig) {
        self.insert_default(format!("{prefix}.source"));
        self.insert_default(format!("{prefix}.base_remote"));
        self.insert_default(format!("{prefix}.base_ref"));
        self.insert_default(format!("{prefix}.require_clean"));
    }

    fn record_integration_defaults(&mut self, profile: &ProfileConfig) {
        if profile.integrations.rtlmq.is_some() {
            self.insert_default("integrations.rtlmq.tests_source");
        } else {
            self.insert_default_container("integrations.rtlmq");
        }
    }

    fn record_selftest_defaults(&mut self, profile: &ProfileConfig) {
        self.insert_default("selftests.enabled");
        self.insert_default("selftests.check_kconfig_sources");
        self.insert_default("selftests.check_makefiles");
        self.insert_default_container("selftests.kernel_builds");
        for (idx, _build) in profile.selftests.kernel_builds.iter().enumerate() {
            let prefix = format!("selftests.kernel_builds[{idx}]");
            self.insert_default(format!("{prefix}.name"));
            self.insert_default(format!("{prefix}.config_target"));
            self.insert_default_container(format!("{prefix}.targets"));
            self.insert_default(format!("{prefix}.output_dir"));
            self.insert_default(format!("{prefix}.jobs"));
            self.insert_default(format!("{prefix}.clean"));
            self.insert_default(format!("{prefix}.make_program"));
            self.insert_default_container(format!("{prefix}.make_args"));
            self.insert_default_container(format!("{prefix}.env"));
        }
        self.insert_default_container("selftests.commands");
    }

    fn insert_default(&mut self, path: impl Into<String>) {
        let path = path.into();
        if !self.contains_value(&path) {
            self.insert(path, ConfigSourceKind::Default, DEFAULT_SOURCE);
        }
    }

    fn insert_default_container(&mut self, path: impl Into<String>) {
        let path = path.into();
        if !self.contains_path_or_descendant(&path) {
            self.insert(path, ConfigSourceKind::Default, DEFAULT_SOURCE);
        }
    }

    fn insert_default_array(&mut self, path: impl Into<String>, len: usize) {
        let path = path.into();
        if self.contains_path_or_descendant(&path) {
            return;
        }
        self.insert(path.clone(), ConfigSourceKind::Default, DEFAULT_SOURCE);
        for idx in 0..len {
            self.insert(
                format!("{path}[{idx}]"),
                ConfigSourceKind::Default,
                DEFAULT_SOURCE,
            );
        }
    }

    fn contains_path_or_descendant(&self, path: &str) -> bool {
        self.values.contains_key(path)
            || self.values.keys().any(|value_path| {
                value_path.starts_with(&format!("{path}."))
                    || value_path.starts_with(&format!("{path}["))
            })
    }
}

#[derive(Debug, Clone)]
pub struct LoadedKslimConfig {
    pub config: KslimConfig,
    pub source_map: ConfigSourceMap,
}

#[derive(Debug, Clone)]
pub struct LoadedProfileConfig {
    pub profile: ProfileConfig,
    pub source_map: ConfigSourceMap,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct SourceKindFixture {
        kind: ConfigSourceKind,
    }

    #[test]
    fn config_source_kind_stable_names_cover_all_variants() {
        let cases = [
            (ConfigSourceKind::Default, "default"),
            (ConfigSourceKind::ConfigFile, "config_file"),
            (ConfigSourceKind::Profile, "profile"),
            (ConfigSourceKind::IncludeFile, "include_file"),
            (ConfigSourceKind::Environment, "environment"),
            (ConfigSourceKind::Cli, "cli"),
        ];

        for (kind, name) in cases {
            assert_eq!(kind.as_str(), name);
            assert_eq!(
                toml::to_string(&SourceKindFixture { kind }).unwrap(),
                format!("kind = \"{name}\"\n")
            );
        }
    }

    #[test]
    fn source_map_records_scalar_array_and_array_table_values() {
        let map = ConfigSourceMap::from_toml_document(
            ConfigSourceKind::Profile,
            "profiles/default.toml",
            r#"
[profile]
name = "default"

[slim]
remove_paths = ["drivers/gpu", "net/bluetooth"]

[[selftests.kernel_builds]]
name = "tiny"
targets = ["vmlinux"]
env = { ARCH = "x86" }
"#,
        )
        .unwrap();

        assert_eq!(
            map.get("profile.name").map(|source| source.kind),
            Some(ConfigSourceKind::Profile)
        );
        assert_eq!(
            map.get("profile.name").map(|source| source.source.as_str()),
            Some("profiles/default.toml")
        );
        assert!(map.contains_value("slim.remove_paths"));
        assert!(map.contains_value("slim.remove_paths[0]"));
        assert!(map.contains_value("slim.remove_paths[1]"));
        assert!(map.contains_value("selftests.kernel_builds"));
        assert!(map.contains_value("selftests.kernel_builds[0].name"));
        assert!(map.contains_value("selftests.kernel_builds[0].targets[0]"));
        assert!(map.contains_value("selftests.kernel_builds[0].env.ARCH"));
    }

    #[test]
    fn source_map_records_include_file_and_environment_source_kinds() {
        let include_map = ConfigSourceMap::from_include_file_document(
            "profiles/includes/bluetooth.toml",
            r#"
[features.remove.bluetooth]
roots = ["net/bluetooth"]
"#,
        )
        .unwrap();

        assert_eq!(
            include_map
                .get("features.remove.bluetooth.roots")
                .map(|source| source.kind),
            Some(ConfigSourceKind::IncludeFile)
        );
        assert_eq!(
            include_map
                .get("features.remove.bluetooth.roots[0]")
                .map(|source| source.source.as_str()),
            Some("profiles/includes/bluetooth.toml")
        );

        let mut map = ConfigSourceMap::default();
        map.insert_environment_override("base.ref", "KSLIM_BASE");
        map.insert_include_file_value(
            "features.remove.wifi.roots[0]",
            "profiles/includes/wifi.toml",
        );

        assert_eq!(
            map.get("base.ref").map(|source| source.kind),
            Some(ConfigSourceKind::Environment)
        );
        assert_eq!(
            map.get("base.ref").map(|source| source.source.as_str()),
            Some("KSLIM_BASE")
        );
        assert_eq!(
            map.get("features.remove.wifi.roots[0]")
                .map(|source| source.kind),
            Some(ConfigSourceKind::IncludeFile)
        );
    }

    #[test]
    fn kslim_source_map_records_default_values_without_overwriting_explicit_values() {
        let contents = r#"
[project]
name = "demo"

[upstream]
name = "linux"
url = "/tmp/linux.git"

[output]
path = "/tmp/output"
"#;
        let config: KslimConfig = toml::from_str(contents).unwrap();
        let map =
            ConfigSourceMap::from_kslim_config_document("kslim.toml", contents, &config).unwrap();

        assert_eq!(
            map.get("project.name").map(|source| source.kind),
            Some(ConfigSourceKind::ConfigFile)
        );
        assert_eq!(
            map.get("output.branch_prefix").map(|source| source.kind),
            Some(ConfigSourceKind::Default)
        );
        assert_eq!(
            map.get("upstream.mode").map(|source| source.kind),
            Some(ConfigSourceKind::Default)
        );
        assert_eq!(
            map.get("git.user_email")
                .map(|source| source.source.as_str()),
            Some(DEFAULT_SOURCE)
        );
        assert_eq!(
            map.get("git.remote_name").map(|source| source.kind),
            Some(ConfigSourceKind::Default)
        );
    }

    #[test]
    fn profile_source_map_records_defaults_and_feature_derived_defaults() {
        let contents = r#"
[profile]
name = "default"

[base]
ref = "v1.0"

[features.remove.bluetooth]
roots = ["net/bluetooth"]

[reports]
formats = ["json"]
"#;
        let profile: ProfileConfig = toml::from_str(contents).unwrap();
        let map = ConfigSourceMap::from_profile_config_document(
            "profiles/default.toml",
            contents,
            &profile,
        )
        .unwrap();

        assert_eq!(
            map.get("base.ref").map(|source| source.kind),
            Some(ConfigSourceKind::Profile)
        );
        assert_eq!(
            map.get("profile.description").map(|source| source.kind),
            Some(ConfigSourceKind::Default)
        );
        assert_eq!(
            map.get("profile.inherits").map(|source| source.kind),
            Some(ConfigSourceKind::Default)
        );
        assert_eq!(
            map.get("slim").map(|source| source.kind),
            Some(ConfigSourceKind::Default)
        );
        assert_eq!(
            map.get("features.remove.bluetooth.roots")
                .map(|source| source.kind),
            Some(ConfigSourceKind::Profile)
        );
        assert_eq!(
            map.get("features.remove.bluetooth.safety")
                .map(|source| source.kind),
            Some(ConfigSourceKind::Default)
        );
        assert_eq!(
            map.get("features.remove.bluetooth.report_only")
                .map(|source| source.kind),
            Some(ConfigSourceKind::Default)
        );
        assert_eq!(
            map.get("reports.formats").map(|source| source.kind),
            Some(ConfigSourceKind::Profile)
        );
        assert_eq!(
            map.get("reports.include_source_map")
                .map(|source| source.kind),
            Some(ConfigSourceKind::Default)
        );
        assert_eq!(
            map.get("arch.primary_arch").map(|source| source.kind),
            Some(ConfigSourceKind::Default)
        );
        assert_eq!(
            map.get("build_matrix.jobs").map(|source| source.kind),
            Some(ConfigSourceKind::Default)
        );
        assert_eq!(
            map.get("integrations.rtlmq").map(|source| source.kind),
            Some(ConfigSourceKind::Default)
        );
        assert_eq!(
            map.get("selftests.enabled").map(|source| source.kind),
            Some(ConfigSourceKind::Default)
        );
    }

    #[test]
    fn profile_source_map_records_profile_inheritance_source() {
        let contents = r#"
[profile]
name = "child"
inherits = "base"

[base]
ref = "v1.0"
"#;
        let profile: ProfileConfig = toml::from_str(contents).unwrap();
        let map = ConfigSourceMap::from_profile_config_document(
            "profiles/child.toml",
            contents,
            &profile,
        )
        .unwrap();

        assert_eq!(
            map.get("profile.inherits").map(|source| source.kind),
            Some(ConfigSourceKind::Profile)
        );
        assert_eq!(
            map.get("profile.inherits")
                .map(|source| source.source.as_str()),
            Some("profiles/child.toml")
        );
    }
}
