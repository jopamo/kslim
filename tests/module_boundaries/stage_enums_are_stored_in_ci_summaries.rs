use super::common::*;

#[test]
fn stage_enums_are_stored_in_ci_summaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let commands = commands_source(root);
    let render = production_source(&root.join("src/command_render.rs"));
    let source = format!("{commands}\n{render}");

    for required in [
        "use crate::generate::{self, GenerateOptions, GenerateStage};",
        "let stage = render_generate_stage_for_ci_summary(result.stage);",
        "println!(\"  stage: {}\", stage);",
        "fn render_generate_stage_for_ci_summary(stage: GenerateStage) -> &'static str",
        "stage.as_str()",
    ] {
        assert!(
            source.contains(required),
            "CI summaries should store typed GenerateStage through {required}"
        );
    }
}
