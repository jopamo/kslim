//! Kbuild rewrite report models.
//!
//! This module owns the report data shapes emitted by Kbuild rewrites and
//! consumed by prune/reducer reporting. Rewrite code fills these models; report
//! renderers turn them into user-visible output elsewhere.

use std::path::PathBuf;

use crate::edit_reason::EditRecord;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct KbuildSkippedLine {
    pub file: PathBuf,
    pub line: usize,
    pub assignment_lhs: String,
    pub reason: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct KbuildRewriteReport {
    pub removed_refs: usize,
    pub edits: Vec<EditRecord>,
    pub skipped_ambiguous_lines: Vec<KbuildSkippedLine>,
}
