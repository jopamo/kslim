use std::path::{Path, PathBuf};

use crate::{config, path_policy};

use crate::state::RequestedGenerateState;
use super::{project_root_for_config, GeneratePlanSourceMaps};

pub(super) fn without_temporary_workspace_or_host_paths(
    requested: &RequestedGenerateState,
    source_maps: GeneratePlanSourceMaps,
) -> GeneratePlanSourceMaps {
    GeneratePlanSourceMaps::new(
        sanitized_source_map(requested, &source_maps.config),
        sanitized_source_map(requested, &source_maps.profile),
        sanitized_source_map(requested, &source_maps.overrides),
    )
}

fn sanitized_source_map(
    requested: &RequestedGenerateState,
    source_map: &config::ConfigSourceMap,
) -> config::ConfigSourceMap {
    let mut sanitized = config::ConfigSourceMap::default();
    for (path, source) in source_map.iter() {
        sanitized.insert(
            path,
            source.kind,
            source_token_without_temporary_workspace_or_host_path(requested, &source.source),
        );
    }
    sanitized
}

fn source_token_without_temporary_workspace_or_host_path(
    requested: &RequestedGenerateState,
    source: &str,
) -> String {
    let requested_config_path = requested.config_path.as_path();
    if paths_equal_to_string(requested_config_path, source) {
        return String::from("<requested-config>");
    }

    let project_root = project_root_for_config(requested_config_path);
    let selected_profile_path = project_root
        .join("profiles")
        .join(format!("{}.toml", requested.selected_profile.as_str()));
    if paths_equal_to_string(selected_profile_path.as_path(), source) {
        return String::from("<selected-profile>");
    }

    let source_path = Path::new(source);
    if !path_policy::is_absolute_path_like(source) {
        return source.to_string();
    }

    if source_path.is_absolute() {
        if let Ok(relative) = source_path.strip_prefix(&project_root) {
            return format!("project:{}", fingerprint_path(relative));
        }

        if is_workspace_path(source_path) {
            return String::from("<workspace-path>");
        }

        if is_temporary_path(source_path) {
            return String::from("<temporary-path>");
        }
    }

    String::from("<host-absolute-path>")
}

fn paths_equal_to_string(path: &Path, source: &str) -> bool {
    path.to_string_lossy() == source
}

fn is_temporary_path(path: &Path) -> bool {
    temporary_roots().iter().any(|root| path.starts_with(root))
}

fn is_workspace_path(path: &Path) -> bool {
    workspace_roots().iter().any(|root| path.starts_with(root))
}

fn temporary_roots() -> Vec<PathBuf> {
    let mut roots = vec![
        std::env::temp_dir(),
        PathBuf::from("/tmp"),
        PathBuf::from("/var/tmp"),
    ];
    roots.sort();
    roots.dedup();
    roots
}

fn workspace_roots() -> Vec<PathBuf> {
    let mut roots = std::env::current_dir().ok().into_iter().collect::<Vec<_>>();
    roots.sort();
    roots.dedup();
    roots
}

fn fingerprint_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::super::{ConfigContentHash, GeneratePlan};
    use super::*;
    use crate::state::{CliOverrides, ProfileName, ResolvedCandidateState};
    use crate::lockfile::ResolvedBase;
    use crate::model::ToolVersion;
    use crate::paths::RequestedConfigPath;

    fn requested_state(config_path: &Path) -> RequestedGenerateState {
        RequestedGenerateState::new(
            RequestedConfigPath::new(config_path).unwrap(),
            ProfileName::new("default").unwrap(),
            CliOverrides {
                dry_run: false,
                deep_dry_run: false,
                report_only: false,
                force: false,
                offline: false,
                base_ref: None,
                feature: None,
                remove_feature: None,
                preserve_feature: None,
                arch: None,
                primary_arch: None,
                secondary_arch: None,
                safety: None,
                max_fixup_passes: None,
                matrix: None,
                strict: false,
                no_strict: false,
                run_selftests: true,
            },
        )
    }

    fn resolved_state() -> ResolvedCandidateState {
        let config = config::default_kslim_config("demo", "/var/lib/kslim-output");
        let profile = config::default_profile_config("v1.0");
        ResolvedCandidateState::from_resolved_inputs(
            &config,
            &profile,
            ResolvedBase {
                upstream: String::from("linux"),
                url: String::from("/var/lib/linux.git"),
                r#ref: String::from("v1.0"),
                commit: String::from("deadbeef"),
                resolved_at: String::from("2026-01-01T00:00:00Z"),
            },
            None,
            "unmodified-upstream",
            "kslim/v1.0/default",
        )
        .unwrap()
    }

    #[test]
    fn source_maps_stored_on_generate_plan_exclude_private_host_paths() {
        let temp_root = PathBuf::from("/var/tmp");
        let workspace_root = std::env::current_dir().unwrap();
        let project = temp_root.join("kslim-plan-temp-project");
        let requested = requested_state(&project.join("kslim.toml"));
        let mut config_map = config::ConfigSourceMap::default();
        config_map.insert(
            "project.name",
            config::ConfigSourceKind::ConfigFile,
            project.join("kslim.toml").to_string_lossy(),
        );
        config_map.insert(
            "include.extra",
            config::ConfigSourceKind::IncludeFile,
            project.join("includes/extra.toml").to_string_lossy(),
        );
        config_map.insert(
            "external.temp",
            config::ConfigSourceKind::IncludeFile,
            temp_root.join("outside-project.toml").to_string_lossy(),
        );
        config_map.insert(
            "external.workspace",
            config::ConfigSourceKind::IncludeFile,
            workspace_root
                .join("workspace-only-source.toml")
                .to_string_lossy(),
        );
        config_map.insert(
            "external.host",
            config::ConfigSourceKind::IncludeFile,
            "/opt/kslim-host-source.toml",
        );
        config_map.insert(
            "external.file_url",
            config::ConfigSourceKind::IncludeFile,
            "file:///var/lib/kslim/source.toml",
        );
        config_map.insert(
            "external.windows",
            config::ConfigSourceKind::IncludeFile,
            r"C:\kslim\source.toml",
        );
        let mut profile = config::ConfigSourceMap::default();
        profile.insert(
            "base.ref",
            config::ConfigSourceKind::Profile,
            project.join("profiles/default.toml").to_string_lossy(),
        );
        let mut overrides = config::ConfigSourceMap::default();
        overrides.insert_cli_override("base.ref", "cli --base");

        let plan = GeneratePlan::from_parts(
            requested,
            resolved_state(),
            ConfigContentHash::new("config-test").unwrap(),
            ToolVersion::new("test-tool").unwrap(),
        )
        .unwrap()
        .with_source_maps(GeneratePlanSourceMaps::new(config_map, profile, overrides))
        .unwrap();
        let sanitized = plan.source_maps.as_ref().unwrap();

        assert_eq!(
            sanitized.config.get("project.name").unwrap().source,
            "<requested-config>"
        );
        assert_eq!(
            sanitized.profile.get("base.ref").unwrap().source,
            "<selected-profile>"
        );
        assert_eq!(
            sanitized.config.get("include.extra").unwrap().source,
            "project:includes/extra.toml"
        );
        assert_eq!(
            sanitized.config.get("external.temp").unwrap().source,
            "<temporary-path>"
        );
        assert_eq!(
            sanitized.config.get("external.workspace").unwrap().source,
            "<workspace-path>"
        );
        assert_eq!(
            sanitized.config.get("external.host").unwrap().source,
            "<host-absolute-path>"
        );
        assert_eq!(
            sanitized.config.get("external.file_url").unwrap().source,
            "<host-absolute-path>"
        );
        assert_eq!(
            sanitized.config.get("external.windows").unwrap().source,
            "<host-absolute-path>"
        );
        assert_eq!(
            sanitized.overrides.get("base.ref").unwrap().source,
            "cli --base"
        );
        assert!(!format!("{sanitized:?}").contains(temp_root.to_string_lossy().as_ref()));
        assert!(!format!("{sanitized:?}").contains(workspace_root.to_string_lossy().as_ref()));
        assert!(!plan
            .fingerprint
            .stable_serialization()
            .contains(temp_root.to_string_lossy().as_ref()));
        assert!(!plan
            .fingerprint
            .stable_serialization()
            .contains(workspace_root.to_string_lossy().as_ref()));
        assert!(!plan
            .fingerprint
            .stable_serialization()
            .contains("/opt/kslim-host-source.toml"));
        assert!(!plan
            .fingerprint
            .stable_serialization()
            .contains("file:///var/lib/kslim/source.toml"));
        assert!(!plan
            .fingerprint
            .stable_serialization()
            .contains(r"C:\kslim\source.toml"));
    }
}
