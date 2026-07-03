//! Facade for the manifest-driven reducer pipeline.
//!
//! Reducer orchestration, pass ordering, fixed-point retry behavior, and
//! reducer reports live under `src/reducer/*`. This file keeps the public
//! reducer API stable for the rest of the program.

mod actions;
mod context;
mod diagnostics;
mod engine;
mod pipeline;
mod report;
mod result;
mod stage;
mod state;

pub use actions::apply_selftest_fixup;
#[cfg(test)]
pub(crate) use actions::validate_reducer_edit_provenance;
#[allow(unused_imports)]
pub use engine::run_fixed_point_loop;
pub(crate) use engine::{run_selftests_with_fixups, SelftestFixedPointFailure};
pub(crate) use pipeline::{run_reducer_after_declared_prune, run_reducer_from_manifest};
#[allow(unused_imports)]
pub use pipeline::{
    run, run_reducer, run_reducer_for_profile, run_reducer_with_abi_policy,
    run_reducer_with_abi_policy_and_preservation,
    run_reducer_with_policies_and_preservation,
};
pub use report::ensure_supported_fallout;
#[cfg(test)]
pub(crate) use report::{
    render_edit_records_json, render_reducer_diagnostics_json, render_unsupported_expression_report,
};
pub(crate) use report::{
    render_reducer_stats_report_artifacts_with_manifest, ReducerFailureReport,
    ReducerReportArtifactNames,
};
#[allow(unused_imports)]
pub use result::{
    BuildMatrixStatus, ConvergenceStatus, DiagnosticSummary, EditSummary, FixupApplication,
    ReducerPassReport, ReducerResult, ReducerStats, ReducerStatus, SkippedSite,
};
#[allow(unused_imports)]
pub(crate) use stage::ReducerStage;
#[allow(unused_imports)]
pub(crate) use state::ReducerAttemptState;
#[cfg(test)]
mod tests;
