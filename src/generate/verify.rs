//! Read-only candidate verification entrypoint.
//!
//! This module may inspect a private candidate tree and its candidate metadata.
//! Legacy generate glue may write private candidate metadata immediately before
//! verification. It must not mutate the candidate tree, write authoritative
//! metadata, update the lockfile, commit output, or publish.

mod fs;
mod invariants;
mod metadata;
mod report;
mod stage;

use anyhow::Result;
use std::path::Path;

#[cfg(test)]
use crate::manifest;

use crate::config::ProfileConfig;
use crate::model::{MetadataFingerprint, TreeFingerprint};
use crate::paths::KernelSourceRoot;
use crate::reducer;
use crate::selftest::SelfTestResult;
use crate::upstream;

use super::plan::GeneratePlan;
use super::state::CandidateTreeState;
use super::{
    candidate, log_generate_stage, reducer_manifest_for_profile, set_generate_stage,
    FailureReportContext, GenerateStage,
};

use fs::{
    ensure_candidate_is_observable, fingerprint_candidate_metadata, fingerprint_candidate_tree,
};
use invariants::{
    verify_no_broad_speculative_fallout_edits, verify_no_unknown_diagnostics_in_strict_mode,
    verify_no_unreasoned_edits, verify_no_unsupported_syntax_in_strict_mode,
    verify_reducer_success, verify_selftest_policy,
};
#[cfg(test)]
use metadata::CANDIDATE_METADATA_FILE;
use metadata::{
    read_candidate_metadata_summary, verify_candidate_metadata_complete,
    verify_no_host_only_absolute_paths_in_committed_candidate_metadata,
    verify_no_raw_logs_in_committed_candidate_metadata,
    verify_no_temporary_paths_in_committed_candidate_metadata,
    verify_only_reproducible_timestamps_in_committed_candidate_metadata, CandidateMetadataSummary,
};
use report::{verify_candidate_reports, verify_report_paths_are_relative_and_normalized};
#[allow(unused_imports)]
pub(crate) use stage::VerificationStage;

#[allow(dead_code)]
pub(crate) const FIXED_VERIFICATION_PIPELINE: &[&str] = &[
    VerificationStage::EnsureCandidateObservable.description(),
    VerificationStage::RejectTemporaryMetadataPaths.description(),
    VerificationStage::RejectHostMetadataPaths.description(),
    VerificationStage::RejectRawMetadataLogs.description(),
    VerificationStage::VerifyReproducibleMetadataTimestamps.description(),
    VerificationStage::ReadCandidateMetadata.description(),
    VerificationStage::FingerprintCandidateTree.description(),
    VerificationStage::VerifyCandidateMetadata.description(),
    VerificationStage::VerifyReducerSuccess.description(),
    VerificationStage::VerifyCandidateReports.description(),
    VerificationStage::VerifyReportPaths.description(),
    VerificationStage::VerifyReasonedEdits.description(),
    VerificationStage::VerifyNoSpeculativeFallout.description(),
    VerificationStage::VerifyNoUnknownDiagnostics.description(),
    VerificationStage::VerifyNoUnsupportedSyntax.description(),
    VerificationStage::VerifySelftestPolicy.description(),
    VerificationStage::FingerprintCandidateMetadata.description(),
];

fn log_verification_stage(stage: VerificationStage) {
    log::info!("generate.verify: stage={}", stage.as_str());
}

/// Proof that legacy generated output passed the pre-candidate-verification
/// checks needed before constructing the final candidate verification proof.
#[derive(Debug)]
pub(crate) struct VerifiedGeneratedOutput {
    tree_path: String,
    selftests: Option<SelfTestResult>,
}

impl VerifiedGeneratedOutput {
    pub(crate) fn tree_path(&self) -> &str {
        &self.tree_path
    }

    pub(crate) fn selftests(&self) -> Option<&SelfTestResult> {
        self.selftests.as_ref()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CandidateVerification {
    tree_fingerprint: TreeFingerprint,
    metadata_fingerprint: MetadataFingerprint,
    reducer_ok: bool,
    selftest_ok: bool,
    report_ok: bool,
}

#[allow(dead_code)]
impl CandidateVerification {
    fn new(
        tree_fingerprint: TreeFingerprint,
        metadata_fingerprint: MetadataFingerprint,
        reducer_ok: bool,
        selftest_ok: bool,
        report_ok: bool,
    ) -> Result<Self> {
        if !(reducer_ok && selftest_ok && report_ok) {
            anyhow::bail!("candidate verification cannot be constructed from failed checks");
        }
        Ok(Self {
            tree_fingerprint,
            metadata_fingerprint,
            reducer_ok,
            selftest_ok,
            report_ok,
        })
    }

    pub(crate) fn tree_fingerprint(&self) -> &TreeFingerprint {
        &self.tree_fingerprint
    }

    pub(crate) fn metadata_fingerprint(&self) -> &MetadataFingerprint {
        &self.metadata_fingerprint
    }

    pub(crate) fn reducer_ok(&self) -> bool {
        self.reducer_ok
    }

    pub(crate) fn selftest_ok(&self) -> bool {
        self.selftest_ok
    }

    pub(crate) fn report_ok(&self) -> bool {
        self.report_ok
    }

    pub(crate) fn all_checks_ok(&self) -> bool {
        self.reducer_ok && self.selftest_ok && self.report_ok
    }
}

#[allow(dead_code)]
pub(crate) fn verify_candidate(
    plan: &GeneratePlan,
    candidate: &CandidateTreeState,
) -> Result<CandidateVerification> {
    log_verification_stage(VerificationStage::EnsureCandidateObservable);
    ensure_candidate_is_observable(candidate)?;
    log_verification_stage(VerificationStage::RejectTemporaryMetadataPaths);
    verify_no_temporary_paths_in_committed_candidate_metadata(
        candidate.metadata_dir.as_path(),
        &[candidate.tree.as_path()],
    )?;
    log_verification_stage(VerificationStage::RejectHostMetadataPaths);
    verify_no_host_only_absolute_paths_in_committed_candidate_metadata(
        candidate.metadata_dir.as_path(),
    )?;
    log_verification_stage(VerificationStage::RejectRawMetadataLogs);
    verify_no_raw_logs_in_committed_candidate_metadata(candidate.metadata_dir.as_path())?;
    log_verification_stage(VerificationStage::VerifyReproducibleMetadataTimestamps);
    verify_only_reproducible_timestamps_in_committed_candidate_metadata(
        candidate.metadata_dir.as_path(),
        &plan.resolved.base.resolved_at,
    )?;
    log_verification_stage(VerificationStage::ReadCandidateMetadata);
    let metadata = read_candidate_metadata_summary(candidate.metadata_dir.as_path())?;
    log_verification_stage(VerificationStage::FingerprintCandidateTree);
    let tree_fingerprint = fingerprint_candidate_tree(candidate.tree.as_path())?;
    log_verification_stage(VerificationStage::VerifyCandidateMetadata);
    verify_candidate_metadata_complete(plan, candidate, &metadata, &tree_fingerprint)?;
    log_verification_stage(VerificationStage::VerifyReducerSuccess);
    let reducer_ok = verify_reducer_success(plan, candidate, &metadata)?;
    log_verification_stage(VerificationStage::VerifyCandidateReports);
    let report_ok = verify_candidate_reports(plan, candidate, &metadata)?;
    log_verification_stage(VerificationStage::VerifyReportPaths);
    verify_report_paths_are_relative_and_normalized(plan, candidate, &metadata)?;
    log_verification_stage(VerificationStage::VerifyReasonedEdits);
    verify_no_unreasoned_edits(plan, candidate, &metadata)?;
    log_verification_stage(VerificationStage::VerifyNoSpeculativeFallout);
    verify_no_broad_speculative_fallout_edits(plan, candidate, &metadata)?;
    log_verification_stage(VerificationStage::VerifyNoUnknownDiagnostics);
    verify_no_unknown_diagnostics_in_strict_mode(plan, candidate, &metadata)?;
    log_verification_stage(VerificationStage::VerifyNoUnsupportedSyntax);
    verify_no_unsupported_syntax_in_strict_mode(plan, candidate, &metadata)?;
    log_verification_stage(VerificationStage::VerifySelftestPolicy);
    let selftest_ok = verify_selftest_policy(plan, candidate, &metadata)?;
    log_verification_stage(VerificationStage::FingerprintCandidateMetadata);
    let metadata_fingerprint = fingerprint_candidate_metadata(candidate.metadata_dir.as_path())?;

    CandidateVerification::new(
        tree_fingerprint,
        metadata_fingerprint,
        reducer_ok,
        selftest_ok,
        report_ok,
    )
}

fn plan_requires_reducer(plan: &GeneratePlan) -> bool {
    !plan.resolved.prune_plan.remove_paths.is_empty()
        || !plan.resolved.prune_plan.remove_configs.is_empty()
        || !plan.resolved.prune_plan.set_defaults.is_empty()
}

fn reducer_artifacts_required(plan: &GeneratePlan, metadata: &CandidateMetadataSummary) -> bool {
    plan_requires_reducer(plan) || metadata.reducer_report_file.is_some()
}

pub(super) fn verify_generated_output(
    tree_path: &str,
    profile: &ProfileConfig,
    run_requested_selftests: bool,
    reducer_stats: &mut reducer::ReducerStats,
    failure: &mut FailureReportContext,
) -> Result<VerifiedGeneratedOutput> {
    log_generate_stage(failure.stage, "verify_generated_output");
    upstream::validate_tree(tree_path)?;
    verify_required_metadata(std::path::Path::new(tree_path))?;

    if run_requested_selftests {
        set_generate_stage(failure, GenerateStage::Selftest);
        let kernel_root = KernelSourceRoot::new(tree_path)?;
        match reducer::run_selftests_with_fixups(&kernel_root, profile, reducer_stats) {
            Ok(result) => {
                failure.reducer_stats = Some(reducer_stats.clone());
                return Ok(VerifiedGeneratedOutput {
                    tree_path: tree_path.to_string(),
                    selftests: Some(result.selftests),
                });
            }
            Err(err) => {
                if let Some(fixed_point) = err.downcast_ref::<reducer::SelftestFixedPointFailure>()
                {
                    failure.reducer_failure =
                        Some(reducer::ReducerFailureReport::from_fixed_point(fixed_point));
                }
                failure.reducer_stats = Some(reducer_stats.clone());
                return Err(err);
            }
        }
    }

    failure.reducer_stats = Some(reducer_stats.clone());
    Ok(VerifiedGeneratedOutput {
        tree_path: tree_path.to_string(),
        selftests: None,
    })
}

fn verify_required_metadata(tree_root: &std::path::Path) -> Result<()> {
    let kslim_dir = tree_root.join(".kslim");
    for required in ["base.toml", "generated.toml", "manifest.txt"] {
        if !kslim_dir.join(required).exists() {
            anyhow::bail!("verification failed: .kslim/{} missing", required);
        }
    }
    Ok(())
}

pub(super) fn write_candidate_metadata_and_verify(
    plan: &GeneratePlan,
    tree_path: &Path,
    integrated: bool,
    reduced: bool,
    selftested: bool,
    reducer_stats: &reducer::ReducerStats,
    profile: &ProfileConfig,
) -> Result<CandidateVerification> {
    let mut candidate = CandidateTreeState::from_materialized_tree(tree_path.to_path_buf())?;
    if integrated {
        candidate.mark_integrated()?;
    }
    if reduced {
        candidate.mark_reduced()?;
    }
    if selftested {
        candidate.mark_selftested()?;
    }

    let removal_manifest =
        reducer_manifest_for_profile(profile, Some(tree_path))?.unwrap_or_default();
    candidate::write_candidate_metadata_for_verified_generate(
        plan,
        &candidate,
        Some(reducer_stats),
        &profile.reducer,
        &removal_manifest,
    )?;
    verify_candidate(plan, &candidate)
}

#[cfg(test)]
mod tests {
    use super::super::state::{
        CliOverrides, ProfileName, RequestedGenerateState, ResolvedCandidateState,
    };
    use super::*;
    use crate::config;
    use crate::lockfile::ResolvedBase;
    use crate::paths::{CandidateMetadataDir, CandidateTreePath, RequestedConfigPath};

    fn create_minimal_tree(root: &Path) {
        for dir in &[
            "arch", "drivers", "fs", "include", "kernel", "mm", "net", "scripts",
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
            std::fs::write(root.join(dir).join(".keep"), "").unwrap();
        }
        std::fs::write(root.join("Makefile"), "# test\n").unwrap();
        std::fs::write(root.join("Kconfig"), "# test\n").unwrap();
    }

    fn requested_state(config_path: &Path) -> RequestedGenerateState {
        RequestedGenerateState::new(
            RequestedConfigPath::new(config_path).unwrap(),
            ProfileName::new("default").unwrap(),
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
                run_selftests: false,
            },
        )
    }

    fn plan_for_profile(
        config_path: &Path,
        output: &Path,
        profile: config::ProfileConfig,
    ) -> GeneratePlan {
        let config = config::default_kslim_config("demo", output.to_str().unwrap());
        let resolved = ResolvedCandidateState::from_resolved_inputs(
            &config,
            &profile,
            ResolvedBase {
                upstream: config.upstream.name.clone(),
                url: config.upstream.url.clone(),
                r#ref: String::from("v1.0"),
                commit: String::from("deadbeef"),
                resolved_at: String::from("2026-01-01T00:00:00Z"),
            },
            None,
            "unmodified-upstream",
            "kslim/v1.0/default",
        )
        .unwrap();
        GeneratePlan::new(requested_state(config_path), resolved).unwrap()
    }

    fn plan_for_tree(config_path: &Path, output: &Path) -> GeneratePlan {
        plan_for_profile(config_path, output, config::default_profile_config("v1.0"))
    }

    fn reducer_plan_for_tree(config_path: &Path, output: &Path) -> GeneratePlan {
        let mut profile = config::default_profile_config("v1.0");
        profile.slim = Some(config::SlimConfig {
            remove_paths: vec![String::from("drivers/gpu")],
            remove_configs: Vec::new(),
            set_defaults: Default::default(),
            unsafe_allow_root_path_removal: false,
        });
        plan_for_profile(config_path, output, profile)
    }

    fn selftests_disabled_plan_for_tree(config_path: &Path, output: &Path) -> GeneratePlan {
        let mut profile = config::default_profile_config("v1.0");
        profile.selftests.enabled = false;
        plan_for_profile(config_path, output, profile)
    }

    fn write_candidate_metadata_for_test(
        metadata_dir: &Path,
        plan: &GeneratePlan,
        candidate: &CandidateTreeState,
        reducer_ran: bool,
    ) {
        write_candidate_metadata_with_selftested_for_test(
            metadata_dir,
            plan,
            candidate,
            reducer_ran,
            candidate.selftested,
        );
    }

    fn write_candidate_metadata_with_selftested_for_test(
        metadata_dir: &Path,
        plan: &GeneratePlan,
        candidate: &CandidateTreeState,
        reducer_ran: bool,
        metadata_selftested: bool,
    ) {
        let reducer_report_file = if reducer_ran {
            format!(
                "reducer_report_file = \"{}\"\n",
                crate::output_repo::REDUCER_REPORT_JSON
            )
        } else {
            String::new()
        };
        std::fs::write(
            metadata_dir.join(manifest::OUTPUT_MANIFEST_FILE_NAME),
            "hash  1  Makefile\n",
        )
        .unwrap();
        let tree_fingerprint = fingerprint_candidate_tree(candidate.tree.as_path()).unwrap();
        std::fs::write(
            metadata_dir.join(CANDIDATE_METADATA_FILE),
            format!(
                concat!(
                    "schema_version = 1\n",
                    "metadata_scope = \"candidate\"\n",
                    "authoritative = false\n",
                    "plan_id = \"{}\"\n",
                    "plan_fingerprint = \"{}\"\n",
                    "tree_fingerprint = \"{}\"\n",
                    "config_content_hash = \"{}\"\n",
                    "generated_by = \"{}\"\n",
                    "selected_profile = \"{}\"\n",
                    "upstream_name = \"{}\"\n",
                    "base_ref = \"{}\"\n",
                    "base_commit = \"{}\"\n",
                    "base_resolved_at = \"{}\"\n",
                    "output_branch = \"{}\"\n",
                    "output_mode = \"{}\"\n",
                    "patch_source_count = {}\n",
                    "patch_commit_count = {}\n",
                    "integration_count = {}\n",
                    "materialized = {}\n",
                    "integrated = {}\n",
                    "pruned = {}\n",
                    "reduced = {}\n",
                    "selftested = {}\n",
                    "reducer_ran = {}\n",
                    "manifest_file = \"{}\"\n",
                    "{}"
                ),
                plan.plan_id.as_str(),
                plan.fingerprint.as_str(),
                tree_fingerprint.as_str(),
                plan.config_content_hash.as_str(),
                plan.created_with.as_str(),
                plan.requested.selected_profile.as_str(),
                &plan.resolved.base.upstream,
                &plan.resolved.base.r#ref,
                &plan.resolved.base.commit,
                &plan.resolved.base.resolved_at,
                &plan.resolved.output_plan.branch,
                &plan.resolved.output_plan.mode,
                plan.resolved.patch_plan.sources.len(),
                plan.resolved.patch_plan.total_patch_count,
                plan.resolved.integration_plan.entries.len(),
                candidate.materialized,
                candidate.integrated,
                candidate.pruned,
                candidate.reduced,
                metadata_selftested,
                reducer_ran,
                manifest::OUTPUT_MANIFEST_FILE_NAME,
                reducer_report_file,
            ),
        )
        .unwrap();
    }

    fn write_empty_reducer_artifacts(metadata_dir: &Path) {
        std::fs::write(
            metadata_dir.join(crate::output_repo::REDUCER_REPORT_JSON),
            concat!(
                "{\n",
                "  \"summary\": { \"edit_records\": 0 },\n",
                "  \"unsupported_fallout\": {\n",
                "    \"unsupported_kconfig_expressions\": 0,\n",
                "    \"unsupported_cpp_expressions\": 0\n",
                "  },\n",
                "  \"artifacts\": {\n",
                "    \"markdown\": \"reducer-report.md\",\n",
                "    \"summary_json\": \"reducer-report.json\",\n",
                "    \"diagnostics_json\": \"diagnostics.json\",\n",
                "    \"edit_summary_json\": \"edit-summary.json\",\n",
                "    \"kconfig_solver_report_json\": \"kconfig-solver-report.json\",\n",
                "    \"kconfig_rewrite_report_json\": \"kconfig-rewrite-report.json\",\n",
                "    \"skipped_sites_json\": null\n",
                "  }\n",
                "}\n"
            ),
        )
        .unwrap();
        std::fs::write(
            metadata_dir.join(crate::output_repo::REDUCER_EDIT_SUMMARY_JSON),
            "{ \"edit_records\": 0, \"edit_record_details\": [] }\n",
        )
        .unwrap();
        std::fs::write(
            metadata_dir.join(crate::output_repo::REDUCER_DIAGNOSTICS_JSON),
            concat!(
                "{\n",
                "  \"unsupported_kconfig_expressions\": [],\n",
                "  \"unsupported_cpp_expressions\": [],\n",
                "  \"skipped_cpp_nested_edge_cases\": [],\n",
                "  \"ambiguous_makefile_lines\": [],\n",
                "  \"skipped_fixup_diagnostics\": []\n",
                "}\n"
            ),
        )
        .unwrap();
        write_empty_kconfig_solver_report(metadata_dir);
        write_empty_kconfig_rewrite_report(metadata_dir);
    }

    fn write_unreasoned_reducer_artifacts(metadata_dir: &Path) {
        std::fs::write(
            metadata_dir.join(crate::output_repo::REDUCER_REPORT_JSON),
            concat!(
                "{\n",
                "  \"summary\": { \"edit_records\": 1 },\n",
                "  \"unsupported_fallout\": {\n",
                "    \"unsupported_kconfig_expressions\": 0,\n",
                "    \"unsupported_cpp_expressions\": 0\n",
                "  },\n",
                "  \"artifacts\": {\n",
                "    \"markdown\": \"reducer-report.md\",\n",
                "    \"summary_json\": \"reducer-report.json\",\n",
                "    \"diagnostics_json\": \"diagnostics.json\",\n",
                "    \"edit_summary_json\": \"edit-summary.json\",\n",
                "    \"kconfig_solver_report_json\": \"kconfig-solver-report.json\",\n",
                "    \"kconfig_rewrite_report_json\": \"kconfig-rewrite-report.json\",\n",
                "    \"skipped_sites_json\": null\n",
                "  }\n",
                "}\n"
            ),
        )
        .unwrap();
        std::fs::write(
            metadata_dir.join(crate::output_repo::REDUCER_EDIT_SUMMARY_JSON),
            r#"{
  "edit_records": 1,
  "edit_record_details": [
    {
      "file": "Kconfig",
      "pass_name": "kconfig.rewrite_relations",
      "edit_kind": "rewrite_line",
      "edit_reason": { "kind": "manifest_config", "payload": "symbol=" },
      "proof_source": { "kind": "removal_manifest_entry", "payload": "symbol=REMOVED" },
      "old": {
        "line_start": 1,
        "line_end": 1,
        "logical_item": "depends on REMOVED",
        "byte_len": 18,
        "sha256": "a938d82cb8893a12cec337ebd5ab76dcaca97e2112566c6e87e97714ad1e9104"
      },
      "new": {
        "logical_item": "depends on OTHER",
        "byte_len": 16,
        "sha256": "5fbeefab48f759ab044f94ca284a3f1a881e05386ffaa5aef9dcbeaa33ac035c"
      },
      "idempotence_marker": "marker"
    }
  ]
}
"#,
        )
        .unwrap();
        std::fs::write(
            metadata_dir.join(crate::output_repo::REDUCER_DIAGNOSTICS_JSON),
            concat!(
                "{\n",
                "  \"unsupported_kconfig_expressions\": [],\n",
                "  \"unsupported_cpp_expressions\": [],\n",
                "  \"skipped_cpp_nested_edge_cases\": [],\n",
                "  \"ambiguous_makefile_lines\": [],\n",
                "  \"skipped_fixup_diagnostics\": []\n",
                "}\n"
            ),
        )
        .unwrap();
        write_empty_kconfig_solver_report(metadata_dir);
        write_empty_kconfig_rewrite_report(metadata_dir);
    }

    fn write_invalid_byte_evidence_reducer_artifacts(metadata_dir: &Path) {
        write_unreasoned_reducer_artifacts(metadata_dir);
        let path = metadata_dir.join(crate::output_repo::REDUCER_EDIT_SUMMARY_JSON);
        let summary = std::fs::read_to_string(&path).unwrap();
        std::fs::write(
            &path,
            summary.replace("\"byte_len\": 18", "\"byte_len\": 17"),
        )
        .unwrap();
    }

    fn write_competing_proof_source_reducer_artifacts(metadata_dir: &Path) {
        write_unreasoned_reducer_artifacts(metadata_dir);
        std::fs::write(
            metadata_dir.join(crate::output_repo::REDUCER_EDIT_SUMMARY_JSON),
            r#"{
  "edit_records": 1,
  "edit_record_details": [
    {
      "file": "drivers/remove.c",
      "pass_name": "prune.remove_path",
      "edit_kind": "remove_path",
      "edit_reason": { "kind": "manifest_path", "payload": "path=drivers/remove.c" },
      "proof_source": { "kind": "removal_manifest_entry", "payload": "path=drivers/other.c" },
      "old": {
        "line_start": null,
        "line_end": null,
        "logical_item": "removed file",
        "byte_len": 12,
        "sha256": "81248938b8f94dac2349482a45c7b507fb5a47e2172e81ee4fb64f6cc2172a87"
      },
      "new": {
        "logical_item": "",
        "byte_len": 0,
        "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
      },
      "idempotence_marker": "marker"
    }
  ]
}
"#,
        )
        .unwrap();
    }

    fn write_speculative_fallout_reducer_artifacts(metadata_dir: &Path) {
        write_unreasoned_reducer_artifacts(metadata_dir);
        std::fs::write(
            metadata_dir.join(crate::output_repo::REDUCER_EDIT_SUMMARY_JSON),
            r#"{
  "edit_records": 1,
  "edit_record_details": [
    {
      "file": "drivers/live.c",
      "pass_name": "test.speculative_fallout",
      "edit_kind": "rewrite_line",
      "edit_reason": { "kind": "build_diagnostic", "payload": "class=UndefinedReference" },
      "proof_source": { "kind": "classified_build_diagnostic", "payload": "class=UndefinedReference key=amdgpu_magic" },
      "old": {
        "line_start": 1,
        "line_end": 1,
        "logical_item": "return amdgpu_magic();",
        "byte_len": 22,
        "sha256": "032a19d905a9b0e5b504de97f4583824dbe6faf571d1723b0ba004ca8f887233"
      },
      "new": {
        "logical_item": "return 0;",
        "byte_len": 9,
        "sha256": "6be2e46a3d0765fd4d08def164c35b75afcc481db6fd11cc5118c4e2c20b634d"
      },
      "idempotence_marker": "marker"
    }
  ]
}
"#,
        )
        .unwrap();
    }

    fn write_unknown_diagnostic_reducer_artifacts(metadata_dir: &Path) {
        write_empty_reducer_artifacts(metadata_dir);
        std::fs::write(
            metadata_dir.join(crate::output_repo::REDUCER_DIAGNOSTICS_JSON),
            r#"{
  "unsupported_kconfig_expressions": [],
  "unsupported_cpp_expressions": [],
  "skipped_cpp_nested_edge_cases": [],
  "ambiguous_makefile_lines": [],
  "skipped_fixup_diagnostics": [
    {
      "kind": "skipped_fixup_diagnostic",
      "fixer_name": null,
      "reason": "unknown diagnostic",
      "diagnostic": {
        "class": "Unknown",
        "file": null,
        "line": null,
        "subject": null,
        "build_target": null,
        "arch": null,
        "config": null
      }
    }
  ]
}
"#,
        )
        .unwrap();
    }

    fn write_classified_unknown_only_reducer_artifacts(metadata_dir: &Path) {
        write_empty_reducer_artifacts(metadata_dir);
        std::fs::write(
            metadata_dir.join(crate::output_repo::REDUCER_DIAGNOSTICS_JSON),
            r#"{
  "classified_diagnostics": [
    {
      "class": "Unknown",
      "file": "drivers/live.c",
      "line": 7,
      "subject": "amdgpu_magic",
      "build_target": "modules",
      "arch": null,
      "config": "defconfig"
    }
  ],
  "unknown_diagnostics": [
    {
      "class": "Unknown",
      "file": "drivers/live.c",
      "line": 7,
      "subject": "amdgpu_magic",
      "build_target": "modules",
      "arch": null,
      "config": "defconfig"
    }
  ],
  "consumed_diagnostics": [],
  "skipped_diagnostics": [],
  "unsupported_kconfig_expressions": [],
  "unsupported_cpp_expressions": [],
  "skipped_cpp_nested_edge_cases": [],
  "ambiguous_makefile_lines": [],
  "skipped_fixup_diagnostics": []
}
"#,
        )
        .unwrap();
    }

    fn write_unsupported_syntax_reducer_artifacts(metadata_dir: &Path) {
        std::fs::write(
            metadata_dir.join(crate::output_repo::REDUCER_REPORT_JSON),
            concat!(
                "{\n",
                "  \"summary\": { \"edit_records\": 0 },\n",
                "  \"unsupported_fallout\": {\n",
                "    \"unsupported_kconfig_expressions\": 1,\n",
                "    \"unsupported_cpp_expressions\": 0\n",
                "  },\n",
                "  \"artifacts\": {\n",
                "    \"markdown\": \"reducer-report.md\",\n",
                "    \"summary_json\": \"reducer-report.json\",\n",
                "    \"diagnostics_json\": \"diagnostics.json\",\n",
                "    \"edit_summary_json\": \"edit-summary.json\",\n",
                "    \"kconfig_solver_report_json\": \"kconfig-solver-report.json\",\n",
                "    \"kconfig_rewrite_report_json\": \"kconfig-rewrite-report.json\",\n",
                "    \"skipped_sites_json\": null\n",
                "  }\n",
                "}\n"
            ),
        )
        .unwrap();
        std::fs::write(
            metadata_dir.join(crate::output_repo::REDUCER_EDIT_SUMMARY_JSON),
            "{ \"edit_records\": 0, \"edit_record_details\": [] }\n",
        )
        .unwrap();
        std::fs::write(
            metadata_dir.join(crate::output_repo::REDUCER_DIAGNOSTICS_JSON),
            r#"{
  "unsupported_kconfig_expressions": [
    {
      "kind": "unsupported_kconfig_expression",
      "file": "Kconfig",
      "line": 7,
      "directive": "depends",
      "expression": "REMOVED + LIVE",
      "reason": "unsupported operator"
    }
  ],
  "unsupported_cpp_expressions": [],
  "skipped_cpp_nested_edge_cases": [],
  "ambiguous_makefile_lines": [],
  "skipped_fixup_diagnostics": []
}
"#,
        )
        .unwrap();
        write_empty_kconfig_solver_report(metadata_dir);
        write_empty_kconfig_rewrite_report(metadata_dir);
    }

    fn write_empty_kconfig_solver_report(metadata_dir: &Path) {
        std::fs::write(
            metadata_dir.join(crate::output_repo::REDUCER_KCONFIG_SOLVER_REPORT_JSON),
            "{ \"schema_version\": 1, \"files_analyzed\": 0 }\n",
        )
        .unwrap();
    }

    fn write_empty_kconfig_rewrite_report(metadata_dir: &Path) {
        std::fs::write(
            metadata_dir.join(crate::output_repo::REDUCER_KCONFIG_REWRITE_REPORT_JSON),
            "{ \"schema_version\": 1, \"kconfig_edit_count\": 0 }\n",
        )
        .unwrap();
    }

    #[test]
    fn test_verify_candidate_entrypoint_returns_candidate_verification() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, false);
        let candidate_metadata_before =
            std::fs::read_to_string(metadata_dir.join("candidate.toml")).unwrap();

        let verification = verify_candidate(&plan, &candidate).unwrap();

        assert!(verification
            .tree_fingerprint()
            .as_str()
            .starts_with("tree-"));
        assert!(verification
            .metadata_fingerprint()
            .as_str()
            .starts_with("metadata-"));
        assert!(verification.reducer_ok());
        assert!(verification.selftest_ok());
        assert!(verification.report_ok());
        assert_eq!(
            std::fs::read_to_string(metadata_dir.join("candidate.toml")).unwrap(),
            candidate_metadata_before
        );
        assert!(!output.exists());
    }

    #[test]
    fn test_candidate_verification_rejects_failed_check_state() {
        let err = CandidateVerification::new(
            TreeFingerprint::new("tree-ok").unwrap(),
            MetadataFingerprint::new("metadata-ok").unwrap(),
            true,
            true,
            false,
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("candidate verification cannot be constructed from failed checks"));

        let verification = CandidateVerification::new(
            TreeFingerprint::new("tree-ok").unwrap(),
            MetadataFingerprint::new("metadata-ok").unwrap(),
            true,
            true,
            true,
        )
        .unwrap();
        assert!(verification.all_checks_ok());
        assert_eq!(verification.tree_fingerprint().as_str(), "tree-ok");
        assert_eq!(verification.metadata_fingerprint().as_str(), "metadata-ok");
    }

    #[test]
    fn test_verify_candidate_rejects_unreduced_candidate_when_reducer_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = reducer_plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, false);

        let err = verify_candidate(&plan, &candidate).unwrap_err().to_string();

        assert!(err.contains("reducer is enabled but candidate state is not reduced"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_reducer_metadata_without_success_when_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = reducer_plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, false);

        let err = verify_candidate(&plan, &candidate).unwrap_err().to_string();

        assert!(err.contains("candidate metadata does not record reducer success"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_accepts_reducer_success_when_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = reducer_plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, true);
        write_empty_reducer_artifacts(&metadata_dir);

        let verification = verify_candidate(&plan, &candidate).unwrap();

        assert!(verification.reducer_ok());
        assert!(verification.selftest_ok());
        assert!(verification.report_ok());
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_missing_selftest_success_when_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = plan_for_tree(&config_path, &output);
        let candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, false);

        let err = verify_candidate(&plan, &candidate).unwrap_err().to_string();

        assert!(err.contains("selected selftest/build matrix is enabled"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_selftest_metadata_without_success_when_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_with_selftested_for_test(
            &metadata_dir,
            &plan,
            &candidate,
            false,
            false,
        );

        let err = verify_candidate(&plan, &candidate).unwrap_err().to_string();

        assert!(err.contains("candidate metadata field selftested mismatch"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_accepts_missing_selftest_when_disabled_by_policy() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = selftests_disabled_plan_for_tree(&config_path, &output);
        let candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, false);

        let verification = verify_candidate(&plan, &candidate).unwrap();

        assert!(verification.selftest_ok());
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_unreasoned_reducer_edits() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = reducer_plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, true);
        write_unreasoned_reducer_artifacts(&metadata_dir);

        let err = verify_candidate(&plan, &candidate).unwrap_err().to_string();

        assert!(err.contains("unreasoned edit reason payload"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_invalid_reducer_edit_byte_evidence() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = reducer_plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, true);
        write_invalid_byte_evidence_reducer_artifacts(&metadata_dir);

        let err = verify_candidate(&plan, &candidate).unwrap_err().to_string();

        assert!(err.contains("invalid old byte length"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_competing_proof_source_payloads_in_strict_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = reducer_plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, true);
        write_competing_proof_source_reducer_artifacts(&metadata_dir);

        let err = verify_candidate(&plan, &candidate).unwrap_err();
        let err = format!("{err:#}");

        assert!(err.contains("competing proof sources"));
        assert!(err.contains("drivers/remove.c"));
        assert!(err.contains("drivers/other.c"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_competing_proof_sources_when_unreasoned_policy_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let mut plan = reducer_plan_for_tree(&config_path, &output);
        plan.resolved.reducer_plan.reject_unreasoned_edits = false;
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, true);
        write_competing_proof_source_reducer_artifacts(&metadata_dir);

        let err = verify_candidate(&plan, &candidate).unwrap_err();
        let err = format!("{err:#}");

        assert!(err.contains("competing proof sources"));
        assert!(err.contains("drivers/remove.c"));
        assert!(err.contains("drivers/other.c"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_speculative_fallout_edits_in_strict_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = reducer_plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, true);
        write_speculative_fallout_reducer_artifacts(&metadata_dir);

        let err = verify_candidate(&plan, &candidate).unwrap_err();
        let err = format!("{err:#}");

        assert!(err.contains("broad speculative fallout"));
        assert!(err.contains("UndefinedReference"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_accepts_speculative_fallout_edits_when_policy_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let mut plan = reducer_plan_for_tree(&config_path, &output);
        plan.resolved.reducer_plan.reject_speculative_fallout_edits = false;
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, true);
        write_speculative_fallout_reducer_artifacts(&metadata_dir);

        let verification = verify_candidate(&plan, &candidate).unwrap();

        assert!(verification.reducer_ok());
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_accepts_unreasoned_reducer_edits_when_policy_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let mut plan = reducer_plan_for_tree(&config_path, &output);
        plan.resolved.reducer_plan.reject_unreasoned_edits = false;
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, true);
        write_unreasoned_reducer_artifacts(&metadata_dir);

        let verification = verify_candidate(&plan, &candidate).unwrap();

        assert!(verification.reducer_ok());
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_unknown_diagnostic_in_strict_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = reducer_plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, true);
        write_unknown_diagnostic_reducer_artifacts(&metadata_dir);

        let err = verify_candidate(&plan, &candidate).unwrap_err().to_string();

        assert!(err.contains("unknown diagnostic in strict mode"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_classified_unknown_diagnostic_in_strict_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = reducer_plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, true);
        write_classified_unknown_only_reducer_artifacts(&metadata_dir);

        let err = verify_candidate(&plan, &candidate).unwrap_err().to_string();

        assert!(err.contains("unknown diagnostic in strict mode"));
        assert!(err.contains("diagnostics report declares"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_accepts_unknown_diagnostic_when_policy_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let mut plan = reducer_plan_for_tree(&config_path, &output);
        plan.resolved.reducer_plan.fail_on_unknown_diagnostics = false;
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, true);
        write_classified_unknown_only_reducer_artifacts(&metadata_dir);

        let verification = verify_candidate(&plan, &candidate).unwrap();

        assert!(verification.reducer_ok());
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_unsupported_syntax_in_strict_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = reducer_plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, true);
        write_unsupported_syntax_reducer_artifacts(&metadata_dir);

        let err = verify_candidate(&plan, &candidate).unwrap_err().to_string();

        assert!(err.contains("unsupported Kconfig syntax in strict mode"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_accepts_unsupported_syntax_when_policy_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let mut plan = reducer_plan_for_tree(&config_path, &output);
        plan.resolved.reducer_plan.report_unsupported_expressions = false;
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, true);
        write_unsupported_syntax_reducer_artifacts(&metadata_dir);

        let verification = verify_candidate(&plan, &candidate).unwrap();

        assert!(verification.reducer_ok());
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_missing_reducer_report_when_reports_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = reducer_plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, true);

        let err = verify_candidate(&plan, &candidate).unwrap_err().to_string();

        assert!(err.contains("reducer report is missing"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_non_normalized_candidate_report_path() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = reducer_plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, true);
        let metadata_path = metadata_dir.join(CANDIDATE_METADATA_FILE);
        let metadata = std::fs::read_to_string(&metadata_path).unwrap().replace(
            "reducer_report_file = \"reducer-report.json\"",
            "reducer_report_file = \"../reducer-report.json\"",
        );
        std::fs::write(&metadata_path, metadata).unwrap();

        let err = verify_candidate(&plan, &candidate).unwrap_err().to_string();

        assert!(err
            .contains("candidate metadata reducer_report_file must be a relative normalized path"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_non_normalized_reducer_report_artifact_path() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = reducer_plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_reduced().unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, true);
        write_empty_reducer_artifacts(&metadata_dir);
        let report_path = metadata_dir.join(crate::output_repo::REDUCER_REPORT_JSON);
        let report = std::fs::read_to_string(&report_path).unwrap().replace(
            "\"markdown\": \"reducer-report.md\"",
            "\"markdown\": \"reports/../reducer-report.md\"",
        );
        std::fs::write(&report_path, report).unwrap();

        let err = verify_candidate(&plan, &candidate).unwrap_err().to_string();

        assert!(err.contains("reducer report artifact markdown must be a relative normalized path"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_host_only_absolute_path_in_committed_candidate_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, false);
        let metadata_path = metadata_dir.join(CANDIDATE_METADATA_FILE);
        let metadata = std::fs::read_to_string(&metadata_path)
            .unwrap()
            .replace("base_ref = \"v1.0\"", "base_ref = \"/tmp/host-only-ref\"");
        std::fs::write(&metadata_path, metadata).unwrap();

        let err = verify_candidate(&plan, &candidate).unwrap_err().to_string();

        assert!(err.contains("host-only absolute path"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_temporary_path_in_committed_candidate_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, false);
        let metadata_path = metadata_dir.join(CANDIDATE_METADATA_FILE);
        let metadata = std::fs::read_to_string(&metadata_path).unwrap().replace(
            "base_ref = \"v1.0\"",
            &format!("base_ref = \"{}\"", tree.display()),
        );
        std::fs::write(&metadata_path, metadata).unwrap();

        let err = verify_candidate(&plan, &candidate).unwrap_err().to_string();

        assert!(err.contains("temporary path"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_timestamp_outside_plan_policy() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, false);
        std::fs::write(
            metadata_dir.join(crate::output_repo::REPORT_FILE),
            "wall-clock: 2026-01-02T00:00:00Z\n",
        )
        .unwrap();

        let err = format!("{:#}", verify_candidate(&plan, &candidate).unwrap_err());

        assert!(err.contains("outside reproducible timestamp policy"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_raw_logs_in_committed_candidate_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, false);
        std::fs::write(
            metadata_dir.join(crate::output_repo::REPORT_FILE),
            "stderr:\nprivate compiler output\n",
        )
        .unwrap();

        let err = format!("{:#}", verify_candidate(&plan, &candidate).unwrap_err());

        assert!(err.contains("raw logs"));
        assert!(err.contains("normalized summaries"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_incomplete_candidate_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        std::fs::write(
            metadata_dir.join(CANDIDATE_METADATA_FILE),
            "selftested = true\n",
        )
        .unwrap();
        let plan = plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_selftested().unwrap();

        let err = verify_candidate(&plan, &candidate).unwrap_err();
        let message = format!("{err:#}");

        assert!(message.contains("failed to parse candidate metadata"));
        assert!(message.contains("missing field"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_candidate_metadata_tree_fingerprint_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let output = tmp.path().join("output");
        let config_path = tmp.path().join("project/kslim.toml");
        create_minimal_tree(&tree);
        let metadata_dir = tree.join(".kslim");
        std::fs::create_dir_all(&metadata_dir).unwrap();
        let plan = plan_for_tree(&config_path, &output);
        let mut candidate = CandidateTreeState::from_materialized_tree(&tree).unwrap();
        candidate.mark_selftested().unwrap();
        write_candidate_metadata_for_test(&metadata_dir, &plan, &candidate, false);
        let metadata_path = metadata_dir.join(CANDIDATE_METADATA_FILE);
        let actual_tree_fingerprint = fingerprint_candidate_tree(candidate.tree.as_path()).unwrap();
        let metadata = std::fs::read_to_string(&metadata_path).unwrap().replace(
            &format!(
                "tree_fingerprint = \"{}\"",
                actual_tree_fingerprint.as_str()
            ),
            "tree_fingerprint = \"tree-not-the-candidate\"",
        );
        std::fs::write(&metadata_path, metadata).unwrap();

        let err = verify_candidate(&plan, &candidate).unwrap_err().to_string();

        assert!(err.contains("candidate metadata field tree_fingerprint mismatch"));
        assert!(!output.exists());
    }

    #[test]
    fn test_verify_candidate_rejects_unmaterialized_state() {
        let tmp = tempfile::tempdir().unwrap();
        let tree = tmp.path().join("candidate");
        let metadata = tree.join(".kslim");
        let output = tmp.path().join("output");
        let candidate = CandidateTreeState::new(
            CandidateTreePath::new(&tree).unwrap(),
            CandidateMetadataDir::new(&metadata).unwrap(),
            false,
            false,
            false,
            false,
            false,
        )
        .unwrap();
        let plan = plan_for_tree(&tmp.path().join("project/kslim.toml"), &output);

        let err = verify_candidate(&plan, &candidate).unwrap_err().to_string();

        assert!(err.contains("cannot verify candidate before materialization"));
        assert!(!output.exists());
    }
}
