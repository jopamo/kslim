//! Conservative device-binding removal proof.
//!
//! Removing a devicetree binding while live DTS/DTSI files or other schemas
//! still reference it leaves an ABI-facing hardware description dangling. This
//! scanner proves absence of live compatible-string and schema-path references,
//! and fails closed when a removed binding has no extractable compatible token.

use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::model::DeviceCompatible;
use crate::path_policy::normalized_relative_path_covers;

const BINDING_ROOT: &str = "Documentation/devicetree/bindings";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DeviceBindingRemovalProof {
    pub binding: PathBuf,
    pub compatible_strings: Vec<DeviceCompatible>,
    pub schema_references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RemovedDeviceBinding {
    binding: PathBuf,
    compatible_strings: Vec<DeviceCompatible>,
    schema_references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LiveDeviceBindingReference {
    file: PathBuf,
    line: usize,
    token: String,
    kind: DeviceBindingReferenceKind,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum DeviceBindingReferenceKind {
    DtsCompatible,
    SchemaCompatible,
    SchemaRef,
}

pub(crate) fn prove_removed_device_bindings_have_no_live_references(
    root: &Path,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> Result<BTreeSet<DeviceBindingRemovalProof>> {
    let files = tree_files(root)?;
    let mut removed_bindings = Vec::new();
    let mut live_reference_files = Vec::new();

    for relative in &files {
        let removed = path_is_removed(relative, removed_paths, removed_dirs, removed_files);
        if removed && is_device_binding_file(relative) {
            removed_bindings.push(scan_removed_device_binding(root, relative)?);
        } else if !removed && is_device_reference_file(relative) {
            live_reference_files.push(relative.clone());
        }
    }

    let mut proofs = BTreeSet::new();
    for binding in removed_bindings {
        let live_references = live_references_for_binding(root, &live_reference_files, &binding)?;
        if !live_references.is_empty() {
            anyhow::bail!(
                "device binding removal requires proof that no live DTS/DTSI/schema reference remains for {}; live reference(s): {}",
                binding.binding.display(),
                render_live_references(&live_references),
            );
        }
        proofs.insert(DeviceBindingRemovalProof {
            binding: binding.binding,
            compatible_strings: binding.compatible_strings,
            schema_references: binding.schema_references,
        });
    }

    Ok(proofs)
}

fn scan_removed_device_binding(root: &Path, relative: &Path) -> Result<RemovedDeviceBinding> {
    let content = std::fs::read_to_string(root.join(relative)).with_context(|| {
        format!(
            "failed to read removed device binding {}",
            relative.display()
        )
    })?;
    let compatible_strings = compatible_strings_in_content(&content)
        .into_iter()
        .collect::<Vec<_>>();
    if compatible_strings.is_empty() {
        anyhow::bail!(
            "device binding removal requires extractable compatible-string proof for {}",
            relative.display(),
        );
    }
    Ok(RemovedDeviceBinding {
        binding: relative.to_path_buf(),
        compatible_strings,
        schema_references: schema_reference_tokens(relative),
    })
}

fn live_references_for_binding(
    root: &Path,
    live_reference_files: &[PathBuf],
    binding: &RemovedDeviceBinding,
) -> Result<BTreeSet<LiveDeviceBindingReference>> {
    let mut references = BTreeSet::new();
    for relative in live_reference_files {
        let content = std::fs::read_to_string(root.join(relative)).with_context(|| {
            format!(
                "failed to read live DTS/DTSI/schema reference file while proving removed device binding {}",
                binding.binding.display()
            )
        })?;
        if is_dts_source_path(relative) {
            insert_compatible_references(
                relative,
                &content,
                &binding.compatible_strings,
                DeviceBindingReferenceKind::DtsCompatible,
                &mut references,
            );
        } else if is_device_binding_file(relative) {
            insert_compatible_references(
                relative,
                &content,
                &binding.compatible_strings,
                DeviceBindingReferenceKind::SchemaCompatible,
                &mut references,
            );
            insert_schema_ref_references(
                relative,
                &content,
                &binding.schema_references,
                &mut references,
            );
        }
    }
    Ok(references)
}

fn insert_compatible_references(
    relative: &Path,
    content: &str,
    compatible_strings: &[DeviceCompatible],
    kind: DeviceBindingReferenceKind,
    references: &mut BTreeSet<LiveDeviceBindingReference>,
) {
    for (line_idx, line) in content.lines().enumerate() {
        for compatible in compatible_strings {
            if line.contains(compatible.as_str()) {
                references.insert(LiveDeviceBindingReference {
                    file: relative.to_path_buf(),
                    line: line_idx + 1,
                    token: compatible.as_str().to_string(),
                    kind: kind.clone(),
                });
            }
        }
    }
}

fn insert_schema_ref_references(
    relative: &Path,
    content: &str,
    schema_references: &[String],
    references: &mut BTreeSet<LiveDeviceBindingReference>,
) {
    for (line_idx, line) in content.lines().enumerate() {
        for schema_ref in schema_references {
            if line.contains(schema_ref) {
                references.insert(LiveDeviceBindingReference {
                    file: relative.to_path_buf(),
                    line: line_idx + 1,
                    token: schema_ref.clone(),
                    kind: DeviceBindingReferenceKind::SchemaRef,
                });
            }
        }
    }
}

fn tree_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry.path().strip_prefix(root).with_context(|| {
            format!(
                "failed to derive root-relative device-binding scan path for {}",
                entry.path().display()
            )
        })?;
        files.push(relative.to_path_buf());
    }
    files.sort();
    Ok(files)
}

fn is_device_reference_file(path: &Path) -> bool {
    is_dts_source_path(path) || is_device_binding_file(path)
}

fn is_device_binding_file(path: &Path) -> bool {
    path.starts_with(BINDING_ROOT)
        && matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("yaml" | "yml" | "txt")
        )
}

fn is_dts_source_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("dts" | "dtsi" | "dtso")
    )
}

fn path_is_removed(
    path: &Path,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> bool {
    removed_paths.contains(path)
        || removed_files.contains(path)
        || removed_dirs
            .iter()
            .any(|dir| normalized_relative_path_covers(dir, path))
}

fn compatible_strings_in_content(content: &str) -> BTreeSet<DeviceCompatible> {
    let mut out = BTreeSet::new();
    let bytes = content.as_bytes();
    let mut offset = 0usize;

    while offset < bytes.len() {
        if !is_compatible_byte(bytes[offset]) {
            offset += 1;
            continue;
        }
        let start = offset;
        while offset < bytes.len() && is_compatible_byte(bytes[offset]) {
            offset += 1;
        }
        let token = &content[start..offset];
        if token.contains(',')
            && token
                .split_once(',')
                .is_some_and(|(vendor, device)| !vendor.is_empty() && !device.is_empty())
        {
            if let Ok(compatible) = DeviceCompatible::new(token) {
                out.insert(compatible);
            }
        }
    }

    out
}

fn is_compatible_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b',' | b'.' | b'_' | b'+' | b'-' | b'/')
}

fn schema_reference_tokens(binding: &Path) -> Vec<String> {
    let Some(relative) = binding
        .strip_prefix(BINDING_ROOT)
        .ok()
        .and_then(path_to_forward_slash_string)
    else {
        return Vec::new();
    };
    let relative = relative.trim_start_matches('/').to_string();
    vec![relative.clone(), format!("/schemas/{relative}")]
}

fn path_to_forward_slash_string(path: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        let std::path::Component::Normal(part) = component else {
            continue;
        };
        parts.push(part.to_str()?);
    }
    Some(parts.join("/"))
}

fn render_live_references(references: &BTreeSet<LiveDeviceBindingReference>) -> String {
    references
        .iter()
        .take(8)
        .map(|reference| {
            format!(
                "{}:{}:{}:{}",
                reference.file.display(),
                reference.line,
                reference.kind.as_str(),
                reference.token,
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

impl DeviceBindingReferenceKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::DtsCompatible => "dts_compatible",
            Self::SchemaCompatible => "schema_compatible",
            Self::SchemaRef => "schema_ref",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compatible_strings_in_content_collects_yaml_and_dts_tokens() {
        let tokens = compatible_strings_in_content(
            r#"
compatible:
  enum:
    - vendor,foo
    - "vendor,bar-v2"
node { compatible = "other,baz"; };
"#,
        );

        assert_eq!(
            tokens,
            BTreeSet::from([
                DeviceCompatible::new("other,baz").unwrap(),
                DeviceCompatible::new("vendor,bar-v2").unwrap(),
                DeviceCompatible::new("vendor,foo").unwrap(),
            ])
        );
    }

    #[test]
    fn test_prove_removed_binding_rejects_live_dts_reference() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "Documentation/devicetree/bindings/vendor/foo.yaml",
            "compatible:\n  const: vendor,foo\n",
        );
        write(
            root,
            "arch/arm/boot/dts/live.dts",
            "/ { compatible = \"vendor,foo\"; };\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from(
            "Documentation/devicetree/bindings/vendor/foo.yaml",
        )]);
        let removed_files = removed_paths.clone();

        let err = format!(
            "{:#}",
            prove_removed_device_bindings_have_no_live_references(
                root,
                &removed_paths,
                &BTreeSet::new(),
                &removed_files,
            )
            .unwrap_err()
        );

        assert!(err.contains("device binding removal requires proof"));
        assert!(err.contains("arch/arm/boot/dts/live.dts:1:dts_compatible:vendor,foo"));
    }

    #[test]
    fn test_prove_removed_binding_rejects_live_schema_reference() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "Documentation/devicetree/bindings/vendor/foo.yaml",
            "compatible:\n  const: vendor,foo\n",
        );
        write(
            root,
            "Documentation/devicetree/bindings/vendor/live.yaml",
            "$ref: /schemas/vendor/foo.yaml#\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from(
            "Documentation/devicetree/bindings/vendor/foo.yaml",
        )]);
        let removed_files = removed_paths.clone();

        let err = format!(
            "{:#}",
            prove_removed_device_bindings_have_no_live_references(
                root,
                &removed_paths,
                &BTreeSet::new(),
                &removed_files,
            )
            .unwrap_err()
        );

        assert!(err.contains("schema_ref:/schemas/vendor/foo.yaml"));
    }

    #[test]
    fn test_prove_removed_binding_allows_only_removed_references() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "Documentation/devicetree/bindings/vendor/foo.yaml",
            "compatible:\n  enum:\n    - vendor,foo\n",
        );
        write(
            root,
            "arch/arm/boot/dts/removed.dtsi",
            "/ { compatible = \"vendor,foo\"; };\n",
        );
        write(
            root,
            "arch/arm/boot/dts/live.dts",
            "/ { model = \"live\"; };\n",
        );
        let removed_paths = BTreeSet::from([
            PathBuf::from("Documentation/devicetree/bindings/vendor/foo.yaml"),
            PathBuf::from("arch/arm/boot/dts/removed.dtsi"),
        ]);
        let removed_files = removed_paths.clone();

        let proofs = prove_removed_device_bindings_have_no_live_references(
            root,
            &removed_paths,
            &BTreeSet::new(),
            &removed_files,
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([DeviceBindingRemovalProof {
                binding: PathBuf::from("Documentation/devicetree/bindings/vendor/foo.yaml"),
                compatible_strings: vec![DeviceCompatible::new("vendor,foo").unwrap()],
                schema_references: vec![
                    String::from("vendor/foo.yaml"),
                    String::from("/schemas/vendor/foo.yaml"),
                ],
            }])
        );
    }

    #[test]
    fn test_prove_removed_binding_rejects_unextractable_compatible_truth() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "Documentation/devicetree/bindings/vendor/generic.yaml",
            "description: generic binding without explicit compatible\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from(
            "Documentation/devicetree/bindings/vendor/generic.yaml",
        )]);
        let removed_files = removed_paths.clone();

        let err = format!(
            "{:#}",
            prove_removed_device_bindings_have_no_live_references(
                root,
                &removed_paths,
                &BTreeSet::new(),
                &removed_files,
            )
            .unwrap_err()
        );

        assert!(err.contains("extractable compatible-string proof"));
    }

    fn write(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }
}
