//! Unit tests for reducer facade entrypoints and result behavior.

use super::*;
use crate::config::{default_profile_config, ReducerConfig, SlimConfig};
use crate::diagnostics::ClassifiedDiagnostic;
use crate::edit_reason::{DiagnosticClass, EditProofSource, EditReason, EditRecord, LineRange};
use crate::fixups::{AppliedFixup, FixupProof, SkippedFixup};
use crate::paths::KernelSourceRoot;
use crate::prune::RemovalAccounting;
use crate::removal_manifest::RemovalManifest;
use crate::selftest::{CapturedCommandFailure, SelfTestFailure};
use crate::tree_index::TreeIndex;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn kernel_root(path: &Path) -> KernelSourceRoot {
    KernelSourceRoot::new(path).unwrap()
}

fn assert_zero_edit_reducer_rerun(result: &ReducerResult) {
    assert!(
        result.stats.edits.is_empty(),
        "already-reduced rerun must report zero edits, got {:?}",
        result.stats.edits
    );
    assert_eq!(result.stats.files_removed, 0);
    assert_eq!(result.stats.dirs_removed, 0);
    assert_eq!(result.stats.configs_disabled, 0);
    assert_eq!(result.stats.defaults_overridden, 0);
    assert_eq!(result.stats.kconfig_refs_removed, 0);
    assert_eq!(result.stats.makefile_refs_removed, 0);
    assert_eq!(result.stats.cpp_report.branches_folded, 0);
    assert_eq!(result.stats.include_report.removed_include_lines, 0);
}

#[path = "tests_cpp_include.rs"]
mod cpp_include;
#[path = "tests_fixups.rs"]
mod fixups;
#[path = "tests_pipeline.rs"]
mod pipeline;
#[path = "tests_result_serialization.rs"]
mod result_serialization;
#[path = "tests_syntax.rs"]
mod syntax;
