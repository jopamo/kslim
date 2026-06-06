use super::common::*;

#[test]
fn stage_enums_are_stored_in_generate_reports() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let plan_report = production_source(&root.join("src/generate/plan_report.rs"));
    let report_writer = production_source(&root.join("src/output_repo/report_writer.rs"));

    for required in [
        "/// Write .kslim/report.txt",
        "pub fn write_report(",
        "stage: GenerateStage",
        "let stage = render_generate_stage_for_report(stage);",
        "fn render_generate_stage_for_report(stage: GenerateStage) -> &'static str",
        "stage.as_str()",
    ] {
        assert!(
            report_writer.contains(required),
            "generate reports should store typed GenerateStage through {required}"
        );
    }

    for required in [
        "fn render_report_only_plan_report(",
        "let stage = GenerateStage::Resolve;",
        "render_generate_stage_for_report(stage)",
        "fn render_generate_stage_for_report(stage: GenerateStage) -> &'static str",
    ] {
        assert!(
            plan_report.contains(required),
            "report-only generate reports should store typed GenerateStage through {required}"
        );
    }
}
