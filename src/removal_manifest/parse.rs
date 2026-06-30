use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::abi::AbiPolicyConfig;
use crate::config::{ArchPolicyConfig, FeaturePreservationInput, SlimConfig};
use crate::generated::normalize_generated_include_roots;
use crate::model::KconfigSymbol;
use crate::path_policy::{normalized_relative_path_covers, path_is_normalized_tree_root};
use crate::paths::RelativeKernelPath;

use super::match_rules::{
    derive_removed_device_binding_proofs, derive_removed_exported_symbol_proofs,
    derive_removed_headers, derive_removed_kbuild_objects, derive_removed_kconfig_sources,
    derive_removed_public_headers, derive_removed_runtime_registration_proofs,
};
use super::model::{RemovalKey, RemovalManifest, RemovalReason};
use super::validate::{
    abi_sensitive_path_requires_own_manifest_truth, derive_removed_path_categories,
    normalize_declared_path, validate_declared_abi_removal_policy,
};

impl RemovalManifest {
    #[allow(dead_code)]
    pub fn from_slim_config(slim: &SlimConfig) -> Result<Self> {
        Self::from_slim_config_with_abi_policy(slim, &AbiPolicyConfig::default())
    }

    pub fn from_slim_config_with_abi_policy(
        slim: &SlimConfig,
        abi_policy: &AbiPolicyConfig,
    ) -> Result<Self> {
        Self::from_slim_config_with_root(slim, None, &[], abi_policy, None)
    }

    pub fn from_slim_config_with_abi_policy_and_preservation(
        slim: &SlimConfig,
        preservation: Option<&FeaturePreservationInput>,
        abi_policy: &AbiPolicyConfig,
    ) -> Result<Self> {
        Self::from_slim_config_with_root(slim, None, &[], abi_policy, preservation)
    }

    #[allow(dead_code)]
    pub fn from_slim_config_for_tree(root: &Path, slim: &SlimConfig) -> Result<Self> {
        Self::from_slim_config_for_tree_with_abi_policy(root, slim, &AbiPolicyConfig::default())
    }

    pub fn from_slim_config_for_tree_with_abi_policy(
        root: &Path,
        slim: &SlimConfig,
        abi_policy: &AbiPolicyConfig,
    ) -> Result<Self> {
        Self::from_slim_config_with_root(slim, Some(root), &[], abi_policy, None)
    }

    pub fn from_slim_config_for_tree_with_abi_policy_and_preservation(
        root: &Path,
        slim: &SlimConfig,
        preservation: Option<&FeaturePreservationInput>,
        abi_policy: &AbiPolicyConfig,
    ) -> Result<Self> {
        Self::from_slim_config_with_root(slim, Some(root), &[], abi_policy, preservation)
    }

    #[allow(dead_code)]
    pub fn from_slim_config_for_tree_with_generated_include_roots(
        root: &Path,
        slim: &SlimConfig,
        generated_include_roots: &[PathBuf],
    ) -> Result<Self> {
        Self::from_slim_config_with_root(
            slim,
            Some(root),
            generated_include_roots,
            &AbiPolicyConfig::default(),
            None,
        )
    }

    #[allow(dead_code)]
    pub fn from_slim_config_for_tree_with_generated_include_roots_and_abi_policy(
        root: &Path,
        slim: &SlimConfig,
        generated_include_roots: &[PathBuf],
        abi_policy: &AbiPolicyConfig,
    ) -> Result<Self> {
        Self::from_slim_config_with_root(
            slim,
            Some(root),
            generated_include_roots,
            abi_policy,
            None,
        )
    }

    fn from_slim_config_with_root(
        slim: &SlimConfig,
        root: Option<&Path>,
        generated_include_roots: &[PathBuf],
        abi_policy: &AbiPolicyConfig,
        preservation: Option<&FeaturePreservationInput>,
    ) -> Result<Self> {
        let generated_include_roots = normalize_generated_include_roots(generated_include_roots)?;
        let (preserved_paths, preserved_config_symbols) =
            normalize_preservation_input(preservation)?;
        let mut seen_paths = BTreeMap::<PathBuf, bool>::new();

        for (idx, raw_path) in slim.remove_paths.iter().enumerate() {
            let declared = normalize_declared_path(raw_path, slim.unsafe_allow_root_path_removal)
                .with_context(|| format!("invalid slim.remove_paths[{idx}]"))?;
            seen_paths
                .entry(declared.path)
                .and_modify(|existing| *existing |= declared.declared_directory)
                .or_insert(declared.declared_directory);
        }
        if seen_paths
            .keys()
            .any(|path| path_is_normalized_tree_root(path))
        {
            seen_paths.retain(|path, _| path_is_normalized_tree_root(path));
        }
        let mut removed_paths = BTreeSet::new();
        let mut declared_dirs = BTreeSet::new();
        let mut reasons = BTreeMap::new();
        for (normalized, declared_directory) in seen_paths {
            validate_declared_abi_removal_policy(&normalized, abi_policy)?;
            if !removed_paths.iter().any(|parent: &PathBuf| {
                normalized_relative_path_covers(parent, &normalized)
                    && !abi_sensitive_path_requires_own_manifest_truth(parent, &normalized)
            }) {
                if declared_directory {
                    declared_dirs.insert(normalized.clone());
                }
                removed_paths.insert(normalized);
            }
        }
        reject_exact_path_preservation_conflicts(&removed_paths, &preserved_paths)?;
        let (removed_paths, declared_dirs) = expand_removed_paths_around_preserved_roots(
            root,
            removed_paths,
            declared_dirs,
            &preserved_paths,
        )?;
        for path in &removed_paths {
            reasons.insert(
                RemovalKey::Path(path.clone()),
                RemovalReason::SlimRemovePath { path: path.clone() },
            );
        }
        let (removed_dirs, removed_files) =
            derive_removed_path_categories(root, &removed_paths, &declared_dirs)?;
        for dir in &removed_dirs {
            reasons.insert(
                RemovalKey::Dir(dir.clone()),
                RemovalReason::SlimRemovePath { path: dir.clone() },
            );
        }
        for file in &removed_files {
            reasons.insert(
                RemovalKey::File(file.clone()),
                RemovalReason::SlimRemovePath { path: file.clone() },
            );
        }
        let removed_headers = derive_removed_headers(
            root,
            &removed_paths,
            &removed_dirs,
            &removed_files,
            &generated_include_roots,
        )?;
        for header in &removed_headers {
            reasons.insert(
                RemovalKey::Header(header.clone()),
                RemovalReason::SlimRemovePath {
                    path: header.as_path().to_path_buf(),
                },
            );
        }
        let removed_public_headers = derive_removed_public_headers(&removed_headers);
        for header in &removed_public_headers {
            reasons.insert(
                RemovalKey::PublicHeader(header.clone()),
                RemovalReason::SlimRemovePath {
                    path: header.as_path().to_path_buf(),
                },
            );
        }
        let removed_kconfig_sources =
            derive_removed_kconfig_sources(root, &removed_paths, &removed_dirs, &removed_files)?;
        for source in &removed_kconfig_sources {
            reasons.insert(
                RemovalKey::KconfigSource(source.clone()),
                RemovalReason::SlimRemovePath {
                    path: source.clone(),
                },
            );
        }
        let removed_kbuild_objects =
            derive_removed_kbuild_objects(root, &removed_paths, &removed_dirs, &removed_files)?;
        for object in &removed_kbuild_objects {
            reasons.insert(
                RemovalKey::KbuildObject(object.clone()),
                RemovalReason::SlimRemovePath {
                    path: PathBuf::from(object.as_str().trim_end_matches('/')),
                },
            );
        }
        let removed_exported_symbols = derive_removed_exported_symbol_proofs(
            root,
            &removed_paths,
            &removed_dirs,
            &removed_files,
        )?;
        let removed_device_bindings = derive_removed_device_binding_proofs(
            root,
            &removed_paths,
            &removed_dirs,
            &removed_files,
        )?;
        let removed_runtime_registrations = derive_removed_runtime_registration_proofs(
            root,
            &removed_paths,
            &removed_dirs,
            &removed_files,
        )?;

        let mut seen_symbols = BTreeSet::new();
        for symbol in &slim.remove_configs {
            if symbol.trim().is_empty() {
                anyhow::bail!("slim.remove_configs must not contain empty values");
            }
            KconfigSymbol::new(symbol.clone()).with_context(|| {
                format!("slim.remove_configs contains invalid Kconfig symbol '{symbol}'")
            })?;
            seen_symbols.insert(symbol.clone());
        }
        let removed_config_symbols = seen_symbols;
        reject_config_preservation_conflicts(&removed_config_symbols, &preserved_config_symbols)?;
        for symbol in &removed_config_symbols {
            reasons.insert(
                RemovalKey::ConfigSymbol(symbol.clone()),
                RemovalReason::SlimRemoveConfig {
                    symbol: symbol.clone(),
                },
            );
        }

        let mut default_overrides = BTreeMap::new();
        for (symbol, value) in &slim.set_defaults {
            if symbol.trim().is_empty() || value.trim().is_empty() {
                anyhow::bail!("slim.set_defaults must not contain empty symbols or values");
            }
            KconfigSymbol::new(symbol.clone()).with_context(|| {
                format!("slim.set_defaults contains invalid Kconfig symbol '{symbol}'")
            })?;
            if removed_config_symbols.contains(symbol) {
                anyhow::bail!(
                    "slim.set_defaults and slim.remove_configs both target '{}'; remove the symbol or override its default, not both",
                    symbol
                );
            }
            default_overrides.insert(symbol.clone(), value.clone());
            reasons.insert(
                RemovalKey::DefaultOverride(symbol.clone()),
                RemovalReason::SlimDefaultOverride {
                    symbol: symbol.clone(),
                    value: value.clone(),
                },
            );
        }

        Ok(Self {
            removed_paths,
            removed_dirs,
            removed_files,
            removed_headers,
            removed_public_headers,
            removed_config_symbols,
            removed_kconfig_sources,
            removed_kbuild_objects,
            removed_device_bindings,
            removed_exported_symbols,
            removed_runtime_registrations,
            abi_policy: abi_policy.clone(),
            arch_policy: ArchPolicyConfig::default(),
            unsafe_allow_root_path_removal: slim.unsafe_allow_root_path_removal,
            preserved_paths,
            preserved_config_symbols,
            reasons,
            default_overrides,
        })
    }
}

fn normalize_preservation_input(
    preservation: Option<&FeaturePreservationInput>,
) -> Result<(BTreeSet<PathBuf>, BTreeSet<String>)> {
    let mut preserved_paths = BTreeSet::new();
    let mut preserved_config_symbols = BTreeSet::new();

    let Some(preservation) = preservation else {
        return Ok((preserved_paths, preserved_config_symbols));
    };

    for raw_path in &preservation.preserve_paths {
        if raw_path.trim().is_empty() {
            anyhow::bail!("features.preserve roots must not contain empty values");
        }
        let path = RelativeKernelPath::new_for_explicit_unsafe_root_removal(raw_path.clone())
            .with_context(|| format!("features.preserve contains invalid root '{raw_path}'"))?;
        preserved_paths.insert(path.as_path().to_path_buf());
    }

    for symbol in &preservation.preserve_configs {
        if symbol.trim().is_empty() {
            anyhow::bail!("features.preserve configs must not contain empty values");
        }
        KconfigSymbol::new(symbol.clone()).with_context(|| {
            format!("features.preserve contains invalid Kconfig symbol '{symbol}'")
        })?;
        preserved_config_symbols.insert(symbol.clone());
    }

    Ok((preserved_paths, preserved_config_symbols))
}

fn reject_exact_path_preservation_conflicts(
    removed_paths: &BTreeSet<PathBuf>,
    preserved_paths: &BTreeSet<PathBuf>,
) -> Result<()> {
    if let Some(path) = removed_paths
        .iter()
        .find(|path| preserved_paths.contains(*path))
    {
        anyhow::bail!(
            "feature preservation conflicts with exact removal path '{}'",
            path.display()
        );
    }
    Ok(())
}

fn reject_config_preservation_conflicts(
    removed_config_symbols: &BTreeSet<String>,
    preserved_config_symbols: &BTreeSet<String>,
) -> Result<()> {
    if let Some(symbol) = removed_config_symbols
        .iter()
        .find(|symbol| preserved_config_symbols.contains(*symbol))
    {
        anyhow::bail!("feature preservation conflicts with removal of Kconfig symbol '{symbol}'");
    }
    Ok(())
}

fn expand_removed_paths_around_preserved_roots(
    root: Option<&Path>,
    removed_paths: BTreeSet<PathBuf>,
    declared_dirs: BTreeSet<PathBuf>,
    preserved_paths: &BTreeSet<PathBuf>,
) -> Result<(BTreeSet<PathBuf>, BTreeSet<PathBuf>)> {
    let Some(root) = root else {
        return Ok((removed_paths, declared_dirs));
    };
    if preserved_paths.is_empty() {
        return Ok((removed_paths, declared_dirs));
    }

    let mut expanded_paths = BTreeSet::new();
    let mut expanded_declared_dirs = BTreeSet::new();
    for path in removed_paths {
        append_removed_path_around_preserved_roots(
            root,
            &path,
            declared_dirs.contains(&path),
            preserved_paths,
            &mut expanded_paths,
            &mut expanded_declared_dirs,
        )?;
    }

    Ok((expanded_paths, expanded_declared_dirs))
}

fn append_removed_path_around_preserved_roots(
    root: &Path,
    relative: &Path,
    declared_directory: bool,
    preserved_paths: &BTreeSet<PathBuf>,
    expanded_paths: &mut BTreeSet<PathBuf>,
    expanded_declared_dirs: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    if preserved_paths
        .iter()
        .any(|preserved| normalized_relative_path_covers(preserved, relative))
    {
        return Ok(());
    }
    if !preserved_paths
        .iter()
        .any(|preserved| normalized_relative_path_covers(relative, preserved))
    {
        let relative = relative.to_path_buf();
        if declared_directory {
            expanded_declared_dirs.insert(relative.clone());
        }
        expanded_paths.insert(relative);
        return Ok(());
    }

    let absolute = if path_is_normalized_tree_root(relative) {
        root.to_path_buf()
    } else {
        root.join(relative)
    };
    let metadata = match std::fs::symlink_metadata(&absolute) {
        Ok(metadata) => metadata,
        Err(_) => {
            let relative = relative.to_path_buf();
            if declared_directory {
                expanded_declared_dirs.insert(relative.clone());
            }
            expanded_paths.insert(relative);
            return Ok(());
        }
    };
    if !metadata.file_type().is_dir() {
        expanded_paths.insert(relative.to_path_buf());
        return Ok(());
    }

    let mut children = std::fs::read_dir(&absolute)
        .with_context(|| format!("failed to read removed directory {}", relative.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("failed to read removed directory {}", relative.display()))?;
    children.sort_by_key(|entry| entry.file_name());

    for child in children {
        let child_path = child.path();
        let child_relative = child_path.strip_prefix(root).with_context(|| {
            format!(
                "failed to derive root-relative path for {}",
                child_path.display()
            )
        })?;
        append_removed_path_around_preserved_roots(
            root,
            child_relative,
            false,
            preserved_paths,
            expanded_paths,
            expanded_declared_dirs,
        )?;
    }

    Ok(())
}
