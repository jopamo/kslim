use anyhow::Result;
use std::fmt;

use crate::config::ProfileConfig;
use crate::diagnostics::classify_selftest_failure;
use crate::fixups::SkippedFixup;
use crate::paths::KernelSourceRoot;
use crate::selftest::{self, SelfTestResult};

use super::context::ReducerContext;
use super::diagnostics::{
    non_convergence_report, raw_diagnostic_excerpt_from_failure,
    record_selftest_failure_diagnostic, render_raw_diagnostic_excerpt,
};
use super::{
    apply_selftest_fixup, BuildMatrixStatus, ConvergenceStatus, ReducerResult, ReducerStage,
    ReducerStats, ReducerStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FixedPointLoopTermination {
    SuccessfulSelectedBuildTestMatrix,
    NoFixerChangedTree,
    MaxPassCountReached,
    UnknownDiagnosticInStrictMode,
    UnsupportedSyntaxInStrictMode,
}

impl FixedPointLoopTermination {
    pub(crate) fn json_value(self) -> &'static str {
        match self {
            Self::SuccessfulSelectedBuildTestMatrix => "successful_selected_build_test_matrix",
            Self::NoFixerChangedTree => "no_fixer_changed_tree",
            Self::MaxPassCountReached => "max_pass_count_reached",
            Self::UnknownDiagnosticInStrictMode => "unknown_diagnostic_in_strict_mode",
            Self::UnsupportedSyntaxInStrictMode => "unsupported_syntax_in_strict_mode",
        }
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::SuccessfulSelectedBuildTestMatrix => "selected build/test matrix passed",
            Self::NoFixerChangedTree => "no fixer changed the tree",
            Self::MaxPassCountReached => "max fixup pass count reached",
            Self::UnknownDiagnosticInStrictMode => "unknown diagnostic in strict mode",
            Self::UnsupportedSyntaxInStrictMode => "unsupported syntax in strict mode",
        }
    }
}

#[derive(Debug)]
pub(crate) struct SelftestFixedPointResult {
    pub(crate) termination: FixedPointLoopTermination,
    pub(crate) fixup_passes: usize,
    pub(crate) selftests: SelfTestResult,
}

#[derive(Debug)]
pub(crate) struct SelftestFixedPointFailure {
    pub(crate) termination: FixedPointLoopTermination,
    pub(crate) fixup_passes: usize,
    pub(crate) failure: selftest::SelfTestFailure,
}

impl fmt::Display for SelftestFixedPointFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.termination {
            FixedPointLoopTermination::NoFixerChangedTree => write!(
                f,
                "fixed-point loop terminated: no fixer changed the tree after {} fixup pass(es)\n{}",
                self.fixup_passes, self.failure
            ),
            FixedPointLoopTermination::MaxPassCountReached => write!(
                f,
                "fixed-point loop terminated: max fixup pass count reached after {} fixup pass(es)\n{}",
                self.fixup_passes, self.failure
            ),
            FixedPointLoopTermination::UnknownDiagnosticInStrictMode => write!(
                f,
                "fixed-point loop terminated: unknown diagnostic in strict mode after {} fixup pass(es)\n{}",
                self.fixup_passes,
                render_raw_diagnostic_excerpt(&raw_diagnostic_excerpt_from_failure(&self.failure))
            ),
            FixedPointLoopTermination::UnsupportedSyntaxInStrictMode => write!(
                f,
                "fixed-point loop terminated: unsupported syntax in strict mode\n{}",
                self.failure
            ),
            FixedPointLoopTermination::SuccessfulSelectedBuildTestMatrix => {
                write!(f, "fixed-point loop terminated after successful build/test matrix")
            }
        }
    }
}

impl std::error::Error for SelftestFixedPointFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.failure)
    }
}

#[allow(dead_code)]
pub fn run_fixed_point_loop(ctx: &mut ReducerContext) -> Result<ReducerResult> {
    log_reducer_stage(ReducerStage::ReindexAndRepeat);
    let profile = ctx
        .fixed_point_profile()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("fixed-point reducer context is missing profile state"))?;
    let root = KernelSourceRoot::new(ctx.root())?;

    reset_loop_state(ctx);

    if strict_unsupported_syntax_in_stats(&profile, ctx.reducer_stats()) {
        {
            let state = ctx.loop_state_mut();
            state.convergence_reason = Some(
                FixedPointLoopTermination::UnsupportedSyntaxInStrictMode
                    .json_value()
                    .into(),
            );
        }
        let mut result = reducer_result_from_loop_stats(ctx.reducer_stats());
        result.set_publication_state(
            ReducerStatus::FailedUnsupportedSyntax,
            BuildMatrixStatus::NotRun,
            ConvergenceStatus::NotEvaluated,
        );
        return Ok(result);
    }

    match run_selftests_with_fixups(&root, &profile, ctx.reducer_stats_mut()) {
        Ok(result) => {
            {
                let state = ctx.loop_state_mut();
                state.fixup_pass_count = result.fixup_passes;
                state.changed = result.fixup_passes > 0;
                state.convergence_reason = Some(result.termination.json_value().into());
            }
            let mut reducer_result = reducer_result_from_loop_stats(ctx.reducer_stats());
            reducer_result.set_publication_state(
                ReducerStatus::Success,
                BuildMatrixStatus::Passed,
                ConvergenceStatus::Converged,
            );
            Ok(reducer_result)
        }
        Err(err) => match err.downcast::<SelftestFixedPointFailure>() {
            Ok(fixed_point) => {
                {
                    let diagnostic = classify_selftest_failure(ctx.root(), &fixed_point.failure);
                    let raw_excerpt = raw_diagnostic_excerpt_from_failure(&fixed_point.failure);
                    let non_convergence = if matches!(
                        fixed_point.termination,
                        FixedPointLoopTermination::NoFixerChangedTree
                            | FixedPointLoopTermination::MaxPassCountReached
                    ) {
                        Some(non_convergence_report(
                            ctx.reducer_stats(),
                            vec![diagnostic.clone()],
                            fixed_point.fixup_passes,
                        ))
                    } else {
                        None
                    };
                    log_reducer_stage(ReducerStage::ClassifyDiagnostics);
                    let state = ctx.loop_state_mut();
                    state.fixup_pass_count = fixed_point.fixup_passes;
                    state.changed = fixed_point.fixup_passes > 0;
                    state.latest_diagnostics.push(diagnostic);
                    state.raw_diagnostic_excerpts.push(raw_excerpt);
                    state.convergence_reason = Some(fixed_point.termination.json_value().into());
                    state.non_convergence = non_convergence;
                }
                let mut reducer_result = reducer_result_from_loop_stats(ctx.reducer_stats());
                match fixed_point.termination {
                    FixedPointLoopTermination::SuccessfulSelectedBuildTestMatrix => {
                        reducer_result.set_publication_state(
                            ReducerStatus::Success,
                            BuildMatrixStatus::Passed,
                            ConvergenceStatus::Converged,
                        );
                    }
                    FixedPointLoopTermination::NoFixerChangedTree
                    | FixedPointLoopTermination::MaxPassCountReached => {
                        reducer_result.set_publication_state(
                            ReducerStatus::FailedNonConvergence,
                            BuildMatrixStatus::Failed,
                            ConvergenceStatus::NotConverged,
                        );
                    }
                    FixedPointLoopTermination::UnknownDiagnosticInStrictMode => {
                        reducer_result.set_publication_state(
                            ReducerStatus::FailedUnknownDiagnostic,
                            BuildMatrixStatus::Failed,
                            ConvergenceStatus::NotEvaluated,
                        );
                    }
                    FixedPointLoopTermination::UnsupportedSyntaxInStrictMode => {
                        reducer_result.set_publication_state(
                            ReducerStatus::FailedUnsupportedSyntax,
                            BuildMatrixStatus::Failed,
                            ConvergenceStatus::NotEvaluated,
                        );
                    }
                }
                Ok(reducer_result)
            }
            Err(err) => Err(err),
        },
    }
}

fn reset_loop_state(ctx: &mut ReducerContext) {
    let state = ctx.loop_state_mut();
    state.pass_index = 0;
    state.fixup_pass_count = 0;
    state.changed = false;
    state.latest_diagnostics.clear();
    state.raw_diagnostic_excerpts.clear();
    state.non_convergence = None;
    state.convergence_reason = None;
}

fn strict_unsupported_syntax_in_stats(
    profile: &ProfileConfig,
    reducer_stats: &ReducerStats,
) -> bool {
    profile.reducer.report_unsupported_expressions
        && (!reducer_stats.unsupported_kconfig_expressions.is_empty()
            || !reducer_stats.unsupported_cpp_expressions.is_empty())
}

fn reducer_result_from_loop_stats(stats: &ReducerStats) -> ReducerResult {
    ReducerResult::from_pipeline_artifacts(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        stats.clone(),
    )
}

pub(crate) fn run_selftests_with_fixups(
    root: &KernelSourceRoot,
    profile: &ProfileConfig,
    reducer_stats: &mut ReducerStats,
) -> Result<SelftestFixedPointResult> {
    log_reducer_stage(ReducerStage::RunSelftests);
    let temp_path = root
        .as_path()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("kernel source root path is not valid UTF-8"))?;

    let max_fixup_passes = profile.reducer.max_fixup_passes;
    let mut fixup_passes = 0usize;

    loop {
        match selftest::run_capture(temp_path, &profile.selftests) {
            Ok(result) => {
                log_selftest_result(&result);
                let fixed_point = SelftestFixedPointResult {
                    termination: FixedPointLoopTermination::SuccessfulSelectedBuildTestMatrix,
                    fixup_passes,
                    selftests: result,
                };
                log_fixed_point_termination(&fixed_point);
                return Ok(fixed_point);
            }
            Err(failure) => {
                let classified = classify_selftest_failure(root.as_path(), &failure);
                record_selftest_failure_diagnostic(reducer_stats, &failure, classified.clone());
                if profile.reducer.fail_on_unknown_diagnostics && classified.is_unknown_class() {
                    reducer_stats.skipped_fixups.push(SkippedFixup {
                        fixer_name: None,
                        diagnostic: classified,
                        reason: String::from("unknown diagnostic"),
                    });
                    let fixed_point = SelftestFixedPointFailure {
                        termination: FixedPointLoopTermination::UnknownDiagnosticInStrictMode,
                        fixup_passes,
                        failure,
                    };
                    log_fixed_point_failure(&fixed_point);
                    return Err(anyhow::anyhow!(fixed_point));
                }

                if fixup_passes >= max_fixup_passes {
                    let fixed_point = SelftestFixedPointFailure {
                        termination: FixedPointLoopTermination::MaxPassCountReached,
                        fixup_passes,
                        failure,
                    };
                    log_fixed_point_failure(&fixed_point);
                    return Err(anyhow::anyhow!(fixed_point));
                }

                log_reducer_stage(ReducerStage::ApplyFixups);
                if apply_selftest_fixup(temp_path, profile, reducer_stats, &failure)? {
                    fixup_passes += 1;
                    log::info!(
                        "selftests: applied deterministic fixup pass {}/{}",
                        fixup_passes,
                        max_fixup_passes
                    );
                    continue;
                }

                let fixed_point = stalled_fixup_fixed_point_failure(
                    profile,
                    reducer_stats,
                    fixup_passes,
                    failure,
                );
                log_fixed_point_failure(&fixed_point);
                return Err(anyhow::anyhow!(fixed_point));
            }
        }
    }
}

fn log_reducer_stage(stage: ReducerStage) {
    log::info!("reducer: stage={}", stage.as_str());
}

fn stalled_fixup_fixed_point_failure(
    profile: &ProfileConfig,
    reducer_stats: &ReducerStats,
    fixup_passes: usize,
    failure: selftest::SelfTestFailure,
) -> SelftestFixedPointFailure {
    let termination = if profile.reducer.fail_on_unknown_diagnostics
        && reducer_stats
            .skipped_fixups
            .last()
            .is_some_and(|skipped| skipped.diagnostic.is_unknown_class())
    {
        FixedPointLoopTermination::UnknownDiagnosticInStrictMode
    } else {
        FixedPointLoopTermination::NoFixerChangedTree
    };

    SelftestFixedPointFailure {
        termination,
        fixup_passes,
        failure,
    }
}

fn log_fixed_point_termination(result: &SelftestFixedPointResult) {
    match result.termination {
        FixedPointLoopTermination::SuccessfulSelectedBuildTestMatrix => {
            log::info!(
                "fixed-point loop terminated: selected build/test matrix passed after {} fixup pass(es)",
                result.fixup_passes
            );
        }
        FixedPointLoopTermination::NoFixerChangedTree => {}
        FixedPointLoopTermination::MaxPassCountReached => {}
        FixedPointLoopTermination::UnknownDiagnosticInStrictMode => {}
        FixedPointLoopTermination::UnsupportedSyntaxInStrictMode => {}
    }
}

fn log_fixed_point_failure(result: &SelftestFixedPointFailure) {
    match result.termination {
        FixedPointLoopTermination::NoFixerChangedTree => {
            log::warn!(
                "fixed-point loop terminated: no fixer changed the tree after {} fixup pass(es)",
                result.fixup_passes
            );
        }
        FixedPointLoopTermination::MaxPassCountReached => {
            log::warn!(
                "fixed-point loop terminated: max fixup pass count reached after {} fixup pass(es)",
                result.fixup_passes
            );
        }
        FixedPointLoopTermination::UnknownDiagnosticInStrictMode => {
            log::warn!(
                "fixed-point loop terminated: unknown diagnostic in strict mode after {} fixup pass(es)",
                result.fixup_passes
            );
        }
        FixedPointLoopTermination::UnsupportedSyntaxInStrictMode => {
            log::warn!("fixed-point loop terminated: unsupported syntax in strict mode");
        }
        FixedPointLoopTermination::SuccessfulSelectedBuildTestMatrix => {}
    }
}

fn log_selftest_result(result: &SelfTestResult) {
    if result.enabled {
        log::info!(
            "selftests: passed {} built-in checks and {} custom command(s)",
            result.built_in_checks,
            result.commands_run
        );
    } else {
        log::info!("selftests: disabled by profile");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::diagnostics::ClassifiedDiagnostic;
    use std::path::PathBuf;
    use std::time::Duration;

    fn create_minimal_tree(root: &std::path::Path) {
        for dir in &[
            "arch", "drivers", "fs", "include", "kernel", "mm", "net", "scripts",
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
            std::fs::write(root.join(dir).join(".keep"), "").unwrap();
        }
        std::fs::write(root.join("Makefile"), "# test\n").unwrap();
        std::fs::write(root.join("Kconfig"), "# test\n").unwrap();
    }

    #[test]
    fn test_fixed_point_loop_terminates_on_successful_selected_build_test_matrix() {
        let tmp = tempfile::tempdir().unwrap();
        create_minimal_tree(tmp.path());

        let marker = tmp.path().join(".kslim-selftest-ok");
        let mut profile = config::default_profile_config("v1.0");
        profile.selftests = config::SelfTestConfig {
            enabled: true,
            check_kconfig_sources: true,
            check_makefiles: true,
            kernel_builds: Vec::new(),
            commands: vec![format!("printf ok > {}", marker.display())],
        };
        let mut reducer_stats = ReducerStats::default();

        let result = run_selftests_with_fixups(
            &KernelSourceRoot::new(tmp.path()).unwrap(),
            &profile,
            &mut reducer_stats,
        )
        .unwrap();

        assert_eq!(
            result.termination,
            FixedPointLoopTermination::SuccessfulSelectedBuildTestMatrix
        );
        assert_eq!(result.fixup_passes, 0);
        assert!(result.selftests.enabled);
        assert_eq!(result.selftests.built_in_checks, 2);
        assert_eq!(result.selftests.kernel_builds_run, 0);
        assert_eq!(result.selftests.commands_run, 1);
        assert_eq!(std::fs::read_to_string(marker).unwrap(), "ok");
        assert!(reducer_stats.applied_fixups.is_empty());
        assert!(reducer_stats.skipped_fixups.is_empty());
    }

    #[test]
    fn test_run_fixed_point_loop_returns_publish_blocking_result_for_unknown_diagnostic() {
        let tmp = tempfile::tempdir().unwrap();
        create_minimal_tree(tmp.path());

        let mut profile = config::default_profile_config("v1.0");
        profile.selftests = config::SelfTestConfig {
            enabled: true,
            check_kconfig_sources: true,
            check_makefiles: true,
            kernel_builds: Vec::new(),
            commands: vec![String::from(
                "printf '%s\n' 'mystery compiler failure' >&2; exit 2",
            )],
        };
        let mut ctx = ReducerContext::with_fixed_point(
            tmp.path(),
            profile,
            ReducerStats {
                ran: true,
                ..ReducerStats::default()
            },
        );

        let result = run_fixed_point_loop(&mut ctx).unwrap();

        assert_eq!(result.status, ReducerStatus::FailedUnknownDiagnostic);
        assert!(!result.publishable);
        assert_eq!(result.final_build_status, BuildMatrixStatus::Failed);
        assert_eq!(result.convergence, ConvergenceStatus::NotEvaluated);
        assert_eq!(ctx.loop_state().pass_index, 0);
        assert_eq!(ctx.loop_state().fixup_pass_count, 0);
        assert!(!ctx.loop_state().changed);
        assert_eq!(
            ctx.loop_state().convergence_reason.as_deref(),
            Some(FixedPointLoopTermination::UnknownDiagnosticInStrictMode.json_value())
        );
        assert!(matches!(
            ctx.loop_state().latest_diagnostics.as_slice(),
            [crate::diagnostics::ClassifiedDiagnostic::Unknown]
        ));
        assert_eq!(ctx.loop_state().raw_diagnostic_excerpts.len(), 1);
        let raw = &ctx.loop_state().raw_diagnostic_excerpts[0];
        assert!(raw.command_context.contains("selftest command"));
        assert!(raw.command_context.contains("printf"));
        assert_eq!(raw.build_target, None);
        assert!(raw.raw_excerpt.contains("mystery compiler failure"));
        assert_eq!(ctx.reducer_stats().skipped_fixups.len(), 1);
    }

    #[test]
    fn test_unknown_diagnostic_excerpt_includes_command_context_and_build_target() {
        let failure = selftest::SelfTestFailure::Command {
            details: selftest::CapturedCommandFailure {
                command: String::from("make olddefconfig"),
                target: Some(String::from("vmlinux")),
                arch: Some(String::from("x86_64")),
                config: Some(String::from("defconfig")),
                stdout: String::from("stdout line\n"),
                stderr: String::from("raw unknown stderr\n"),
                exit_status: Some(2),
                elapsed: Duration::from_millis(10),
            },
        };

        let excerpt = raw_diagnostic_excerpt_from_failure(&failure);
        let rendered = render_raw_diagnostic_excerpt(&excerpt);

        assert_eq!(
            excerpt.command_context,
            "selftest command: make olddefconfig"
        );
        assert_eq!(excerpt.build_target.as_deref(), Some("vmlinux"));
        assert!(excerpt.raw_excerpt.contains("stdout line"));
        assert!(excerpt.raw_excerpt.contains("raw unknown stderr"));
        assert!(excerpt.raw_excerpt.contains("arch: x86_64"));
        assert!(excerpt.raw_excerpt.contains("config: defconfig"));
        assert!(rendered.contains("command context: selftest command: make olddefconfig"));
        assert!(rendered.contains("build target: vmlinux"));
        assert!(rendered.contains("raw diagnostic excerpt:"));
    }

    #[test]
    fn test_run_fixed_point_loop_marks_unsupported_syntax_non_publishable_before_selftests() {
        let tmp = tempfile::tempdir().unwrap();
        create_minimal_tree(tmp.path());

        let mut profile = config::default_profile_config("v1.0");
        profile.selftests = config::SelfTestConfig {
            enabled: true,
            check_kconfig_sources: true,
            check_makefiles: true,
            kernel_builds: Vec::new(),
            commands: vec![String::from("exit 0")],
        };
        let mut ctx = ReducerContext::with_fixed_point(
            tmp.path(),
            profile,
            ReducerStats {
                ran: true,
                unsupported_kconfig_expressions: vec![
                    crate::kconfig::UnsupportedKconfigExpression {
                        file: PathBuf::from("Kconfig"),
                        line: 1,
                        directive: String::from("if"),
                        expression: String::from("REMOVED + LIVE"),
                        reason: String::from("unsupported expression"),
                    },
                ],
                ..ReducerStats::default()
            },
        );

        let result = run_fixed_point_loop(&mut ctx).unwrap();

        assert_eq!(result.status, ReducerStatus::FailedUnsupportedSyntax);
        assert!(!result.publishable);
        assert_eq!(result.final_build_status, BuildMatrixStatus::NotRun);
        assert_eq!(result.convergence, ConvergenceStatus::NotEvaluated);
        assert_eq!(
            ctx.loop_state().convergence_reason.as_deref(),
            Some(FixedPointLoopTermination::UnsupportedSyntaxInStrictMode.json_value())
        );
        assert!(ctx.loop_state().latest_diagnostics.is_empty());
    }

    #[test]
    fn test_run_fixed_point_loop_marks_non_convergence_non_publishable() {
        let tmp = tempfile::tempdir().unwrap();
        create_minimal_tree(tmp.path());
        std::fs::create_dir_all(tmp.path().join("drivers/live")).unwrap();
        std::fs::write(
            tmp.path().join("drivers/live/helper.c"),
            "#include \"missing/private.h\"\nint helper;\n",
        )
        .unwrap();

        let mut profile = config::default_profile_config("v1.0");
        profile.selftests = config::SelfTestConfig {
            enabled: true,
            check_kconfig_sources: true,
            check_makefiles: true,
            kernel_builds: Vec::new(),
            commands: vec![String::from(
                "printf '%s\n' 'drivers/live/helper.c:1:10: fatal error: missing/private.h: No such file or directory' >&2; exit 2",
            )],
        };
        let mut ctx = ReducerContext::with_fixed_point(
            tmp.path(),
            profile,
            ReducerStats {
                ran: true,
                ..ReducerStats::default()
            },
        );

        let result = run_fixed_point_loop(&mut ctx).unwrap();

        assert_eq!(result.status, ReducerStatus::FailedNonConvergence);
        assert!(!result.publishable);
        assert_eq!(result.final_build_status, BuildMatrixStatus::Failed);
        assert_eq!(result.convergence, ConvergenceStatus::NotConverged);
        assert_eq!(ctx.loop_state().fixup_pass_count, 0);
        assert!(!ctx.loop_state().latest_diagnostics.is_empty());
        assert_eq!(ctx.reducer_stats().skipped_fixups.len(), 1);
        let non_convergence = ctx.loop_state().non_convergence.as_ref().unwrap();
        assert_eq!(non_convergence.pass_count, 0);
        assert!(!non_convergence.publishable);
        assert!(matches!(
            non_convergence.remaining_diagnostics.as_slice(),
            [crate::diagnostics::ClassifiedDiagnostic::MissingHeader { .. }]
        ));
        assert_eq!(non_convergence.fixers_skipped.len(), 1);
        assert!(
            non_convergence.fixers_skipped[0].contains("missing-header")
                || non_convergence.fixers_skipped[0].contains("deterministic fixup")
        );
    }

    #[test]
    fn test_run_fixed_point_loop_stops_when_max_pass_count_reached() {
        let tmp = tempfile::tempdir().unwrap();
        create_minimal_tree(tmp.path());
        std::fs::create_dir_all(tmp.path().join("drivers/gpu/drm/amd/amdgpu")).unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/gpu/drm")).unwrap();
        std::fs::write(
            tmp.path().join("drivers/gpu/drm/helper.c"),
            "#include <amd/amdgpu/amdgpu_missing.h>\nint helper;\n",
        )
        .unwrap();

        let mut profile = config::default_profile_config("v1.0");
        profile.reducer.max_fixup_passes = 1;
        profile.selftests = config::SelfTestConfig {
            enabled: true,
            check_kconfig_sources: true,
            check_makefiles: true,
            kernel_builds: Vec::new(),
            commands: vec![String::from(
                "printf '%s\n' 'drivers/gpu/drm/helper.c:1:10: fatal error: amd/amdgpu/amdgpu_missing.h: No such file or directory' >&2; exit 2",
            )],
        };
        let mut ctx = ReducerContext::with_fixed_point(
            tmp.path(),
            profile,
            ReducerStats {
                ran: true,
                removal: crate::prune::RemovalAccounting {
                    removed_files: Vec::new(),
                    removed_dirs: vec![PathBuf::from("drivers/gpu/drm/amd/amdgpu")],
                    removed_config_symbols: Vec::new(),
                    empty_parents_cleaned: Vec::new(),
                    missing_paths: Vec::new(),
                },
                ..ReducerStats::default()
            },
        );

        let result = run_fixed_point_loop(&mut ctx).unwrap();

        assert_eq!(result.status, ReducerStatus::FailedNonConvergence);
        assert!(!result.publishable);
        assert_eq!(result.convergence, ConvergenceStatus::NotConverged);
        assert_eq!(ctx.loop_state().fixup_pass_count, 1);
        assert!(ctx.loop_state().changed);
        assert_eq!(
            ctx.loop_state().convergence_reason.as_deref(),
            Some(FixedPointLoopTermination::MaxPassCountReached.json_value())
        );
        assert_eq!(ctx.reducer_stats().applied_fixups.len(), 1);
        let non_convergence = ctx.loop_state().non_convergence.as_ref().unwrap();
        assert_eq!(non_convergence.pass_count, 1);
        assert!(non_convergence.remaining_diagnostics.len() == 1);
        assert!(non_convergence.fixers_skipped.is_empty());
        assert!(!non_convergence.publishable);
    }

    #[test]
    fn test_run_fixed_point_loop_returns_success_when_selected_build_test_matrix_passes() {
        let tmp = tempfile::tempdir().unwrap();
        create_minimal_tree(tmp.path());

        let mut profile = config::default_profile_config("v1.0");
        profile.selftests = config::SelfTestConfig {
            enabled: true,
            check_kconfig_sources: true,
            check_makefiles: true,
            kernel_builds: Vec::new(),
            commands: vec![String::from("exit 0")],
        };
        let mut ctx =
            ReducerContext::with_fixed_point(tmp.path(), profile, ReducerStats::default());

        let result = run_fixed_point_loop(&mut ctx).unwrap();

        assert_eq!(result.status, ReducerStatus::Success);
        assert!(result.publishable);
        assert_eq!(result.final_build_status, BuildMatrixStatus::Passed);
        assert_eq!(result.convergence, ConvergenceStatus::Converged);
        assert_eq!(ctx.loop_state().fixup_pass_count, 0);
        assert_eq!(
            ctx.loop_state().convergence_reason.as_deref(),
            Some(FixedPointLoopTermination::SuccessfulSelectedBuildTestMatrix.json_value())
        );
    }

    #[test]
    fn test_fixed_point_loop_terminates_when_no_fixer_changed_tree() {
        let tmp = tempfile::tempdir().unwrap();
        create_minimal_tree(tmp.path());
        std::fs::create_dir_all(tmp.path().join("drivers/live")).unwrap();
        std::fs::write(
            tmp.path().join("drivers/live/helper.c"),
            "#include \"missing/private.h\"\nint helper;\n",
        )
        .unwrap();

        let mut profile = config::default_profile_config("v1.0");
        profile.selftests = config::SelfTestConfig {
            enabled: true,
            check_kconfig_sources: true,
            check_makefiles: true,
            kernel_builds: Vec::new(),
            commands: vec![String::from(
                "printf '%s\n' 'drivers/live/helper.c:1:10: fatal error: missing/private.h: No such file or directory' >&2; exit 2",
            )],
        };
        let mut reducer_stats = ReducerStats {
            ran: true,
            ..ReducerStats::default()
        };

        let err = run_selftests_with_fixups(
            &KernelSourceRoot::new(tmp.path()).unwrap(),
            &profile,
            &mut reducer_stats,
        )
        .unwrap_err();
        let fixed_point = err
            .downcast_ref::<SelftestFixedPointFailure>()
            .expect("expected fixed-point termination error");

        assert_eq!(
            fixed_point.termination,
            FixedPointLoopTermination::NoFixerChangedTree
        );
        assert_eq!(fixed_point.fixup_passes, 0);
        assert!(err.to_string().contains("no fixer changed the tree"));
        assert!(err.to_string().contains("missing/private.h"));
        assert!(reducer_stats.edits.is_empty());
        assert!(reducer_stats.applied_fixups.is_empty());
        assert_eq!(reducer_stats.skipped_fixups.len(), 1);
    }

    #[test]
    fn test_fixed_point_loop_terminates_on_unknown_diagnostic_in_strict_mode() {
        let tmp = tempfile::tempdir().unwrap();
        create_minimal_tree(tmp.path());

        let mut profile = config::default_profile_config("v1.0");
        assert!(profile.reducer.fail_on_unknown_diagnostics);
        profile.selftests = config::SelfTestConfig {
            enabled: true,
            check_kconfig_sources: true,
            check_makefiles: true,
            kernel_builds: Vec::new(),
            commands: vec![String::from(
                "printf '%s\n' 'mystery compiler failure' >&2; exit 2",
            )],
        };
        let mut reducer_stats = ReducerStats {
            ran: true,
            ..ReducerStats::default()
        };

        let err = run_selftests_with_fixups(
            &KernelSourceRoot::new(tmp.path()).unwrap(),
            &profile,
            &mut reducer_stats,
        )
        .unwrap_err();
        let fixed_point = err
            .downcast_ref::<SelftestFixedPointFailure>()
            .expect("expected fixed-point termination error");

        assert_eq!(
            fixed_point.termination,
            FixedPointLoopTermination::UnknownDiagnosticInStrictMode
        );
        assert_eq!(fixed_point.fixup_passes, 0);
        assert!(err
            .to_string()
            .contains("unknown diagnostic in strict mode"));
        assert!(err.to_string().contains("mystery compiler failure"));
        assert!(reducer_stats.edits.is_empty());
        assert!(reducer_stats.applied_fixups.is_empty());
        assert_eq!(reducer_stats.skipped_fixups.len(), 1);
        assert_eq!(reducer_stats.skipped_fixups[0].reason, "unknown diagnostic");
        assert!(matches!(
            reducer_stats.classified_diagnostics.as_slice(),
            [ClassifiedDiagnostic::Unknown]
        ));
        assert_eq!(reducer_stats.raw_diagnostic_excerpts.len(), 1);
        assert!(reducer_stats.raw_diagnostic_excerpts[0]
            .raw_excerpt
            .contains("mystery compiler failure"));
    }

    #[test]
    fn test_fixed_point_loop_treats_unknown_diagnostic_class_as_strict_unknown() {
        let tmp = tempfile::tempdir().unwrap();
        create_minimal_tree(tmp.path());

        let mut profile = config::default_profile_config("v1.0");
        assert!(profile.reducer.fail_on_unknown_diagnostics);
        profile.selftests = config::SelfTestConfig {
            enabled: true,
            check_kconfig_sources: true,
            check_makefiles: true,
            kernel_builds: Vec::new(),
            commands: vec![String::from(
                "printf '%s\n' \"drivers/gpu/drm/helper.c:7:2: error: 'amdgpu_magic' undeclared (first use in this function)\" >&2; exit 2",
            )],
        };
        let mut reducer_stats = ReducerStats {
            ran: true,
            ..ReducerStats::default()
        };

        let err = run_selftests_with_fixups(
            &KernelSourceRoot::new(tmp.path()).unwrap(),
            &profile,
            &mut reducer_stats,
        )
        .unwrap_err();
        let fixed_point = err
            .downcast_ref::<SelftestFixedPointFailure>()
            .expect("expected fixed-point termination error");

        assert_eq!(
            fixed_point.termination,
            FixedPointLoopTermination::UnknownDiagnosticInStrictMode
        );
        assert_eq!(fixed_point.fixup_passes, 0);
        assert_eq!(reducer_stats.skipped_fixups.len(), 1);
        assert_eq!(reducer_stats.skipped_fixups[0].reason, "unknown diagnostic");
        assert!(matches!(
            reducer_stats.skipped_fixups[0].diagnostic,
            ClassifiedDiagnostic::UndeclaredIdentifier { .. }
        ));
    }

    #[test]
    fn test_unknown_diagnostic_in_strict_mode_stops_before_max_pass_check() {
        let tmp = tempfile::tempdir().unwrap();
        create_minimal_tree(tmp.path());

        let mut profile = config::default_profile_config("v1.0");
        profile.reducer.max_fixup_passes = 0;
        assert!(profile.reducer.fail_on_unknown_diagnostics);
        profile.selftests = config::SelfTestConfig {
            enabled: true,
            check_kconfig_sources: true,
            check_makefiles: true,
            kernel_builds: Vec::new(),
            commands: vec![String::from(
                "printf '%s\n' 'mystery compiler failure' >&2; exit 2",
            )],
        };
        let mut reducer_stats = ReducerStats {
            ran: true,
            ..ReducerStats::default()
        };

        let err = run_selftests_with_fixups(
            &KernelSourceRoot::new(tmp.path()).unwrap(),
            &profile,
            &mut reducer_stats,
        )
        .unwrap_err();
        let fixed_point = err
            .downcast_ref::<SelftestFixedPointFailure>()
            .expect("expected fixed-point termination error");

        assert_eq!(
            fixed_point.termination,
            FixedPointLoopTermination::UnknownDiagnosticInStrictMode
        );
        assert_eq!(fixed_point.fixup_passes, 0);
        assert!(!err.to_string().contains("max fixup pass count reached"));
        assert!(err
            .to_string()
            .contains("command context: selftest command"));
        assert!(err.to_string().contains("raw diagnostic excerpt"));
        assert!(err.to_string().contains("mystery compiler failure"));
    }

    #[test]
    fn test_fixed_point_loop_terminates_when_max_pass_count_reached() {
        let tmp = tempfile::tempdir().unwrap();
        create_minimal_tree(tmp.path());
        std::fs::create_dir_all(tmp.path().join("drivers/gpu/drm/amd/amdgpu")).unwrap();
        std::fs::create_dir_all(tmp.path().join("drivers/gpu/drm")).unwrap();
        std::fs::write(
            tmp.path().join("drivers/gpu/drm/helper.c"),
            "#include <amd/amdgpu/amdgpu_missing.h>\nint helper;\n",
        )
        .unwrap();

        let mut profile = config::default_profile_config("v1.0");
        profile.reducer.max_fixup_passes = 1;
        profile.selftests = config::SelfTestConfig {
            enabled: true,
            check_kconfig_sources: true,
            check_makefiles: true,
            kernel_builds: Vec::new(),
            commands: vec![String::from(
                "printf '%s\n' 'drivers/gpu/drm/helper.c:1:10: fatal error: amd/amdgpu/amdgpu_missing.h: No such file or directory' >&2; exit 2",
            )],
        };
        let mut reducer_stats = ReducerStats {
            ran: true,
            removal: crate::prune::RemovalAccounting {
                removed_files: Vec::new(),
                removed_dirs: vec![PathBuf::from("drivers/gpu/drm/amd/amdgpu")],
                removed_config_symbols: Vec::new(),
                empty_parents_cleaned: Vec::new(),
                missing_paths: Vec::new(),
            },
            ..ReducerStats::default()
        };

        let err = run_selftests_with_fixups(
            &KernelSourceRoot::new(tmp.path()).unwrap(),
            &profile,
            &mut reducer_stats,
        )
        .unwrap_err();
        let fixed_point = err
            .downcast_ref::<SelftestFixedPointFailure>()
            .expect("expected fixed-point termination error");

        assert_eq!(
            fixed_point.termination,
            FixedPointLoopTermination::MaxPassCountReached
        );
        assert_eq!(fixed_point.fixup_passes, 1);
        assert!(err.to_string().contains("max fixup pass count reached"));
        assert!(err.to_string().contains("amdgpu_missing.h"));
        assert_eq!(reducer_stats.applied_fixups.len(), 1);
        assert!(reducer_stats.skipped_fixups.is_empty());
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("drivers/gpu/drm/helper.c")).unwrap(),
            "int helper;\n"
        );
    }
}
