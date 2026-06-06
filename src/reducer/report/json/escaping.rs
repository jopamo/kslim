//! JSON scalar escaping and compact serialization helpers.

pub(super) fn bool_json(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

pub(super) fn json_compact<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("reducer report JSON serialization should not fail")
}

pub(super) fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
