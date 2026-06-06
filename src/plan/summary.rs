use anyhow::Result;
use std::path::Path;

use super::{resolve_generate_plan, ConfigLoader, GeneratePlan, SourceResolver};
use crate::generate::GenerateOptions;
use crate::state::{CliOverrides, ProfileName, RequestedGenerateState};
use crate::paths::RequestedConfigPath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeneratePlanSummary {
    pub(crate) plan_id: String,
    pub(crate) fingerprint: String,
    pub(crate) config_content_hash: String,
    pub(crate) tool_version: String,
    pub(crate) profile: String,
    pub(crate) base_ref: String,
    pub(crate) base_commit: String,
    pub(crate) output_path: String,
    pub(crate) output_branch: String,
    pub(crate) mode: String,
    pub(crate) patch_sources: usize,
    pub(crate) patch_count: usize,
    pub(crate) integration_count: usize,
    pub(crate) feature_source: String,
    pub(crate) remove_path_count: usize,
    pub(crate) remove_config_count: usize,
    pub(crate) preserve_path_count: usize,
    pub(crate) preserve_config_count: usize,
    pub(crate) selftests_enabled: bool,
    pub(crate) kernel_build_count: usize,
    pub(crate) selftest_command_count: usize,
}

impl GeneratePlanSummary {
    pub(crate) fn from_plan(plan: &GeneratePlan) -> Self {
        let resolved = &plan.resolved;
        Self {
            plan_id: plan.plan_id.as_str().to_string(),
            fingerprint: plan.fingerprint.as_str().to_string(),
            config_content_hash: plan.config_content_hash.as_str().to_string(),
            tool_version: plan.created_with.as_str().to_string(),
            profile: plan.requested.selected_profile.as_str().to_string(),
            base_ref: resolved.base.r#ref.clone(),
            base_commit: resolved.base.commit.clone(),
            output_path: resolved
                .output_plan
                .output_path
                .as_path()
                .display()
                .to_string(),
            output_branch: resolved.output_plan.branch.clone(),
            mode: resolved.output_plan.mode.clone(),
            patch_sources: resolved.patch_plan.sources.len(),
            patch_count: resolved.patch_plan.total_patch_count,
            integration_count: resolved.integration_plan.entries.len(),
            feature_source: resolved
                .feature_resolution
                .source()
                .stable_name()
                .to_string(),
            remove_path_count: resolved.feature_resolution.remove_paths().len(),
            remove_config_count: resolved.feature_resolution.remove_configs().len(),
            preserve_path_count: resolved.feature_resolution.preserve_paths().len(),
            preserve_config_count: resolved.feature_resolution.preserve_configs().len(),
            selftests_enabled: resolved.selftest_plan.enabled,
            kernel_build_count: resolved.selftest_plan.kernel_builds.len(),
            selftest_command_count: resolved.selftest_plan.commands.len(),
        }
    }
}

pub(crate) fn resolve_plan_summary(
    root: &Path,
    profile_name: &str,
    opts: GenerateOptions,
) -> Result<GeneratePlanSummary> {
    let requested = RequestedGenerateState::new(
        RequestedConfigPath::new(root.join("kslim.toml"))?,
        ProfileName::new(profile_name.to_string())?,
        CliOverrides::from_options(&opts),
    );
    let plan = resolve_generate_plan(
        requested,
        &ConfigLoader::new(),
        &SourceResolver::new(),
    )?;
    Ok(GeneratePlanSummary::from_plan(&plan))
}
