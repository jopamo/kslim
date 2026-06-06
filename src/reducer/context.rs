use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::config::ProfileConfig;
use crate::diagnostics::ClassifiedDiagnostic;
use crate::edit_reason::{validate_edit_records, EditRecord};

use super::diagnostics::{NonConvergenceReport, RawDiagnosticExcerpt};
use super::{ReducerStats, SkippedSite};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ReducerContext {
    root: PathBuf,
    fixed_point_profile: Option<ProfileConfig>,
    reducer_stats: ReducerStats,
    loop_state: ReducerLoopState,
}

#[allow(dead_code)]
impl ReducerContext {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            fixed_point_profile: None,
            reducer_stats: ReducerStats::default(),
            loop_state: ReducerLoopState::default(),
        }
    }

    pub fn with_fixed_point(
        root: impl Into<PathBuf>,
        profile: ProfileConfig,
        reducer_stats: ReducerStats,
    ) -> Self {
        Self {
            root: root.into(),
            fixed_point_profile: Some(profile),
            reducer_stats,
            loop_state: ReducerLoopState::default(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn fixed_point_profile(&self) -> Option<&ProfileConfig> {
        self.fixed_point_profile.as_ref()
    }

    pub(crate) fn reducer_stats(&self) -> &ReducerStats {
        &self.reducer_stats
    }

    pub(crate) fn reducer_stats_mut(&mut self) -> &mut ReducerStats {
        &mut self.reducer_stats
    }

    pub(crate) fn loop_state(&self) -> &ReducerLoopState {
        &self.loop_state
    }

    pub(crate) fn loop_state_mut(&mut self) -> &mut ReducerLoopState {
        &mut self.loop_state
    }
}

pub type Diagnostic = ClassifiedDiagnostic;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReducerLoopState {
    pub pass_index: usize,
    pub fixup_pass_count: usize,
    pub changed: bool,
    pub latest_diagnostics: Vec<Diagnostic>,
    pub raw_diagnostic_excerpts: Vec<RawDiagnosticExcerpt>,
    pub non_convergence: Option<NonConvergenceReport>,
    pub convergence_reason: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReducerPassOutcome {
    pub touched_files: Vec<PathBuf>,
    pub edits: Vec<EditRecord>,
    pub diagnostics: Vec<Diagnostic>,
    pub skipped_sites: Vec<SkippedSite>,
    pub changed: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReducerPassChangeScope {
    CandidateTree,
    ExternalState,
}

#[allow(dead_code)]
pub trait ReducerPass {
    fn name(&self) -> &'static str;
    fn run(&self, ctx: &mut ReducerContext) -> Result<ReducerPassOutcome>;
}

#[allow(dead_code)]
pub fn validate_pass_outcome(
    outcome: &ReducerPassOutcome,
    change_scope: ReducerPassChangeScope,
) -> Result<()> {
    validate_edit_records(&outcome.edits)
        .context("invalid canonical proof source in reducer pass outcome")?;

    if outcome.changed
        && outcome.edits.is_empty()
        && change_scope != ReducerPassChangeScope::ExternalState
    {
        anyhow::bail!(
            "pass outcome changed candidate tree without edit records; use ExternalState only for explicit external state updates"
        );
    }

    let mut touched_files = BTreeSet::new();
    for path in &outcome.touched_files {
        validate_candidate_relative_path("touched file", path)?;
        if !touched_files.insert(path.clone()) {
            anyhow::bail!("pass outcome has duplicate touched file {}", path.display());
        }
    }

    for edit in &outcome.edits {
        validate_candidate_relative_path("edit file", &edit.file)?;
        if !touched_files.contains(&edit.file) {
            anyhow::bail!(
                "pass outcome edit path {} is missing from touched_files",
                edit.file.display()
            );
        }
    }

    for diagnostic in &outcome.diagnostics {
        if let Some(path) = diagnostic.file() {
            validate_candidate_relative_path("diagnostic file", path)?;
        }
    }

    for skipped in &outcome.skipped_sites {
        if let Some(path) = &skipped.file {
            validate_candidate_relative_path("skipped site file", path)?;
        }
    }

    Ok(())
}

fn validate_candidate_relative_path(label: &str, path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        anyhow::bail!("pass outcome {label} path is empty");
    }

    for component in path.components() {
        match component {
            std::path::Component::Normal(_) => {}
            _ => {
                anyhow::bail!(
                    "pass outcome {label} path must be normalized and relative to candidate root: {}",
                    path.display()
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit_reason::{EditProofSource, EditReason};

    struct NoopPass;

    impl ReducerPass for NoopPass {
        fn name(&self) -> &'static str {
            "test.noop"
        }

        fn run(&self, ctx: &mut ReducerContext) -> Result<ReducerPassOutcome> {
            assert_eq!(ctx.root(), Path::new("candidate"));
            Ok(ReducerPassOutcome::default())
        }
    }

    #[test]
    fn reducer_pass_trait_exposes_name_and_run_contract() {
        let pass = NoopPass;
        let mut ctx = ReducerContext::new("candidate");

        let outcome = pass.run(&mut ctx).unwrap();

        assert_eq!(pass.name(), "test.noop");
        assert_eq!(outcome, ReducerPassOutcome::default());
    }

    #[test]
    fn reducer_pass_outcome_records_public_pass_effects() {
        let diagnostic = Diagnostic::MissingMakeTarget {
            target: String::from("drivers/foo/foo.o"),
            build_target: None,
            arch: None,
            config: None,
        };
        let skipped = SkippedSite {
            kind: String::from("skipped_fixup"),
            file: Some(PathBuf::from("drivers/foo/Makefile")),
            line: Some(7),
            reason: String::from("ambiguous stale reference"),
        };

        let outcome = ReducerPassOutcome {
            touched_files: vec![PathBuf::from("drivers/foo/Makefile")],
            edits: Vec::new(),
            diagnostics: vec![diagnostic.clone()],
            skipped_sites: vec![skipped.clone()],
            changed: false,
        };

        assert_eq!(
            outcome.touched_files,
            vec![PathBuf::from("drivers/foo/Makefile")]
        );
        assert_eq!(outcome.edits, Vec::<EditRecord>::new());
        assert_eq!(outcome.diagnostics, vec![diagnostic]);
        assert_eq!(outcome.skipped_sites, vec![skipped]);
        assert!(!outcome.changed);
    }

    fn manifest_path_edit(path: &str) -> EditRecord {
        EditRecord::new(
            PathBuf::from(path),
            None,
            String::from("before\n"),
            String::new(),
            EditReason::ManifestPath {
                path: PathBuf::from(path),
            },
            EditProofSource::removal_manifest_path(PathBuf::from(path)),
            "test.pass",
        )
    }

    #[test]
    fn validate_pass_outcome_accepts_reasoned_relative_edit_outcome() {
        let outcome = ReducerPassOutcome {
            touched_files: vec![PathBuf::from("drivers/foo/old.c")],
            edits: vec![manifest_path_edit("drivers/foo/old.c")],
            diagnostics: vec![Diagnostic::MissingHeader {
                source_file: PathBuf::from("drivers/live/new.c"),
                line: 3,
                header: String::from("old.h"),
                build_target: None,
                arch: None,
                config: None,
            }],
            skipped_sites: vec![SkippedSite {
                kind: String::from("skipped_fixup"),
                file: Some(PathBuf::from("drivers/live/new.c")),
                line: Some(3),
                reason: String::from("reported only"),
            }],
            changed: true,
        };

        validate_pass_outcome(&outcome, ReducerPassChangeScope::CandidateTree).unwrap();
    }

    #[test]
    fn validate_pass_outcome_rejects_noncanonical_proof_source() {
        let mut edit = manifest_path_edit("drivers/foo/old.c");
        edit.proof_source = EditProofSource::stale_kbuild_reference(String::from("foo.o"));
        let outcome = ReducerPassOutcome {
            touched_files: vec![PathBuf::from("drivers/foo/old.c")],
            edits: vec![edit],
            changed: true,
            ..ReducerPassOutcome::default()
        };

        let err =
            validate_pass_outcome(&outcome, ReducerPassChangeScope::CandidateTree).unwrap_err();
        let err = format!("{err:#}");

        assert!(err.contains("canonical proof source"));
        assert!(err.contains("multiple competing proof sources"));
    }

    #[test]
    fn validate_pass_outcome_rejects_changed_candidate_tree_without_edits() {
        let outcome = ReducerPassOutcome {
            touched_files: vec![PathBuf::from("drivers/foo/old.c")],
            changed: true,
            ..ReducerPassOutcome::default()
        };

        let err = validate_pass_outcome(&outcome, ReducerPassChangeScope::CandidateTree)
            .unwrap_err()
            .to_string();

        assert!(err.contains("changed candidate tree without edit records"));
        validate_pass_outcome(&outcome, ReducerPassChangeScope::ExternalState).unwrap();
    }

    #[test]
    fn validate_pass_outcome_rejects_edit_missing_from_touched_files() {
        let outcome = ReducerPassOutcome {
            touched_files: vec![PathBuf::from("drivers/foo/other.c")],
            edits: vec![manifest_path_edit("drivers/foo/old.c")],
            changed: true,
            ..ReducerPassOutcome::default()
        };

        let err = validate_pass_outcome(&outcome, ReducerPassChangeScope::CandidateTree)
            .unwrap_err()
            .to_string();

        assert!(err.contains("missing from touched_files"));
    }

    #[test]
    fn validate_pass_outcome_rejects_non_relative_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let absolute = tmp.path().join("candidate/drivers/foo.c");

        for outcome in [
            ReducerPassOutcome {
                touched_files: vec![absolute.clone()],
                ..ReducerPassOutcome::default()
            },
            ReducerPassOutcome {
                diagnostics: vec![Diagnostic::MissingHeader {
                    source_file: absolute.clone(),
                    line: 1,
                    header: String::from("foo.h"),
                    build_target: None,
                    arch: None,
                    config: None,
                }],
                ..ReducerPassOutcome::default()
            },
            ReducerPassOutcome {
                skipped_sites: vec![SkippedSite {
                    kind: String::from("skipped_fixup"),
                    file: Some(PathBuf::from("../escape.c")),
                    line: None,
                    reason: String::from("escape"),
                }],
                ..ReducerPassOutcome::default()
            },
        ] {
            let err = validate_pass_outcome(&outcome, ReducerPassChangeScope::CandidateTree)
                .unwrap_err()
                .to_string();
            assert!(
                err.contains("relative to candidate root"),
                "unexpected validation error: {err}"
            );
        }
    }
}
