//! Stable line-oriented serialization helpers.

pub(crate) fn append_stable_key_value_line(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push('=');
    out.push_str(&escape_stable_value(value));
    out.push('\n');
}

pub(crate) fn escape_stable_value(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

pub(crate) fn bool_token(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_key_value_lines_escape_control_characters() {
        let mut out = String::new();

        append_stable_key_value_line(&mut out, "field", "slash\\line\ncarriage\rtab\tend");

        assert_eq!(out, "field=slash\\\\line\\ncarriage\\rtab\\tend\n");
    }

    #[test]
    fn bool_tokens_are_stable_lowercase_literals() {
        assert_eq!(bool_token(true), "true");
        assert_eq!(bool_token(false), "false");
    }
}

