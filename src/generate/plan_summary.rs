//! Compatibility facade for generate plan summaries.
//!
//! Immutable plan summary ownership lives in `crate::plan`; this module
//! preserves existing generate-local call sites while migration proceeds.

pub(crate) use crate::plan::{resolve_plan_summary, GeneratePlanSummary};
