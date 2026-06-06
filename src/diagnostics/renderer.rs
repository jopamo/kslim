//! Diagnostic rendering helpers.
//!
//! This module owns stable text and JSON rendering for classified diagnostics.
//! Callers supply path, text-sanitization, and JSON-escaping policy so report
//! safety remains with the report layer.

use std::path::Path;

use super::ClassifiedDiagnostic;

pub(crate) fn render_classified_diagnostic_md(
    diagnostic: &ClassifiedDiagnostic,
    path_text: impl Fn(&Path) -> String,
    sanitize_text: impl Fn(&str) -> String,
) -> String {
    let file = diagnostic
        .file()
        .map(path_text)
        .unwrap_or_else(|| String::from("<none>"));
    let line = diagnostic
        .line()
        .map(|line| line.to_string())
        .unwrap_or_else(|| String::from("<none>"));
    let subject = diagnostic
        .subject()
        .map(sanitize_text)
        .unwrap_or_else(|| String::from("<none>"));
    format!(
        "class={} file={} line={} subject={}",
        diagnostic.class().stable_name(),
        file,
        line,
        subject
    )
}

pub(crate) fn render_classified_diagnostic_json(
    diagnostic: &ClassifiedDiagnostic,
    path_text: impl Fn(&Path) -> String,
    sanitize_text: impl Fn(&str) -> String,
    json_escape: impl Fn(&str) -> String,
) -> String {
    let file = diagnostic.file().map(path_text);
    let line = diagnostic.line().map(|line| line.to_string());
    let subject = diagnostic.subject().map(&sanitize_text);
    let build_target = diagnostic.build_target().map(&sanitize_text);
    let arch = diagnostic.arch().map(&sanitize_text);
    let config = diagnostic.config().map(&sanitize_text);

    format!(
        concat!(
            "{{",
            "\"class\":\"{}\",",
            "\"file\":{},",
            "\"line\":{},",
            "\"subject\":{},",
            "\"build_target\":{},",
            "\"arch\":{},",
            "\"config\":{}",
            "}}"
        ),
        diagnostic.class().stable_name(),
        json_string_or_null(file.as_deref(), &json_escape),
        line.unwrap_or_else(|| String::from("null")),
        json_string_or_null(subject.as_deref(), &json_escape),
        json_string_or_null(build_target.as_deref(), &json_escape),
        json_string_or_null(arch.as_deref(), &json_escape),
        json_string_or_null(config.as_deref(), &json_escape),
    )
}

fn json_string_or_null(value: Option<&str>, json_escape: &impl Fn(&str) -> String) -> String {
    value
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| String::from("null"))
}
