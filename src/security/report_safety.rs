//! Report and committed-metadata safety policy.

use anyhow::Result;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) const TEMPORARY_PATH_ERROR_HINT: &str =
    "temporary paths may appear only in non-authoritative attempt metadata";

pub(crate) fn validate_report_text_has_no_temporary_paths(
    artifact_label: &str,
    contents: &str,
    temporary_roots: &[&Path],
) -> Result<()> {
    for marker in temporary_path_markers(temporary_roots) {
        if contents.contains(&marker) {
            anyhow::bail!(
                "output candidate validation failed: {} contains temporary path '{}'; {}",
                artifact_label,
                marker,
                TEMPORARY_PATH_ERROR_HINT
            );
        }
    }
    Ok(())
}

pub(crate) fn temporary_path_markers(paths: &[&Path]) -> Vec<String> {
    let mut markers = BTreeSet::new();
    for path in paths {
        if path.as_os_str().is_empty() {
            continue;
        }
        markers.insert(path.to_string_lossy().to_string());
        if let Ok(canonical) = path.canonicalize() {
            markers.insert(canonical.to_string_lossy().to_string());
        }
    }
    markers
        .into_iter()
        .filter(|marker| !marker.trim().is_empty())
        .collect()
}

pub(crate) fn validate_report_text_has_no_host_absolute_paths(
    artifact_label: &str,
    contents: &str,
) -> Result<()> {
    if let Some(marker) = find_host_specific_absolute_path_marker(contents) {
        anyhow::bail!(
            "committed metadata {} contains host-only absolute path {:?}; host paths may appear only in non-authoritative attempt metadata",
            artifact_label,
            marker
        );
    }
    Ok(())
}

pub(crate) fn find_host_specific_absolute_path_marker(contents: &str) -> Option<String> {
    contents
        .split(is_host_path_token_boundary)
        .filter_map(host_path_marker_from_token)
        .next()
}

fn host_path_marker_from_token(token: &str) -> Option<String> {
    let token = trim_host_path_token(token);
    if token.is_empty() {
        return None;
    }
    if is_host_specific_absolute_path(token) {
        return Some(token.to_string());
    }
    if let Some((_, value)) = token.rsplit_once('=') {
        let value = trim_host_path_token(value);
        if is_host_specific_absolute_path(value) {
            return Some(value.to_string());
        }
    }
    if let Some(file_url) = token.find("file:").map(|index| &token[index..]) {
        if is_host_specific_absolute_path(file_url) {
            return Some(file_url.to_string());
        }
    }
    if !token.contains("://") {
        if let Some((_, value)) = token.rsplit_once(':') {
            let value = trim_host_path_token(value);
            if is_host_specific_absolute_path(value) {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn is_host_path_token_boundary(ch: char) -> bool {
    ch.is_whitespace()
        || matches!(
            ch,
            '"' | '\'' | '`' | ',' | ';' | '|'
        )
}

fn trim_host_path_token(token: &str) -> &str {
    let token = token.trim_matches(|ch: char| {
        matches!(
            ch,
            '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | ',' | ';'
        )
    });
    token.trim_end_matches(['.', ','])
}

pub(crate) fn is_host_specific_absolute_path(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }
    if starts_with_serialized_escape_prefix(value) {
        return false;
    }
    is_posix_absolute_path_like(value)
        || is_file_url_absolute_path_like(value)
        || is_windows_absolute_path_like(value)
}

fn starts_with_serialized_escape_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0] == b'\\'
        && bytes[1] == b'\\'
        && matches!(bytes[2], b'n' | b'r' | b't' | b'u' | b'"' | b'\'' | b'\\')
}

fn is_posix_absolute_path_like(value: &str) -> bool {
    if !Path::new(value).is_absolute() {
        return false;
    }

    let Some(first_component) = value
        .trim_start_matches('/')
        .split('/')
        .find(|component| !component.is_empty())
    else {
        return false;
    };

    let mut chars = first_component.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}

fn is_file_url_absolute_path_like(value: &str) -> bool {
    value
        .strip_prefix("file:")
        .is_some_and(|path| path.starts_with('/') || path.starts_with('\\'))
}

fn is_windows_absolute_path_like(value: &str) -> bool {
    let bytes = value.as_bytes();
    if is_windows_unc_absolute_path_like(value) {
        return true;
    }
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

fn is_windows_unc_absolute_path_like(value: &str) -> bool {
    if !(value.starts_with("\\\\") || value.starts_with("//")) {
        return false;
    }
    let rest = &value[2..];
    if rest.starts_with("?\\") || rest.starts_with("?/") {
        return true;
    }

    let mut components = rest
        .split(['\\', '/'])
        .filter(|component| !component.is_empty());
    let Some(server) = components.next() else {
        return false;
    };
    let Some(share) = components.next() else {
        return false;
    };
    plausible_unc_component(server, true) && plausible_unc_component(share, false)
}

fn plausible_unc_component(component: &str, is_server: bool) -> bool {
    let mut chars = component.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if is_server {
        if !first.is_ascii_alphanumeric() {
            return false;
        }
        if !component.chars().any(|ch| ch.is_ascii_alphabetic() || ch == '.') {
            return false;
        }
    } else if !first.is_ascii_alphanumeric() && first != '$' {
        return false;
    }

    chars.all(|ch| {
        ch.is_ascii_alphanumeric()
            || matches!(ch, '.' | '-' | '_' | '$' | ' ')
    })
}

pub(crate) fn validate_report_text_has_no_raw_logs(
    artifact_label: &str,
    contents: &str,
) -> Result<()> {
    if let Some(marker) = raw_log_marker(contents) {
        anyhow::bail!(
            "committed metadata {} contains raw log marker {}; raw logs may appear only in non-authoritative attempt metadata or CI artifacts; committed metadata must use normalized summaries",
            artifact_label,
            marker
        );
    }
    Ok(())
}

pub(crate) fn raw_log_file_marker(relative_path: &str, file_name: &str) -> Option<String> {
    let file_name = file_name.to_ascii_lowercase();
    if file_name.ends_with(".log")
        || file_name.ends_with(".out")
        || file_name.ends_with(".err")
        || matches!(
            file_name.as_str(),
            "stdout" | "stdout.txt" | "stderr" | "stderr.txt"
        )
    {
        Some(relative_path.to_string())
    } else {
        None
    }
}

pub(crate) fn raw_log_marker(contents: &str) -> Option<String> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(contents) {
        if let Some(marker) = raw_log_marker_from_json(&value) {
            return Some(marker);
        }
    }
    if let Ok(value) = toml::from_str::<toml::Value>(contents) {
        if let Some(marker) = raw_log_marker_from_toml(&value) {
            return Some(marker);
        }
    }
    raw_log_marker_from_text(contents)
}

fn raw_log_marker_from_json(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(object) => object.iter().find_map(|(key, value)| {
            if is_raw_log_key(key) {
                Some(format!("{key:?}"))
            } else {
                raw_log_marker_from_json(value)
            }
        }),
        serde_json::Value::Array(values) => values.iter().find_map(raw_log_marker_from_json),
        _ => None,
    }
}

fn raw_log_marker_from_toml(value: &toml::Value) -> Option<String> {
    match value {
        toml::Value::Table(table) => table.iter().find_map(|(key, value)| {
            if is_raw_log_key(key) {
                Some(format!("{key:?}"))
            } else {
                raw_log_marker_from_toml(value)
            }
        }),
        toml::Value::Array(values) => values.iter().find_map(raw_log_marker_from_toml),
        _ => None,
    }
}

fn raw_log_marker_from_text(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let line = line.trim_start().to_ascii_lowercase();
        for label in [
            "stdout:",
            "stderr:",
            "raw diagnostic excerpt:",
            "raw_excerpt:",
            "raw_excerpts:",
        ] {
            if line.starts_with(label) {
                return Some(format!("{label:?}"));
            }
        }
    }
    None
}

fn is_raw_log_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "raw_excerpt"
            | "raw_excerpts"
            | "raw_diagnostic_excerpt"
            | "raw_diagnostic_excerpts"
            | "raw_diagnostics"
            | "raw_diagnostics_by_command"
            | "raw_log"
            | "raw_logs"
            | "stdout"
            | "stderr"
    )
}

pub(crate) fn validate_reproducible_timestamp(label: &str, value: &str) -> Result<()> {
    if !is_reproducible_timestamp(value) {
        anyhow::bail!(
            "{} must be a reproducible RFC3339 timestamp derived from the resolved base commit",
            label
        );
    }
    Ok(())
}

pub(crate) fn is_reproducible_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20 && bytes.len() != 25 {
        return false;
    }

    for index in [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18] {
        if !bytes[index].is_ascii_digit() {
            return false;
        }
    }

    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return false;
    }

    match bytes.len() {
        20 => bytes[19] == b'Z',
        25 => {
            (bytes[19] == b'+' || bytes[19] == b'-')
                && bytes[20].is_ascii_digit()
                && bytes[21].is_ascii_digit()
                && bytes[22] == b':'
                && bytes[23].is_ascii_digit()
                && bytes[24].is_ascii_digit()
        }
        _ => false,
    }
}

pub(crate) fn timestamp_markers(contents: &str) -> Vec<String> {
    let mut markers = BTreeSet::new();
    let bytes = contents.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if let Some(end) = timestamp_marker_end(bytes, index) {
            markers.insert(contents[index..end].to_string());
            index = end;
        } else {
            index += 1;
        }
    }
    markers.into_iter().collect()
}

fn timestamp_marker_end(bytes: &[u8], start: usize) -> Option<usize> {
    if start > 0 && bytes[start - 1].is_ascii_alphanumeric() {
        return None;
    }
    if start + 20 > bytes.len() || !timestamp_prefix_matches(bytes, start) {
        return None;
    }

    let mut zone_start = start + 19;
    if bytes.get(zone_start) == Some(&b'.') {
        let fraction_start = zone_start + 1;
        let mut fraction_end = fraction_start;
        while bytes.get(fraction_end).is_some_and(u8::is_ascii_digit) {
            fraction_end += 1;
        }
        if fraction_end == fraction_start {
            return None;
        }
        zone_start = fraction_end;
    }

    let end = match bytes.get(zone_start) {
        Some(b'Z') => zone_start + 1,
        Some(b'+') | Some(b'-')
            if zone_start + 6 <= bytes.len()
                && bytes[zone_start + 1].is_ascii_digit()
                && bytes[zone_start + 2].is_ascii_digit()
                && bytes[zone_start + 3] == b':'
                && bytes[zone_start + 4].is_ascii_digit()
                && bytes[zone_start + 5].is_ascii_digit() =>
        {
            zone_start + 6
        }
        _ => return None,
    };

    if bytes
        .get(end)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        return None;
    }

    Some(end)
}

fn timestamp_prefix_matches(bytes: &[u8], start: usize) -> bool {
    for offset in [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18] {
        if !bytes[start + offset].is_ascii_digit() {
            return false;
        }
    }
    bytes[start + 4] == b'-'
        && bytes[start + 7] == b'-'
        && bytes[start + 10] == b'T'
        && bytes[start + 13] == b':'
        && bytes[start + 16] == b':'
}
