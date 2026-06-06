//! Explicit generate state models.
//!
//! This module owns typed state snapshots for the generate lifecycle. Requested
//! state is the immutable user request captured before any candidate tree is
//! resolved, materialized, mutated, committed, or published.
//! Resolved, candidate, published, and failure state must stay in separate state objects.
//! Failure state cannot be converted into published state.
//! Publication flows through CommittedOutputSnapshot::from_successful_commit.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::{
    AbiPolicyConfig, BuildMatrixConfig, FeatureIntentConfig, FeatureReportModeConfig,
    FeatureSafetyLevel, FeatureTestMatrixConfig, KernelBuildConfig, KslimConfig, ProfileConfig,
    ReducerConfig, SelfTestConfig,
};
use crate::feature::{
    FeatureConflictReport, FeatureGraph, FeatureIntent, FeatureIntentAction, FeatureNode,
};
use crate::lockfile::ResolvedBase;
use crate::model::{
    AcpiId, ArchName, DeviceCompatible, DocumentationPath, ExportedSymbol, FirmwarePath,
    GitCommitId, HeaderPath, Initcall, KconfigSymbol, KselftestTarget, KunitSuite, ModuleAlias,
    ModuleName, OutputBranchName, PciId, ReportPath, RuntimeRegistrationSurface, SamplePath,
    ToolPath, UapiPath, UsbId,
};
use crate::patches::{self, PatchInfo};
use crate::paths::{
    AttemptMetadataDir, CandidateMetadataDir, CandidateTreePath, KernelBuildDir, LockfilePath,
    OutputRepoPath, PublishedMetadataDir, RelativeKernelPath, RequestedConfigPath,
};
use crate::removal_manifest::RemovalManifest;

use crate::generate::{GenerateOptions, GenerateStage, SuccessfulCommitResult};

mod fingerprint;
pub(crate) use fingerprint::{
    AbiPolicyFingerprint, ArchPolicyFingerprint, FeatureGraphFingerprint,
    RemovalManifestFingerprint,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum GenerateStatePhase {
    Requested,
    Resolved,
    Candidate,
    OutputTarget,
    Published,
    Failure,
}

impl GenerateStatePhase {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::Resolved => "resolved",
            Self::Candidate => "candidate",
            Self::OutputTarget => "output_target",
            Self::Published => "published",
            Self::Failure => "failure",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct GenerateStateIdentity {
    pub(crate) phase: GenerateStatePhase,
    pub(crate) key: String,
}

impl GenerateStateIdentity {
    pub(crate) fn new(phase: GenerateStatePhase, key: impl Into<String>) -> Result<Self> {
        let key = key.into();
        if key.trim().is_empty() {
            anyhow::bail!("generate state identity for {} is empty", phase.as_str());
        }
        Ok(Self { phase, key })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ProfileName(String);

impl ProfileName {
    pub(crate) fn new(name: impl Into<String>) -> Result<Self> {
        let name = crate::config::normalize_profile_name(&name.into())?;
        Ok(Self(name))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    #[cfg(test)]
    pub(crate) fn new_unchecked_for_test(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CliOverrides {
    pub(crate) dry_run: bool,
    pub(crate) deep_dry_run: bool,
    pub(crate) report_only: bool,
    pub(crate) force: bool,
    pub(crate) offline: bool,
    pub(crate) base_ref: Option<String>,
    pub(crate) feature: Option<String>,
    pub(crate) remove_feature: Option<String>,
    pub(crate) preserve_feature: Option<String>,
    pub(crate) arch: Option<String>,
    pub(crate) primary_arch: Option<String>,
    pub(crate) secondary_arch: Option<String>,
    pub(crate) safety: Option<String>,
    pub(crate) max_fixup_passes: Option<usize>,
    pub(crate) matrix: Option<String>,
    pub(crate) strict: bool,
    pub(crate) no_strict: bool,
    pub(crate) run_selftests: bool,
}

impl CliOverrides {
    pub(crate) fn from_options(opts: &GenerateOptions) -> Self {
        Self {
            dry_run: opts.dry_run,
            deep_dry_run: opts.deep_dry_run,
            report_only: opts.report_only,
            force: opts.force,
            offline: opts.offline,
            base_ref: opts.normalized_base_ref_for_request(),
            feature: opts.normalized_feature_for_request(),
            remove_feature: opts.normalized_remove_feature_for_request(),
            preserve_feature: opts.normalized_preserve_feature_for_request(),
            arch: opts.normalized_arch_for_request(),
            primary_arch: opts.normalized_primary_arch_for_request(),
            secondary_arch: opts.normalized_secondary_arch_for_request(),
            safety: opts.normalized_safety_for_request(),
            max_fixup_passes: opts.max_fixup_passes,
            matrix: opts.normalized_matrix_for_request(),
            strict: opts.strict,
            no_strict: opts.no_strict,
            run_selftests: opts.run_selftests,
        }
    }

    pub(crate) fn normalized(mut self) -> Self {
        self.base_ref = self
            .base_ref
            .as_deref()
            .map(|base_ref| base_ref.trim().to_string());
        self.feature = self
            .feature
            .as_deref()
            .map(|feature| feature.trim().to_string());
        self.remove_feature = self
            .remove_feature
            .as_deref()
            .map(|feature| feature.trim().to_string());
        self.preserve_feature = self
            .preserve_feature
            .as_deref()
            .map(|feature| feature.trim().to_string());
        self.arch = self.arch.as_deref().map(|arch| arch.trim().to_string());
        self.primary_arch = self
            .primary_arch
            .as_deref()
            .map(|arch| arch.trim().to_string());
        self.secondary_arch = self
            .secondary_arch
            .as_deref()
            .map(|arch| arch.trim().to_string());
        self.safety = self
            .safety
            .as_deref()
            .map(|safety| safety.trim().to_string());
        self.matrix = self
            .matrix
            .as_deref()
            .map(|matrix| matrix.trim().to_ascii_lowercase());
        self
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.base_ref.as_deref().is_some_and(str::is_empty) {
            anyhow::bail!("cli --base must not be empty");
        }
        if self.feature.as_deref().is_some_and(str::is_empty) {
            anyhow::bail!("cli --feature must not be empty");
        }
        if self.remove_feature.as_deref().is_some_and(str::is_empty) {
            anyhow::bail!("cli --remove-feature must not be empty");
        }
        if self.preserve_feature.as_deref().is_some_and(str::is_empty) {
            anyhow::bail!("cli --preserve-feature must not be empty");
        }
        if self.arch.as_deref().is_some_and(str::is_empty) {
            anyhow::bail!("cli --arch must not be empty");
        }
        if self.primary_arch.as_deref().is_some_and(str::is_empty) {
            anyhow::bail!("cli --primary-arch must not be empty");
        }
        if self.secondary_arch.as_deref().is_some_and(str::is_empty) {
            anyhow::bail!("cli --secondary-arch must not be empty");
        }
        if self.safety.as_deref().is_some_and(str::is_empty) {
            anyhow::bail!("cli --safety must not be empty");
        }
        if self.matrix.as_deref().is_some_and(str::is_empty) {
            anyhow::bail!("cli --matrix must not be empty");
        }
        if self.strict && self.no_strict {
            anyhow::bail!("cli --strict and --no-strict are mutually exclusive");
        }
        if self.dry_run && self.deep_dry_run {
            anyhow::bail!("cli --dry-run and --deep-dry-run are mutually exclusive");
        }
        if self.report_only && self.deep_dry_run {
            anyhow::bail!("cli --report-only and --deep-dry-run are mutually exclusive");
        }
        if let Some(arch) = self.arch.as_deref() {
            crate::config::normalize_arch_name(arch)
                .map_err(|err| anyhow::anyhow!("cli --arch is invalid: {:#}", err))?;
        }
        if let Some(arch) = self.primary_arch.as_deref() {
            crate::config::normalize_arch_name(arch)
                .map_err(|err| anyhow::anyhow!("cli --primary-arch is invalid: {:#}", err))?;
        }
        if let Some(arch) = self.secondary_arch.as_deref() {
            crate::config::normalize_arch_name(arch)
                .map_err(|err| anyhow::anyhow!("cli --secondary-arch is invalid: {:#}", err))?;
        }
        if let Some(safety) = self.safety.as_deref() {
            if crate::config::FeatureSafetyLevel::from_cli_name(safety).is_none() {
                anyhow::bail!(
                    "cli --safety is invalid: expected conservative, normal, aggressive, surgical, or unsafe"
                );
            }
        }
        if let Some(matrix) = self.matrix.as_deref() {
            crate::matrix::normalize_cli_matrix(matrix)?;
        }
        if self.arch.is_some() && self.primary_arch.is_some() {
            anyhow::bail!("cli --arch and --primary-arch are mutually exclusive");
        }
        if self.arch.is_some() && self.secondary_arch.is_some() {
            anyhow::bail!("cli --arch and --secondary-arch are mutually exclusive");
        }
        if [
            self.feature.as_ref(),
            self.remove_feature.as_ref(),
            self.preserve_feature.as_ref(),
        ]
        .iter()
        .filter(|feature| feature.is_some())
        .count()
            > 1
        {
            anyhow::bail!(
                "cli --feature, --remove-feature, and --preserve-feature are mutually exclusive"
            );
        }
        Ok(())
    }

    fn identity_fragment(&self) -> String {
        format!(
            "dry_run={}:deep_dry_run={}:report_only={}:force={}:offline={}:strict={}:no_strict={}:base_override={}:feature_override={}:remove_feature_override={}:preserve_feature_override={}:arch_override={}:primary_arch_override={}:secondary_arch_override={}:safety_override={}:max_fixup_passes_override={}:matrix_override={}:run_selftests={}",
            self.dry_run,
            self.deep_dry_run,
            self.report_only,
            self.force,
            self.offline,
            self.strict,
            self.no_strict,
            self.base_ref.as_deref().unwrap_or("<none>"),
            self.feature.as_deref().unwrap_or("<none>"),
            self.remove_feature.as_deref().unwrap_or("<none>"),
            self.preserve_feature.as_deref().unwrap_or("<none>"),
            self.arch.as_deref().unwrap_or("<none>"),
            self.primary_arch.as_deref().unwrap_or("<none>"),
            self.secondary_arch.as_deref().unwrap_or("<none>"),
            self.safety.as_deref().unwrap_or("<none>"),
            self.max_fixup_passes
                .map(|passes| passes.to_string())
                .unwrap_or_else(|| String::from("<none>")),
            self.matrix.as_deref().unwrap_or("<none>"),
            self.run_selftests
        )
    }

    pub(crate) fn strictness_cli_source(&self) -> Option<&'static str> {
        if self.strict {
            Some("cli --strict")
        } else if self.no_strict {
            Some("cli --no-strict")
        } else {
            None
        }
    }

    pub(crate) fn profile_feature_selection(&self) -> crate::config::ProfileFeatureSelection<'_> {
        crate::config::ProfileFeatureSelection::new(
            self.feature.as_deref(),
            self.remove_feature.as_deref(),
            self.preserve_feature.as_deref(),
            self.arch.as_deref(),
            self.primary_arch.as_deref(),
            self.secondary_arch.as_deref(),
            self.safety.as_deref(),
        )
    }

    pub(crate) fn apply_profile_overrides(&self, profile: ProfileConfig) -> Result<ProfileConfig> {
        let mut profile =
            crate::config::select_profile_features(profile, self.profile_feature_selection())?;
        if self.strict {
            profile.reducer.enable_strict_mode();
        } else if self.no_strict {
            profile.reducer.disable_strict_mode();
        }
        if let Some(max_fixup_passes) = self.max_fixup_passes {
            profile.reducer.max_fixup_passes = max_fixup_passes;
        }
        crate::matrix::apply_cli_matrix_override(&mut profile, self.matrix.as_deref())?;
        Ok(profile)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestedGenerateState {
    pub(crate) config_path: RequestedConfigPath,
    pub(crate) selected_profile: ProfileName,
    pub(crate) cli_overrides: CliOverrides,
}

impl RequestedGenerateState {
    pub(crate) fn new(
        config_path: RequestedConfigPath,
        selected_profile: ProfileName,
        cli_overrides: CliOverrides,
    ) -> Self {
        Self {
            config_path,
            selected_profile,
            cli_overrides: cli_overrides.normalized(),
        }
    }

    pub(crate) fn from_inputs(
        config_path: impl Into<PathBuf>,
        profile: &ProfileConfig,
        opts: &GenerateOptions,
    ) -> Result<Self> {
        let requested = Self::new(
            RequestedConfigPath::new(config_path)?,
            ProfileName::new(profile.profile.name.clone())?,
            CliOverrides::from_options(opts),
        );
        requested.cli_overrides.validate()?;
        Ok(requested)
    }

    pub(crate) fn identity(&self) -> Result<GenerateStateIdentity> {
        GenerateStateIdentity::new(
            GenerateStatePhase::Requested,
            format!(
                "requested:config={}:profile={}:{}",
                self.config_path.as_path().display(),
                self.selected_profile.as_str(),
                self.cli_overrides.identity_fragment()
            ),
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct ResolvedCandidateState {
    pub(crate) base: ResolvedBase,
    pub(crate) patch_plan: PatchPlan,
    pub(crate) integration_plan: IntegrationPlan,
    pub(crate) feature_intent_plan: FeatureIntentPlan,
    pub(crate) feature_graph_fingerprint: FeatureGraphFingerprint,
    pub(crate) removal_manifest_fingerprint: RemovalManifestFingerprint,
    pub(crate) abi_policy_fingerprint: AbiPolicyFingerprint,
    pub(crate) arch_policy_fingerprint: ArchPolicyFingerprint,
    pub(crate) feature_resolution: FeatureResolutionState,
    pub(crate) feature_conflicts: FeatureConflictReport,
    pub(crate) abi_decision: AbiDecisionState,
    pub(crate) prune_plan: PrunePlan,
    pub(crate) reducer_plan: ReducerPlan,
    pub(crate) build_matrix_plan: BuildMatrixPlan,
    pub(crate) selftest_plan: SelftestPlan,
    pub(crate) output_plan: OutputPlan,
}

impl ResolvedCandidateState {
    pub(crate) fn from_resolved_inputs(
        config: &KslimConfig,
        profile: &ProfileConfig,
        base: ResolvedBase,
        patch_infos: Option<&[PatchInfo]>,
        mode: impl Into<String>,
        target_branch: impl Into<String>,
    ) -> Result<Self> {
        let feature_intent_plan = FeatureIntentPlan::from_profile(profile)?;
        let feature_resolution = FeatureResolutionState::from_profile(profile)?;
        let feature_conflicts = FeatureConflictReport::from_profile(profile)?;
        let feature_graph_fingerprint = FeatureGraphFingerprint::from_resolved_feature_graph(
            &feature_intent_plan,
            &feature_resolution,
        )?;
        let removal_manifest_fingerprint = RemovalManifestFingerprint::from_profile(profile)?;
        let abi_policy_fingerprint =
            AbiPolicyFingerprint::from_policy(feature_resolution.abi_policy())?;
        let arch_policy_fingerprint = ArchPolicyFingerprint::from_profile(profile)?;
        let abi_decision = AbiDecisionState::from_feature_resolution(&feature_resolution)?;
        let prune_plan = PrunePlan::from_feature_resolution(&feature_resolution);
        let output_plan = OutputPlan::new(config, profile, &base, target_branch, mode)?;
        Ok(Self {
            base,
            patch_plan: PatchPlan::from_patch_infos(patch_infos),
            integration_plan: IntegrationPlan::from_profile(profile),
            feature_intent_plan,
            feature_graph_fingerprint,
            removal_manifest_fingerprint,
            abi_policy_fingerprint,
            arch_policy_fingerprint,
            feature_resolution,
            feature_conflicts,
            abi_decision,
            prune_plan,
            reducer_plan: ReducerPlan::from_config(&profile.reducer),
            build_matrix_plan: BuildMatrixPlan::from_config(&profile.build_matrix)?,
            selftest_plan: SelftestPlan::from_config(&profile.selftests)?,
            output_plan,
        })
    }

    pub(crate) fn reject_unresolved_feature_conflicts_in_strict_mode(&self) -> Result<()> {
        self.feature_conflicts
            .reject_blocking_conflicts_in_strict_mode(self.reducer_plan.strict_mode())
    }
}

fn stable_plan_item_id(kind: &str, fields: &[(&str, &str)]) -> String {
    crate::core::stable_id(kind, fields)
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PatchPlan {
    pub(crate) sources: Vec<PatchSourcePlan>,
    pub(crate) total_patch_count: usize,
}

impl PatchPlan {
    fn from_patch_infos(patch_infos: Option<&[PatchInfo]>) -> Self {
        let sources = patch_infos
            .unwrap_or_default()
            .iter()
            .map(PatchSourcePlan::from_patch_info)
            .collect::<Vec<_>>();
        let total_patch_count = patch_infos.map(patches::total_patch_count).unwrap_or(0);
        Self {
            sources,
            total_patch_count,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PatchSourcePlan {
    pub(crate) stable_id: String,
    pub(crate) source: String,
    pub(crate) worktree_path: String,
    pub(crate) branch: String,
    pub(crate) head_commit: String,
    pub(crate) merge_base: String,
    pub(crate) base_remote: String,
    pub(crate) base_ref: String,
    pub(crate) patch_count: usize,
}

impl PatchSourcePlan {
    fn from_patch_info(info: &PatchInfo) -> Self {
        let patch_count = info.patch_count.to_string();
        Self {
            stable_id: stable_plan_item_id(
                "patch-source",
                &[
                    ("source", &info.source),
                    ("branch", &info.branch),
                    ("head_commit", &info.head_commit),
                    ("merge_base", &info.merge_base),
                    ("base_remote", &info.base_remote),
                    ("base_ref", &info.base_ref),
                    ("patch_count", &patch_count),
                ],
            ),
            source: info.source.clone(),
            worktree_path: info.worktree_path.clone(),
            branch: info.branch.clone(),
            head_commit: info.head_commit.clone(),
            merge_base: info.merge_base.clone(),
            base_remote: info.base_remote.clone(),
            base_ref: info.base_ref.clone(),
            patch_count: info.patch_count,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntegrationPlan {
    pub(crate) entries: Vec<IntegrationEntryPlan>,
    pub(crate) rtlmq: Option<RtlmqIntegrationPlan>,
}

impl IntegrationPlan {
    fn from_profile(profile: &ProfileConfig) -> Self {
        let rtlmq = profile.integrations.rtlmq.as_ref().map(|rtlmq| {
            let tests_source = if rtlmq.tests_source.is_some() {
                "present"
            } else {
                "absent"
            };
            RtlmqIntegrationPlan {
                stable_id: stable_plan_item_id(
                    "integration-rtlmq",
                    &[("kind", "rtlmq"), ("tests_source", tests_source)],
                ),
                source: rtlmq.source.clone(),
                tests_source: rtlmq.tests_source.clone(),
            }
        });
        let entries = rtlmq
            .as_ref()
            .map(|rtlmq| {
                vec![IntegrationEntryPlan {
                    stable_id: rtlmq.stable_id.clone(),
                    kind: String::from("rtlmq"),
                }]
            })
            .unwrap_or_default();
        Self { entries, rtlmq }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntegrationEntryPlan {
    pub(crate) stable_id: String,
    pub(crate) kind: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RtlmqIntegrationPlan {
    pub(crate) stable_id: String,
    pub(crate) source: String,
    pub(crate) tests_source: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureIntentPlan {
    pub(crate) intents: Vec<FeatureIntentEntryPlan>,
}

impl FeatureIntentPlan {
    fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Ok(FeatureIntentPlan::from_feature_graph(&graph))
    }

    fn from_feature_graph(graph: &FeatureGraph) -> Self {
        let mut intents = graph
            .nodes()
            .map(FeatureIntentEntryPlan::from_feature_node)
            .collect::<Vec<_>>();
        intents.sort_by(|left, right| {
            left.action
                .cmp(&right.action)
                .then_with(|| left.name.cmp(&right.name))
        });
        Self { intents }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureIntentEntryPlan {
    pub(crate) stable_id: String,
    pub(crate) action: String,
    pub(crate) name: String,
    pub(crate) kind: Option<String>,
    pub(crate) roots: Vec<RelativeKernelPath>,
    pub(crate) remove_paths: Vec<RelativeKernelPath>,
    pub(crate) configs: Vec<KconfigSymbol>,
    pub(crate) remove_configs: Vec<KconfigSymbol>,
    pub(crate) exported_symbols: Vec<ExportedSymbol>,
    pub(crate) remove_exported_symbols: Vec<ExportedSymbol>,
    pub(crate) module_names: Vec<ModuleName>,
    pub(crate) remove_module_names: Vec<ModuleName>,
    pub(crate) module_aliases: Vec<ModuleAlias>,
    pub(crate) remove_module_aliases: Vec<ModuleAlias>,
    pub(crate) device_compatibles: Vec<DeviceCompatible>,
    pub(crate) remove_device_compatibles: Vec<DeviceCompatible>,
    pub(crate) acpi_ids: Vec<AcpiId>,
    pub(crate) remove_acpi_ids: Vec<AcpiId>,
    pub(crate) pci_ids: Vec<PciId>,
    pub(crate) remove_pci_ids: Vec<PciId>,
    pub(crate) usb_ids: Vec<UsbId>,
    pub(crate) remove_usb_ids: Vec<UsbId>,
    pub(crate) firmware_paths: Vec<FirmwarePath>,
    pub(crate) remove_firmware_paths: Vec<FirmwarePath>,
    pub(crate) initcalls: Vec<Initcall>,
    pub(crate) remove_initcalls: Vec<Initcall>,
    pub(crate) runtime_registrations: Vec<RuntimeRegistrationSurface>,
    pub(crate) remove_runtime_registrations: Vec<RuntimeRegistrationSurface>,
    pub(crate) docs: Vec<DocumentationPath>,
    pub(crate) remove_docs: Vec<DocumentationPath>,
    pub(crate) tools: Vec<ToolPath>,
    pub(crate) remove_tools: Vec<ToolPath>,
    pub(crate) samples: Vec<SamplePath>,
    pub(crate) remove_samples: Vec<SamplePath>,
    pub(crate) kunit_suites: Vec<KunitSuite>,
    pub(crate) remove_kunit_suites: Vec<KunitSuite>,
    pub(crate) kselftest_targets: Vec<KselftestTarget>,
    pub(crate) remove_kselftest_targets: Vec<KselftestTarget>,
    pub(crate) allow_public_header_removal: bool,
    pub(crate) allow_uapi_header_removal: bool,
    pub(crate) arch_scope: Vec<ArchName>,
    pub(crate) safety: Option<FeatureSafetyLevel>,
    pub(crate) preserve_uapi: bool,
    pub(crate) preserve_module_aliases: bool,
    pub(crate) require_clean_boot: bool,
    pub(crate) report_only: bool,
}

impl FeatureIntentEntryPlan {
    pub(crate) fn from_feature_node(node: &FeatureNode) -> Self {
        Self::from_intent(node.intent())
    }

    #[allow(dead_code)]
    pub(crate) fn from_config(
        action: &str,
        name: &str,
        intent: &FeatureIntentConfig,
    ) -> Result<Self> {
        let action = FeatureIntentAction::from_stable_name(action)?;
        let intent = FeatureIntent::from_config(action, name, intent)?;
        Ok(Self::from_intent(&intent))
    }

    pub(crate) fn from_intent(intent: &FeatureIntent) -> Self {
        let kind = intent.kind.map(|kind| kind.stable_name());
        let roots_key = intent.roots_key();
        let remove_paths_key = intent.remove_paths_key();
        let configs_key = intent.configs_key();
        let remove_configs_key = intent.remove_configs_key();
        let exported_symbols_key = intent.exported_symbols_key();
        let remove_exported_symbols_key = intent.remove_exported_symbols_key();
        let module_names_key = intent.module_names_key();
        let remove_module_names_key = intent.remove_module_names_key();
        let module_aliases_key = intent.module_aliases_key();
        let remove_module_aliases_key = intent.remove_module_aliases_key();
        let device_compatibles_key = intent.device_compatibles_key();
        let remove_device_compatibles_key = intent.remove_device_compatibles_key();
        let acpi_ids_key = intent.acpi_ids_key();
        let remove_acpi_ids_key = intent.remove_acpi_ids_key();
        let pci_ids_key = intent.pci_ids_key();
        let remove_pci_ids_key = intent.remove_pci_ids_key();
        let usb_ids_key = intent.usb_ids_key();
        let remove_usb_ids_key = intent.remove_usb_ids_key();
        let firmware_paths_key = intent.firmware_paths_key();
        let remove_firmware_paths_key = intent.remove_firmware_paths_key();
        let initcalls_key = intent.initcalls_key();
        let remove_initcalls_key = intent.remove_initcalls_key();
        let runtime_registrations_key = intent.runtime_registrations_key();
        let remove_runtime_registrations_key = intent.remove_runtime_registrations_key();
        let docs_key = intent.docs_key();
        let remove_docs_key = intent.remove_docs_key();
        let tools_key = intent.tools_key();
        let remove_tools_key = intent.remove_tools_key();
        let samples_key = intent.samples_key();
        let remove_samples_key = intent.remove_samples_key();
        let kunit_suites_key = intent.kunit_suites_key();
        let remove_kunit_suites_key = intent.remove_kunit_suites_key();
        let kselftest_targets_key = intent.kselftest_targets_key();
        let remove_kselftest_targets_key = intent.remove_kselftest_targets_key();
        let arch_scope_key = intent.arch_scope_key();
        let safety_key = intent
            .safety
            .map(|safety| safety.as_str())
            .unwrap_or("<none>");
        let stable_id = stable_plan_item_id(
            "feature-intent",
            &[
                ("action", intent.action.stable_name()),
                ("name", intent.id.as_str()),
                ("kind", kind.unwrap_or("<none>")),
                ("roots", roots_key.as_str()),
                ("remove_paths", remove_paths_key.as_str()),
                ("configs", configs_key.as_str()),
                ("remove_configs", remove_configs_key.as_str()),
                ("exported_symbols", exported_symbols_key.as_str()),
                (
                    "remove_exported_symbols",
                    remove_exported_symbols_key.as_str(),
                ),
                ("module_names", module_names_key.as_str()),
                ("remove_module_names", remove_module_names_key.as_str()),
                ("module_aliases", module_aliases_key.as_str()),
                ("remove_module_aliases", remove_module_aliases_key.as_str()),
                ("device_compatibles", device_compatibles_key.as_str()),
                (
                    "remove_device_compatibles",
                    remove_device_compatibles_key.as_str(),
                ),
                ("acpi_ids", acpi_ids_key.as_str()),
                ("remove_acpi_ids", remove_acpi_ids_key.as_str()),
                ("pci_ids", pci_ids_key.as_str()),
                ("remove_pci_ids", remove_pci_ids_key.as_str()),
                ("usb_ids", usb_ids_key.as_str()),
                ("remove_usb_ids", remove_usb_ids_key.as_str()),
                ("firmware_paths", firmware_paths_key.as_str()),
                ("remove_firmware_paths", remove_firmware_paths_key.as_str()),
                ("initcalls", initcalls_key.as_str()),
                ("remove_initcalls", remove_initcalls_key.as_str()),
                ("runtime_registrations", runtime_registrations_key.as_str()),
                (
                    "remove_runtime_registrations",
                    remove_runtime_registrations_key.as_str(),
                ),
                ("docs", docs_key.as_str()),
                ("remove_docs", remove_docs_key.as_str()),
                ("tools", tools_key.as_str()),
                ("remove_tools", remove_tools_key.as_str()),
                ("samples", samples_key.as_str()),
                ("remove_samples", remove_samples_key.as_str()),
                ("kunit_suites", kunit_suites_key.as_str()),
                ("remove_kunit_suites", remove_kunit_suites_key.as_str()),
                ("kselftest_targets", kselftest_targets_key.as_str()),
                (
                    "remove_kselftest_targets",
                    remove_kselftest_targets_key.as_str(),
                ),
                (
                    "allow_public_header_removal",
                    bool_token(intent.allow_public_header_removal),
                ),
                (
                    "allow_uapi_header_removal",
                    bool_token(intent.allow_uapi_header_removal),
                ),
                ("arch_scope", arch_scope_key.as_str()),
                ("safety", safety_key),
                ("preserve_uapi", bool_token(intent.preserve_uapi)),
                (
                    "preserve_module_aliases",
                    bool_token(intent.preserve_module_aliases),
                ),
                ("require_clean_boot", bool_token(intent.require_clean_boot)),
                ("report_only", bool_token(intent.report_only)),
            ],
        );
        Self {
            stable_id,
            action: intent.action.stable_name().to_string(),
            name: intent.id.as_str().to_string(),
            kind: kind.map(str::to_string),
            roots: intent
                .roots
                .iter()
                .map(|root| root.as_relative_kernel_path().clone())
                .collect(),
            remove_paths: intent.remove_paths.clone(),
            configs: intent.configs.clone(),
            remove_configs: intent.remove_configs.clone(),
            exported_symbols: intent.exported_symbols.clone(),
            remove_exported_symbols: intent.remove_exported_symbols.clone(),
            module_names: intent.module_names.clone(),
            remove_module_names: intent.remove_module_names.clone(),
            module_aliases: intent.module_aliases.clone(),
            remove_module_aliases: intent.remove_module_aliases.clone(),
            device_compatibles: intent.device_compatibles.clone(),
            remove_device_compatibles: intent.remove_device_compatibles.clone(),
            acpi_ids: intent.acpi_ids.clone(),
            remove_acpi_ids: intent.remove_acpi_ids.clone(),
            pci_ids: intent.pci_ids.clone(),
            remove_pci_ids: intent.remove_pci_ids.clone(),
            usb_ids: intent.usb_ids.clone(),
            remove_usb_ids: intent.remove_usb_ids.clone(),
            firmware_paths: intent.firmware_paths.clone(),
            remove_firmware_paths: intent.remove_firmware_paths.clone(),
            initcalls: intent.initcalls.clone(),
            remove_initcalls: intent.remove_initcalls.clone(),
            runtime_registrations: intent.runtime_registrations.clone(),
            remove_runtime_registrations: intent.remove_runtime_registrations.clone(),
            docs: intent.docs.clone(),
            remove_docs: intent.remove_docs.clone(),
            tools: intent.tools.clone(),
            remove_tools: intent.remove_tools.clone(),
            samples: intent.samples.clone(),
            remove_samples: intent.remove_samples.clone(),
            kunit_suites: intent.kunit_suites.clone(),
            remove_kunit_suites: intent.remove_kunit_suites.clone(),
            kselftest_targets: intent.kselftest_targets.clone(),
            remove_kselftest_targets: intent.remove_kselftest_targets.clone(),
            allow_public_header_removal: intent.allow_public_header_removal,
            allow_uapi_header_removal: intent.allow_uapi_header_removal,
            arch_scope: intent.scope.arch_scope().to_vec(),
            safety: intent.safety,
            preserve_uapi: intent.preserve_uapi,
            preserve_module_aliases: intent.preserve_module_aliases,
            require_clean_boot: intent.require_clean_boot,
            report_only: intent.report_only,
        }
    }
}

fn sorted_arch_names(values: &[String]) -> Result<Vec<ArchName>> {
    let mut values = values
        .iter()
        .map(|value| ArchName::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn bool_token(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureResolutionSource {
    NoRemoval,
    DirectSlim,
    NamedFeatureRemove,
    CombinedSlimAndNamedFeature,
}

#[allow(dead_code)]
impl FeatureResolutionSource {
    pub(crate) const ALL: [Self; 4] = [
        Self::NoRemoval,
        Self::DirectSlim,
        Self::NamedFeatureRemove,
        Self::CombinedSlimAndNamedFeature,
    ];

    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::NoRemoval => "no_removal_input",
            Self::DirectSlim => "direct_slim_input",
            Self::NamedFeatureRemove => "named_feature_remove_input",
            Self::CombinedSlimAndNamedFeature => "combined_slim_and_named_feature_input",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureResolutionState {
    source: FeatureResolutionSource,
    remove_paths: Vec<RelativeKernelPath>,
    remove_configs: Vec<KconfigSymbol>,
    preserve_paths: Vec<RelativeKernelPath>,
    preserve_configs: Vec<KconfigSymbol>,
    set_defaults: BTreeMap<KconfigSymbol, String>,
    abi_policy: AbiPolicyConfig,
    feature_safety_levels: BTreeMap<String, FeatureSafetyLevel>,
    feature_arch_scopes: BTreeMap<String, Vec<ArchName>>,
    feature_test_matrices: BTreeMap<String, FeatureTestMatrixConfig>,
    feature_report_modes: BTreeMap<String, FeatureReportModeConfig>,
    unsafe_allow_root_path_removal: bool,
}

#[allow(dead_code)]
impl FeatureResolutionState {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source: FeatureResolutionSource,
        remove_paths: Vec<RelativeKernelPath>,
        remove_configs: Vec<KconfigSymbol>,
        preserve_paths: Vec<RelativeKernelPath>,
        preserve_configs: Vec<KconfigSymbol>,
        set_defaults: BTreeMap<KconfigSymbol, String>,
        abi_policy: AbiPolicyConfig,
        feature_safety_levels: BTreeMap<String, FeatureSafetyLevel>,
        feature_arch_scopes: BTreeMap<String, Vec<ArchName>>,
        feature_test_matrices: BTreeMap<String, FeatureTestMatrixConfig>,
        feature_report_modes: BTreeMap<String, FeatureReportModeConfig>,
        unsafe_allow_root_path_removal: bool,
    ) -> Result<Self> {
        let mut remove_paths = remove_paths;
        remove_paths.sort();
        remove_paths.dedup();
        let mut remove_configs = remove_configs;
        remove_configs.sort();
        remove_configs.dedup();
        let mut preserve_paths = preserve_paths;
        preserve_paths.sort();
        preserve_paths.dedup();
        let mut preserve_configs = preserve_configs;
        preserve_configs.sort();
        preserve_configs.dedup();
        let mut feature_arch_scopes = feature_arch_scopes;
        for scopes in feature_arch_scopes.values_mut() {
            scopes.sort();
            scopes.dedup();
        }

        if source == FeatureResolutionSource::NoRemoval
            && (!remove_paths.is_empty()
                || !remove_configs.is_empty()
                || !set_defaults.is_empty()
                || !feature_safety_levels.is_empty()
                || unsafe_allow_root_path_removal)
        {
            anyhow::bail!("feature resolution without removal input cannot contain removal facts");
        }

        Ok(Self {
            source,
            remove_paths,
            remove_configs,
            preserve_paths,
            preserve_configs,
            set_defaults,
            abi_policy,
            feature_safety_levels,
            feature_arch_scopes,
            feature_test_matrices,
            feature_report_modes,
            unsafe_allow_root_path_removal,
        })
    }

    fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let direct_slim_input = profile.removal_input().is_some_and(|slim| !slim.is_noop());
        let named_feature_remove_input = profile
            .features
            .remove
            .values()
            .any(|intent| intent.declares_removal_input());
        if let Some((name, _)) = profile
            .features
            .remove
            .iter()
            .find(|(_, intent)| intent.safety.is_some() && !intent.declares_removal_input())
        {
            anyhow::bail!("features.remove.{name}.safety requires removal input");
        }
        for (section, intents) in [
            ("features.remove", &profile.features.remove),
            ("features.preserve", &profile.features.preserve),
        ] {
            if let Some((name, _)) = intents.iter().find(|(_, intent)| {
                !intent.arch_scope.is_empty() && !intent.declares_feature_input()
            }) {
                anyhow::bail!("{section}.{name}.arch_scope requires feature input");
            }
            if let Some((name, _)) = intents.iter().find(|(_, intent)| {
                intent.declares_test_matrix() && !intent.declares_feature_input()
            }) {
                anyhow::bail!("{section}.{name}.require_clean_boot requires feature input");
            }
            if let Some((name, _)) = intents.iter().find(|(_, intent)| {
                intent.declares_report_mode() && !intent.declares_feature_input()
            }) {
                anyhow::bail!("{section}.{name}.report_only requires feature input");
            }
        }
        let source = match (direct_slim_input, named_feature_remove_input) {
            (false, false) => FeatureResolutionSource::NoRemoval,
            (true, false) => FeatureResolutionSource::DirectSlim,
            (false, true) => FeatureResolutionSource::NamedFeatureRemove,
            (true, true) => FeatureResolutionSource::CombinedSlimAndNamedFeature,
        };

        let removal_input = profile.effective_removal_input();
        let preservation_input = profile.effective_preservation_input();
        let abi_policy = profile.effective_abi_policy();
        let feature_safety_levels = profile.effective_feature_safety_levels();
        let feature_test_matrices = profile.effective_feature_test_matrices();
        let feature_report_modes = profile.effective_feature_report_modes();
        let feature_arch_scopes = profile
            .effective_feature_arch_scopes()
            .into_iter()
            .map(|(name, scopes)| {
                Ok((
                    name,
                    scopes
                        .into_iter()
                        .map(ArchName::new)
                        .collect::<Result<Vec<_>>>()?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        if removal_input.is_none() && preservation_input.is_none() {
            return Self::new(
                source,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                BTreeMap::new(),
                abi_policy,
                feature_safety_levels,
                feature_arch_scopes,
                feature_test_matrices,
                feature_report_modes,
                false,
            );
        }
        let slim = removal_input.unwrap_or_default();
        let manifest = RemovalManifest::from_slim_config_with_abi_policy_and_preservation(
            &slim,
            preservation_input.as_ref(),
            &abi_policy,
        )?;
        Self::new(
            source,
            manifest
                .removed_paths()
                .iter()
                .map(|path| {
                    if slim.unsafe_allow_root_path_removal && path == Path::new(".") {
                        RelativeKernelPath::new_for_explicit_unsafe_root_removal(path.clone())
                    } else {
                        RelativeKernelPath::new(path.clone())
                    }
                })
                .collect::<Result<Vec<_>>>()?,
            manifest
                .removed_config_symbols_vec()
                .into_iter()
                .map(KconfigSymbol::new)
                .collect::<Result<Vec<_>>>()?,
            manifest
                .preserved_paths()
                .iter()
                .map(|path| RelativeKernelPath::new_for_explicit_unsafe_root_removal(path.clone()))
                .collect::<Result<Vec<_>>>()?,
            manifest
                .preserved_config_symbols_vec()
                .into_iter()
                .map(KconfigSymbol::new)
                .collect::<Result<Vec<_>>>()?,
            manifest
                .default_overrides()
                .iter()
                .map(|(symbol, value)| Ok((KconfigSymbol::new(symbol.clone())?, value.clone())))
                .collect::<Result<BTreeMap<_, _>>>()?,
            abi_policy,
            feature_safety_levels,
            feature_arch_scopes,
            feature_test_matrices,
            feature_report_modes,
            slim.unsafe_allow_root_path_removal,
        )
    }

    pub(crate) fn source(&self) -> FeatureResolutionSource {
        self.source
    }

    pub(crate) fn remove_paths(&self) -> &[RelativeKernelPath] {
        &self.remove_paths
    }

    pub(crate) fn remove_configs(&self) -> &[KconfigSymbol] {
        &self.remove_configs
    }

    pub(crate) fn preserve_paths(&self) -> &[RelativeKernelPath] {
        &self.preserve_paths
    }

    pub(crate) fn preserve_configs(&self) -> &[KconfigSymbol] {
        &self.preserve_configs
    }

    pub(crate) fn set_defaults(&self) -> &BTreeMap<KconfigSymbol, String> {
        &self.set_defaults
    }

    pub(crate) fn abi_policy(&self) -> &AbiPolicyConfig {
        &self.abi_policy
    }

    pub(crate) fn feature_safety_levels(&self) -> &BTreeMap<String, FeatureSafetyLevel> {
        &self.feature_safety_levels
    }

    pub(crate) fn feature_arch_scopes(&self) -> &BTreeMap<String, Vec<ArchName>> {
        &self.feature_arch_scopes
    }

    pub(crate) fn feature_test_matrices(&self) -> &BTreeMap<String, FeatureTestMatrixConfig> {
        &self.feature_test_matrices
    }

    pub(crate) fn feature_report_modes(&self) -> &BTreeMap<String, FeatureReportModeConfig> {
        &self.feature_report_modes
    }

    pub(crate) fn unsafe_allow_root_path_removal(&self) -> bool {
        self.unsafe_allow_root_path_removal
    }

    pub(crate) fn is_noop(&self) -> bool {
        self.remove_paths.is_empty()
            && self.remove_configs.is_empty()
            && self.set_defaults.is_empty()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AbiDecisionState {
    policy: AbiPolicyConfig,
    allow_public_header_removal: bool,
    allow_uapi_header_removal: bool,
    approved_public_headers: Vec<HeaderPath>,
    approved_uapi_paths: Vec<UapiPath>,
}

#[allow(dead_code)]
impl AbiDecisionState {
    pub(crate) fn new(
        policy: AbiPolicyConfig,
        remove_paths: &[RelativeKernelPath],
    ) -> Result<Self> {
        let mut approved_public_headers = Vec::new();
        let mut approved_uapi_paths = Vec::new();

        for path in remove_paths {
            let path = path.as_path();
            if let Ok(uapi_path) = UapiPath::new(path.to_path_buf()) {
                if !policy.allow_uapi_header_removal {
                    anyhow::bail!(
                        "ABI decision rejected UAPI removal without explicit approval: {}; set abi.allow_uapi_header_removal = true",
                        uapi_path.as_str()
                    );
                }
                approved_uapi_paths.push(uapi_path);
                continue;
            }

            if crate::abi::is_public_header_path(path) {
                let Ok(header) = HeaderPath::new(path.to_string_lossy().into_owned()) else {
                    continue;
                };
                if !policy.allow_public_header_removal {
                    anyhow::bail!(
                        "ABI decision rejected public header removal without explicit approval: {}; set abi.allow_public_header_removal = true",
                        header.as_str()
                    );
                }
                approved_public_headers.push(header);
            }
        }

        approved_public_headers.sort();
        approved_public_headers.dedup();
        approved_uapi_paths.sort();
        approved_uapi_paths.dedup();

        Ok(Self {
            allow_public_header_removal: policy.allow_public_header_removal,
            allow_uapi_header_removal: policy.allow_uapi_header_removal,
            policy,
            approved_public_headers,
            approved_uapi_paths,
        })
    }

    fn from_feature_resolution(resolution: &FeatureResolutionState) -> Result<Self> {
        Self::new(resolution.abi_policy().clone(), resolution.remove_paths())
    }

    pub(crate) fn policy(&self) -> &AbiPolicyConfig {
        &self.policy
    }

    pub(crate) fn allow_public_header_removal(&self) -> bool {
        self.allow_public_header_removal
    }

    pub(crate) fn allow_uapi_header_removal(&self) -> bool {
        self.allow_uapi_header_removal
    }

    pub(crate) fn approved_public_headers(&self) -> &[HeaderPath] {
        &self.approved_public_headers
    }

    pub(crate) fn approved_uapi_paths(&self) -> &[UapiPath] {
        &self.approved_uapi_paths
    }

    pub(crate) fn has_abi_sensitive_removals(&self) -> bool {
        !self.approved_public_headers.is_empty() || !self.approved_uapi_paths.is_empty()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrunePlan {
    pub(crate) remove_paths: Vec<RelativeKernelPath>,
    pub(crate) remove_configs: Vec<KconfigSymbol>,
    pub(crate) preserve_paths: Vec<RelativeKernelPath>,
    pub(crate) preserve_configs: Vec<KconfigSymbol>,
    pub(crate) set_defaults: BTreeMap<KconfigSymbol, String>,
    pub(crate) abi_policy: AbiPolicyConfig,
    pub(crate) unsafe_allow_root_path_removal: bool,
}

impl PrunePlan {
    fn from_feature_resolution(resolution: &FeatureResolutionState) -> Self {
        Self {
            remove_paths: resolution.remove_paths.clone(),
            remove_configs: resolution.remove_configs.clone(),
            preserve_paths: resolution.preserve_paths.clone(),
            preserve_configs: resolution.preserve_configs.clone(),
            set_defaults: resolution.set_defaults.clone(),
            abi_policy: resolution.abi_policy.clone(),
            unsafe_allow_root_path_removal: resolution.unsafe_allow_root_path_removal,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReducerPlan {
    pub(crate) max_fixup_passes: usize,
    pub(crate) report_unsupported_expressions: bool,
    pub(crate) fail_on_unknown_diagnostics: bool,
    pub(crate) reject_unproven_fixups: bool,
    pub(crate) reject_unreasoned_edits: bool,
    pub(crate) reject_speculative_fallout_edits: bool,
    pub(crate) fail_on_missing_prune_paths: bool,
    pub(crate) ignore_unsupported_special_removals: bool,
}

impl ReducerPlan {
    fn from_config(config: &ReducerConfig) -> Self {
        Self {
            max_fixup_passes: config.max_fixup_passes,
            report_unsupported_expressions: config.report_unsupported_expressions,
            fail_on_unknown_diagnostics: config.fail_on_unknown_diagnostics,
            reject_unproven_fixups: config.reject_unproven_fixups,
            reject_unreasoned_edits: config.reject_unreasoned_edits,
            reject_speculative_fallout_edits: config.reject_speculative_fallout_edits,
            fail_on_missing_prune_paths: config.fail_on_missing_prune_paths,
            ignore_unsupported_special_removals: config.ignore_unsupported_special_removals,
        }
    }

    pub(crate) fn strict_mode(&self) -> bool {
        self.report_unsupported_expressions
            && self.fail_on_unknown_diagnostics
            && self.reject_unproven_fixups
            && self.reject_unreasoned_edits
            && self.reject_speculative_fallout_edits
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BuildMatrixPlan {
    pub(crate) enabled: bool,
    pub(crate) presets: Vec<String>,
    pub(crate) arches: Vec<ArchName>,
    pub(crate) config_targets: Vec<String>,
    pub(crate) targets: Vec<String>,
    pub(crate) randconfig_seed: Option<String>,
    pub(crate) jobs: Option<usize>,
    pub(crate) fail_on_error: bool,
}

impl BuildMatrixPlan {
    fn from_config(config: &BuildMatrixConfig) -> Result<Self> {
        if config
            .randconfig_seed
            .as_deref()
            .is_some_and(|seed| seed.trim().is_empty())
        {
            anyhow::bail!("build matrix plan randconfig seed must not be empty");
        }
        if config.jobs == Some(0) {
            anyhow::bail!("build matrix plan jobs must be greater than zero");
        }
        Ok(Self {
            enabled: config.enabled,
            presets: sorted_nonempty_strings(&config.presets)?,
            arches: sorted_arch_names(&config.arches)?,
            config_targets: sorted_nonempty_strings(&config.config_targets)?,
            targets: sorted_nonempty_strings(&config.targets)?,
            randconfig_seed: config.randconfig_seed.clone(),
            jobs: config.jobs,
            fail_on_error: config.fail_on_error,
        })
    }
}

fn sorted_nonempty_strings(values: &[String]) -> Result<Vec<String>> {
    let mut values = values.to_vec();
    if values.iter().any(|value| value.trim().is_empty()) {
        anyhow::bail!("build matrix plan list values must not be empty");
    }
    values.sort();
    values.dedup();
    Ok(values)
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelftestPlan {
    pub(crate) enabled: bool,
    pub(crate) check_kconfig_sources: bool,
    pub(crate) check_makefiles: bool,
    pub(crate) kernel_builds: Vec<KernelBuildPlan>,
    pub(crate) commands: Vec<String>,
}

impl SelftestPlan {
    fn from_config(config: &SelfTestConfig) -> Result<Self> {
        Ok(Self {
            enabled: config.enabled,
            check_kconfig_sources: config.check_kconfig_sources,
            check_makefiles: config.check_makefiles,
            kernel_builds: config
                .kernel_builds
                .iter()
                .map(KernelBuildPlan::from_config)
                .collect::<Result<Vec<_>>>()?,
            commands: config.commands.clone(),
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KernelBuildPlan {
    pub(crate) name: Option<String>,
    pub(crate) arch: Option<ArchName>,
    pub(crate) config_target: Option<String>,
    pub(crate) targets: Vec<String>,
    pub(crate) output_dir: Option<KernelBuildDir>,
    pub(crate) jobs: Option<usize>,
    pub(crate) clean: bool,
    pub(crate) make_program: Option<String>,
    pub(crate) make_args: Vec<String>,
    pub(crate) env: BTreeMap<String, String>,
}

impl KernelBuildPlan {
    fn from_config(config: &KernelBuildConfig) -> Result<Self> {
        Ok(Self {
            name: config.name.clone(),
            arch: config
                .env
                .get("ARCH")
                .map(|arch| ArchName::new(arch.as_str()))
                .transpose()?,
            config_target: config.config_target.clone(),
            targets: config.targets.clone(),
            output_dir: config
                .output_dir
                .as_deref()
                .map(KernelBuildDir::new)
                .transpose()?,
            jobs: config.jobs,
            clean: config.clean,
            make_program: config.make_program.clone(),
            make_args: config.make_args.clone(),
            env: config.env.clone(),
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputPlan {
    pub(crate) output_path: OutputRepoPath,
    pub(crate) branch: String,
    pub(crate) mode: String,
    pub(crate) lockfile_path: Option<LockfilePath>,
    pub(crate) naming: OutputNamingPlan,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum OutputPlanMode {
    UnmodifiedUpstream,
    Slimmed,
}

#[allow(dead_code)]
impl OutputPlanMode {
    pub(crate) const ALL: [Self; 2] = [Self::UnmodifiedUpstream, Self::Slimmed];

    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::UnmodifiedUpstream => "unmodified-upstream",
            Self::Slimmed => "slimmed",
        }
    }

    pub(crate) fn from_profile(profile: &ProfileConfig) -> Self {
        if profile.effective_removal_input().is_none() {
            Self::UnmodifiedUpstream
        } else {
            Self::Slimmed
        }
    }

    fn from_stable_name(value: &str) -> Result<Self> {
        match value {
            "unmodified-upstream" => Ok(Self::UnmodifiedUpstream),
            "slimmed" => Ok(Self::Slimmed),
            _ => anyhow::bail!(
                "resolved generate mode must be a stable token: expected unmodified-upstream or slimmed"
            ),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputNamingPlan {
    pub(crate) project_name: String,
    pub(crate) profile_name: String,
    pub(crate) branch_prefix: String,
    pub(crate) explicit_branch: Option<String>,
    pub(crate) base_ref: String,
    pub(crate) base_commit: String,
}

impl OutputPlan {
    fn new(
        config: &KslimConfig,
        profile: &ProfileConfig,
        base: &ResolvedBase,
        branch: impl Into<String>,
        mode: impl Into<String>,
    ) -> Result<Self> {
        let output_path = PathBuf::from(&config.output.path);
        if output_path.as_os_str().is_empty() {
            anyhow::bail!("resolved output path is empty");
        }
        let output_path = OutputRepoPath::new(output_path)?;
        let branch = branch.into();
        if branch.trim().is_empty() {
            anyhow::bail!("resolved output branch is empty");
        }
        let mode = mode.into();
        let mode = OutputPlanMode::from_stable_name(&mode)?
            .stable_name()
            .to_string();
        Ok(Self {
            output_path,
            branch,
            mode,
            lockfile_path: None,
            naming: OutputNamingPlan {
                project_name: config.project.name.clone(),
                profile_name: profile.profile.name.clone(),
                branch_prefix: config.output.branch_prefix.clone(),
                explicit_branch: config.output.branch.clone(),
                base_ref: base.r#ref.clone(),
                base_commit: base.commit.clone(),
            },
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CandidateTreeState {
    pub(crate) tree: CandidateTreePath,
    pub(crate) metadata_dir: CandidateMetadataDir,
    pub(crate) materialized: bool,
    pub(crate) integrated: bool,
    pub(crate) pruned: bool,
    pub(crate) reduced: bool,
    pub(crate) selftested: bool,
}

#[allow(dead_code)]
impl CandidateTreeState {
    pub(crate) fn new(
        tree: CandidateTreePath,
        metadata_dir: CandidateMetadataDir,
        materialized: bool,
        integrated: bool,
        pruned: bool,
        reduced: bool,
        selftested: bool,
    ) -> Result<Self> {
        if !materialized && (integrated || pruned || reduced || selftested) {
            anyhow::bail!("candidate tree state cannot advance before materialization");
        }
        let metadata_dir =
            CandidateMetadataDir::new_in_candidate_tree(&tree, metadata_dir.as_path())?;
        Ok(Self {
            tree,
            metadata_dir,
            materialized,
            integrated,
            pruned,
            reduced,
            selftested,
        })
    }

    pub(crate) fn from_materialized_tree(tree: impl Into<PathBuf>) -> Result<Self> {
        let tree = CandidateTreePath::new(tree)?;
        let metadata_dir = crate::output_repo::candidate_metadata_dir(&tree)?;
        Self::new(tree, metadata_dir, true, false, false, false, false)
    }

    pub(crate) fn mark_integrated(&mut self) -> Result<()> {
        self.ensure_materialized_for("integrated")?;
        self.integrated = true;
        Ok(())
    }

    pub(crate) fn mark_pruned(&mut self) -> Result<()> {
        self.ensure_materialized_for("pruned")?;
        self.pruned = true;
        Ok(())
    }

    pub(crate) fn mark_reduced(&mut self) -> Result<()> {
        self.ensure_materialized_for("reduced")?;
        self.reduced = true;
        Ok(())
    }

    pub(crate) fn mark_selftested(&mut self) -> Result<()> {
        self.ensure_materialized_for("selftested")?;
        self.selftested = true;
        Ok(())
    }

    fn ensure_materialized_for(&self, phase: &str) -> Result<()> {
        if !self.materialized {
            anyhow::bail!(
                "candidate tree state cannot be marked {} before materialization",
                phase
            );
        }
        Ok(())
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PublishedSnapshotState {
    output_repo: OutputRepoPath,
    metadata_dir: PublishedMetadataDir,
    branch: OutputBranchName,
    commit: GitCommitId,
    lockfile: LockfilePath,
}

/// Private proof material for converting a successful commit phase into
/// published snapshot state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommittedOutputSnapshot {
    output_repo: OutputRepoPath,
    branch: OutputBranchName,
    commit: GitCommitId,
    lockfile: LockfilePath,
}

impl CommittedOutputSnapshot {
    pub(crate) fn from_successful_commit(
        output_repo: impl Into<PathBuf>,
        lockfile: LockfilePath,
        commit: &SuccessfulCommitResult,
    ) -> Result<Self> {
        let output_repo = OutputRepoPath::new(output_repo)?;
        Ok(Self {
            output_repo,
            branch: OutputBranchName::new(commit.branch.clone())?,
            commit: GitCommitId::new(commit.output_commit.clone())?,
            lockfile,
        })
    }
}

impl PublishedSnapshotState {
    pub(crate) fn from_committed_output(snapshot: CommittedOutputSnapshot) -> Result<Self> {
        let metadata_dir = crate::output_repo::published_metadata_dir(&snapshot.output_repo)?;
        Ok(Self {
            output_repo: snapshot.output_repo,
            metadata_dir,
            branch: snapshot.branch,
            commit: snapshot.commit,
            lockfile: snapshot.lockfile,
        })
    }

    pub(crate) fn output_repo(&self) -> &OutputRepoPath {
        &self.output_repo
    }

    pub(crate) fn metadata_dir(&self) -> &PublishedMetadataDir {
        &self.metadata_dir
    }

    pub(crate) fn branch(&self) -> &OutputBranchName {
        &self.branch
    }

    pub(crate) fn commit(&self) -> &GitCommitId {
        &self.commit
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum GenerateErrorKind {
    Resolve,
    Materialize,
    Integrate,
    Reduce,
    Selftest,
    Metadata,
    Commit,
    Publish,
}

impl GenerateErrorKind {
    pub(crate) fn from_stage(stage: GenerateStage) -> Self {
        match stage {
            GenerateStage::Resolve => Self::Resolve,
            GenerateStage::Materialize => Self::Materialize,
            GenerateStage::Integrate => Self::Integrate,
            GenerateStage::Prune | GenerateStage::Reduce => Self::Reduce,
            GenerateStage::Selftest => Self::Selftest,
            GenerateStage::Metadata => Self::Metadata,
            GenerateStage::Commit => Self::Commit,
            GenerateStage::Publish => Self::Publish,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Resolve => "resolve",
            Self::Materialize => "materialize",
            Self::Integrate => "integrate",
            Self::Reduce => "reduce",
            Self::Selftest => "selftest",
            Self::Metadata => "metadata",
            Self::Commit => "commit",
            Self::Publish => "publish",
        }
    }
}

impl std::fmt::Display for GenerateErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenerateAttemptFailure {
    stage: GenerateStage,
    error_kind: GenerateErrorKind,
    message: String,
    attempt_metadata_dir: AttemptMetadataDir,
    partial_reports: Vec<ReportPath>,
}

impl GenerateAttemptFailure {
    pub(crate) fn from_stage(
        stage: GenerateStage,
        message: impl Into<String>,
        attempt_metadata_dir: AttemptMetadataDir,
        partial_reports: Vec<ReportPath>,
    ) -> Result<Self> {
        Self::new(
            stage,
            GenerateErrorKind::from_stage(stage),
            message,
            attempt_metadata_dir,
            partial_reports,
        )
    }

    pub(crate) fn new(
        stage: GenerateStage,
        error_kind: GenerateErrorKind,
        message: impl Into<String>,
        attempt_metadata_dir: AttemptMetadataDir,
        partial_reports: Vec<ReportPath>,
    ) -> Result<Self> {
        let message = message.into();
        if message.trim().is_empty() {
            anyhow::bail!("generate attempt failure message is empty");
        }
        let mut partial_reports = partial_reports;
        for report in &partial_reports {
            if !report.as_path().starts_with(attempt_metadata_dir.as_path()) {
                anyhow::bail!(
                    "generate attempt failure report outside attempt metadata: {}",
                    report.as_path().display()
                );
            }
        }
        partial_reports.sort();
        partial_reports.dedup();
        Ok(Self {
            stage,
            error_kind,
            message,
            attempt_metadata_dir,
            partial_reports,
        })
    }

    pub(crate) fn stage(&self) -> GenerateStage {
        self.stage
    }

    pub(crate) fn error_kind(&self) -> GenerateErrorKind {
        self.error_kind
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    #[allow(dead_code)]
    pub(crate) fn attempt_metadata_dir(&self) -> &AttemptMetadataDir {
        &self.attempt_metadata_dir
    }

    #[allow(dead_code)]
    pub(crate) fn partial_reports(&self) -> &[ReportPath] {
        &self.partial_reports
    }
}

#[cfg(test)]
mod selftest_tests;
#[cfg(test)]
mod tests;
