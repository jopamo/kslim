use crate::state::{OutputPlanMode, RequestedGenerateState, ResolvedCandidateState};
use crate::generate::GenerateOptions;
use crate::config::{self, KslimConfig, ProfileConfig};
use crate::core::{
    append_stable_key_value_line, bool_token, escape_stable_value, sha256_hex as core_sha256_hex,
};
use crate::lockfile::{self, ResolvedBase};
use crate::model::ToolVersion;
use crate::paths::LockfilePath;
use crate::{output_repo, patches, upstream};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
mod build_matrix_fingerprint;
#[cfg(test)]
mod content_hash_tests;
mod feature_intent_fingerprint;
#[cfg(test)]
mod fingerprint_serialization_tests;
mod source_map_fingerprint;
mod source_map_sanitization;
mod frozen_plan;
mod summary;

#[allow(unused_imports)]
pub(crate) use frozen_plan::{
    ensure_tree_matches_frozen_base, load_frozen_plan, write_frozen_plan_for_request,
    FrozenPlanInputs, LoadedFrozenPlan,
};
pub(crate) use summary::{resolve_plan_summary, GeneratePlanSummary};
/// Stable identity for an immutable generate plan.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PlanId(String);
#[allow(dead_code)]
impl PlanId {
    pub(crate) fn new(id: impl Into<String>) -> Result<Self> {
        let id = id.into();
        if id.trim().is_empty() {
            anyhow::bail!("generate plan id is empty");
        }
        Ok(Self(id))
    }
    fn from_fingerprint(fingerprint: &PlanFingerprint) -> Result<Self> {
        let digest = fingerprint
            .as_str()
            .strip_prefix("fingerprint-")
            .unwrap_or_else(|| fingerprint.as_str());
        Self::new(format!("plan-{digest}"))
    }
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ConfigContentHash(String);
#[allow(dead_code)]
impl ConfigContentHash {
    fn from_resolved_state(resolved: &ResolvedCandidateState) -> Result<Self> {
        let mut source = String::new();
        append_fingerprint_line(&mut source, "format", "kslim-resolved-config-content-v1");
        append_resolved_candidate_fingerprint_lines(&mut source, resolved, false);
        Self::new(format!("config-{}", sha256_hex(&source)))
    }
    pub(crate) fn new(hash: impl Into<String>) -> Result<Self> {
        let hash = hash.into();
        if hash.trim().is_empty() {
            anyhow::bail!("generate plan config content hash is empty");
        }
        Ok(Self(hash))
    }
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlanFingerprint {
    digest: String,
    stable_serialization: String,
}
#[allow(dead_code)]
impl PlanFingerprint {
    fn from_parts(
        requested: &RequestedGenerateState,
        resolved: &ResolvedCandidateState,
        config_content_hash: &ConfigContentHash,
        tool_version: &ToolVersion,
        source_maps: Option<&GeneratePlanSourceMaps>,
    ) -> Result<Self> {
        let stable_serialization = stable_plan_fingerprint_serialization(
            requested,
            resolved,
            config_content_hash,
            tool_version,
            source_maps,
        );
        Self::new(
            format!("fingerprint-{}", sha256_hex(&stable_serialization)),
            stable_serialization,
        )
    }
    pub(crate) fn new(
        digest: impl Into<String>,
        stable_serialization: impl Into<String>,
    ) -> Result<Self> {
        let digest = digest.into();
        if digest.trim().is_empty() {
            anyhow::bail!("generate plan fingerprint is empty");
        }
        let stable_serialization = stable_serialization.into();
        if stable_serialization.trim().is_empty() {
            anyhow::bail!("generate plan fingerprint serialization is empty");
        }
        Ok(Self {
            digest,
            stable_serialization,
        })
    }
    pub(crate) fn as_str(&self) -> &str {
        &self.digest
    }
    pub(crate) fn stable_serialization(&self) -> &str {
        &self.stable_serialization
    }
}
/// Immutable generate plan tying requested state to resolved candidate state.
///
/// A `GeneratePlan` is still candidate truth only. It records enough resolved
/// input to drive later materialization, but it does not contain candidate tree
/// state, published output state, or failure state.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct GeneratePlan {
    pub(crate) requested: RequestedGenerateState,
    pub(crate) resolved: ResolvedCandidateState,
    pub(crate) plan_id: PlanId,
    pub(crate) created_with: ToolVersion,
    pub(crate) config_content_hash: ConfigContentHash,
    pub(crate) fingerprint: PlanFingerprint,
    pub(crate) source_maps: Option<GeneratePlanSourceMaps>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct GeneratePlanSourceMaps {
    pub(crate) config: config::ConfigSourceMap,
    pub(crate) profile: config::ConfigSourceMap,
    pub(crate) overrides: config::ConfigSourceMap,
}
impl GeneratePlanSourceMaps {
    pub(crate) fn new(
        config: config::ConfigSourceMap,
        profile: config::ConfigSourceMap,
        overrides: config::ConfigSourceMap,
    ) -> Self {
        Self {
            config,
            profile,
            overrides,
        }
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.config.is_empty() && self.profile.is_empty() && self.overrides.is_empty()
    }
}
#[allow(dead_code)]
impl GeneratePlan {
    pub(crate) fn new(
        requested: RequestedGenerateState,
        resolved: ResolvedCandidateState,
    ) -> Result<Self> {
        let config_content_hash = ConfigContentHash::from_resolved_state(&resolved)?;
        Self::from_parts(
            requested,
            resolved,
            config_content_hash,
            ToolVersion::current()?,
        )
    }
    pub(crate) fn from_parts(
        requested: RequestedGenerateState,
        resolved: ResolvedCandidateState,
        config_content_hash: ConfigContentHash,
        created_with: ToolVersion,
    ) -> Result<Self> {
        let fingerprint = PlanFingerprint::from_parts(
            &requested,
            &resolved,
            &config_content_hash,
            &created_with,
            None,
        )?;
        let plan_id = PlanId::from_fingerprint(&fingerprint)?;
        Ok(Self {
            requested,
            resolved,
            plan_id,
            created_with,
            config_content_hash,
            fingerprint,
            source_maps: None,
        })
    }
    pub(crate) fn with_source_maps(mut self, source_maps: GeneratePlanSourceMaps) -> Result<Self> {
        let source_maps = source_map_sanitization::without_temporary_workspace_or_host_paths(
            &self.requested,
            source_maps,
        );
        self.source_maps = Some(source_maps);
        self.fingerprint = PlanFingerprint::from_parts(
            &self.requested,
            &self.resolved,
            &self.config_content_hash,
            &self.created_with,
            self.source_maps.as_ref(),
        )?;
        self.plan_id = PlanId::from_fingerprint(&self.fingerprint)?;
        Ok(self)
    }
}
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub(crate) struct ConfigLoader;
#[allow(dead_code)]
impl ConfigLoader {
    pub(crate) fn new() -> Self {
        Self
    }
    fn load_requested(&self, requested: &RequestedGenerateState) -> Result<LoadedGenerateInputs> {
        let root = project_root_for_config(requested.config_path.as_path());
        requested.cli_overrides.validate()?;
        let loaded_config =
            config::load_kslim_config_file_with_source_map(requested.config_path.as_path())?;
        let mut config = loaded_config.config;
        let loaded_profile =
            config::load_profile_with_source_map(&root, requested.selected_profile.as_str())?;
        let mut profile = loaded_profile.profile;
        let mut override_source_map = config::ConfigSourceMap::default();
        if requested.cli_overrides.base_ref.is_some() {
            override_source_map.insert_cli_override("base.ref", "cli --base");
        }
        if requested.cli_overrides.max_fixup_passes.is_some() {
            override_source_map
                .insert_cli_override("reducer.max_fixup_passes", "cli --max-fixup-passes");
        }
        if requested.cli_overrides.matrix.is_some() {
            override_source_map.insert_cli_override("selftests.matrix", "cli --matrix");
        }
        let feature_selection = requested.cli_overrides.profile_feature_selection();
        profile = requested.cli_overrides.apply_profile_overrides(profile)?;
        config::insert_profile_feature_selection_cli_overrides(
            &mut override_source_map,
            feature_selection,
        );
        if let Some(source) = requested.cli_overrides.strictness_cli_source() {
            config::insert_profile_strictness_cli_overrides(&mut override_source_map, source);
        }
        let upstream_path = Path::new(&config.upstream.url);
        if !upstream_path.is_absolute() && !config.upstream.url.trim().is_empty() {
            config.upstream.url = root.join(upstream_path).to_string_lossy().to_string();
        }
        config::validate_config(&config)?;
        Ok(LoadedGenerateInputs {
            config,
            profile,
            config_source_map: loaded_config.source_map,
            profile_source_map: loaded_profile.source_map,
            override_source_map,
        })
    }
}
#[allow(dead_code)]
struct LoadedGenerateInputs {
    config: KslimConfig,
    profile: ProfileConfig,
    config_source_map: config::ConfigSourceMap,
    profile_source_map: config::ConfigSourceMap,
    override_source_map: config::ConfigSourceMap,
}
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub(crate) struct SourceResolver;
#[allow(dead_code)]
impl SourceResolver {
    pub(crate) fn new() -> Self {
        Self
    }
    fn upstream_path(&self, config: &KslimConfig) -> Result<String> {
        upstream::check_access(&config.upstream.url).map(|path| path.to_string())
    }
    fn resolve_base(
        &self,
        requested: &RequestedGenerateState,
        config: &KslimConfig,
        profile: &ProfileConfig,
        upstream_path: &str,
    ) -> Result<ResolvedBase> {
        requested.cli_overrides.validate()?;
        let ref_name = requested
            .cli_overrides
            .base_ref
            .as_deref()
            .unwrap_or(&profile.base.r#ref);
        let commit = upstream::resolve_ref(upstream_path, ref_name)?;
        let commit_date = upstream::ref_timestamp(upstream_path, ref_name)
            .with_context(|| format!("failed to read reproducible timestamp for {}", ref_name))?;
        Ok(ResolvedBase {
            upstream: config.upstream.name.clone(),
            url: config.upstream.url.clone(),
            r#ref: ref_name.to_string(),
            commit,
            resolved_at: commit_date,
        })
    }
    fn inspect_patch_sources(
        &self,
        profile: &ProfileConfig,
    ) -> Result<Option<Vec<patches::PatchInfo>>> {
        inspect_patch_sources(profile)
    }
}
fn offline_resolved_base(
    requested: &RequestedGenerateState,
    config: &KslimConfig,
    profile: &ProfileConfig,
) -> Result<ResolvedBase> {
    crate::network_policy::require_cli_no_network_endpoint("upstream.url", &config.upstream.url)?;
    let ref_name = requested
        .cli_overrides
        .base_ref
        .as_deref()
        .unwrap_or(&profile.base.r#ref);
    let root = project_root_for_config(requested.config_path.as_path());
    let lockfile_path = LockfilePath::new_in_project_root(&root)?;
    lockfile::load_resolved_base_for_request(
        &lockfile_path,
        &config.upstream.name,
        &config.upstream.url,
        ref_name,
    )
}
#[allow(dead_code)]
pub(crate) fn resolve_generate_plan(
    requested: RequestedGenerateState,
    loader: &ConfigLoader,
    resolver: &SourceResolver,
) -> Result<GeneratePlan> {
    let LoadedGenerateInputs {
        config,
        profile,
        config_source_map,
        profile_source_map,
        override_source_map,
    } = loader.load_requested(&requested)?;
    let resolved = if requested.cli_overrides.offline {
        offline_resolved_base(&requested, &config, &profile)?
    } else {
        let upstream_path = resolver.upstream_path(&config)?;
        resolver.resolve_base(&requested, &config, &profile, &upstream_path)?
    };
    let patch_infos = resolver.inspect_patch_sources(&profile)?;
    let mode = candidate_mode(&profile);
    let target_branch = output_repo::branch_name(&config, &profile, &resolved);
    let project_root = project_root_for_config(requested.config_path.as_path());
    let mut resolved_state = ResolvedCandidateState::from_resolved_inputs(
        &config,
        &profile,
        resolved,
        patch_infos.as_deref(),
        &mode,
        &target_branch,
    )?;
    resolved_state.output_plan.lockfile_path =
        Some(LockfilePath::new_in_project_root(&project_root)?);
    let config_content_hash = ConfigContentHash::from_resolved_state(&resolved_state)?;
    let source_maps =
        GeneratePlanSourceMaps::new(config_source_map, profile_source_map, override_source_map);
    GeneratePlan::from_parts(
        requested,
        resolved_state,
        config_content_hash,
        ToolVersion::current()?,
    )?
    .with_source_maps(source_maps)
}
#[allow(dead_code)]
fn project_root_for_config(path: &Path) -> PathBuf {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}
fn stable_plan_fingerprint_serialization(
    requested: &RequestedGenerateState,
    resolved: &ResolvedCandidateState,
    config_content_hash: &ConfigContentHash,
    tool_version: &ToolVersion,
    source_maps: Option<&GeneratePlanSourceMaps>,
) -> String {
    let mut out = String::new();
    append_fingerprint_line(&mut out, "format", PLAN_FINGERPRINT_SERIALIZATION_FORMAT);
    append_fingerprint_line(&mut out, "version", PLAN_FINGERPRINT_SCHEMA_VERSION);
    append_fingerprint_line(&mut out, "tool_version", tool_version.as_str());
    append_fingerprint_line(
        &mut out,
        "config_content_hash",
        config_content_hash.as_str(),
    );
    source_map_fingerprint::append_source_map_fingerprint_lines(&mut out, requested, source_maps);
    append_fingerprint_line(
        &mut out,
        "requested.selected_profile",
        requested.selected_profile.as_str(),
    );
    let cli = &requested.cli_overrides;
    for (field, value) in [
        ("dry_run", cli.dry_run),
        ("deep_dry_run", cli.deep_dry_run),
        ("report_only", cli.report_only),
        ("force", cli.force),
        ("offline", cli.offline),
        ("strict", cli.strict),
        ("no_strict", cli.no_strict),
    ] {
        append_fingerprint_line(
            &mut out,
            &format!("requested.cli_overrides.{field}"),
            bool_string(value),
        );
    }
    for (field, value) in [
        ("base_ref", cli.base_ref.as_deref()),
        ("feature", cli.feature.as_deref()),
        ("remove_feature", cli.remove_feature.as_deref()),
        ("preserve_feature", cli.preserve_feature.as_deref()),
        ("arch", cli.arch.as_deref()),
        ("primary_arch", cli.primary_arch.as_deref()),
        ("secondary_arch", cli.secondary_arch.as_deref()),
        ("safety", cli.safety.as_deref()),
        ("matrix", cli.matrix.as_deref()),
    ] {
        append_fingerprint_line(
            &mut out,
            &format!("requested.cli_overrides.{field}"),
            value.unwrap_or("<none>"),
        );
    }
    append_fingerprint_line(
        &mut out,
        "requested.cli_overrides.max_fixup_passes",
        &cli.max_fixup_passes
            .map(|passes| passes.to_string())
            .unwrap_or_else(|| String::from("<none>")),
    );
    append_fingerprint_line(
        &mut out,
        "requested.cli_overrides.run_selftests",
        bool_string(requested.cli_overrides.run_selftests),
    );
    append_resolved_candidate_fingerprint_lines(&mut out, resolved, false);
    out
}
const PLAN_FINGERPRINT_SERIALIZATION_FORMAT: &str = "kslim-generate-plan-fingerprint-v1";
const PLAN_FINGERPRINT_SCHEMA_VERSION: &str = "1";
fn append_resolved_candidate_fingerprint_lines(
    out: &mut String,
    resolved: &ResolvedCandidateState,
    include_path_fields: bool,
) {
    append_fingerprint_line(out, "resolved.base.upstream", &resolved.base.upstream);
    append_fingerprint_line(out, "resolved.base.ref", &resolved.base.r#ref);
    append_fingerprint_line(out, "resolved.base.commit", &resolved.base.commit);
    append_fingerprint_line(out, "resolved.base.resolved_at", &resolved.base.resolved_at);
    append_fingerprint_line(
        out,
        "resolved.patch_plan.source_count",
        &resolved.patch_plan.sources.len().to_string(),
    );
    append_fingerprint_line(
        out,
        "resolved.patch_plan.total_patch_count",
        &resolved.patch_plan.total_patch_count.to_string(),
    );
    for (idx, source) in resolved.patch_plan.sources.iter().enumerate() {
        let prefix = format!("resolved.patch_plan.sources.{idx}");
        append_fingerprint_line(out, &format!("{prefix}.stable_id"), &source.stable_id);
        append_fingerprint_line(out, &format!("{prefix}.source"), &source.source);
        if include_path_fields {
            append_fingerprint_line(
                out,
                &format!("{prefix}.worktree_path"),
                &source.worktree_path,
            );
        }
        append_fingerprint_line(out, &format!("{prefix}.branch"), &source.branch);
        append_fingerprint_line(out, &format!("{prefix}.head_commit"), &source.head_commit);
        append_fingerprint_line(out, &format!("{prefix}.merge_base"), &source.merge_base);
        append_fingerprint_line(out, &format!("{prefix}.base_remote"), &source.base_remote);
        append_fingerprint_line(out, &format!("{prefix}.base_ref"), &source.base_ref);
        append_fingerprint_line(
            out,
            &format!("{prefix}.patch_count"),
            &source.patch_count.to_string(),
        );
    }
    append_fingerprint_line(
        out,
        "resolved.integration_plan.entry_count",
        &resolved.integration_plan.entries.len().to_string(),
    );
    for (idx, entry) in resolved.integration_plan.entries.iter().enumerate() {
        let prefix = format!("resolved.integration_plan.entries.{idx}");
        append_fingerprint_line(out, &format!("{prefix}.stable_id"), &entry.stable_id);
        append_fingerprint_line(out, &format!("{prefix}.kind"), &entry.kind);
    }
    if let Some(rtlmq) = &resolved.integration_plan.rtlmq {
        append_fingerprint_line(
            out,
            "resolved.integration_plan.rtlmq.stable_id",
            &rtlmq.stable_id,
        );
        if include_path_fields {
            append_fingerprint_line(out, "resolved.integration_plan.rtlmq.source", &rtlmq.source);
            append_fingerprint_line(
                out,
                "resolved.integration_plan.rtlmq.tests_source",
                rtlmq.tests_source.as_deref().unwrap_or("<none>"),
            );
        }
    } else {
        append_fingerprint_line(out, "resolved.integration_plan.rtlmq", "<none>");
    }
    feature_intent_fingerprint::append_feature_intent_fingerprint_lines(out, resolved);
    append_fingerprint_line(
        out,
        "resolved.feature_resolution.source",
        resolved.feature_resolution.source().stable_name(),
    );
    for path in resolved.feature_resolution.remove_paths() {
        append_fingerprint_line(
            out,
            "resolved.feature_resolution.remove_paths",
            &path.as_path().to_string_lossy(),
        );
    }
    for config in resolved.feature_resolution.remove_configs() {
        append_fingerprint_line(
            out,
            "resolved.feature_resolution.remove_configs",
            config.as_str(),
        );
    }
    for path in resolved.feature_resolution.preserve_paths() {
        append_fingerprint_line(
            out,
            "resolved.feature_resolution.preserve_paths",
            &path.as_path().to_string_lossy(),
        );
    }
    for config in resolved.feature_resolution.preserve_configs() {
        append_fingerprint_line(
            out,
            "resolved.feature_resolution.preserve_configs",
            config.as_str(),
        );
    }
    for (symbol, value) in resolved.feature_resolution.set_defaults() {
        append_fingerprint_line(
            out,
            "resolved.feature_resolution.set_defaults.symbol",
            symbol.as_str(),
        );
        append_fingerprint_line(out, "resolved.feature_resolution.set_defaults.value", value);
    }
    for (feature, safety) in resolved.feature_resolution.feature_safety_levels() {
        append_fingerprint_line(
            out,
            "resolved.feature_resolution.feature_safety_levels.feature",
            feature,
        );
        append_fingerprint_line(
            out,
            "resolved.feature_resolution.feature_safety_levels.level",
            safety.as_str(),
        );
    }
    for (feature, scopes) in resolved.feature_resolution.feature_arch_scopes() {
        append_fingerprint_line(
            out,
            "resolved.feature_resolution.feature_arch_scopes.feature",
            feature,
        );
        for arch in scopes {
            append_fingerprint_line(
                out,
                "resolved.feature_resolution.feature_arch_scopes.arch",
                arch.as_str(),
            );
        }
    }
    for (feature, matrix) in resolved.feature_resolution.feature_test_matrices() {
        append_fingerprint_line(
            out,
            "resolved.feature_resolution.feature_test_matrices.feature",
            feature,
        );
        append_fingerprint_line(
            out,
            "resolved.feature_resolution.feature_test_matrices.require_clean_boot",
            bool_string(matrix.require_clean_boot),
        );
    }
    for (feature, mode) in resolved.feature_resolution.feature_report_modes() {
        append_fingerprint_line(
            out,
            "resolved.feature_resolution.feature_report_modes.feature",
            feature,
        );
        append_fingerprint_line(
            out,
            "resolved.feature_resolution.feature_report_modes.report_only",
            bool_string(mode.report_only),
        );
    }
    append_fingerprint_line(
        out,
        "resolved.feature_resolution.abi_policy.allow_public_header_removal",
        &resolved
            .feature_resolution
            .abi_policy()
            .allow_public_header_removal
            .to_string(),
    );
    append_fingerprint_line(
        out,
        "resolved.feature_resolution.abi_policy.allow_uapi_header_removal",
        &resolved
            .feature_resolution
            .abi_policy()
            .allow_uapi_header_removal
            .to_string(),
    );
    append_fingerprint_line(
        out,
        "resolved.feature_resolution.unsafe_allow_root_path_removal",
        &resolved
            .feature_resolution
            .unsafe_allow_root_path_removal()
            .to_string(),
    );
    append_fingerprint_line(
        out,
        "resolved.feature_conflicts.total",
        &resolved.feature_conflicts.len().to_string(),
    );
    append_fingerprint_line(
        out,
        "resolved.feature_conflicts.blocking",
        &resolved.feature_conflicts.blocking_count().to_string(),
    );
    for conflict in resolved.feature_conflicts.conflicts() {
        append_fingerprint_line(
            out,
            "resolved.feature_conflicts.key",
            &conflict.stable_key(),
        );
        append_fingerprint_line(
            out,
            "resolved.feature_conflicts.strict_blocking",
            bool_string(conflict.strict_blocking()),
        );
        append_fingerprint_line(
            out,
            "resolved.feature_conflicts.summary",
            conflict.summary(),
        );
        append_fingerprint_line(
            out,
            "resolved.feature_conflicts.action",
            conflict.suggested_action(),
        );
    }
    append_fingerprint_line(
        out,
        "resolved.abi_decision.allow_public_header_removal",
        bool_string(resolved.abi_decision.allow_public_header_removal()),
    );
    append_fingerprint_line(
        out,
        "resolved.abi_decision.allow_uapi_header_removal",
        bool_string(resolved.abi_decision.allow_uapi_header_removal()),
    );
    for header in resolved.abi_decision.approved_public_headers() {
        append_fingerprint_line(
            out,
            "resolved.abi_decision.approved_public_headers",
            header.as_str(),
        );
    }
    for path in resolved.abi_decision.approved_uapi_paths() {
        append_fingerprint_line(
            out,
            "resolved.abi_decision.approved_uapi_paths",
            path.as_str(),
        );
    }
    for path in &resolved.prune_plan.remove_paths {
        append_fingerprint_line(
            out,
            "resolved.prune_plan.remove_paths",
            &path.as_path().to_string_lossy(),
        );
    }
    for config in &resolved.prune_plan.remove_configs {
        append_fingerprint_line(out, "resolved.prune_plan.remove_configs", config.as_str());
    }
    for path in &resolved.prune_plan.preserve_paths {
        append_fingerprint_line(
            out,
            "resolved.prune_plan.preserve_paths",
            &path.as_path().to_string_lossy(),
        );
    }
    for config in &resolved.prune_plan.preserve_configs {
        append_fingerprint_line(out, "resolved.prune_plan.preserve_configs", config.as_str());
    }
    for (symbol, value) in &resolved.prune_plan.set_defaults {
        append_fingerprint_line(
            out,
            "resolved.prune_plan.set_defaults.symbol",
            symbol.as_str(),
        );
        append_fingerprint_line(out, "resolved.prune_plan.set_defaults.value", value);
    }
    append_fingerprint_line(
        out,
        "resolved.prune_plan.abi_policy.allow_public_header_removal",
        &resolved
            .prune_plan
            .abi_policy
            .allow_public_header_removal
            .to_string(),
    );
    append_fingerprint_line(
        out,
        "resolved.prune_plan.abi_policy.allow_uapi_header_removal",
        &resolved
            .prune_plan
            .abi_policy
            .allow_uapi_header_removal
            .to_string(),
    );
    append_fingerprint_line(
        out,
        "resolved.prune_plan.unsafe_allow_root_path_removal",
        &resolved
            .prune_plan
            .unsafe_allow_root_path_removal
            .to_string(),
    );
    append_fingerprint_line(
        out,
        "resolved.reducer_plan.max_fixup_passes",
        &resolved.reducer_plan.max_fixup_passes.to_string(),
    );
    append_fingerprint_line(
        out,
        "resolved.reducer_plan.report_unsupported_expressions",
        bool_string(resolved.reducer_plan.report_unsupported_expressions),
    );
    append_fingerprint_line(
        out,
        "resolved.reducer_plan.fail_on_unknown_diagnostics",
        bool_string(resolved.reducer_plan.fail_on_unknown_diagnostics),
    );
    append_fingerprint_line(
        out,
        "resolved.reducer_plan.reject_unproven_fixups",
        bool_string(resolved.reducer_plan.reject_unproven_fixups),
    );
    append_fingerprint_line(
        out,
        "resolved.reducer_plan.reject_unreasoned_edits",
        bool_string(resolved.reducer_plan.reject_unreasoned_edits),
    );
    append_fingerprint_line(
        out,
        "resolved.reducer_plan.reject_speculative_fallout_edits",
        bool_string(resolved.reducer_plan.reject_speculative_fallout_edits),
    );
    append_fingerprint_line(
        out,
        "resolved.reducer_plan.fail_on_missing_prune_paths",
        bool_string(resolved.reducer_plan.fail_on_missing_prune_paths),
    );
    append_fingerprint_line(
        out,
        "resolved.reducer_plan.ignore_unsupported_special_removals",
        bool_string(resolved.reducer_plan.ignore_unsupported_special_removals),
    );
    build_matrix_fingerprint::append_build_matrix_fingerprint_lines(
        out,
        &resolved.build_matrix_plan,
    );
    append_fingerprint_line(
        out,
        "resolved.selftest_plan.enabled",
        bool_string(resolved.selftest_plan.enabled),
    );
    append_fingerprint_line(
        out,
        "resolved.selftest_plan.check_kconfig_sources",
        bool_string(resolved.selftest_plan.check_kconfig_sources),
    );
    append_fingerprint_line(
        out,
        "resolved.selftest_plan.check_makefiles",
        bool_string(resolved.selftest_plan.check_makefiles),
    );
    for (idx, build) in resolved.selftest_plan.kernel_builds.iter().enumerate() {
        let prefix = format!("resolved.selftest_plan.kernel_builds.{idx}");
        append_fingerprint_line(
            out,
            &format!("{prefix}.name"),
            build.name.as_deref().unwrap_or("<none>"),
        );
        append_fingerprint_line(
            out,
            &format!("{prefix}.config_target"),
            build.config_target.as_deref().unwrap_or("<none>"),
        );
        append_fingerprint_line(
            out,
            &format!("{prefix}.arch"),
            build
                .arch
                .as_ref()
                .map(|arch| arch.as_str())
                .unwrap_or("<none>"),
        );
        for target in &build.targets {
            append_fingerprint_line(out, &format!("{prefix}.targets"), target);
        }
        let output_dir = build
            .output_dir
            .as_ref()
            .map(|dir| dir.as_path().to_string_lossy().into_owned())
            .unwrap_or_else(|| "<none>".to_string());
        append_fingerprint_line(out, &format!("{prefix}.output_dir"), &output_dir);
        append_fingerprint_line(
            out,
            &format!("{prefix}.jobs"),
            &build
                .jobs
                .map(|jobs| jobs.to_string())
                .unwrap_or_else(|| "<none>".to_string()),
        );
        append_fingerprint_line(out, &format!("{prefix}.clean"), bool_string(build.clean));
        append_fingerprint_line(
            out,
            &format!("{prefix}.make_program"),
            build.make_program.as_deref().unwrap_or("<none>"),
        );
        for arg in &build.make_args {
            append_fingerprint_line(out, &format!("{prefix}.make_args"), arg);
        }
        for (name, value) in &build.env {
            append_fingerprint_line(out, &format!("{prefix}.env.name"), name);
            append_fingerprint_line(out, &format!("{prefix}.env.value"), value);
        }
    }
    for command in &resolved.selftest_plan.commands {
        append_fingerprint_line(out, "resolved.selftest_plan.commands", command);
    }
    if include_path_fields {
        append_fingerprint_line(
            out,
            "resolved.output_plan.output_path",
            &resolved.output_plan.output_path.as_path().to_string_lossy(),
        );
    }
    append_fingerprint_line(
        out,
        "resolved.output_plan.branch",
        &resolved.output_plan.branch,
    );
    append_fingerprint_line(out, "resolved.output_plan.mode", &resolved.output_plan.mode);
    append_fingerprint_line(
        out,
        "resolved.output_plan.naming.project_name",
        &resolved.output_plan.naming.project_name,
    );
    append_fingerprint_line(
        out,
        "resolved.output_plan.naming.profile_name",
        &resolved.output_plan.naming.profile_name,
    );
    append_fingerprint_line(
        out,
        "resolved.output_plan.naming.branch_prefix",
        &resolved.output_plan.naming.branch_prefix,
    );
    append_fingerprint_line(
        out,
        "resolved.output_plan.naming.explicit_branch",
        resolved
            .output_plan
            .naming
            .explicit_branch
            .as_deref()
            .unwrap_or("<none>"),
    );
    append_fingerprint_line(
        out,
        "resolved.output_plan.naming.base_ref",
        &resolved.output_plan.naming.base_ref,
    );
    append_fingerprint_line(
        out,
        "resolved.output_plan.naming.base_commit",
        &resolved.output_plan.naming.base_commit,
    );
}
fn append_fingerprint_line(out: &mut String, key: &str, value: &str) {
    append_stable_key_value_line(out, key, value);
}
#[allow(dead_code)]
fn escape_fingerprint_value(value: &str) -> String {
    escape_stable_value(value)
}
fn bool_string(value: bool) -> &'static str {
    bool_token(value)
}
fn sha256_hex(value: &str) -> String {
    core_sha256_hex(value)
}
/// Immutable result of resolving a requested generate invocation.
///
/// Building this plan may inspect read-only sources, but it must not
/// materialize, mutate, verify, commit, or publish a candidate tree.
#[derive(Debug, Clone)]
pub(crate) struct CandidatePlan {
    pub(crate) generate_plan: GeneratePlan,
    pub(crate) patch_infos: Option<Vec<patches::PatchInfo>>,
}
#[allow(dead_code)]
pub(crate) fn resolve_candidate_plan(
    config: &KslimConfig,
    profile: &ProfileConfig,
    opts: &GenerateOptions,
    requested: RequestedGenerateState,
) -> Result<CandidatePlan> {
    resolve_candidate_plan_with_source_maps(config, profile, opts, requested, None)
}
pub(crate) fn resolve_candidate_plan_with_source_maps(
    config: &KslimConfig,
    profile: &ProfileConfig,
    opts: &GenerateOptions,
    requested: RequestedGenerateState,
    source_maps: Option<GeneratePlanSourceMaps>,
) -> Result<CandidatePlan> {
    requested.cli_overrides.validate()?;
    let selected_profile = requested
        .cli_overrides
        .apply_profile_overrides(profile.clone())?;
    let profile = &selected_profile;
    let ref_name = requested
        .cli_overrides
        .base_ref
        .as_deref()
        .unwrap_or(&profile.base.r#ref);
    let (resolved, patch_infos, mode, target_branch) = if let Some(frozen) =
        opts.frozen_plan.as_ref()
    {
        (
            frozen.resolved_base.clone(),
            Some(frozen.patch_infos.clone()),
            frozen.mode.clone(),
            frozen.output_branch.clone(),
        )
    } else if requested.cli_overrides.offline {
        let resolved = offline_resolved_base(&requested, config, profile)?;
        let patch_infos = inspect_patch_sources(profile)?;
        let mode = candidate_mode(profile);
        let target_branch = output_repo::branch_name(config, profile, &resolved);
        (resolved, patch_infos, mode, target_branch)
    } else {
        let upstream_path = upstream::check_access(&config.upstream.url)?;
        let commit = upstream::resolve_ref(upstream_path.as_str(), ref_name)?;
        let commit_date = upstream::ref_timestamp(upstream_path.as_str(), ref_name)
            .with_context(|| format!("failed to read reproducible timestamp for {}", ref_name))?;
        let resolved = ResolvedBase {
            upstream: config.upstream.name.clone(),
            url: config.upstream.url.clone(),
            r#ref: ref_name.to_string(),
            commit,
            resolved_at: commit_date,
        };
        let patch_infos = inspect_patch_sources(profile)?;
        let mode = candidate_mode(profile);
        let target_branch = output_repo::branch_name(config, profile, &resolved);
        (resolved, patch_infos, mode, target_branch)
    };
    let resolved_state = ResolvedCandidateState::from_resolved_inputs(
        config,
        profile,
        resolved.clone(),
        patch_infos.as_deref(),
        &mode,
        &target_branch,
    )?;
    let config_content_hash = ConfigContentHash::from_resolved_state(&resolved_state)?;
    let mut generate_plan = GeneratePlan::from_parts(
        requested,
        resolved_state,
        config_content_hash,
        ToolVersion::current()?,
    )?;
    if let Some(source_maps) = source_maps {
        generate_plan = generate_plan.with_source_maps(source_maps)?;
    }
    if let Some(frozen) = opts.frozen_plan.as_ref() {
        frozen.verify_resolved_plan(&generate_plan)?;
    }
    Ok(CandidatePlan {
        generate_plan,
        patch_infos,
    })
}
fn inspect_patch_sources(profile: &ProfileConfig) -> Result<Option<Vec<patches::PatchInfo>>> {
    let Some(patches_cfg) = &profile.patches else {
        return Ok(None);
    };
    patches::inspect_all(patches_cfg)
        .map(Some)
        .context("failed to inspect patch worktree source for generate")
}
fn candidate_mode(profile: &ProfileConfig) -> String {
    OutputPlanMode::from_profile(profile)
        .stable_name()
        .to_string()
}
#[cfg(test)]
mod tests {
    use crate::state::{CliOverrides, ProfileName};
    use super::*;
    use crate::config;
    use crate::paths::RequestedConfigPath;
    use std::path::Path;
    use std::process::Command;
    fn git_in(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        if !output.status.success() {
            panic!(
                "git {:?} failed in {}: {}",
                args,
                dir.display(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }
    fn test_resolved_base(commit: &str) -> ResolvedBase {
        ResolvedBase {
            upstream: String::from("linux"),
            url: String::from("/tmp/linux.git"),
            r#ref: String::from("v1.0"),
            commit: commit.to_string(),
            resolved_at: String::from("2026-01-01T00:00:00Z"),
        }
    }
    fn test_requested_state() -> RequestedGenerateState {
        RequestedGenerateState::new(
            RequestedConfigPath::new("/tmp/project/kslim.toml").unwrap(),
            ProfileName::new("default").unwrap(),
            default_cli_overrides(),
        )
    }
    fn default_cli_overrides() -> CliOverrides {
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
        }
    }
    fn test_resolved_state(commit: &str) -> ResolvedCandidateState {
        let config = config::default_kslim_config("demo", "/tmp/output");
        let mut profile = config::default_profile_config("v1.0");
        profile.integrations.rtlmq = Some(config::RtlmqIntegrationConfig {
            source: String::from("/tmp/rtlmq"),
            tests_source: None,
        });
        let patch_infos = vec![patches::PatchInfo {
            source: String::from("worktree"),
            worktree_path: String::from("/tmp/patches"),
            branch: String::from("topic"),
            head_commit: String::from("abc123"),
            merge_base: String::from("base123"),
            base_remote: String::from("origin"),
            base_ref: String::from("main"),
            patch_count: 3,
        }];
        ResolvedCandidateState::from_resolved_inputs(
            &config,
            &profile,
            test_resolved_base(commit),
            Some(&patch_infos),
            "unmodified-upstream",
            "kslim/v1.0/default",
        )
        .unwrap()
    }
    fn test_resolved_state_with_base(ref_name: &str, commit: &str) -> ResolvedCandidateState {
        let config = config::default_kslim_config("demo", "/tmp/output");
        let profile = config::default_profile_config(ref_name);
        ResolvedCandidateState::from_resolved_inputs(
            &config,
            &profile,
            ResolvedBase {
                upstream: String::from("linux"),
                url: String::from("/tmp/linux.git"),
                r#ref: ref_name.to_string(),
                commit: commit.to_string(),
                resolved_at: String::from("2026-01-01T00:00:00Z"),
            },
            None,
            "unmodified-upstream",
            format!("kslim/{ref_name}/default"),
        )
        .unwrap()
    }
    fn test_resolved_state_with_feature_safety(
        level: config::FeatureSafetyLevel,
    ) -> ResolvedCandidateState {
        let config = config::default_kslim_config("demo", "/tmp/output");
        let mut profile = config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            config::FeatureIntentConfig {
                roots: vec![String::from("net/bluetooth")],
                safety: Some(level),
                ..config::FeatureIntentConfig::default()
            },
        );
        ResolvedCandidateState::from_resolved_inputs(
            &config,
            &profile,
            test_resolved_base("deadbeef"),
            None,
            "slimmed",
            "kslim/v1.0/default",
        )
        .unwrap()
    }
    fn test_resolved_state_with_feature_arch_scope(
        arch_scope: Vec<String>,
    ) -> ResolvedCandidateState {
        let config = config::default_kslim_config("demo", "/tmp/output");
        let mut profile = config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            config::FeatureIntentConfig {
                roots: vec![String::from("net/bluetooth")],
                arch_scope,
                ..config::FeatureIntentConfig::default()
            },
        );
        ResolvedCandidateState::from_resolved_inputs(
            &config,
            &profile,
            test_resolved_base("deadbeef"),
            None,
            "slimmed",
            "kslim/v1.0/default",
        )
        .unwrap()
    }
    fn test_resolved_state_with_feature_test_matrix(
        require_clean_boot: bool,
    ) -> ResolvedCandidateState {
        let config = config::default_kslim_config("demo", "/tmp/output");
        let mut profile = config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            config::FeatureIntentConfig {
                roots: vec![String::from("net/bluetooth")],
                require_clean_boot,
                ..config::FeatureIntentConfig::default()
            },
        );
        ResolvedCandidateState::from_resolved_inputs(
            &config,
            &profile,
            test_resolved_base("deadbeef"),
            None,
            "slimmed",
            "kslim/v1.0/default",
        )
        .unwrap()
    }
    fn test_resolved_state_with_feature_report_mode(report_only: bool) -> ResolvedCandidateState {
        let config = config::default_kslim_config("demo", "/tmp/output");
        let mut profile = config::default_profile_config("v1.0");
        profile.features.remove.insert(
            String::from("bluetooth"),
            config::FeatureIntentConfig {
                roots: vec![String::from("net/bluetooth")],
                report_only,
                ..config::FeatureIntentConfig::default()
            },
        );
        ResolvedCandidateState::from_resolved_inputs(
            &config,
            &profile,
            test_resolved_base("deadbeef"),
            None,
            "slimmed",
            "kslim/v1.0/default",
        )
        .unwrap()
    }
    fn requested_state_with_config_and_override(
        config_path: &str,
        base_ref: Option<&str>,
    ) -> RequestedGenerateState {
        RequestedGenerateState::new(
            RequestedConfigPath::new(config_path).unwrap(),
            ProfileName::new("default").unwrap(),
            CliOverrides {
                base_ref: base_ref.map(str::to_string),
                ..default_cli_overrides()
            },
        )
    }
    fn test_source_maps_for_project(project_root: &str) -> GeneratePlanSourceMaps {
        let mut cfg = config::ConfigSourceMap::default();
        cfg.insert(
            "project.name",
            config::ConfigSourceKind::ConfigFile,
            format!("{project_root}/kslim.toml"),
        );
        cfg.insert(
            "output.branch_prefix",
            config::ConfigSourceKind::Default,
            "built-in default",
        );
        let mut profile = config::ConfigSourceMap::default();
        profile.insert(
            "base.ref",
            config::ConfigSourceKind::Profile,
            format!("{project_root}/profiles/default.toml"),
        );
        let mut overrides = config::ConfigSourceMap::default();
        overrides.insert_cli_override("base.ref", "cli --base");
        GeneratePlanSourceMaps::new(cfg, profile, overrides)
    }
    fn test_plan_with_source_maps(
        requested: RequestedGenerateState,
        resolved: ResolvedCandidateState,
        content_hash: ConfigContentHash,
        tool: ToolVersion,
        project_root: &str,
    ) -> GeneratePlan {
        GeneratePlan::from_parts(requested, resolved, content_hash, tool)
            .unwrap()
            .with_source_maps(test_source_maps_for_project(project_root))
            .unwrap()
    }
    #[test]
    fn test_generate_plan_captures_requested_and_resolved_state() {
        let requested = test_requested_state();
        let resolved = test_resolved_state("deadbeef");
        let plan = GeneratePlan::new(requested.clone(), resolved).unwrap();
        assert_eq!(plan.requested, requested);
        assert_eq!(plan.resolved.base.commit, "deadbeef");
        assert_eq!(plan.resolved.patch_plan.total_patch_count, 3);
        assert!(plan.resolved.patch_plan.sources[0]
            .stable_id
            .starts_with("patch-source-"));
        assert_eq!(plan.resolved.integration_plan.entries[0].kind, "rtlmq");
        assert!(plan.resolved.integration_plan.entries[0]
            .stable_id
            .starts_with("integration-rtlmq-"));
        assert_eq!(plan.resolved.output_plan.branch, "kslim/v1.0/default");
        assert_eq!(plan.created_with.as_str(), env!("CARGO_PKG_VERSION"));
        assert!(plan.config_content_hash.as_str().starts_with("config-"));
        assert!(plan.fingerprint.as_str().starts_with("fingerprint-"));
        assert_eq!(
            plan.plan_id.as_str().strip_prefix("plan-"),
            plan.fingerprint.as_str().strip_prefix("fingerprint-")
        );
        assert!(plan.plan_id.as_str().starts_with("plan-"));
        assert_eq!(plan.plan_id.as_str().len(), "plan-".len() + 64);
        let serialization = plan.fingerprint.stable_serialization();
        assert!(serialization.contains(&format!("tool_version={}", env!("CARGO_PKG_VERSION"))));
        assert!(serialization.contains("requested.selected_profile=default"));
        assert!(serialization.contains("requested.cli_overrides.run_selftests=true"));
        assert!(serialization.contains("resolved.base.commit=deadbeef"));
        assert!(serialization.contains("resolved.patch_plan.sources.0.stable_id=patch-source-"));
        assert!(serialization
            .contains("resolved.integration_plan.entries.0.stable_id=integration-rtlmq-"));
        assert!(!serialization.contains("/tmp/project/kslim.toml"));
        assert!(!serialization.contains("/tmp/output"));
    }
    #[test]
    fn test_generate_plan_id_is_stable_for_same_request_and_resolved_state() {
        let first =
            GeneratePlan::new(test_requested_state(), test_resolved_state("deadbeef")).unwrap();
        let second =
            GeneratePlan::new(test_requested_state(), test_resolved_state("deadbeef")).unwrap();
        let changed =
            GeneratePlan::new(test_requested_state(), test_resolved_state("feedface")).unwrap();
        assert_eq!(first.plan_id, second.plan_id);
        assert_eq!(first.fingerprint, second.fingerprint);
        assert_ne!(first.plan_id, changed.plan_id);
        assert_ne!(first.fingerprint, changed.fingerprint);
    }
    #[test]
    fn test_plan_fingerprint_tracks_feature_safety_level() {
        let content_hash = ConfigContentHash::new("config-test").unwrap();
        let tool = ToolVersion::new("test-tool").unwrap();
        let surgical = GeneratePlan::from_parts(
            test_requested_state(),
            test_resolved_state_with_feature_safety(config::FeatureSafetyLevel::Surgical),
            content_hash.clone(),
            tool.clone(),
        )
        .unwrap();
        let conservative = GeneratePlan::from_parts(
            test_requested_state(),
            test_resolved_state_with_feature_safety(config::FeatureSafetyLevel::Conservative),
            content_hash,
            tool,
        )
        .unwrap();
        assert_ne!(surgical.fingerprint, conservative.fingerprint);
        assert!(surgical
            .fingerprint
            .stable_serialization()
            .contains("resolved.feature_resolution.feature_safety_levels.feature=bluetooth"));
        assert!(surgical
            .fingerprint
            .stable_serialization()
            .contains("resolved.feature_resolution.feature_safety_levels.level=surgical"));
    }
    #[test]
    fn test_plan_fingerprint_tracks_feature_arch_scope() {
        let content_hash = ConfigContentHash::new("config-test").unwrap();
        let tool = ToolVersion::new("test-tool").unwrap();
        let x86 = GeneratePlan::from_parts(
            test_requested_state(),
            test_resolved_state_with_feature_arch_scope(vec![String::from("x86")]),
            content_hash.clone(),
            tool.clone(),
        )
        .unwrap();
        let arm64 = GeneratePlan::from_parts(
            test_requested_state(),
            test_resolved_state_with_feature_arch_scope(vec![String::from("arm64")]),
            content_hash,
            tool,
        )
        .unwrap();
        assert_ne!(x86.fingerprint, arm64.fingerprint);
        assert!(x86
            .fingerprint
            .stable_serialization()
            .contains("resolved.feature_resolution.feature_arch_scopes.feature=bluetooth"));
        assert!(x86
            .fingerprint
            .stable_serialization()
            .contains("resolved.feature_resolution.feature_arch_scopes.arch=x86"));
    }
    #[test]
    fn test_plan_fingerprint_tracks_feature_test_matrix() {
        let content_hash = ConfigContentHash::new("config-test").unwrap();
        let tool = ToolVersion::new("test-tool").unwrap();
        let clean_boot = GeneratePlan::from_parts(
            test_requested_state(),
            test_resolved_state_with_feature_test_matrix(true),
            content_hash.clone(),
            tool.clone(),
        )
        .unwrap();
        let no_feature_matrix = GeneratePlan::from_parts(
            test_requested_state(),
            test_resolved_state_with_feature_test_matrix(false),
            content_hash,
            tool,
        )
        .unwrap();

        assert_ne!(clean_boot.fingerprint, no_feature_matrix.fingerprint);
        assert!(clean_boot
            .fingerprint
            .stable_serialization()
            .contains("resolved.feature_resolution.feature_test_matrices.feature=bluetooth"));
        assert!(clean_boot
            .fingerprint
            .stable_serialization()
            .contains("resolved.feature_resolution.feature_test_matrices.require_clean_boot=true"));
    }

    #[test]
    fn test_plan_fingerprint_tracks_feature_report_mode() {
        let content_hash = ConfigContentHash::new("config-test").unwrap();
        let tool = ToolVersion::new("test-tool").unwrap();
        let report_only = GeneratePlan::from_parts(
            test_requested_state(),
            test_resolved_state_with_feature_report_mode(true),
            content_hash.clone(),
            tool.clone(),
        )
        .unwrap();
        let normal = GeneratePlan::from_parts(
            test_requested_state(),
            test_resolved_state_with_feature_report_mode(false),
            content_hash,
            tool,
        )
        .unwrap();

        assert_ne!(report_only.fingerprint, normal.fingerprint);
        assert!(report_only
            .fingerprint
            .stable_serialization()
            .contains("resolved.feature_resolution.feature_report_modes.feature=bluetooth"));
        assert!(report_only
            .fingerprint
            .stable_serialization()
            .contains("resolved.feature_resolution.feature_report_modes.report_only=true"));
    }

    #[test]
    fn test_plan_fingerprint_tracks_request_and_base_but_not_request_temp_path() {
        let content_hash = ConfigContentHash::new("config-test").unwrap();
        let tool = ToolVersion::new("test-tool").unwrap();
        let requested =
            requested_state_with_config_and_override("/tmp/attempt-one/kslim.toml", None);
        let resolved = test_resolved_state_with_base("v1.0", "deadbeef");

        let first = GeneratePlan::from_parts(
            requested.clone(),
            resolved.clone(),
            content_hash.clone(),
            tool.clone(),
        )
        .unwrap();
        let identical = GeneratePlan::from_parts(
            requested,
            resolved.clone(),
            content_hash.clone(),
            tool.clone(),
        )
        .unwrap();
        assert_eq!(first.fingerprint, identical.fingerprint);

        let cli_override_changed = GeneratePlan::from_parts(
            requested_state_with_config_and_override("/tmp/attempt-one/kslim.toml", Some("HEAD")),
            resolved.clone(),
            content_hash.clone(),
            tool.clone(),
        )
        .unwrap();
        assert_ne!(first.fingerprint, cli_override_changed.fingerprint);

        let profile_base_changed = GeneratePlan::from_parts(
            requested_state_with_config_and_override("/tmp/attempt-one/kslim.toml", None),
            test_resolved_state_with_base("v2.0", "feedface"),
            content_hash.clone(),
            tool.clone(),
        )
        .unwrap();
        assert_ne!(first.fingerprint, profile_base_changed.fingerprint);

        let temp_path_changed = GeneratePlan::from_parts(
            requested_state_with_config_and_override("/tmp/attempt-two/kslim.toml", None),
            resolved,
            content_hash,
            tool,
        )
        .unwrap();
        assert_eq!(first.fingerprint, temp_path_changed.fingerprint);
        assert!(!first
            .fingerprint
            .stable_serialization()
            .contains("/tmp/attempt-one"));
    }

    #[test]
    fn test_plan_fingerprint_tracks_source_map_without_host_paths() {
        let content_hash = ConfigContentHash::new("config-test").unwrap();
        let tool = ToolVersion::new("test-tool").unwrap();
        let requested_a =
            requested_state_with_config_and_override("/tmp/attempt-one/kslim.toml", Some("HEAD"));
        let requested_b =
            requested_state_with_config_and_override("/tmp/attempt-two/kslim.toml", Some("HEAD"));
        let resolved = test_resolved_state_with_base("HEAD", "deadbeef");

        let without_source_map = GeneratePlan::from_parts(
            requested_a.clone(),
            resolved.clone(),
            content_hash.clone(),
            tool.clone(),
        )
        .unwrap();
        let first = test_plan_with_source_maps(
            requested_a.clone(),
            resolved.clone(),
            content_hash.clone(),
            tool.clone(),
            "/tmp/attempt-one",
        );
        let second = test_plan_with_source_maps(
            requested_b,
            resolved.clone(),
            content_hash.clone(),
            tool.clone(),
            "/tmp/attempt-two",
        );

        assert_ne!(without_source_map.fingerprint, first.fingerprint);
        assert_ne!(without_source_map.plan_id, first.plan_id);
        assert_eq!(first.fingerprint, second.fingerprint);

        let serialization = first.fingerprint.stable_serialization();
        assert!(serialization.contains("source_map.available=true"));
        assert!(serialization.contains("source_map.config.entry_count=2"));
        assert!(serialization.contains("source_map.config.project.name.kind=config_file"));
        assert!(serialization.contains("source_map.config.project.name.source=<requested-config>"));
        assert!(serialization.contains("source_map.profile.base.ref.source=<selected-profile>"));
        assert!(serialization.contains("source_map.overrides.base.ref.kind=cli"));
        assert!(
            !serialization.contains("/tmp/attempt-one"),
            "fingerprint serialization leaked requested config source path"
        );
        let mut changed_source_maps = test_source_maps_for_project("/tmp/attempt-one");
        changed_source_maps.config.insert(
            "project.name",
            config::ConfigSourceKind::Default,
            "built-in default",
        );
        let changed = GeneratePlan::from_parts(requested_a, resolved, content_hash, tool)
            .unwrap()
            .with_source_maps(changed_source_maps)
            .unwrap();
        assert_ne!(first.fingerprint, changed.fingerprint);
    }

    #[test]
    fn test_resolved_plan_fingerprint_inputs_ignore_host_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let upstream = tmp.path().join("upstream");
        std::fs::create_dir_all(&upstream).unwrap();
        std::fs::write(upstream.join("README"), "v1\n").unwrap();
        git_in(&upstream, &["init"]);
        git_in(&upstream, &["config", "user.email", "test@kslim.local"]);
        git_in(&upstream, &["config", "user.name", "kslim test"]);
        git_in(&upstream, &["add", "-A"]);
        git_in(&upstream, &["commit", "-m", "initial"]);
        git_in(&upstream, &["tag", "v1.0"]);
        let output_a = tmp.path().join("host-a/output");
        let output_b = tmp.path().join("host-b/output");
        let rtlmq_a = tmp.path().join("host-a/rtlmq");
        let rtlmq_b = tmp.path().join("host-b/rtlmq");
        let rtlmq_tests_a = tmp.path().join("host-a/rtlmq-tests");
        let rtlmq_tests_b = tmp.path().join("host-b/rtlmq-tests");
        let mut config_a = config::default_kslim_config("demo", output_a.to_str().unwrap());
        config_a.upstream.url = upstream.join(".git").to_string_lossy().to_string();
        let mut config_b = config_a.clone();
        config_b.output.path = output_b.to_string_lossy().to_string();
        let mut profile_a = config::default_profile_config("v1.0");
        profile_a.integrations.rtlmq = Some(config::RtlmqIntegrationConfig {
            source: rtlmq_a.to_string_lossy().to_string(),
            tests_source: Some(rtlmq_tests_a.to_string_lossy().to_string()),
        });
        let mut profile_b = profile_a.clone();
        profile_b.integrations.rtlmq = Some(config::RtlmqIntegrationConfig {
            source: rtlmq_b.to_string_lossy().to_string(),
            tests_source: Some(rtlmq_tests_b.to_string_lossy().to_string()),
        });
        let opts = GenerateOptions {
            dry_run: false,
            deep_dry_run: false,
            report_only: false,
            keep_temp: false,
            max_fixup_passes: None,
            matrix: None,
            offline: false,
            frozen_plan: None,
            force: false,
            base_ref: None,
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            strict: false,
            no_strict: false,
            run_selftests: true,
        };
        let requested_a = RequestedGenerateState::new(
            RequestedConfigPath::new(tmp.path().join("host-a/kslim.toml")).unwrap(),
            ProfileName::new("default").unwrap(),
            CliOverrides::from_options(&opts),
        );
        let requested_b = RequestedGenerateState::new(
            RequestedConfigPath::new(tmp.path().join("host-b/kslim.toml")).unwrap(),
            ProfileName::new("default").unwrap(),
            CliOverrides::from_options(&opts),
        );
        let first = resolve_candidate_plan(&config_a, &profile_a, &opts, requested_a)
            .unwrap()
            .generate_plan;
        let second = resolve_candidate_plan(&config_b, &profile_b, &opts, requested_b)
            .unwrap()
            .generate_plan;
        assert_eq!(first.config_content_hash, second.config_content_hash);
        assert_eq!(first.fingerprint, second.fingerprint);
        assert_eq!(
            first.resolved.integration_plan.entries[0].stable_id,
            second.resolved.integration_plan.entries[0].stable_id
        );
        let serialization = first.fingerprint.stable_serialization();
        for host_path in [
            output_a,
            output_b,
            rtlmq_a,
            rtlmq_b,
            rtlmq_tests_a,
            rtlmq_tests_b,
        ] {
            let host_path = host_path.to_string_lossy();
            assert!(
                !serialization.contains(host_path.as_ref()),
                "fingerprint serialization leaked host path {host_path}"
            );
        }
    }

    #[test]
    fn test_generate_plan_rejects_empty_identity_parts() {
        let err = PlanId::new(" ").unwrap_err().to_string();
        assert!(err.contains("generate plan id is empty"));

        let err = ToolVersion::new("").unwrap_err().to_string();
        assert!(err.contains("tool version must not be empty"));

        let err = ConfigContentHash::new(" ").unwrap_err().to_string();
        assert!(err.contains("generate plan config content hash is empty"));

        let err = PlanFingerprint::new(" ", "format=kslim\n")
            .unwrap_err()
            .to_string();
        assert!(err.contains("generate plan fingerprint is empty"));
    }

    #[test]
    fn test_config_loader_preserves_requested_config_file_source_map() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let output = tmp.path().join("output");
        std::fs::create_dir_all(project.join("profiles")).unwrap();

        let config_path = project.join("custom.toml");
        std::fs::write(
            &config_path,
            format!(
                r#"
[project]
name = "demo-custom"

[upstream]
name = "linux"
url = "/tmp/linux.git"

[output]
path = "{}"
"#,
                output.display()
            ),
        )
        .unwrap();
        std::fs::write(
            project.join("profiles/default.toml"),
            r#"
[profile]
name = "default"

[base]
ref = "v1.0"
"#,
        )
        .unwrap();
        let requested = RequestedGenerateState::new(
            RequestedConfigPath::new(&config_path).unwrap(),
            ProfileName::new("default").unwrap(),
            CliOverrides {
                dry_run: true,
                deep_dry_run: false,
                run_selftests: false,
                ..default_cli_overrides()
            },
        );
        let loaded = ConfigLoader::new().load_requested(&requested).unwrap();
        assert_eq!(loaded.config.project.name, "demo-custom");
        assert_eq!(loaded.profile.profile.name, "default");
        assert!(loaded
            .config_source_map
            .get("project.name")
            .unwrap()
            .source
            .ends_with("custom.toml"));
        assert_eq!(
            loaded
                .config_source_map
                .get("output.branch_prefix")
                .map(|source| source.kind),
            Some(config::ConfigSourceKind::Default)
        );
        assert!(loaded
            .profile_source_map
            .get("profile.name")
            .unwrap()
            .source
            .ends_with("profiles/default.toml"));
        assert_eq!(
            loaded
                .profile_source_map
                .get("selftests.enabled")
                .map(|source| source.kind),
            Some(config::ConfigSourceKind::Default)
        );
        assert!(loaded.override_source_map.is_empty());
    }

    #[test]
    fn test_config_loader_records_cli_base_override_separately_from_profile_source() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let output = tmp.path().join("output");
        std::fs::create_dir_all(project.join("profiles")).unwrap();
        let config_path = project.join("kslim.toml");
        let config = config::default_kslim_config("demo", output.to_str().unwrap());
        let profile = config::default_profile_config("v1.0");
        std::fs::write(&config_path, toml::to_string_pretty(&config).unwrap()).unwrap();
        std::fs::write(
            project.join("profiles/default.toml"),
            toml::to_string_pretty(&profile).unwrap(),
        )
        .unwrap();
        let requested = RequestedGenerateState::new(
            RequestedConfigPath::new(&config_path).unwrap(),
            ProfileName::new("default").unwrap(),
            CliOverrides {
                dry_run: true,
                deep_dry_run: false,
                base_ref: Some("HEAD".to_string()),
                run_selftests: false,
                ..default_cli_overrides()
            },
        );
        let loaded = ConfigLoader::new().load_requested(&requested).unwrap();
        assert_eq!(
            loaded
                .profile_source_map
                .get("base.ref")
                .map(|source| source.kind),
            Some(config::ConfigSourceKind::Profile)
        );
        assert_eq!(
            loaded
                .override_source_map
                .get("base.ref")
                .map(|source| source.kind),
            Some(config::ConfigSourceKind::Cli)
        );
        assert_eq!(
            loaded
                .override_source_map
                .get("base.ref")
                .map(|source| source.source.as_str()),
            Some("cli --base")
        );
    }

    #[test]
    fn test_resolve_generate_plan_loads_request_and_resolves_candidate_truth() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let upstream = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        std::fs::create_dir_all(project.join("profiles")).unwrap();
        std::fs::create_dir_all(&upstream).unwrap();
        std::fs::write(upstream.join("README"), "v1\n").unwrap();
        git_in(&upstream, &["init"]);
        git_in(&upstream, &["config", "user.email", "test@kslim.local"]);
        git_in(&upstream, &["config", "user.name", "kslim test"]);
        git_in(&upstream, &["add", "-A"]);
        git_in(&upstream, &["commit", "-m", "initial"]);
        git_in(&upstream, &["tag", "v1.0"]);
        std::fs::write(upstream.join("README"), "head\n").unwrap();
        git_in(&upstream, &["add", "-A"]);
        git_in(&upstream, &["commit", "-m", "head"]);
        let mut config = config::default_kslim_config("demo", output.to_str().unwrap());
        config.upstream.url = upstream.join(".git").to_string_lossy().to_string();
        let profile = config::default_profile_config("v1.0");
        std::fs::write(
            project.join("kslim.toml"),
            toml::to_string_pretty(&config).unwrap(),
        )
        .unwrap();
        std::fs::write(
            project.join("profiles/default.toml"),
            toml::to_string_pretty(&profile).unwrap(),
        )
        .unwrap();
        let requested = RequestedGenerateState::new(
            RequestedConfigPath::new(project.join("kslim.toml")).unwrap(),
            ProfileName::new("default").unwrap(),
            CliOverrides {
                dry_run: true,
                deep_dry_run: false,
                force: true,
                base_ref: Some("HEAD".to_string()),
                run_selftests: false,
                ..default_cli_overrides()
            },
        );
        let plan = resolve_generate_plan(
            requested.clone(),
            &ConfigLoader::new(),
            &SourceResolver::new(),
        )
        .unwrap();
        assert_eq!(plan.requested, requested);
        assert_eq!(plan.resolved.base.r#ref, "HEAD");
        assert_eq!(
            plan.resolved.base.commit,
            git_in(&upstream, &["rev-parse", "HEAD"])
        );
        assert_eq!(
            plan.resolved.base.resolved_at,
            git_in(&upstream, &["log", "-1", "--format=%aI", "HEAD"])
        );
        let expected_lockfile = project.join("kslim.lock");
        assert_eq!(
            plan.resolved.output_plan.output_path.as_path(),
            output.as_path()
        );
        assert_eq!(
            plan.resolved
                .output_plan
                .lockfile_path
                .as_ref()
                .map(|path| path.as_path()),
            Some(expected_lockfile.as_path())
        );
        assert_eq!(plan.resolved.output_plan.mode, "unmodified-upstream");
        assert_eq!(plan.resolved.patch_plan.total_patch_count, 0);
        assert_eq!(
            plan.resolved.selftest_plan.enabled,
            profile.selftests.enabled
        );
        assert!(plan.plan_id.as_str().starts_with("plan-"));
        let source_maps = plan.source_maps.as_ref().unwrap();
        assert_eq!(
            source_maps
                .config
                .get("project.name")
                .map(|source| source.kind),
            Some(config::ConfigSourceKind::ConfigFile)
        );
        assert_eq!(
            source_maps
                .profile
                .get("base.ref")
                .map(|source| source.kind),
            Some(config::ConfigSourceKind::Profile)
        );
        assert_eq!(
            source_maps
                .overrides
                .get("base.ref")
                .map(|source| source.kind),
            Some(config::ConfigSourceKind::Cli)
        );
        assert!(!plan.resolved.output_plan.output_path.as_path().exists());
    }
    #[test]
    fn test_resolve_candidate_plan_uses_cli_base_override_without_mutating_output() {
        let tmp = tempfile::tempdir().unwrap();
        let upstream = tmp.path().join("upstream");
        let output = tmp.path().join("output");
        std::fs::create_dir_all(&upstream).unwrap();
        std::fs::write(upstream.join("README"), "v1\n").unwrap();
        git_in(&upstream, &["init"]);
        git_in(&upstream, &["config", "user.email", "test@kslim.local"]);
        git_in(&upstream, &["config", "user.name", "kslim test"]);
        git_in(&upstream, &["add", "-A"]);
        git_in(&upstream, &["commit", "-m", "initial"]);
        git_in(&upstream, &["tag", "v1.0"]);
        std::fs::write(upstream.join("README"), "head\n").unwrap();
        git_in(&upstream, &["add", "-A"]);
        git_in(&upstream, &["commit", "-m", "head"]);
        let mut config = config::default_kslim_config("demo", output.to_str().unwrap());
        config.upstream.url = upstream.join(".git").to_string_lossy().to_string();
        let profile = config::default_profile_config("v1.0");
        let opts = GenerateOptions {
            dry_run: false,
            deep_dry_run: false,
            report_only: false,
            keep_temp: false,
            max_fixup_passes: None,
            matrix: None,
            offline: false,
            frozen_plan: None,
            force: false,
            base_ref: Some("HEAD".to_string()),
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            strict: false,
            no_strict: false,
            run_selftests: false,
        };
        let requested = RequestedGenerateState::new(
            RequestedConfigPath::new("/tmp/project/kslim.toml").unwrap(),
            ProfileName::new("default").unwrap(),
            CliOverrides::from_options(&opts),
        );
        let plan = resolve_candidate_plan(&config, &profile, &opts, requested).unwrap();
        let generate_plan = &plan.generate_plan;
        let resolved = &generate_plan.resolved;
        assert_eq!(
            generate_plan.requested.cli_overrides.base_ref.as_deref(),
            Some("HEAD")
        );
        assert_eq!(resolved.base.r#ref, "HEAD");
        assert_eq!(
            resolved.base.commit,
            git_in(&upstream, &["rev-parse", "HEAD"])
        );
        assert_eq!(
            resolved.base.resolved_at,
            git_in(&upstream, &["log", "-1", "--format=%aI", "HEAD"])
        );
        assert_eq!(resolved.output_plan.mode, "unmodified-upstream");
        assert_eq!(
            resolved.output_plan.branch,
            output_repo::branch_name(&config, &profile, &resolved.base)
        );
        assert!(plan.patch_infos.is_none());
        assert_eq!(resolved.patch_plan.total_patch_count, 0);
        assert_eq!(resolved.base.r#ref, "HEAD");
        assert_eq!(resolved.patch_plan.total_patch_count, 0);
        assert_eq!(resolved.output_plan.mode, "unmodified-upstream");
        assert!(!output.exists());
    }

    #[test]
    fn test_candidate_mode_marks_effective_slim_input() {
        let mut profile = config::default_profile_config("v1.0");
        assert_eq!(candidate_mode(&profile), "unmodified-upstream");

        profile.slim = Some(config::SlimConfig {
            remove_paths: vec!["drivers/gpu/drm/amd/amdgpu".to_string()],
            remove_configs: Vec::new(),
            set_defaults: Default::default(),
            unsafe_allow_root_path_removal: false,
        });

        assert_eq!(candidate_mode(&profile), "slimmed");

        profile.slim = None;
        profile.features.remove.insert(
            String::from("amdgpu"),
            config::FeatureIntentConfig {
                roots: vec![String::from("drivers/gpu/drm/amd/amdgpu")],
                ..config::FeatureIntentConfig::default()
            },
        );

        assert_eq!(candidate_mode(&profile), "slimmed");
    }
}
