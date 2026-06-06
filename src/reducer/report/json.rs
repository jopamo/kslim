//! Reducer JSON report facade.

mod canonical;
mod escaping;
mod schema;
mod serializer;
#[cfg(test)]
mod tests;

pub(crate) use serializer::{
    render_edit_records_json, render_kconfig_rewrite_report_json, render_kconfig_solver_report_json,
    render_reducer_diagnostics_json, render_reducer_edit_summary_json, render_reducer_report_json,
    render_reducer_skipped_sites_json,
};
