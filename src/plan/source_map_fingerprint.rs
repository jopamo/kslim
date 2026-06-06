use std::path::Path;

use crate::config;

use crate::state::RequestedGenerateState;
use super::{append_fingerprint_line, project_root_for_config, GeneratePlanSourceMaps};

pub(super) fn append_source_map_fingerprint_lines(
    out: &mut String,
    requested: &RequestedGenerateState,
    source_maps: Option<&GeneratePlanSourceMaps>,
) {
    let Some(source_maps) = source_maps else {
        append_fingerprint_line(out, "source_map.available", "false");
        return;
    };

    append_fingerprint_line(out, "source_map.available", "true");
    append_config_source_map_fingerprint_lines(out, requested, "config", &source_maps.config);
    append_config_source_map_fingerprint_lines(out, requested, "profile", &source_maps.profile);
    append_config_source_map_fingerprint_lines(out, requested, "overrides", &source_maps.overrides);
}

fn append_config_source_map_fingerprint_lines(
    out: &mut String,
    requested: &RequestedGenerateState,
    label: &str,
    source_map: &config::ConfigSourceMap,
) {
    append_fingerprint_line(
        out,
        &format!("source_map.{label}.entry_count"),
        &source_map.len().to_string(),
    );
    for (path, source) in source_map.iter() {
        let prefix = format!("source_map.{label}.{path}");
        append_fingerprint_line(out, &format!("{prefix}.kind"), source.kind.as_str());
        append_fingerprint_line(
            out,
            &format!("{prefix}.source"),
            &source_map_source_fingerprint_token(requested, &source.source),
        );
    }
}

fn source_map_source_fingerprint_token(requested: &RequestedGenerateState, source: &str) -> String {
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
    if source_path.is_absolute() {
        if let Ok(relative) = source_path.strip_prefix(&project_root) {
            return format!("project:{}", fingerprint_path(relative));
        }
        return String::from("<absolute-path>");
    }

    source.to_string()
}

fn paths_equal_to_string(path: &Path, source: &str) -> bool {
    path.to_string_lossy() == source
}

fn fingerprint_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
