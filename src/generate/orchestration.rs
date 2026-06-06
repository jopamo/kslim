//! Generate command orchestration entrypoints.
//!
//! This module owns the public generate transaction wrapper: requested-state
//! setup, failure-context setup, final lockfile publication, and failed-run
//! rollback/report recording. Stage-specific helpers remain beside their owned
//! generate modules while this root split continues.

use anyhow::Result;
use std::path::PathBuf;

use crate::config::{KslimConfig, ProfileConfig};
use crate::output_repo;
use crate::paths::{AttemptMetadataDir, LockfilePath};

use super::state::RequestedGenerateState;
use super::{
    clear_project_failure_artifacts, ensure_non_authoritative_attempt_path, failure,
    generate_inner, log_generate_stage, project_attempt_metadata_dir, project_failure_report_path,
    record_generate_attempt_failure, reducer_manifest_for_profile,
    rollback_failed_run_lockfile_state, rollback_output_repo_failure_atomic_state,
    rollback_published_metadata_failure_atomic_state, set_generate_stage,
    write_authoritative_lockfile, write_project_last_attempt, write_project_reducer_failure_report,
    FailureGenerateState, FailureReportContext, GenerateOptions, GeneratePlanSourceMaps,
    GenerateResult, GenerateStage,
};

/// Execute the full generate transaction:
/// prepare -> source -> resolve -> materialize -> verify -> commit
#[allow(dead_code)]
pub fn generate(
    config: &KslimConfig,
    profile: &ProfileConfig,
    opts: &GenerateOptions,
) -> Result<GenerateResult> {
    generate_with_source_maps(config, profile, opts, None)
}

pub(crate) fn generate_with_source_maps(
    config: &KslimConfig,
    profile: &ProfileConfig,
    opts: &GenerateOptions,
    source_maps: Option<GeneratePlanSourceMaps>,
) -> Result<GenerateResult> {
    let project_root = crate::fsutil::find_kslim_root().ok();
    let mut failure = FailureReportContext {
        stage: GenerateStage::Resolve,
        ..FailureReportContext::default()
    };
    let requested_config_path = project_root
        .as_ref()
        .map(|root| root.as_std_path().join("kslim.toml"))
        .unwrap_or_else(|| PathBuf::from("kslim.toml"));
    log_generate_stage(failure.stage, "enter");
    let requested = RequestedGenerateState::from_inputs(requested_config_path, profile, opts)?;
    let active_profile = requested
        .cli_overrides
        .apply_profile_overrides(profile.clone())?;
    failure.states.record_requested(requested.clone())?;
    if let Some(project_root) = project_root.as_ref() {
        let lockfile_path = LockfilePath::new_in_project_root(project_root.as_std_path())?;
        failure.lockfile_rollback = Some(
            crate::lockfile::capture_lockfile_failure_atomic_state(&lockfile_path)?,
        );
        failure
            .states
            .record_failure(FailureGenerateState::from_project_root(
                project_root.as_std_path(),
            )?)?;
    }
    let mut result = generate_inner(
        config,
        &active_profile,
        opts,
        requested,
        source_maps,
        project_root.as_ref().map(|root| root.as_std_path()),
        &mut failure,
    );
    if opts.dry_run || opts.deep_dry_run || (opts.report_only && result.is_ok()) {
        return result;
    }
    if result.is_ok() {
        let lockfile_inputs = project_root
            .as_ref()
            .zip(failure.resolved.clone())
            .zip(failure.mode.clone())
            .map(|((project_root, resolved), mode)| {
                (project_root.as_std_path().to_path_buf(), resolved, mode)
            });
        if let (Some((project_root, resolved, mode)), Ok(generate_result)) =
            (lockfile_inputs, result.as_ref())
        {
            set_generate_stage(&mut failure, GenerateStage::Publish);
            // `kslim.lock` is the final authoritative publication marker.
            // Non-authoritative success cleanup must happen before it, so no
            // later filesystem write can make the lockfile point at state that
            // was not the last committed publication step.
            if let Err(report_err) = clear_project_failure_artifacts(project_root.as_path()) {
                log::warn!(
                    "generate: stage={} failed to clear stale project failure artifacts before final lockfile update: {:#}",
                    failure.stage.as_str(),
                    report_err
                );
            }
            if let Err(err) = write_authoritative_lockfile(
                project_root.as_path(),
                config,
                &active_profile,
                &resolved,
                &mode,
                generate_result,
            ) {
                result = Err(err);
            } else {
                failure.output_repo_rollback = None;
                if let Ok(generate_result) = result.as_mut() {
                    generate_result.stage = failure.stage;
                }
            }
        }
    }
    if result.is_err() {
        if let Some(output_rollback) = failure.output_repo_rollback.take() {
            if let Err(rollback_err) =
                rollback_output_repo_failure_atomic_state(&config.output.path, &output_rollback)
            {
                let original_err = result.err().unwrap();
                result = Err(anyhow::anyhow!(
                    "generate failed: {:#}; output rollback also failed: {:#}",
                    original_err,
                    rollback_err
                ));
            }
        }
    }
    match &result {
        Ok(_) => {}
        Err(err) => {
            if let Some(project_root) = project_root.as_ref() {
                let failure_message = format!("{:#}", err);
                if let Err(report_err) = record_generate_attempt_failure(
                    project_root.as_std_path(),
                    &mut failure,
                    &failure_message,
                ) {
                    log::warn!(
                        "generate: stage={} failed to record typed attempt failure: {:#}",
                        failure.stage.as_str(),
                        report_err
                    );
                }
                let report_path = project_failure_report_path(project_root.as_std_path());
                if let Err(report_err) = ensure_non_authoritative_attempt_path(
                    project_root.as_std_path(),
                    report_path.as_path(),
                )
                .and_then(|()| {
                    output_repo::write_failure_report(
                        report_path.as_path(),
                        config,
                        &active_profile,
                        failure.resolved.as_ref(),
                        failure.mode.as_deref(),
                        failure.patch_infos.as_deref(),
                        failure.stage,
                        &failure_message,
                        failure.file_count,
                        failure.total_bytes,
                        failure.reducer_stats.as_ref(),
                    )
                }) {
                    log::warn!(
                        "generate: stage={} failed to write failure report: {:#}",
                        failure.stage.as_str(),
                        report_err
                    );
                }
                if let Err(report_err) = write_project_last_attempt(
                    project_root.as_std_path(),
                    &failure,
                    &failure_message,
                ) {
                    log::warn!(
                        "generate: stage={} failed to write last-attempt metadata: {:#}",
                        failure.stage.as_str(),
                        report_err
                    );
                }
                if let Err(report_err) =
                    write_project_reducer_failure_report(project_root.as_std_path(), &failure)
                {
                    log::warn!(
                        "generate: stage={} failed to write reducer failure report: {:#}",
                        failure.stage.as_str(),
                        report_err
                    );
                }
                let attempt_dir = project_attempt_metadata_dir(project_root.as_std_path());
                if let Err(report_err) =
                    ensure_non_authoritative_attempt_path(project_root.as_std_path(), &attempt_dir)
                        .and_then(|()| {
                            let reducer_manifest =
                                reducer_manifest_for_profile(&active_profile, None)?;
                            output_repo::write_reducer_metadata_at_dir_with_context(
                                attempt_dir.as_path(),
                                failure.reducer_stats.as_ref(),
                                Some(&active_profile.reducer),
                                reducer_manifest.as_ref(),
                            )
                        })
                {
                    log::warn!(
                        "generate: stage={} failed to write structured reducer failure artifacts: {:#}",
                        failure.stage.as_str(),
                        report_err
                    );
                }
                let attempt_dir = AttemptMetadataDir::new(project_attempt_metadata_dir(
                    project_root.as_std_path(),
                ));
                if let Err(report_err) = attempt_dir.and_then(|attempt_dir| {
                    ensure_non_authoritative_attempt_path(
                        project_root.as_std_path(),
                        attempt_dir.as_path(),
                    )?;
                    failure::record_generate_failure(
                        failure.generate_plan.as_ref(),
                        failure.stage,
                        err,
                        &attempt_dir,
                    )
                }) {
                    log::warn!(
                        "generate: stage={} failed to write typed generate failure metadata: {:#}",
                        failure.stage.as_str(),
                        report_err
                    );
                }
            }
        }
    }
    if result.is_err() {
        if let Some(published_metadata_rollback) = failure.published_metadata_rollback.as_ref() {
            if let Err(rollback_err) =
                rollback_published_metadata_failure_atomic_state(published_metadata_rollback)
            {
                let original_err = result.err().unwrap();
                result = Err(anyhow::anyhow!(
                    "generate failed: {:#}; published metadata rollback also failed: {:#}",
                    original_err,
                    rollback_err
                ));
            }
        }
    }
    if result.is_err() {
        if let Err(rollback_err) = rollback_failed_run_lockfile_state(&failure) {
            let original_err = result.err().unwrap();
            result = Err(anyhow::anyhow!(
                "generate failed: {:#}; lockfile rollback also failed: {:#}",
                original_err,
                rollback_err
            ));
        }
    }
    result
}
