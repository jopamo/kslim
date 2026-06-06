use anyhow::Result;
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::config::{AbiPolicyConfig, ArchPolicyConfig, ProfileConfig};
use crate::removal_manifest::{RemovalKey, RemovalManifest, RemovalReason};

use super::{bool_token, stable_plan_item_id, FeatureIntentPlan, FeatureResolutionState};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FeatureGraphFingerprint(String);

#[allow(dead_code)]
impl FeatureGraphFingerprint {
    pub(crate) fn new(fingerprint: impl Into<String>) -> Result<Self> {
        let fingerprint = fingerprint.into();
        if fingerprint.trim().is_empty() {
            anyhow::bail!("resolved feature graph fingerprint is empty");
        }
        Ok(Self(fingerprint))
    }

    pub(super) fn from_resolved_feature_graph(
        intent_plan: &FeatureIntentPlan,
        resolution: &FeatureResolutionState,
    ) -> Result<Self> {
        let mut hasher = Sha256::new();
        hash_feature_graph_line(&mut hasher, "format", "kslim-resolved-feature-graph-v1");
        hash_feature_graph_line(
            &mut hasher,
            "intent_count",
            &intent_plan.intents.len().to_string(),
        );
        for (idx, intent) in intent_plan.intents.iter().enumerate() {
            let prefix = format!("intents.{idx}");
            hash_feature_graph_line(
                &mut hasher,
                &format!("{prefix}.stable_id"),
                &intent.stable_id,
            );
            hash_feature_graph_line(&mut hasher, &format!("{prefix}.action"), &intent.action);
            hash_feature_graph_line(&mut hasher, &format!("{prefix}.name"), &intent.name);
            hash_feature_graph_line(
                &mut hasher,
                &format!("{prefix}.kind"),
                intent.kind.as_deref().unwrap_or("<none>"),
            );
            for path in &intent.roots {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.roots"),
                    &path.as_path().to_string_lossy(),
                );
            }
            for path in &intent.remove_paths {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_paths"),
                    &path.as_path().to_string_lossy(),
                );
            }
            for config in &intent.configs {
                hash_feature_graph_line(&mut hasher, &format!("{prefix}.configs"), config.as_str());
            }
            for config in &intent.remove_configs {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_configs"),
                    config.as_str(),
                );
            }
            for symbol in &intent.exported_symbols {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.exported_symbols"),
                    symbol.as_str(),
                );
            }
            for symbol in &intent.remove_exported_symbols {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_exported_symbols"),
                    symbol.as_str(),
                );
            }
            for module in &intent.module_names {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.module_names"),
                    module.as_str(),
                );
            }
            for module in &intent.remove_module_names {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_module_names"),
                    module.as_str(),
                );
            }
            for alias in &intent.module_aliases {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.module_aliases"),
                    alias.as_str(),
                );
            }
            for alias in &intent.remove_module_aliases {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_module_aliases"),
                    alias.as_str(),
                );
            }
            for compatible in &intent.device_compatibles {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.device_compatibles"),
                    compatible.as_str(),
                );
            }
            for compatible in &intent.remove_device_compatibles {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_device_compatibles"),
                    compatible.as_str(),
                );
            }
            for id in &intent.acpi_ids {
                hash_feature_graph_line(&mut hasher, &format!("{prefix}.acpi_ids"), id.as_str());
            }
            for id in &intent.remove_acpi_ids {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_acpi_ids"),
                    id.as_str(),
                );
            }
            for id in &intent.pci_ids {
                hash_feature_graph_line(&mut hasher, &format!("{prefix}.pci_ids"), id.as_str());
            }
            for id in &intent.remove_pci_ids {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_pci_ids"),
                    id.as_str(),
                );
            }
            for id in &intent.usb_ids {
                hash_feature_graph_line(&mut hasher, &format!("{prefix}.usb_ids"), id.as_str());
            }
            for id in &intent.remove_usb_ids {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_usb_ids"),
                    id.as_str(),
                );
            }
            for path in &intent.firmware_paths {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.firmware_paths"),
                    path.as_str(),
                );
            }
            for path in &intent.remove_firmware_paths {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_firmware_paths"),
                    path.as_str(),
                );
            }
            for initcall in &intent.initcalls {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.initcalls"),
                    initcall.as_str(),
                );
            }
            for initcall in &intent.remove_initcalls {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_initcalls"),
                    initcall.as_str(),
                );
            }
            for surface in &intent.runtime_registrations {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.runtime_registrations"),
                    surface.as_str(),
                );
            }
            for surface in &intent.remove_runtime_registrations {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_runtime_registrations"),
                    surface.as_str(),
                );
            }
            for path in &intent.docs {
                hash_feature_graph_line(&mut hasher, &format!("{prefix}.docs"), path.as_str());
            }
            for path in &intent.remove_docs {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_docs"),
                    path.as_str(),
                );
            }
            for path in &intent.tools {
                hash_feature_graph_line(&mut hasher, &format!("{prefix}.tools"), path.as_str());
            }
            for path in &intent.remove_tools {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_tools"),
                    path.as_str(),
                );
            }
            for path in &intent.samples {
                hash_feature_graph_line(&mut hasher, &format!("{prefix}.samples"), path.as_str());
            }
            for path in &intent.remove_samples {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_samples"),
                    path.as_str(),
                );
            }
            for suite in &intent.kunit_suites {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.kunit_suites"),
                    suite.as_str(),
                );
            }
            for suite in &intent.remove_kunit_suites {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_kunit_suites"),
                    suite.as_str(),
                );
            }
            for target in &intent.kselftest_targets {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.kselftest_targets"),
                    target.as_str(),
                );
            }
            for target in &intent.remove_kselftest_targets {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.remove_kselftest_targets"),
                    target.as_str(),
                );
            }
            hash_feature_graph_line(
                &mut hasher,
                &format!("{prefix}.allow_public_header_removal"),
                bool_token(intent.allow_public_header_removal),
            );
            hash_feature_graph_line(
                &mut hasher,
                &format!("{prefix}.allow_uapi_header_removal"),
                bool_token(intent.allow_uapi_header_removal),
            );
            for arch in &intent.arch_scope {
                hash_feature_graph_line(
                    &mut hasher,
                    &format!("{prefix}.arch_scope"),
                    arch.as_str(),
                );
            }
            hash_feature_graph_line(
                &mut hasher,
                &format!("{prefix}.safety"),
                intent
                    .safety
                    .map(|safety| safety.as_str())
                    .unwrap_or("<none>"),
            );
            hash_feature_graph_line(
                &mut hasher,
                &format!("{prefix}.preserve_uapi"),
                bool_token(intent.preserve_uapi),
            );
            hash_feature_graph_line(
                &mut hasher,
                &format!("{prefix}.preserve_module_aliases"),
                bool_token(intent.preserve_module_aliases),
            );
            hash_feature_graph_line(
                &mut hasher,
                &format!("{prefix}.require_clean_boot"),
                bool_token(intent.require_clean_boot),
            );
            hash_feature_graph_line(
                &mut hasher,
                &format!("{prefix}.report_only"),
                bool_token(intent.report_only),
            );
        }
        hash_feature_graph_line(
            &mut hasher,
            "resolution.source",
            resolution.source().stable_name(),
        );
        for path in resolution.remove_paths() {
            hash_feature_graph_line(
                &mut hasher,
                "resolution.remove_paths",
                &path.as_path().to_string_lossy(),
            );
        }
        for config in resolution.remove_configs() {
            hash_feature_graph_line(&mut hasher, "resolution.remove_configs", config.as_str());
        }
        for path in resolution.preserve_paths() {
            hash_feature_graph_line(
                &mut hasher,
                "resolution.preserve_paths",
                &path.as_path().to_string_lossy(),
            );
        }
        for config in resolution.preserve_configs() {
            hash_feature_graph_line(&mut hasher, "resolution.preserve_configs", config.as_str());
        }
        for (symbol, value) in resolution.set_defaults() {
            hash_feature_graph_line(
                &mut hasher,
                "resolution.set_defaults.symbol",
                symbol.as_str(),
            );
            hash_feature_graph_line(&mut hasher, "resolution.set_defaults.value", value);
        }
        for (feature, safety) in resolution.feature_safety_levels() {
            hash_feature_graph_line(&mut hasher, "resolution.feature_safety.feature", feature);
            hash_feature_graph_line(
                &mut hasher,
                "resolution.feature_safety.level",
                safety.as_str(),
            );
        }
        for (feature, scopes) in resolution.feature_arch_scopes() {
            hash_feature_graph_line(
                &mut hasher,
                "resolution.feature_arch_scope.feature",
                feature,
            );
            for arch in scopes {
                hash_feature_graph_line(
                    &mut hasher,
                    "resolution.feature_arch_scope.arch",
                    arch.as_str(),
                );
            }
        }
        for (feature, matrix) in resolution.feature_test_matrices() {
            hash_feature_graph_line(
                &mut hasher,
                "resolution.feature_test_matrix.feature",
                feature,
            );
            hash_feature_graph_line(
                &mut hasher,
                "resolution.feature_test_matrix.require_clean_boot",
                bool_token(matrix.require_clean_boot),
            );
        }
        for (feature, mode) in resolution.feature_report_modes() {
            hash_feature_graph_line(
                &mut hasher,
                "resolution.feature_report_mode.feature",
                feature,
            );
            hash_feature_graph_line(
                &mut hasher,
                "resolution.feature_report_mode.report_only",
                bool_token(mode.report_only),
            );
        }
        hash_feature_graph_line(
            &mut hasher,
            "resolution.abi.allow_public_header_removal",
            bool_token(resolution.abi_policy().allow_public_header_removal),
        );
        hash_feature_graph_line(
            &mut hasher,
            "resolution.abi.allow_uapi_header_removal",
            bool_token(resolution.abi_policy().allow_uapi_header_removal),
        );
        hash_feature_graph_line(
            &mut hasher,
            "resolution.unsafe_allow_root_path_removal",
            bool_token(resolution.unsafe_allow_root_path_removal()),
        );
        Self::new(format!("feature-graph-{}", hex::encode(hasher.finalize())))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

fn hash_feature_graph_line(hasher: &mut Sha256, key: &str, value: &str) {
    hasher.update(key.as_bytes());
    hasher.update(b"\0");
    hasher.update(value.len().to_string().as_bytes());
    hasher.update(b"\0");
    hasher.update(value.as_bytes());
    hasher.update(b"\0");
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RemovalManifestFingerprint(String);

#[allow(dead_code)]
impl RemovalManifestFingerprint {
    pub(crate) fn new(fingerprint: impl Into<String>) -> Result<Self> {
        let fingerprint = fingerprint.into();
        if fingerprint.trim().is_empty() {
            anyhow::bail!("removal manifest fingerprint is empty");
        }
        Ok(Self(fingerprint))
    }

    pub(super) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let slim = profile.effective_removal_input().unwrap_or_default();
        let preservation = profile.effective_preservation_input();
        let abi_policy = profile.effective_abi_policy();
        let manifest = RemovalManifest::from_slim_config_with_abi_policy_and_preservation(
            &slim,
            preservation.as_ref(),
            &abi_policy,
        )?;
        Self::from_manifest(&manifest)
    }

    fn from_manifest(manifest: &RemovalManifest) -> Result<Self> {
        let mut hasher = Sha256::new();
        hash_removal_manifest_line(&mut hasher, "format", "kslim-removal-manifest-v1");
        hash_removal_manifest_line(
            &mut hasher,
            "abi_policy.allow_public_header_removal",
            bool_token(manifest.abi_policy.allow_public_header_removal),
        );
        hash_removal_manifest_line(
            &mut hasher,
            "abi_policy.allow_uapi_header_removal",
            bool_token(manifest.abi_policy.allow_uapi_header_removal),
        );
        hash_removal_manifest_line(
            &mut hasher,
            "unsafe_allow_root_path_removal",
            bool_token(manifest.unsafe_allow_root_path_removal),
        );
        for path in manifest.removed_paths() {
            hash_removal_manifest_line(
                &mut hasher,
                "removed_paths",
                &removal_manifest_path_token(path),
            );
        }
        for path in &manifest.removed_dirs {
            hash_removal_manifest_line(
                &mut hasher,
                "removed_dirs",
                &removal_manifest_path_token(path),
            );
        }
        for path in &manifest.removed_files {
            hash_removal_manifest_line(
                &mut hasher,
                "removed_files",
                &removal_manifest_path_token(path),
            );
        }
        for header in &manifest.removed_headers {
            hash_removal_manifest_line(&mut hasher, "removed_headers", header.as_str());
        }
        for header in &manifest.removed_public_headers {
            hash_removal_manifest_line(&mut hasher, "removed_public_headers", header.as_str());
        }
        for symbol in manifest.removed_config_symbols() {
            hash_removal_manifest_line(&mut hasher, "removed_config_symbols", symbol);
        }
        for path in &manifest.removed_kconfig_sources {
            hash_removal_manifest_line(
                &mut hasher,
                "removed_kconfig_sources",
                &removal_manifest_path_token(path),
            );
        }
        for object in &manifest.removed_kbuild_objects {
            hash_removal_manifest_line(&mut hasher, "removed_kbuild_objects", object.as_str());
        }
        for proof in manifest.removed_device_bindings_vec() {
            hash_removal_manifest_line(
                &mut hasher,
                "removed_device_bindings.binding",
                &removal_manifest_path_token(&proof.binding),
            );
            for compatible in proof.compatible_strings {
                hash_removal_manifest_line(
                    &mut hasher,
                    "removed_device_bindings.compatible_strings",
                    compatible.as_str(),
                );
            }
            for reference in proof.schema_references {
                hash_removal_manifest_line(
                    &mut hasher,
                    "removed_device_bindings.schema_references",
                    &reference,
                );
            }
        }
        for proof in manifest.removed_exported_symbols_vec() {
            hash_removal_manifest_line(
                &mut hasher,
                "removed_exported_symbols.symbol",
                proof.symbol.as_str(),
            );
            hash_removal_manifest_line(
                &mut hasher,
                "removed_exported_symbols.provider",
                &removal_manifest_path_token(&proof.provider),
            );
            hash_removal_manifest_line(
                &mut hasher,
                "removed_exported_symbols.export_macro",
                &proof.export_macro,
            );
            hash_removal_manifest_line(
                &mut hasher,
                "removed_exported_symbols.line",
                &proof.line.to_string(),
            );
        }
        for proof in manifest.removed_runtime_registrations_vec() {
            hash_removal_manifest_line(
                &mut hasher,
                "removed_runtime_registrations.provider",
                &removal_manifest_path_token(&proof.provider),
            );
            hash_removal_manifest_line(
                &mut hasher,
                "removed_runtime_registrations.registration_macro",
                &proof.registration_macro,
            );
            for entry_point in proof.entry_points {
                hash_removal_manifest_line(
                    &mut hasher,
                    "removed_runtime_registrations.entry_points",
                    &entry_point,
                );
            }
            hash_removal_manifest_line(
                &mut hasher,
                "removed_runtime_registrations.line",
                &proof.line.to_string(),
            );
        }
        for path in manifest.preserved_paths() {
            hash_removal_manifest_line(
                &mut hasher,
                "preserved_paths",
                &removal_manifest_path_token(path),
            );
        }
        for symbol in manifest.preserved_config_symbols() {
            hash_removal_manifest_line(&mut hasher, "preserved_config_symbols", symbol);
        }
        for (symbol, value) in manifest.default_overrides() {
            hash_removal_manifest_line(&mut hasher, "default_overrides.symbol", symbol);
            hash_removal_manifest_line(&mut hasher, "default_overrides.value", value);
        }
        for (key, reason) in &manifest.reasons {
            hash_removal_manifest_line(&mut hasher, "reasons.key", &removal_key_token(key));
            hash_removal_manifest_line(
                &mut hasher,
                "reasons.reason",
                &removal_reason_token(reason),
            );
        }
        Self::new(format!(
            "removal-manifest-{}",
            hex::encode(hasher.finalize())
        ))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

fn hash_removal_manifest_line(hasher: &mut Sha256, key: &str, value: &str) {
    hasher.update(key.as_bytes());
    hasher.update(b"\0");
    hasher.update(value.len().to_string().as_bytes());
    hasher.update(b"\0");
    hasher.update(value.as_bytes());
    hasher.update(b"\0");
}

fn removal_manifest_path_token(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn removal_key_token(key: &RemovalKey) -> String {
    match key {
        RemovalKey::Path(path) => format!("path:{}", removal_manifest_path_token(path)),
        RemovalKey::Dir(path) => format!("dir:{}", removal_manifest_path_token(path)),
        RemovalKey::File(path) => format!("file:{}", removal_manifest_path_token(path)),
        RemovalKey::Header(header) => format!("header:{}", header.as_str()),
        RemovalKey::PublicHeader(header) => format!("public_header:{}", header.as_str()),
        RemovalKey::ConfigSymbol(symbol) => format!("config_symbol:{symbol}"),
        RemovalKey::KconfigSource(path) => {
            format!("kconfig_source:{}", removal_manifest_path_token(path))
        }
        RemovalKey::KbuildObject(object) => format!("kbuild_object:{}", object.as_str()),
        RemovalKey::DefaultOverride(symbol) => format!("default_override:{symbol}"),
    }
}

fn removal_reason_token(reason: &RemovalReason) -> String {
    match reason {
        RemovalReason::SlimRemovePath { path } => {
            format!(
                "slim_remove_path:path={}",
                removal_manifest_path_token(path)
            )
        }
        RemovalReason::SlimRemoveConfig { symbol } => {
            format!("slim_remove_config:symbol={symbol}")
        }
        RemovalReason::SlimDefaultOverride { symbol, value } => {
            format!("slim_default_override:symbol={symbol}:value={value}")
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AbiPolicyFingerprint(String);

#[allow(dead_code)]
impl AbiPolicyFingerprint {
    pub(crate) fn new(fingerprint: impl Into<String>) -> Result<Self> {
        let fingerprint = fingerprint.into();
        if fingerprint.trim().is_empty() {
            anyhow::bail!("ABI policy fingerprint is empty");
        }
        Ok(Self(fingerprint))
    }

    pub(super) fn from_policy(policy: &AbiPolicyConfig) -> Result<Self> {
        Self::new(stable_plan_item_id(
            "abi-policy",
            &[
                (
                    "allow_public_header_removal",
                    bool_token(policy.allow_public_header_removal),
                ),
                (
                    "allow_uapi_header_removal",
                    bool_token(policy.allow_uapi_header_removal),
                ),
            ],
        ))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ArchPolicyFingerprint(String);

#[allow(dead_code)]
impl ArchPolicyFingerprint {
    pub(crate) fn new(fingerprint: impl Into<String>) -> Result<Self> {
        let fingerprint = fingerprint.into();
        if fingerprint.trim().is_empty() {
            anyhow::bail!("arch policy fingerprint is empty");
        }
        Ok(Self(fingerprint))
    }

    pub(super) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        Self::from_policy(&profile.arch)
    }

    fn from_policy(policy: &ArchPolicyConfig) -> Result<Self> {
        let primary_arch = policy
            .primary_arch
            .as_deref()
            .map(normalized_arch_policy_name)
            .transpose()?;
        let secondary_arches = normalized_arch_policy_list(&policy.secondary_arches)?;
        let disabled_arches = normalized_arch_policy_list(&policy.disabled_arches)?;
        let secondary_arches_key = secondary_arches.join("|");
        let disabled_arches_key = disabled_arches.join("|");
        Self::new(stable_plan_item_id(
            "arch-policy",
            &[
                ("primary_arch", primary_arch.as_deref().unwrap_or("<none>")),
                ("secondary_arches", secondary_arches_key.as_str()),
                ("disabled_arches", disabled_arches_key.as_str()),
                (
                    "allow_arch_local_removal",
                    bool_token(policy.allow_arch_local_removal),
                ),
                (
                    "preserve_arch_shared",
                    bool_token(policy.preserve_arch_shared),
                ),
            ],
        ))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

fn normalized_arch_policy_list(values: &[String]) -> Result<Vec<String>> {
    let mut values = values
        .iter()
        .map(|value| normalized_arch_policy_name(value))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn normalized_arch_policy_name(value: &str) -> Result<String> {
    crate::config::normalize_arch_name(value)
}
