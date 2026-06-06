use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::abi::AbiPolicyConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KslimConfig {
    pub project: ProjectConfig,
    pub upstream: UpstreamConfig,
    pub output: OutputConfig,
    #[serde(default)]
    pub git: GitConfig,
    #[serde(default)]
    pub publish: Option<PublishConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamConfig {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub cache: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutputConfig {
    /// Managed output repository path requested by project-root config.
    pub path: String,
    /// Prefix used when deriving branch and tag names from the base/profile.
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,
    /// Exact branch to use instead of derived branch naming.
    #[serde(default)]
    pub branch: Option<String>,
}

impl OutputConfig {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            branch_prefix: default_branch_prefix(),
            branch: None,
        }
    }

    pub fn has_explicit_branch(&self) -> bool {
        self.branch
            .as_deref()
            .is_some_and(|branch| !branch.trim().is_empty())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    #[serde(default = "default_git_user_email")]
    pub user_email: String,
    #[serde(default = "default_git_user_name")]
    pub user_name: String,
    #[serde(default = "default_git_remote_name")]
    pub remote_name: String,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            user_email: default_git_user_email(),
            user_name: default_git_user_name(),
            remote_name: default_git_remote_name(),
        }
    }
}

fn default_branch_prefix() -> String {
    "kslim".to_string()
}

fn default_git_user_email() -> String {
    "kslim@localhost".to_string()
}

fn default_git_user_name() -> String {
    "kslim".to_string()
}

fn default_git_remote_name() -> String {
    "origin".to_string()
}

fn default_true() -> bool {
    true
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_true(value: &bool) -> bool {
    *value
}

fn default_max_fixup_passes() -> usize {
    3
}

fn default_patch_source() -> String {
    "worktree".to_string()
}

fn default_patch_base_remote() -> String {
    "upstream".to_string()
}

fn default_patch_base_ref() -> String {
    "master".to_string()
}

fn default_report_formats() -> Vec<String> {
    ["text", "markdown", "json"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn is_default_report_formats(formats: &[String]) -> bool {
    formats == default_report_formats().as_slice()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishConfig {
    pub remote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub profile: ProfileSection,
    pub base: BaseSection,
    /// Canonical user-facing subsystem removal input.
    ///
    /// Reducer internals may derive richer manifests from this section, but
    /// profiles continue to declare removal intent through `[slim]`.
    #[serde(default)]
    pub slim: Option<SlimConfig>,
    /// Named feature intent is parsed but resolved separately from legacy
    /// direct `[slim]` removal intent.
    #[serde(default, skip_serializing_if = "FeatureConfig::is_empty")]
    pub features: FeatureConfig,
    #[serde(default)]
    pub abi: AbiPolicyConfig,
    /// Architecture selection policy is parsed separately from build matrix
    /// entries and is not resolved by the reducer yet.
    #[serde(default, skip_serializing_if = "ArchPolicyConfig::is_default")]
    pub arch: ArchPolicyConfig,
    /// Future build verification matrix intent. Effective build execution
    /// remains under `[selftests]` until matrix planning lands.
    #[serde(default, skip_serializing_if = "BuildMatrixConfig::is_default")]
    pub build_matrix: BuildMatrixConfig,
    /// Future runtime validation matrix intent. Effective runtime checks must
    /// be explicit commands until runtime matrix planning lands.
    #[serde(default, skip_serializing_if = "RuntimeMatrixConfig::is_default")]
    pub runtime_matrix: RuntimeMatrixConfig,
    /// Future report selection policy. Effective report artifacts remain fixed
    /// until report planning lands.
    #[serde(default, skip_serializing_if = "ReportConfig::is_default")]
    pub reports: ReportConfig,
    /// Future security trust-boundary policy. Effective security checks remain
    /// fixed and fail-closed until security planning lands.
    #[serde(default, skip_serializing_if = "SecurityConfig::is_default")]
    pub security: SecurityConfig,
    /// Future performance/work-shape policy. Effective hot-path behavior stays
    /// fixed until performance planning lands.
    #[serde(default, skip_serializing_if = "PerformanceConfig::is_default")]
    pub performance: PerformanceConfig,
    #[serde(default)]
    pub patches: Option<PatchConfig>,
    #[serde(default)]
    pub integrations: IntegrationsConfig,
    #[serde(default)]
    pub reducer: ReducerConfig,
    #[serde(default)]
    pub selftests: SelfTestConfig,
}

impl ProfileConfig {
    /// Return direct legacy `[slim]` removal input for this profile.
    pub fn removal_input(&self) -> Option<&SlimConfig> {
        self.slim.as_ref()
    }

    pub fn has_named_feature(&self, feature: &str) -> bool {
        self.features.remove.contains_key(feature) || self.features.preserve.contains_key(feature)
    }

    pub fn has_named_remove_feature(&self, feature: &str) -> bool {
        self.features.remove.contains_key(feature)
    }

    pub fn has_named_preserve_feature(&self, feature: &str) -> bool {
        self.features.preserve.contains_key(feature)
    }

    pub fn with_only_named_feature(&self, feature: &str) -> Self {
        let mut profile = self.clone();
        profile.features.remove.retain(|name, _| name == feature);
        profile.features.preserve.retain(|name, _| name == feature);
        profile
    }

    pub fn with_only_named_remove_feature(&self, feature: &str) -> Self {
        let mut profile = self.clone();
        profile.features.remove.retain(|name, _| name == feature);
        profile.features.preserve.clear();
        profile
    }

    pub fn with_only_named_preserve_feature(&self, feature: &str) -> Self {
        let mut profile = self.clone();
        profile.features.remove.clear();
        profile.features.preserve.retain(|name, _| name == feature);
        profile
    }

    pub fn with_selected_feature_arches(&self, arches: &[String]) -> Self {
        let mut profile = self.clone();
        profile
            .features
            .remove
            .retain(|_, intent| arches.iter().any(|arch| intent.applies_to_arch(arch)));
        profile
            .features
            .preserve
            .retain(|_, intent| arches.iter().any(|arch| intent.applies_to_arch(arch)));
        profile
    }

    /// Return the removal input resolved from direct `[slim]` and supported
    /// named `[features.remove.*]` intent.
    pub fn effective_removal_input(&self) -> Option<SlimConfig> {
        let mut slim = self.slim.clone().unwrap_or_default();
        for intent in self.features.remove.values() {
            slim.remove_paths.extend(intent.roots.iter().cloned());
            slim.remove_paths
                .extend(intent.remove_paths.iter().cloned());
            slim.remove_configs.extend(intent.configs.iter().cloned());
            slim.remove_configs
                .extend(intent.remove_configs.iter().cloned());
        }
        if slim.is_noop() {
            None
        } else {
            Some(slim)
        }
    }

    /// Return supported named `[features.preserve.*]` intent as an input that
    /// later removal planning can keep out of candidate mutation.
    pub fn effective_preservation_input(&self) -> Option<FeaturePreservationInput> {
        let mut input = FeaturePreservationInput::default();
        for intent in self.features.preserve.values() {
            input.preserve_paths.extend(intent.roots.iter().cloned());
            input
                .preserve_configs
                .extend(intent.configs.iter().cloned());
        }
        if input.is_noop() {
            None
        } else {
            Some(input)
        }
    }

    /// Return profile-wide ABI policy with supported named-feature removal
    /// approvals folded in for resolved reducer planning.
    pub fn effective_abi_policy(&self) -> AbiPolicyConfig {
        let mut policy = self.abi.clone();
        for intent in self.features.remove.values() {
            policy.allow_public_header_removal |= intent.allow_public_header_removal;
            policy.allow_uapi_header_removal |= intent.allow_uapi_header_removal;
        }
        policy
    }

    /// Return resolved per-feature removal safety levels. Named removals
    /// default to normal safety when they declare removal input without an
    /// explicit level.
    pub fn effective_feature_safety_levels(&self) -> BTreeMap<String, FeatureSafetyLevel> {
        self.features
            .remove
            .iter()
            .filter(|(_, intent)| intent.declares_removal_input() || intent.safety.is_some())
            .map(|(name, intent)| (name.clone(), intent.safety.unwrap_or_default()))
            .collect()
    }

    /// Return explicit per-feature architecture scopes from supported named
    /// remove/preserve intent.
    pub fn effective_feature_arch_scopes(&self) -> BTreeMap<String, Vec<String>> {
        let mut scopes = BTreeMap::new();
        for (name, intent) in &self.features.remove {
            if !intent.arch_scope.is_empty() {
                scopes.insert(name.clone(), intent.arch_scope.clone());
            }
        }
        for (name, intent) in &self.features.preserve {
            if !intent.arch_scope.is_empty() {
                scopes.insert(name.clone(), intent.arch_scope.clone());
            }
        }
        scopes
    }

    /// Return explicit per-feature test matrix requirements from supported
    /// named remove/preserve intent.
    pub fn effective_feature_test_matrices(&self) -> BTreeMap<String, FeatureTestMatrixConfig> {
        let mut matrices = BTreeMap::new();
        for (name, intent) in &self.features.remove {
            let matrix = intent.test_matrix();
            if !matrix.is_default() {
                matrices.insert(name.clone(), matrix);
            }
        }
        for (name, intent) in &self.features.preserve {
            let matrix = intent.test_matrix();
            if !matrix.is_default() {
                matrices.insert(name.clone(), matrix);
            }
        }
        matrices
    }

    /// Return explicit per-feature report modes from supported named
    /// remove/preserve intent.
    pub fn effective_feature_report_modes(&self) -> BTreeMap<String, FeatureReportModeConfig> {
        let mut modes = BTreeMap::new();
        for (name, intent) in &self.features.remove {
            let mode = intent.report_mode();
            if !mode.is_default() {
                modes.insert(name.clone(), mode);
            }
        }
        for (name, intent) in &self.features.preserve {
            let mode = intent.report_mode();
            if !mode.is_default() {
                modes.insert(name.clone(), mode);
            }
        }
        modes
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FeatureConfig {
    /// Named feature removals keyed by stable feature name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub remove: BTreeMap<String, FeatureIntentConfig>,
    /// Named feature preservation requests keyed by stable feature name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub preserve: BTreeMap<String, FeatureIntentConfig>,
}

impl FeatureConfig {
    pub fn is_empty(&self) -> bool {
        self.remove.is_empty() && self.preserve.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FeaturePreservationInput {
    pub preserve_paths: Vec<String>,
    pub preserve_configs: Vec<String>,
}

impl FeaturePreservationInput {
    pub fn is_noop(&self) -> bool {
        self.preserve_paths.is_empty() && self.preserve_configs.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FeatureIntentConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Kernel-tree roots associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roots: Vec<String>,
    /// Explicit kernel-tree paths to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_paths: Vec<String>,
    /// Kconfig symbols associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub configs: Vec<String>,
    /// Explicit Kconfig symbols to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_configs: Vec<String>,
    /// Exported symbols associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exported_symbols: Vec<String>,
    /// Explicit exported symbols to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_exported_symbols: Vec<String>,
    /// Kernel module names associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub module_names: Vec<String>,
    /// Explicit kernel module names to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_module_names: Vec<String>,
    /// Kernel module aliases associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub module_aliases: Vec<String>,
    /// Explicit kernel module aliases to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_module_aliases: Vec<String>,
    /// Devicetree compatible strings associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub device_compatibles: Vec<String>,
    /// Explicit devicetree compatible strings to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_device_compatibles: Vec<String>,
    /// ACPI hardware IDs associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acpi_ids: Vec<String>,
    /// Explicit ACPI hardware IDs to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_acpi_ids: Vec<String>,
    /// PCI vendor/device IDs associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pci_ids: Vec<String>,
    /// Explicit PCI vendor/device IDs to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_pci_ids: Vec<String>,
    /// USB vendor/product IDs associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub usb_ids: Vec<String>,
    /// Explicit USB vendor/product IDs to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_usb_ids: Vec<String>,
    /// Firmware-loader relative paths associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub firmware_paths: Vec<String>,
    /// Explicit firmware-loader relative paths to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_firmware_paths: Vec<String>,
    /// Initcall entry-point identifiers associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initcalls: Vec<String>,
    /// Explicit initcall entry-point identifiers to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_initcalls: Vec<String>,
    /// Runtime registration macro/entry-point surfaces associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_registrations: Vec<String>,
    /// Explicit runtime registration macro/entry-point surfaces to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_runtime_registrations: Vec<String>,
    /// Documentation paths associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub docs: Vec<String>,
    /// Explicit documentation paths to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_docs: Vec<String>,
    /// Tool paths associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    /// Explicit tool paths to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_tools: Vec<String>,
    /// Sample paths associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub samples: Vec<String>,
    /// Explicit sample paths to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_samples: Vec<String>,
    /// KUnit suites associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kunit_suites: Vec<String>,
    /// Explicit KUnit suites to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_kunit_suites: Vec<String>,
    /// kselftest targets associated with this feature intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kselftest_targets: Vec<String>,
    /// Explicit kselftest targets to remove for this named feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_kselftest_targets: Vec<String>,
    /// Explicitly allow public-header ABI removal for this feature intent.
    #[serde(default, skip_serializing_if = "is_false")]
    pub allow_public_header_removal: bool,
    /// Explicitly allow UAPI removal for this feature intent.
    #[serde(default, skip_serializing_if = "is_false")]
    pub allow_uapi_header_removal: bool,
    /// Kernel architectures this feature intent applies to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arch_scope: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety: Option<FeatureSafetyLevel>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub preserve_uapi: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub preserve_module_aliases: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub require_clean_boot: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub report_only: bool,
}

impl FeatureIntentConfig {
    pub fn applies_to_arch(&self, arch: &str) -> bool {
        self.arch_scope.is_empty()
            || self
                .arch_scope
                .iter()
                .any(|candidate| candidate.as_str() == arch)
    }

    pub fn declares_removal_input(&self) -> bool {
        !self.roots.is_empty()
            || !self.remove_paths.is_empty()
            || !self.configs.is_empty()
            || !self.remove_configs.is_empty()
            || !self.exported_symbols.is_empty()
            || !self.remove_exported_symbols.is_empty()
            || !self.module_names.is_empty()
            || !self.remove_module_names.is_empty()
            || !self.module_aliases.is_empty()
            || !self.remove_module_aliases.is_empty()
            || !self.device_compatibles.is_empty()
            || !self.remove_device_compatibles.is_empty()
            || !self.acpi_ids.is_empty()
            || !self.remove_acpi_ids.is_empty()
            || !self.pci_ids.is_empty()
            || !self.remove_pci_ids.is_empty()
            || !self.usb_ids.is_empty()
            || !self.remove_usb_ids.is_empty()
            || !self.firmware_paths.is_empty()
            || !self.remove_firmware_paths.is_empty()
            || !self.initcalls.is_empty()
            || !self.remove_initcalls.is_empty()
            || !self.runtime_registrations.is_empty()
            || !self.remove_runtime_registrations.is_empty()
            || !self.docs.is_empty()
            || !self.remove_docs.is_empty()
            || !self.tools.is_empty()
            || !self.remove_tools.is_empty()
            || !self.samples.is_empty()
            || !self.remove_samples.is_empty()
            || !self.kunit_suites.is_empty()
            || !self.remove_kunit_suites.is_empty()
            || !self.kselftest_targets.is_empty()
            || !self.remove_kselftest_targets.is_empty()
    }

    pub fn declares_feature_input(&self) -> bool {
        !self.roots.is_empty()
            || !self.remove_paths.is_empty()
            || !self.configs.is_empty()
            || !self.remove_configs.is_empty()
            || !self.exported_symbols.is_empty()
            || !self.remove_exported_symbols.is_empty()
            || !self.module_names.is_empty()
            || !self.remove_module_names.is_empty()
            || !self.module_aliases.is_empty()
            || !self.remove_module_aliases.is_empty()
            || !self.device_compatibles.is_empty()
            || !self.remove_device_compatibles.is_empty()
            || !self.acpi_ids.is_empty()
            || !self.remove_acpi_ids.is_empty()
            || !self.pci_ids.is_empty()
            || !self.remove_pci_ids.is_empty()
            || !self.usb_ids.is_empty()
            || !self.remove_usb_ids.is_empty()
            || !self.firmware_paths.is_empty()
            || !self.remove_firmware_paths.is_empty()
            || !self.initcalls.is_empty()
            || !self.remove_initcalls.is_empty()
            || !self.runtime_registrations.is_empty()
            || !self.remove_runtime_registrations.is_empty()
            || !self.docs.is_empty()
            || !self.remove_docs.is_empty()
            || !self.tools.is_empty()
            || !self.remove_tools.is_empty()
            || !self.samples.is_empty()
            || !self.remove_samples.is_empty()
            || !self.kunit_suites.is_empty()
            || !self.remove_kunit_suites.is_empty()
            || !self.kselftest_targets.is_empty()
            || !self.remove_kselftest_targets.is_empty()
    }

    pub fn declares_test_matrix(&self) -> bool {
        self.require_clean_boot
    }

    pub fn test_matrix(&self) -> FeatureTestMatrixConfig {
        FeatureTestMatrixConfig {
            require_clean_boot: self.require_clean_boot,
        }
    }

    pub fn declares_report_mode(&self) -> bool {
        self.report_only
    }

    pub fn report_mode(&self) -> FeatureReportModeConfig {
        FeatureReportModeConfig {
            report_only: self.report_only,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
#[serde(rename_all = "snake_case")]
pub enum FeatureSafetyLevel {
    Conservative,
    #[default]
    Normal,
    Aggressive,
    Surgical,
    Unsafe,
}

impl FeatureSafetyLevel {
    pub fn from_cli_name(value: &str) -> Option<Self> {
        match value {
            "conservative" => Some(Self::Conservative),
            "normal" => Some(Self::Normal),
            "aggressive" => Some(Self::Aggressive),
            "surgical" => Some(Self::Surgical),
            "unsafe" => Some(Self::Unsafe),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Conservative => "conservative",
            Self::Normal => "normal",
            Self::Aggressive => "aggressive",
            Self::Surgical => "surgical",
            Self::Unsafe => "unsafe",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FeatureTestMatrixConfig {
    /// Request a clean boot requirement for this feature intent.
    #[serde(default, skip_serializing_if = "is_false")]
    pub require_clean_boot: bool,
}

impl FeatureTestMatrixConfig {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FeatureReportModeConfig {
    /// Request report-only treatment for this feature intent.
    #[serde(default, skip_serializing_if = "is_false")]
    pub report_only: bool,
}

impl FeatureReportModeConfig {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArchPolicyConfig {
    /// Main architecture used for architecture-scoped feature intent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_arch: Option<String>,
    /// Additional architectures that must remain valid for verification.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secondary_arches: Vec<String>,
    /// Architectures intentionally excluded from verification/resolution scope.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_arches: Vec<String>,
    /// Opt-in for removing files known to be local to disabled architectures.
    #[serde(default, skip_serializing_if = "is_false")]
    pub allow_arch_local_removal: bool,
    /// Preserve files shared across architectures unless later policy says
    /// otherwise.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub preserve_arch_shared: bool,
}

impl Default for ArchPolicyConfig {
    fn default() -> Self {
        Self {
            primary_arch: None,
            secondary_arches: Vec::new(),
            disabled_arches: Vec::new(),
            allow_arch_local_removal: false,
            preserve_arch_shared: true,
        }
    }
}

impl ArchPolicyConfig {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildMatrixConfig {
    /// Enables the future matrix planner once support lands.
    #[serde(default, skip_serializing_if = "is_false")]
    pub enabled: bool,
    /// Named matrix presets such as `default`, `extended`, or `hardening`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub presets: Vec<String>,
    /// Kernel architectures selected for matrix verification.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arches: Vec<String>,
    /// Kernel config targets such as `defconfig` or `allmodconfig`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub config_targets: Vec<String>,
    /// Kernel build targets such as `vmlinux`, `modules`, or `headers_install`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<String>,
    /// Stable randconfig seed for matrix entries that require one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub randconfig_seed: Option<String>,
    /// Optional parallelism cap for matrix build entries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jobs: Option<usize>,
    /// Build matrix failures block generation by default.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub fail_on_error: bool,
}

impl Default for BuildMatrixConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            presets: Vec::new(),
            arches: Vec::new(),
            config_targets: Vec::new(),
            targets: Vec::new(),
            randconfig_seed: None,
            jobs: None,
            fail_on_error: true,
        }
    }
}

impl BuildMatrixConfig {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeMatrixConfig {
    /// Enables the future runtime matrix planner once support lands.
    #[serde(default, skip_serializing_if = "is_false")]
    pub enabled: bool,
    /// Kernel architectures selected for runtime boot validation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boot_arches: Vec<String>,
    /// QEMU machine names or board labels selected for boot validation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub qemu_machines: Vec<String>,
    /// KUnit suites selected for runtime-adjacent verification.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kunit_suites: Vec<String>,
    /// kselftest targets selected for userspace/runtime validation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kselftest_targets: Vec<String>,
    /// Run selected module load/alias smoke checks.
    #[serde(default, skip_serializing_if = "is_false")]
    pub module_smoke: bool,
    /// Treat panic/oops/warning/init-failure dmesg classifiers as fatal.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub require_clean_dmesg: bool,
    /// Optional boot timeout for each runtime matrix entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boot_timeout_seconds: Option<u64>,
    /// Runtime matrix failures block generation by default.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub fail_on_error: bool,
}

impl Default for RuntimeMatrixConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            boot_arches: Vec::new(),
            qemu_machines: Vec::new(),
            kunit_suites: Vec::new(),
            kselftest_targets: Vec::new(),
            module_smoke: false,
            require_clean_dmesg: true,
            boot_timeout_seconds: None,
            fail_on_error: true,
        }
    }
}

impl RuntimeMatrixConfig {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReportConfig {
    /// Requested committed report formats.
    #[serde(
        default = "default_report_formats",
        skip_serializing_if = "is_default_report_formats"
    )]
    pub formats: Vec<String>,
    /// Include structured edit records in committed reducer reports.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub include_edit_records: bool,
    /// Include structured diagnostics and skipped-site summaries.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub include_diagnostics: bool,
    /// Include future config source-map detail in reports.
    #[serde(default, skip_serializing_if = "is_false")]
    pub include_source_map: bool,
    /// Redact host-specific absolute paths from committed reports.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub redact_host_paths: bool,
    /// Raw logs are not committed by default; they belong to attempt/CI
    /// artifacts unless a future policy explicitly supports them.
    #[serde(default, skip_serializing_if = "is_false")]
    pub include_raw_logs: bool,
    /// Report validation failures block generation by default.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub fail_on_error: bool,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            formats: default_report_formats(),
            include_edit_records: true,
            include_diagnostics: true,
            include_source_map: false,
            redact_host_paths: true,
            include_raw_logs: false,
            fail_on_error: true,
        }
    }
}

impl ReportConfig {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityConfig {
    /// Allow network-backed inputs. Disabled by default; kslim expects local,
    /// read-only upstream inputs at the trust boundary.
    #[serde(default, skip_serializing_if = "is_false")]
    pub allow_network: bool,
    /// Require upstream input to remain local unless a future policy explicitly
    /// supports network authority.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub require_local_upstream: bool,
    /// Reject host-specific absolute paths in committed metadata.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub reject_host_paths_in_committed_metadata: bool,
    /// Reject temporary workspace paths in committed metadata.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub reject_temp_paths_in_committed_metadata: bool,
    /// Reject raw logs in committed metadata and reports.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub reject_raw_logs_in_committed_metadata: bool,
    /// Require committed timestamps to come from reproducible declared sources.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub require_reproducible_timestamps: bool,
    /// Require metadata readers/writers to use phase-typed path wrappers.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub require_phase_typed_metadata: bool,
    /// Future explicit compatibility mode for security downgrades.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility_mode: Option<String>,
    /// Security policy violations block generation by default.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub fail_on_policy_violation: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allow_network: false,
            require_local_upstream: true,
            reject_host_paths_in_committed_metadata: true,
            reject_temp_paths_in_committed_metadata: true,
            reject_raw_logs_in_committed_metadata: true,
            require_reproducible_timestamps: true,
            require_phase_typed_metadata: true,
            compatibility_mode: None,
            fail_on_policy_violation: true,
        }
    }
}

impl SecurityConfig {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PerformanceConfig {
    /// Enables the future performance policy planner once support lands.
    #[serde(default, skip_serializing_if = "is_false")]
    pub enabled: bool,
    /// Optional global worker-thread cap for future CPU-bound reducer work.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_worker_threads: Option<usize>,
    /// Optional I/O worker cap for future filesystem-heavy phases.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_io_threads: Option<usize>,
    /// Future opt-in for persistent tree-index caching.
    #[serde(default, skip_serializing_if = "is_false")]
    pub cache_tree_index: bool,
    /// Future opt-in for incremental index refresh policy.
    #[serde(default, skip_serializing_if = "is_false")]
    pub incremental_reindex: bool,
    /// Future opt-in for committed timing summaries.
    #[serde(default, skip_serializing_if = "is_false")]
    pub collect_timing_metrics: bool,
    /// Future opt-in for hot-path profiling hooks.
    #[serde(default, skip_serializing_if = "is_false")]
    pub profile_hot_paths: bool,
    /// Performance policy violations block generation by default.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub fail_on_regression: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_worker_threads: None,
            max_io_threads: None,
            cache_tree_index: false,
            incremental_reindex: false,
            collect_timing_metrics: false,
            profile_hot_paths: false,
            fail_on_regression: true,
        }
    }
}

impl PerformanceConfig {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntegrationsConfig {
    #[serde(default)]
    pub rtlmq: Option<RtlmqIntegrationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtlmqIntegrationConfig {
    pub source: String,
    #[serde(default)]
    pub tests_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SlimConfig {
    /// Paths to remove from the generated tree.
    #[serde(default)]
    pub remove_paths: Vec<String>,
    /// Kconfig symbols to remove from the generated tree.
    #[serde(default)]
    pub remove_configs: Vec<String>,
    /// Kconfig default overrides to apply after pruning.
    #[serde(default)]
    pub set_defaults: BTreeMap<String, String>,
    /// Unsafe opt-in for declaring the kernel tree root itself as removed.
    #[serde(default, skip_serializing_if = "is_false")]
    pub unsafe_allow_root_path_removal: bool,
}

impl SlimConfig {
    pub fn is_noop(&self) -> bool {
        self.remove_paths.is_empty()
            && self.remove_configs.is_empty()
            && self.set_defaults.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReducerConfig {
    #[serde(default = "default_max_fixup_passes")]
    pub max_fixup_passes: usize,
    #[serde(default = "default_true")]
    pub report_unsupported_expressions: bool,
    #[serde(default = "default_true")]
    pub fail_on_unknown_diagnostics: bool,
    #[serde(default = "default_true")]
    pub reject_unproven_fixups: bool,
    #[serde(default = "default_true")]
    pub reject_unreasoned_edits: bool,
    #[serde(default = "default_true")]
    pub reject_speculative_fallout_edits: bool,
    #[serde(default)]
    pub fail_on_missing_prune_paths: bool,
    #[serde(default)]
    pub ignore_unsupported_special_removals: bool,
}

impl Default for ReducerConfig {
    fn default() -> Self {
        Self {
            max_fixup_passes: default_max_fixup_passes(),
            report_unsupported_expressions: true,
            fail_on_unknown_diagnostics: true,
            reject_unproven_fixups: true,
            reject_unreasoned_edits: true,
            reject_speculative_fallout_edits: true,
            fail_on_missing_prune_paths: false,
            ignore_unsupported_special_removals: false,
        }
    }
}

impl ReducerConfig {
    pub fn enable_strict_mode(&mut self) {
        self.report_unsupported_expressions = true;
        self.fail_on_unknown_diagnostics = true;
        self.reject_unproven_fixups = true;
        self.reject_unreasoned_edits = true;
        self.reject_speculative_fallout_edits = true;
    }

    pub fn disable_strict_mode(&mut self) {
        self.report_unsupported_expressions = false;
        self.fail_on_unknown_diagnostics = false;
        self.reject_unproven_fixups = false;
        self.reject_unreasoned_edits = false;
        self.reject_speculative_fallout_edits = false;
    }

    pub fn strict_mode(&self) -> bool {
        self.report_unsupported_expressions
            && self.fail_on_unknown_diagnostics
            && self.reject_unproven_fixups
            && self.reject_unreasoned_edits
            && self.reject_speculative_fallout_edits
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PatchConfig {
    Single(PatchSourceConfig),
    Multi(PatchSourcesConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchSourceConfig {
    #[serde(default = "default_patch_source")]
    pub source: String,
    pub path: String,
    #[serde(default = "default_patch_base_remote")]
    pub base_remote: String,
    #[serde(default = "default_patch_base_ref")]
    pub base_ref: String,
    #[serde(default = "default_true")]
    pub require_clean: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PatchSourcesConfig {
    #[serde(default)]
    pub sources: Vec<PatchSourceConfig>,
}

impl PatchConfig {
    pub fn sources(&self) -> Vec<&PatchSourceConfig> {
        match self {
            Self::Single(source) => vec![source],
            Self::Multi(config) => config.sources.iter().collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfTestConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub check_kconfig_sources: bool,
    #[serde(default = "default_true")]
    pub check_makefiles: bool,
    #[serde(default)]
    pub kernel_builds: Vec<KernelBuildConfig>,
    #[serde(default)]
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelBuildConfig {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub config_target: Option<String>,
    #[serde(default)]
    pub targets: Vec<String>,
    #[serde(default)]
    pub output_dir: Option<String>,
    #[serde(default)]
    pub jobs: Option<usize>,
    #[serde(default = "default_true")]
    pub clean: bool,
    #[serde(default)]
    pub make_program: Option<String>,
    #[serde(default)]
    pub make_args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSection {
    pub name: String,
    /// Future parent profile intent. The loader parses this field so source
    /// maps and validation can fail closed instead of silently ignoring it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherits: Option<String>,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseSection {
    pub r#ref: String,
}

impl Default for SelfTestConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_kconfig_sources: true,
            check_makefiles: true,
            kernel_builds: Vec::new(),
            commands: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests;
