use super::common::*;

#[test]
fn reducer_pipeline_module_owns_fixed_pass_table() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let reducer = production_source(&root.join("src/reducer/mod.rs"));
    let pipeline = production_source(&root.join("src/reducer/pipeline.rs"));
    let context = production_source(&root.join("src/reducer/context.rs"));
    let actions = production_source(&root.join("src/reducer/actions.rs"));
    let engine = production_source(&root.join("src/reducer/engine.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        reducer.contains("mod pipeline;")
            && reducer.contains("mod context;")
            && reducer.contains("mod actions;")
            && reducer.contains("mod engine;"),
        "src/reducer/mod.rs should register reducer pipeline/context/actions/engine modules"
    );

    for required in [
        "pub(crate) const FIXED_REDUCER_PIPELINE: &[&str]",
        "ReducerStage::BuildManifest.description()",
        "ReducerStage::BuildInitialIndex.description()",
        "ReducerStage::PruneDeclaredPaths.description()",
        "ReducerStage::RebuildFullIndex.description()",
        "ReducerStage::RewriteKconfig.description()",
        "ReducerStage::RebuildKconfigIndex.description()",
        "ReducerStage::RewriteKbuild.description()",
        "ReducerStage::RebuildKbuildIndex.description()",
        "ReducerStage::FoldPreprocessor.description()",
        "ReducerStage::RebuildCHeaderIndex.description()",
        "ReducerStage::RewriteIncludes.description()",
        "ReducerStage::RunSelftests.description()",
        "ReducerStage::ClassifyDiagnostics.description()",
        "ReducerStage::ApplyFixups.description()",
        "ReducerStage::ReindexAndRepeat.description()",
        "fn finish_reducer_after_declared_prune",
        "audit_declared_prune_edits",
        "audit_kconfig_stage_edits",
        "audit_mutating_pass_edits",
        "rebuild_tree_index_after_prune",
        "rebuild_kconfig_index_after_rewrite",
        "rebuild_kbuild_index_after_rewrite",
        "rebuild_c_header_index_after_cpp",
        "rebuild_c_header_index_after_include",
    ] {
        assert!(
            pipeline.contains(required),
            "src/reducer/pipeline.rs should own fixed reducer pipeline item {required}"
        );
    }

    for forbidden in [
        "pub trait ReducerPass",
        "pub struct ReducerContext",
        "pub fn validate_pass_outcome",
        "pub fn run_fixed_point_loop",
        "fn validate_reducer_edit_records",
    ] {
        assert!(
            !pipeline.contains(forbidden),
            "src/reducer/pipeline.rs should not own pass trait, context, fixed-point loop, or edit validation item {forbidden}"
        );
    }

    assert!(
        context.contains("pub trait ReducerPass")
            && context.contains("pub struct ReducerContext")
            && context.contains("pub fn validate_pass_outcome"),
        "src/reducer/context.rs should own reducer pass trait, context, and pass outcome validation"
    );
    assert!(
        actions.contains("pub(crate) fn audit_mutating_pass_edits")
            && actions.contains("fn validate_reducer_edit_records"),
        "src/reducer/actions.rs should own reducer edit validation gates for mutating passes"
    );
    assert!(
        engine.contains("pub fn run_fixed_point_loop(ctx: &mut ReducerContext) -> Result<ReducerResult>"),
        "src/reducer/engine.rs should own the fixed-point loop entrypoint"
    );

    assert!(
        architecture.contains("`reducer/*`")
            && architecture.contains("pass traits")
            && architecture.contains("fixed pass tables")
            && architecture.contains("reducer context")
            && architecture.contains("fixed-point loop")
            && architecture.contains("edit validation"),
        "docs/architecture.md should document reducer module ownership"
    );
}
