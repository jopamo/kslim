//! Shared validation helpers for split value-model modules.

use anyhow::Result;
use std::path::{Component, Path};

use crate::path_policy::{path_contains_parent_traversal, path_is_absolute_like, path_is_empty_like};

pub(super) fn non_empty_model_value(label: &str, value: impl Into<String>) -> Result<String> {
    let value = value.into();
    if value.trim().is_empty() {
        anyhow::bail!("{label} must not be empty");
    }
    Ok(value)
}

pub(super) fn is_c_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

pub(super) fn is_c_identifier_continue(ch: char) -> bool {
    is_c_identifier_start(ch) || ch.is_ascii_digit()
}

pub(super) fn is_c_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    is_c_identifier_start(first) && chars.all(is_c_identifier_continue)
}

pub(super) fn is_module_name_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

pub(super) fn is_module_name_continue(ch: char) -> bool {
    is_module_name_start(ch) || ch == '-'
}

pub(super) fn is_module_alias_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '*' | '?' | ':' | '.' | ',' | '_' | '-' | '+' | '=')
}

pub(super) fn is_kunit_suite_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.')
}

pub(super) fn is_kselftest_target_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/')
}

pub(super) fn is_device_compatible_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, ',' | '.' | '_' | '+' | '-' | '/')
}

pub(super) fn is_acpi_id_char(ch: char) -> bool {
    ch.is_ascii_uppercase() || ch.is_ascii_digit()
}

pub(super) fn is_upper_hex_digit(ch: char) -> bool {
    ch.is_ascii_digit() || matches!(ch, 'A'..='F')
}

pub(super) fn source_file_path_matches(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("c" | "S" | "rs")
    )
}

pub(super) fn normalized_relative_model_path_parts(label: &str, path: &Path) -> Result<Vec<String>> {
    normalized_relative_model_path_parts_against(label, path, "kernel tree")
}

pub(super) fn normalized_relative_model_path_parts_against(
    label: &str,
    path: &Path,
    base_label: &str,
) -> Result<Vec<String>> {
    if path_is_empty_like(path) {
        anyhow::bail!("{label} must not be empty");
    }
    if path_contains_parent_traversal(path) {
        anyhow::bail!("{label} must not contain '..': {}", path.display());
    }
    if path_is_absolute_like(path) {
        anyhow::bail!(
            "{label} must be relative to the {base_label}: {}",
            path.display()
        );
    }

    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => {
                let Some(part) = part.to_str() else {
                    anyhow::bail!("{label} contains non-UTF-8 component");
                };
                if part.chars().any(char::is_whitespace) {
                    anyhow::bail!("{label} contains whitespace: {}", path.display());
                }
                if part.contains('\\') {
                    anyhow::bail!("{label} contains invalid separator: {}", path.display());
                }
                parts.push(part.to_string());
            }
            Component::ParentDir => {
                anyhow::bail!("{label} must not contain '..': {}", path.display());
            }
            Component::RootDir | Component::Prefix(_) => {
                anyhow::bail!(
                    "{label} must be relative to the {base_label}: {}",
                    path.display()
                );
            }
        }
    }

    if parts.is_empty() {
        anyhow::bail!(
            "{label} must not resolve to the {base_label} root: {}",
            path.display()
        );
    }

    Ok(parts)
}

pub(super) fn uapi_path_parts_match(parts: &[String]) -> bool {
    match parts {
        [include, uapi, ..] if include == "include" && uapi == "uapi" => true,
        [include, generated, uapi, ..]
            if include == "include" && generated == "generated" && uapi == "uapi" =>
        {
            true
        }
        [arch, _arch_name, include, uapi, ..]
            if arch == "arch" && include == "include" && uapi == "uapi" =>
        {
            true
        }
        [arch, _arch_name, include, generated, uapi, ..]
            if arch == "arch"
                && include == "include"
                && generated == "generated"
                && uapi == "uapi" =>
        {
            true
        }
        _ => false,
    }
}

pub(super) fn generated_artifact_path_parts_match(parts: &[String]) -> bool {
    match parts {
        [include, generated] if include == "include" && generated == "generated" => true,
        [include, generated, child, ..]
            if include == "include" && generated == "generated" && child != "uapi" =>
        {
            true
        }
        [include, config, ..] if include == "include" && config == "config" => true,
        [arch, _arch_name, include, generated]
            if arch == "arch" && include == "include" && generated == "generated" =>
        {
            true
        }
        [arch, _arch_name, include, generated, child, ..]
            if arch == "arch"
                && include == "include"
                && generated == "generated"
                && child != "uapi" =>
        {
            true
        }
        [artifact] => matches!(
            artifact.as_str(),
            ".config" | "Module.symvers" | "modules.order" | "System.map" | "vmlinux" | "vmlinux.o"
        ),
        _ => false,
    }
}

pub(super) fn documentation_path_parts_match(parts: &[String]) -> bool {
    matches!(parts, [documentation, ..] if documentation == "Documentation")
}

pub(super) fn tool_path_parts_match(parts: &[String]) -> bool {
    matches!(parts, [tools, ..] if tools == "tools")
}

pub(super) fn sample_path_parts_match(parts: &[String]) -> bool {
    matches!(parts, [samples, ..] if samples == "samples")
}
