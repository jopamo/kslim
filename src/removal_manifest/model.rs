use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::abi::AbiPolicyConfig;
use crate::hardware::DeviceBindingRemovalProof;
use crate::exported_symbols::ExportedSymbolRemovalProof;
use crate::model::{HeaderPath, KbuildObject};
use crate::runtime::RuntimeRegistrationRemovalProof;

pub type RelativePathBuf = PathBuf;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RemovalKey {
    Path(RelativePathBuf),
    Dir(RelativePathBuf),
    File(RelativePathBuf),
    Header(HeaderPath),
    PublicHeader(HeaderPath),
    ConfigSymbol(String),
    KconfigSource(RelativePathBuf),
    KbuildObject(KbuildObject),
    DefaultOverride(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RemovalReason {
    SlimRemovePath { path: RelativePathBuf },
    SlimRemoveConfig { symbol: String },
    SlimDefaultOverride { symbol: String, value: String },
}

/// Authoritative reducer truth derived from profile removal intent.
///
/// User-facing config continues to declare direct `[slim]` removals and named
/// feature removals/preservations in profile TOML, but reducer passes consume
/// this normalized manifest so policy checks happen once and path-based edits
/// operate on a fail-closed relative form.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemovalManifest {
    pub removed_paths: BTreeSet<RelativePathBuf>,
    pub removed_dirs: BTreeSet<RelativePathBuf>,
    pub removed_files: BTreeSet<RelativePathBuf>,
    pub removed_headers: BTreeSet<HeaderPath>,
    pub removed_public_headers: BTreeSet<HeaderPath>,
    pub removed_config_symbols: BTreeSet<String>,
    pub removed_kconfig_sources: BTreeSet<RelativePathBuf>,
    pub removed_kbuild_objects: BTreeSet<KbuildObject>,
    pub(crate) removed_device_bindings: BTreeSet<DeviceBindingRemovalProof>,
    pub(crate) removed_exported_symbols: BTreeSet<ExportedSymbolRemovalProof>,
    pub(crate) removed_runtime_registrations: BTreeSet<RuntimeRegistrationRemovalProof>,
    pub abi_policy: AbiPolicyConfig,
    pub unsafe_allow_root_path_removal: bool,
    pub preserved_paths: BTreeSet<RelativePathBuf>,
    pub preserved_config_symbols: BTreeSet<String>,
    pub reasons: BTreeMap<RemovalKey, RemovalReason>,
    pub default_overrides: BTreeMap<String, String>,
}

impl RemovalManifest {
    pub fn is_noop(&self) -> bool {
        self.removed_paths.is_empty()
            && self.removed_config_symbols.is_empty()
            && self.default_overrides.is_empty()
    }

    pub fn removed_paths(&self) -> &BTreeSet<RelativePathBuf> {
        &self.removed_paths
    }

    pub fn removed_config_symbols(&self) -> &BTreeSet<String> {
        &self.removed_config_symbols
    }

    pub fn preserved_paths(&self) -> &BTreeSet<RelativePathBuf> {
        &self.preserved_paths
    }

    pub fn preserved_config_symbols(&self) -> &BTreeSet<String> {
        &self.preserved_config_symbols
    }

    pub fn default_overrides(&self) -> &BTreeMap<String, String> {
        &self.default_overrides
    }

    pub fn removed_paths_vec(&self) -> Vec<RelativePathBuf> {
        self.removed_paths.iter().cloned().collect()
    }

    pub fn removed_config_symbols_vec(&self) -> Vec<String> {
        self.removed_config_symbols.iter().cloned().collect()
    }

    pub fn preserved_paths_vec(&self) -> Vec<RelativePathBuf> {
        self.preserved_paths.iter().cloned().collect()
    }

    pub fn preserved_config_symbols_vec(&self) -> Vec<String> {
        self.preserved_config_symbols.iter().cloned().collect()
    }

    pub fn removed_header_paths_vec(&self) -> Vec<RelativePathBuf> {
        self.removed_headers
            .iter()
            .map(|header| header.as_path().to_path_buf())
            .collect()
    }

    pub fn removed_kconfig_sources_vec(&self) -> Vec<RelativePathBuf> {
        self.removed_kconfig_sources.iter().cloned().collect()
    }

    pub fn removed_kbuild_objects_vec(&self) -> Vec<KbuildObject> {
        self.removed_kbuild_objects.iter().cloned().collect()
    }

    pub(crate) fn removed_exported_symbols_vec(&self) -> Vec<ExportedSymbolRemovalProof> {
        self.removed_exported_symbols.iter().cloned().collect()
    }

    pub(crate) fn removed_device_bindings_vec(&self) -> Vec<DeviceBindingRemovalProof> {
        self.removed_device_bindings.iter().cloned().collect()
    }

    pub(crate) fn removed_runtime_registrations_vec(&self) -> Vec<RuntimeRegistrationRemovalProof> {
        self.removed_runtime_registrations.iter().cloned().collect()
    }
}
