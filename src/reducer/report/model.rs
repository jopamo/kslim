use super::super::engine::{FixedPointLoopTermination, SelftestFixedPointFailure};

#[derive(Debug, Clone)]
pub(crate) struct ReducerFailureReport {
    pub(crate) termination: FixedPointLoopTermination,
    pub(crate) fixup_passes: Option<usize>,
    pub(crate) failure: String,
}

impl ReducerFailureReport {
    pub(crate) fn from_fixed_point(failure: &SelftestFixedPointFailure) -> Self {
        Self {
            termination: failure.termination,
            fixup_passes: Some(failure.fixup_passes),
            failure: failure.failure.to_string(),
        }
    }

    pub(crate) fn unsupported_syntax(failure: &str) -> Self {
        Self {
            termination: FixedPointLoopTermination::UnsupportedSyntaxInStrictMode,
            fixup_passes: None,
            failure: failure.to_string(),
        }
    }
}

pub(crate) const REDUCER_REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ReducerReportArtifactNames<'a> {
    pub(crate) markdown: &'a str,
    pub(crate) summary_json: &'a str,
    pub(crate) diagnostics_json: &'a str,
    pub(crate) edit_summary_json: &'a str,
    pub(crate) kconfig_solver_report_json: &'a str,
    pub(crate) kconfig_rewrite_report_json: &'a str,
    pub(crate) skipped_sites_json: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderedReducerReportArtifacts {
    pub(crate) markdown: String,
    pub(crate) summary_json: String,
    pub(crate) diagnostics_json: String,
    pub(crate) edit_summary_json: String,
    pub(crate) kconfig_solver_report_json: String,
    pub(crate) kconfig_rewrite_report_json: String,
    pub(crate) skipped_sites_json: Option<String>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct KbuildEditReport {
    pub(super) removed_directory_refs: usize,
    pub(super) removed_object_refs: usize,
    pub(super) cleaned_composite_objects: usize,
    pub(super) removed_stale_include_paths: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnknownDiagnosticPolicy {
    pub(crate) reject_unknown_diagnostics: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnsupportedSyntaxPolicy {
    pub(crate) reject_unsupported_syntax: bool,
}
