use super::common::*;

#[test]
fn reducer_context_actions_and_diagnostics_modules_define_reducer_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let reducer = production_source(&root.join("src/reducer/mod.rs"));
    let context = production_source(&root.join("src/reducer/context.rs"));
    let actions = production_source(&root.join("src/reducer/actions.rs"));
    let diagnostics = production_source(&root.join("src/reducer/diagnostics.rs"));
    let stage = production_source(&root.join("src/reducer/stage.rs"));

    for required in [
        "mod context;",
        "mod actions;",
        "mod diagnostics;",
        "mod stage;",
    ] {
        assert!(
            reducer.contains(required),
            "reducer/mod.rs should register reducer split module {required}"
        );
    }
    assert!(
        !reducer.contains("mod pass;"),
        "reducer/mod.rs should not keep the old mixed pass module"
    );

    for required in [
        "pub trait ReducerPass",
        "fn name(&self) -> &'static str;",
        "fn run(&self, ctx: &mut ReducerContext) -> Result<ReducerPassOutcome>;",
        "pub struct ReducerContext",
        "pub struct ReducerLoopState",
        "pub pass_index: usize",
        "pub fixup_pass_count: usize",
        "pub latest_diagnostics: Vec<Diagnostic>",
        "pub raw_diagnostic_excerpts: Vec<RawDiagnosticExcerpt>",
        "pub non_convergence: Option<NonConvergenceReport>",
        "pub convergence_reason: Option<String>",
        "pub struct ReducerPassOutcome",
        "pub type Diagnostic = ClassifiedDiagnostic;",
        "pub touched_files: Vec<PathBuf>",
        "pub edits: Vec<EditRecord>",
        "pub diagnostics: Vec<Diagnostic>",
        "pub skipped_sites: Vec<SkippedSite>",
        "pub changed: bool",
        "pub enum ReducerPassChangeScope",
        "CandidateTree",
        "ExternalState",
        "pub fn validate_pass_outcome",
        "validate_edit_records(&outcome.edits)",
        "validate_candidate_relative_path",
    ] {
        assert!(
            context.contains(required),
            "reducer/context.rs should define reducer context/contract item {required}"
        );
    }

    for required in [
        "pub fn apply_selftest_fixup",
        "pub(crate) fn audit_mutating_pass_edits",
        "pub(crate) fn validate_reducer_edit_provenance",
        "fn validate_reducer_edit_records",
        "fn apply_additional_cpp_fold_after_config_truth_update",
    ] {
        assert!(
            actions.contains(required),
            "reducer/actions.rs should own reducer mutation action item {required}"
        );
    }

    for required in [
        "pub struct RawDiagnosticExcerpt",
        "pub command_context: String",
        "pub build_target: Option<String>",
        "pub raw_excerpt: String",
        "pub struct NonConvergenceReport",
        "pub pass_count: usize",
        "pub remaining_diagnostics: Vec<ClassifiedDiagnostic>",
        "pub fixers_skipped: Vec<String>",
        "pub publishable: bool",
        "raw_diagnostic_excerpt_from_failure",
        "render_raw_diagnostic_excerpt",
        "captured_command_raw_excerpt",
        "non_convergence_report",
        "record_selftest_failure_diagnostic",
    ] {
        assert!(
            diagnostics.contains(required),
            "reducer/diagnostics.rs should own reducer diagnostic item {required}"
        );
    }

    for required in [
        "pub(crate) enum ReducerStage",
        "BuildManifest",
        "BuildInitialIndex",
        "PruneDeclaredPaths",
        "RebuildFullIndex",
        "RewriteKconfig",
        "RebuildKconfigIndex",
        "RewriteKbuild",
        "RebuildKbuildIndex",
        "FoldPreprocessor",
        "RebuildCHeaderIndex",
        "RewriteIncludes",
        "RunSelftests",
        "ClassifyDiagnostics",
        "ApplyFixups",
        "ReindexAndRepeat",
        "pub(crate) const ALL",
        "pub(crate) const fn stable_name",
        "#[serde(rename = \"build_manifest\")]",
        "#[serde(rename = \"reindex_and_repeat\")]",
    ] {
        assert!(
            stage.contains(required),
            "reducer/stage.rs should define stable reducer stage item {required}"
        );
    }
}
