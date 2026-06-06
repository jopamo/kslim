use anyhow::{Context, Result};

use crate::model::ReportPath;

use super::super::plan::GeneratePlan;
use super::super::GenerateStage;
use super::metadata::write_candidate_failure_attempt_metadata;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CandidateBuildStageFailure {
    pub(super) stage: GenerateStage,
    pub(super) partial_reports: Vec<ReportPath>,
}

impl CandidateBuildStageFailure {
    pub(super) fn new(stage: GenerateStage) -> Self {
        Self {
            stage,
            partial_reports: Vec::new(),
        }
    }

    pub(super) fn with_partial_reports(
        stage: GenerateStage,
        mut partial_reports: Vec<ReportPath>,
    ) -> Self {
        partial_reports.sort();
        partial_reports.dedup();
        Self {
            stage,
            partial_reports,
        }
    }
}

impl std::fmt::Display for CandidateBuildStageFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "candidate build failed during {} stage",
            self.stage.as_str()
        )
    }
}

impl std::error::Error for CandidateBuildStageFailure {}

pub(super) fn record_candidate_stage<T>(
    stage: GenerateStage,
    operation: impl FnOnce() -> Result<T>,
) -> Result<T> {
    operation().with_context(|| CandidateBuildStageFailure::new(stage))
}

pub(super) fn record_candidate_failure_attempt(
    plan: &GeneratePlan,
    err: anyhow::Error,
) -> anyhow::Error {
    let stage = candidate_error_stage(&err);
    let existing_partial_reports = candidate_error_partial_reports(&err);
    let message = format!("{err:#}");

    match write_candidate_failure_attempt_metadata(plan, stage, &message, &existing_partial_reports)
    {
        Ok(partial_reports) => err.context(CandidateBuildStageFailure::with_partial_reports(
            stage,
            partial_reports,
        )),
        Err(attempt_err) => err.context(format!(
            "failed to write candidate attempt metadata: {attempt_err:#}"
        )),
    }
}

fn candidate_error_stage(err: &anyhow::Error) -> GenerateStage {
    err.downcast_ref::<CandidateBuildStageFailure>()
        .map(|failure| failure.stage)
        .unwrap_or(GenerateStage::Materialize)
}

fn candidate_error_partial_reports(err: &anyhow::Error) -> Vec<ReportPath> {
    err.downcast_ref::<CandidateBuildStageFailure>()
        .map(|failure| failure.partial_reports.clone())
        .unwrap_or_default()
}
