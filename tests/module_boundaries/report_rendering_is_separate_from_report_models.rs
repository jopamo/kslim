use super::common::*;

#[test]
fn report_rendering_is_separate_from_report_models() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_model = production_source(&root.join("src/reducer/report/model.rs"));
    let report_render = production_source(&root.join("src/reducer/report/render.rs"));
    let report_json = production_sources(
        &root,
        &[
            "src/reducer/report/json.rs",
            "src/reducer/report/json/schema.rs",
            "src/reducer/report/json/serializer.rs",
            "src/reducer/report/json/escaping.rs",
            "src/reducer/report/json/canonical.rs",
        ],
    );
    let report_text = production_source(&root.join("src/reducer/report/text.rs"));
    let report_summary = production_source(&root.join("src/reducer/report/summary.rs"));

    for required_model in [
        "pub(crate) struct ReducerFailureReport",
        "pub(crate) struct ReducerReportArtifactNames",
        "pub(crate) struct RenderedReducerReportArtifacts",
        "pub(super) struct KbuildEditReport",
        "pub(crate) struct UnknownDiagnosticPolicy",
        "pub(crate) struct UnsupportedSyntaxPolicy",
    ] {
        assert!(
            report_model.contains(required_model),
            "reducer/report/model.rs should own report model item {required_model}"
        );
    }

    for forbidden_rendering in [
        "fn render_",
        "format!",
        "push_str",
        "json_escape",
        "serde_json::",
        "println!",
        "write!(",
    ] {
        assert!(
            !report_model.contains(forbidden_rendering),
            "reducer/report/model.rs must not own report rendering; found {forbidden_rendering}"
        );
    }

    assert!(
        report_render.contains("text::render_reducer_report_md")
            && report_render.contains("json::render_reducer_report_json")
            && report_json.contains("render_reducer_report_json")
            && report_text.contains("render_reducer_report_md")
            && report_summary.contains("render_unsupported_expression_report"),
        "reducer report rendering should live in render/json/text/summary modules"
    );

    for (path, renderer) in [
        ("src/reducer/report/render.rs", report_render.as_str()),
        ("src/reducer/report/json.rs", report_json.as_str()),
        ("src/reducer/report/text.rs", report_text.as_str()),
        ("src/reducer/report/summary.rs", report_summary.as_str()),
    ] {
        for forbidden_model_definition in [
            "pub struct ",
            "pub(crate) struct ",
            "pub(super) struct ",
            "pub enum ",
            "pub(crate) enum ",
            "pub(super) enum ",
        ] {
            assert!(
                !renderer.contains(forbidden_model_definition),
                "{path} should render existing report models, not define public report models; found {forbidden_model_definition}"
            );
        }
    }
}
