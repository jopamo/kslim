//! Reducer attempt state.
//!
//! A reducer attempt is non-authoritative: it may describe the reducer stage,
//! terminal reducer status, and partial reports written under attempt metadata,
//! but it is not published metadata and cannot update lockfile truth.

use anyhow::Result;

use crate::model::ReportPath;
use crate::paths::AttemptMetadataDir;

use super::{ReducerStage, ReducerStatus};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReducerAttemptState {
    stage: ReducerStage,
    status: Option<ReducerStatus>,
    attempt_metadata_dir: AttemptMetadataDir,
    partial_reports: Vec<ReportPath>,
}

#[allow(dead_code)]
impl ReducerAttemptState {
    pub(crate) fn new(
        stage: ReducerStage,
        status: Option<ReducerStatus>,
        attempt_metadata_dir: AttemptMetadataDir,
        partial_reports: Vec<ReportPath>,
    ) -> Result<Self> {
        let mut partial_reports = partial_reports;
        for report in &partial_reports {
            if !report.as_path().starts_with(attempt_metadata_dir.as_path()) {
                anyhow::bail!(
                    "reducer attempt report outside attempt metadata: {}",
                    report.as_path().display()
                );
            }
        }
        partial_reports.sort();
        partial_reports.dedup();
        Ok(Self {
            stage,
            status,
            attempt_metadata_dir,
            partial_reports,
        })
    }

    pub(crate) fn in_progress(
        stage: ReducerStage,
        attempt_metadata_dir: AttemptMetadataDir,
        partial_reports: Vec<ReportPath>,
    ) -> Result<Self> {
        Self::new(stage, None, attempt_metadata_dir, partial_reports)
    }

    pub(crate) fn completed(
        stage: ReducerStage,
        status: ReducerStatus,
        attempt_metadata_dir: AttemptMetadataDir,
        partial_reports: Vec<ReportPath>,
    ) -> Result<Self> {
        Self::new(stage, Some(status), attempt_metadata_dir, partial_reports)
    }

    pub(crate) fn stage(&self) -> ReducerStage {
        self.stage
    }

    pub(crate) fn status(&self) -> Option<ReducerStatus> {
        self.status
    }

    pub(crate) fn attempt_metadata_dir(&self) -> &AttemptMetadataDir {
        &self.attempt_metadata_dir
    }

    pub(crate) fn partial_reports(&self) -> &[ReportPath] {
        &self.partial_reports
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.status.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reducer_attempt_state_captures_only_non_authoritative_attempt_facts() {
        let attempt_dir = AttemptMetadataDir::new("/tmp/project/.kslim/attempt").unwrap();
        let partial_reports = vec![
            ReportPath::new("/tmp/project/.kslim/attempt/reducer-report.json").unwrap(),
            ReportPath::new("/tmp/project/.kslim/attempt/diagnostics.json").unwrap(),
            ReportPath::new("/tmp/project/.kslim/attempt/reducer-report.json").unwrap(),
        ];
        let sorted_partial_reports = vec![
            ReportPath::new("/tmp/project/.kslim/attempt/diagnostics.json").unwrap(),
            ReportPath::new("/tmp/project/.kslim/attempt/reducer-report.json").unwrap(),
        ];

        let attempt = ReducerAttemptState::completed(
            ReducerStage::ClassifyDiagnostics,
            ReducerStatus::FailedUnknownDiagnostic,
            attempt_dir.clone(),
            partial_reports,
        )
        .unwrap();

        assert_eq!(attempt.stage(), ReducerStage::ClassifyDiagnostics);
        assert_eq!(
            attempt.status(),
            Some(ReducerStatus::FailedUnknownDiagnostic)
        );
        assert_eq!(attempt.attempt_metadata_dir(), &attempt_dir);
        assert_eq!(attempt.partial_reports(), sorted_partial_reports.as_slice());
        assert!(attempt.is_complete());
    }

    #[test]
    fn reducer_attempt_state_rejects_reports_outside_attempt_metadata() {
        let attempt_dir = AttemptMetadataDir::new("/tmp/project/.kslim/attempt").unwrap();

        let err = ReducerAttemptState::in_progress(
            ReducerStage::FoldPreprocessor,
            attempt_dir,
            vec![ReportPath::new("/tmp/project/.kslim/reducer-report.json").unwrap()],
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("reducer attempt report outside attempt metadata"));
    }

    #[test]
    fn reducer_attempt_state_can_represent_in_progress_attempts() {
        let attempt = ReducerAttemptState::in_progress(
            ReducerStage::RewriteKconfig,
            AttemptMetadataDir::new("/tmp/project/.kslim/attempt").unwrap(),
            Vec::new(),
        )
        .unwrap();

        assert_eq!(attempt.stage(), ReducerStage::RewriteKconfig);
        assert_eq!(attempt.status(), None);
        assert!(!attempt.is_complete());
    }
}
