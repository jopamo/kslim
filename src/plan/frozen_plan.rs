use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::config::{self, KslimConfig, ProfileConfig};
use crate::lockfile::ResolvedBase;
use crate::model::ToolVersion;
use crate::patches::PatchInfo;
use crate::paths::RequestedConfigPath;

use super::{
    resolve_generate_plan, ConfigLoader, GeneratePlan, GeneratePlanSourceMaps,
    GeneratePlanSummary, SourceResolver,
};
use crate::generate::GenerateOptions;
use crate::state::{CliOverrides, ProfileName, RequestedGenerateState};

const FROZEN_PLAN_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenPlanDocument {
    schema_version: u32,
    tool_version: String,
    plan_id: String,
    fingerprint: String,
    config_content_hash: String,
    selected_profile: String,
    cli_overrides: CliOverrides,
    config: KslimConfig,
    profile: ProfileConfig,
    source_maps: GeneratePlanSourceMaps,
    resolved_base: ResolvedBase,
    mode: String,
    output_branch: String,
    #[serde(default)]
    patch_infos: Vec<PatchInfo>,
}

#[derive(Debug, Clone)]
pub(crate) struct FrozenPlanInputs {
    pub(crate) plan_id: String,
    pub(crate) fingerprint: String,
    pub(crate) config_content_hash: String,
    pub(crate) cli_overrides: CliOverrides,
    pub(crate) resolved_base: ResolvedBase,
    pub(crate) mode: String,
    pub(crate) output_branch: String,
    pub(crate) source_maps: GeneratePlanSourceMaps,
    pub(crate) patch_infos: Vec<PatchInfo>,
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedFrozenPlan {
    pub(crate) config: KslimConfig,
    pub(crate) profile: ProfileConfig,
    pub(crate) inputs: FrozenPlanInputs,
}

impl FrozenPlanInputs {
    pub(crate) fn to_generate_options(&self, keep_temp: bool) -> GenerateOptions {
        let cli = &self.cli_overrides;
        GenerateOptions {
            dry_run: cli.dry_run,
            deep_dry_run: cli.deep_dry_run,
            report_only: cli.report_only,
            keep_temp,
            max_fixup_passes: cli.max_fixup_passes,
            matrix: cli.matrix.clone(),
            offline: cli.offline,
            frozen_plan: Some(self.clone()),
            force: cli.force,
            base_ref: cli.base_ref.clone(),
            feature: cli.feature.clone(),
            remove_feature: cli.remove_feature.clone(),
            preserve_feature: cli.preserve_feature.clone(),
            arch: cli.arch.clone(),
            primary_arch: cli.primary_arch.clone(),
            secondary_arch: cli.secondary_arch.clone(),
            safety: cli.safety.clone(),
            strict: cli.strict,
            no_strict: cli.no_strict,
            run_selftests: cli.run_selftests,
        }
    }

    pub(crate) fn verify_resolved_plan(&self, plan: &GeneratePlan) -> Result<()> {
        ensure_equal("plan_id", plan.plan_id.as_str(), &self.plan_id)?;
        ensure_equal("fingerprint", plan.fingerprint.as_str(), &self.fingerprint)?;
        ensure_equal(
            "config_content_hash",
            plan.config_content_hash.as_str(),
            &self.config_content_hash,
        )?;
        ensure_equal(
            "resolved base ref",
            &plan.resolved.base.r#ref,
            &self.resolved_base.r#ref,
        )?;
        ensure_equal(
            "resolved base commit",
            &plan.resolved.base.commit,
            &self.resolved_base.commit,
        )?;
        ensure_equal(
            "output branch",
            &plan.resolved.output_plan.branch,
            &self.output_branch,
        )?;
        ensure_equal("mode", &plan.resolved.output_plan.mode, &self.mode)?;
        Ok(())
    }
}

pub(crate) fn load_frozen_plan(path: &Path) -> Result<LoadedFrozenPlan> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read frozen plan {}", path.display()))?;
    let document: FrozenPlanDocument = toml::from_str(&contents)
        .with_context(|| format!("failed to parse frozen plan {}", path.display()))?;
    validate_document_header(&document)?;
    crate::network_policy::require_local_upstream_url(&document.resolved_base.url)?;
    document.cli_overrides.validate()?;
    validate_frozen_cli_overrides(&document.cli_overrides)?;
    config::validate_config(&document.config)?;
    config::validate_profile(&document.profile)?;
    ensure_equal(
        "selected profile",
        &document.profile.profile.name,
        &document.selected_profile,
    )?;

    Ok(LoadedFrozenPlan {
        config: document.config,
        profile: document.profile,
        inputs: FrozenPlanInputs {
            plan_id: document.plan_id,
            fingerprint: document.fingerprint,
            config_content_hash: document.config_content_hash,
            cli_overrides: document.cli_overrides,
            resolved_base: document.resolved_base,
            mode: document.mode,
            output_branch: document.output_branch,
            source_maps: document.source_maps,
            patch_infos: document.patch_infos,
        },
    })
}

pub(crate) fn write_frozen_plan_for_request(
    root: &Path,
    profile_name: &str,
    opts: &GenerateOptions,
    output: &Path,
) -> Result<GeneratePlanSummary> {
    let requested = RequestedGenerateState::new(
        RequestedConfigPath::new(root.join("kslim.toml"))?,
        ProfileName::new(profile_name.to_string())?,
        CliOverrides::from_options(opts),
    );
    let loaded_config = config::load_kslim_config(root)?;
    let loaded_profile = config::load_profile(root, profile_name)?;
    let active_profile = requested
        .cli_overrides
        .apply_profile_overrides(loaded_profile.clone())?;
    let plan = resolve_generate_plan(
        requested,
        &ConfigLoader::new(),
        &SourceResolver::new(),
    )?;
    let patch_infos = plan
        .resolved
        .patch_plan
        .sources
        .iter()
        .map(|source| PatchInfo {
            source: source.source.clone(),
            worktree_path: source.worktree_path.clone(),
            branch: source.branch.clone(),
            head_commit: source.head_commit.clone(),
            merge_base: source.merge_base.clone(),
            base_remote: source.base_remote.clone(),
            base_ref: source.base_ref.clone(),
            patch_count: source.patch_count,
        })
        .collect::<Vec<_>>();
    let document = FrozenPlanDocument {
        schema_version: FROZEN_PLAN_SCHEMA_VERSION,
        tool_version: ToolVersion::current()?.as_str().to_string(),
        plan_id: plan.plan_id.as_str().to_string(),
        fingerprint: plan.fingerprint.as_str().to_string(),
        config_content_hash: plan.config_content_hash.as_str().to_string(),
        selected_profile: profile_name.to_string(),
        cli_overrides: plan.requested.cli_overrides.clone(),
        config: loaded_config,
        profile: active_profile,
        source_maps: plan.source_maps.clone().unwrap_or_default(),
        resolved_base: plan.resolved.base.clone(),
        mode: plan.resolved.output_plan.mode.clone(),
        output_branch: plan.resolved.output_plan.branch.clone(),
        patch_infos,
    };
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        crate::fsutil::ensure_dir(parent)?;
    }
    std::fs::write(output, toml::to_string_pretty(&document)?)
        .with_context(|| format!("failed to write frozen plan {}", output.display()))?;
    Ok(GeneratePlanSummary::from_plan(&plan))
}

pub(crate) fn ensure_tree_matches_frozen_base(
    tree: &Path,
    inputs: &FrozenPlanInputs,
) -> Result<()> {
    let tree = tree
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("frozen-plan tree path is not valid UTF-8"))?;
    let head = crate::git::head_commit(tree)
        .with_context(|| format!("--frozen-plan requires tree {} to be a git worktree", tree))?;
    if head.trim() != inputs.resolved_base.commit {
        anyhow::bail!(
            "--frozen-plan tree HEAD {} does not match plan base commit {}",
            head.trim(),
            inputs.resolved_base.commit
        );
    }
    Ok(())
}

fn validate_document_header(document: &FrozenPlanDocument) -> Result<()> {
    if document.schema_version != FROZEN_PLAN_SCHEMA_VERSION {
        anyhow::bail!(
            "unsupported frozen plan schema_version {}; expected {}",
            document.schema_version,
            FROZEN_PLAN_SCHEMA_VERSION
        );
    }
    let current = ToolVersion::current()?;
    if document.tool_version != current.as_str() {
        anyhow::bail!(
            "frozen plan tool_version {} is not compatible with current {}",
            document.tool_version,
            current.as_str()
        );
    }
    Ok(())
}

fn validate_frozen_cli_overrides(cli: &CliOverrides) -> Result<()> {
    if cli.dry_run || cli.deep_dry_run || cli.report_only || cli.force {
        anyhow::bail!("frozen plan contains unsupported execution-mode CLI overrides");
    }
    if !cli.run_selftests {
        anyhow::bail!("frozen plan cannot disable generate selftests");
    }
    Ok(())
}

fn ensure_equal(label: &str, actual: &str, expected: &str) -> Result<()> {
    if actual != expected {
        anyhow::bail!(
            "frozen plan {} mismatch: expected {}, got {}",
            label,
            expected,
            actual
        );
    }
    Ok(())
}
