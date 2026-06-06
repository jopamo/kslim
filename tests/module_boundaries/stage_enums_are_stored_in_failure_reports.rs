use super::common::*;

#[test]
fn stage_enums_are_stored_in_failure_reports() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_writer = production_source(&root.join("src/output_repo/report_writer.rs"));

    for required in [
        "pub fn write_failure_report(",
        "stage: GenerateStage",
        "let stage = render_generate_stage_for_report(stage);",
        "fn render_generate_stage_for_report(stage: GenerateStage) -> &'static str",
        "stage.as_str()",
    ] {
        assert!(
            report_writer.contains(required),
            "failure reports should store typed GenerateStage through {required}"
        );
    }
}
