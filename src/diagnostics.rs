//! Build and compiler diagnostic classification for reducer fixups.
//!
//! The current classifier is intentionally narrow. It recognizes only
//! deterministic failure shapes that kslim can prove back to reducer state.

mod classifier;
mod command_capture;
mod model;
mod renderer;

pub use classifier::classify_selftest_failure;
pub use model::ClassifiedDiagnostic;
pub(crate) use renderer::{render_classified_diagnostic_json, render_classified_diagnostic_md};

#[cfg(test)]
mod tests;
