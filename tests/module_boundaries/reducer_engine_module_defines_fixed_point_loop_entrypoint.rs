use super::common::*;

#[test]
fn reducer_engine_module_defines_fixed_point_loop_entrypoint() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let reducer = production_source(&root.join("src/reducer/mod.rs"));
    let engine = production_source(&root.join("src/reducer/engine.rs"));

    assert!(
        reducer.contains("pub use engine::run_fixed_point_loop;"),
        "reducer/mod.rs should re-export fixed-point engine entrypoint"
    );

    for required in [
        "pub fn run_fixed_point_loop(ctx: &mut ReducerContext) -> Result<ReducerResult>",
        "strict_unsupported_syntax_in_stats",
        "FixedPointLoopTermination::SuccessfulSelectedBuildTestMatrix",
        "FixedPointLoopTermination::NoFixerChangedTree",
        "FixedPointLoopTermination::MaxPassCountReached",
        "FixedPointLoopTermination::UnknownDiagnosticInStrictMode",
        "FixedPointLoopTermination::UnsupportedSyntaxInStrictMode",
        "is_unknown_class",
        "record_selftest_failure_diagnostic",
        "ReducerStatus::FailedNonConvergence",
        "ReducerStatus::FailedUnknownDiagnostic",
        "ReducerStatus::FailedUnsupportedSyntax",
        "BuildMatrixStatus::Passed",
        "BuildMatrixStatus::Failed",
        "ConvergenceStatus::NotConverged",
    ] {
        assert!(
            engine.contains(required),
            "reducer/engine.rs should define fixed-point engine item {required}"
        );
    }
}
