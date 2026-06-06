use super::common::*;

#[test]
fn diagnostics_renderer_lives_in_renderer_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let diagnostics = production_source(&root.join("src/diagnostics.rs"));
    let renderer = production_source(&root.join("src/diagnostics/renderer.rs"));
    let summary = production_source(&root.join("src/reducer/report/summary.rs"));
    let json = production_sources(
        &root,
        &[
            "src/reducer/report/json.rs",
            "src/reducer/report/json/schema.rs",
            "src/reducer/report/json/serializer.rs",
            "src/reducer/report/json/escaping.rs",
            "src/reducer/report/json/canonical.rs",
        ],
    );
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod renderer;",
        "pub(crate) use renderer::{render_classified_diagnostic_json, render_classified_diagnostic_md};",
    ] {
        assert!(
            diagnostics.contains(required),
            "src/diagnostics.rs should expose diagnostic rendering through {required}"
        );
    }

    for required in [
        "pub(crate) fn render_classified_diagnostic_md(",
        "class={} file={} line={} subject={}",
        "diagnostic.class().stable_name()",
        "diagnostic.file()",
        "diagnostic.line()",
        "diagnostic.subject()",
        "pub(crate) fn render_classified_diagnostic_json(",
        "\\\"class\\\":\\\"{}\\\"",
        "\\\"file\\\":{}",
        "\\\"build_target\\\":{}",
        "diagnostic.build_target()",
        "diagnostic.arch()",
        "diagnostic.config()",
        "fn json_string_or_null(",
    ] {
        assert!(
            renderer.contains(required),
            "src/diagnostics/renderer.rs should own diagnostic rendering detail {required}"
        );
    }

    for required in [
        "crate::diagnostics::render_classified_diagnostic_md(",
        "committed_path_string",
        "sanitize_committed_result_text",
    ] {
        assert!(
            summary.contains(required),
            "reducer markdown summary should delegate diagnostic rendering through {required}"
        );
    }

    for required in [
        "crate::diagnostics::render_classified_diagnostic_json(",
        "committed_path_string",
        "sanitize_committed_result_text",
        "json_escape",
    ] {
        assert!(
            json.contains(required),
            "reducer JSON report should delegate diagnostic rendering through {required}"
        );
    }

    for forbidden in [
        "\nfn render_classified_diagnostic_md(",
        "\nfn render_classified_diagnostic_json(",
    ] {
        assert!(
            !diagnostics.contains(forbidden),
            "src/diagnostics.rs should not keep diagnostic renderer body {forbidden}"
        );
    }

    for required in ["`src/diagnostics/renderer.rs`", "Diagnostic renderer"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document diagnostic renderer ownership {required}"
        );
    }
}
