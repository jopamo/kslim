use super::common::*;

#[test]
fn reducer_result_module_owns_reducer_result_shape() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let reducer = production_source(&root.join("src/reducer/mod.rs"));
    let result = production_source(&root.join("src/reducer/result.rs"));

    assert!(
        reducer.contains("mod result;"),
        "reducer/mod.rs should register the reducer result module"
    );

    for required in [
        "pub struct ReducerResult",
        "pub status: ReducerStatus",
        "pub publishable: bool",
        "pub passes: Vec<ReducerPassReport>",
        "pub edit_summary: EditSummary",
        "pub diagnostic_summary: DiagnosticSummary",
        "pub touched_files: Vec<PathBuf>",
        "pub skipped_sites: Vec<SkippedSite>",
        "pub fixups_applied: Vec<FixupApplication>",
        "pub final_build_status: BuildMatrixStatus",
        "pub convergence: ConvergenceStatus",
        "pub enum ReducerStatus",
        "Success",
        "FailedUnknownDiagnostic",
        "FailedUnsupportedSyntax",
        "FailedNonConvergence",
        "FailedBuildMatrix",
        "FailedInternalInvariant",
        "use serde::{Serialize, Serializer};",
        "#[serde(rename_all = \"snake_case\")]",
        "#[serde(skip_serializing)]",
        "REDUCER_RESULT_HOST_PATH_REDACTION",
        "serialize_committed_paths",
        "serialize_committed_optional_path",
        "serialize_committed_text",
        "set_publication_state",
    ] {
        assert!(
            result.contains(required),
            "reducer/result.rs should own reducer result item {required}"
        );
    }
}
