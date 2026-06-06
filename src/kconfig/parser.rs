use super::KconfigSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KconfigEntryKind {
    Config,
    Menuconfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum KconfigDirective {
    Entry {
        kind: KconfigEntryKind,
        symbol: String,
    },
    DependsOn {
        expr: String,
    },
    Select {
        symbol: String,
        condition: Option<String>,
    },
    Imply {
        symbol: String,
        condition: Option<String>,
    },
    VisibleIf {
        expr: String,
    },
    If {
        expr: String,
    },
    Default {
        value: String,
        condition: Option<String>,
    },
    Source {
        source: KconfigSource,
    },
}
pub(crate) fn parse_kconfig_source(line: &str) -> Option<KconfigSource> {
    match parse_kconfig_directive(line)? {
        KconfigDirective::Source { source } => Some(source),
        _ => None,
    }
}
pub(super) fn parse_kconfig_directive(line: &str) -> Option<KconfigDirective> {
    let (directive_text, _) = split_kconfig_trailing_comment(line);
    let trimmed = directive_text.trim_start();

    if let Some(rest) = strip_kconfig_keyword(trimmed, "config") {
        return parse_kconfig_entry(rest, KconfigEntryKind::Config);
    }
    if let Some(rest) = strip_kconfig_keyword(trimmed, "menuconfig") {
        return parse_kconfig_entry(rest, KconfigEntryKind::Menuconfig);
    }
    if let Some(rest) = trimmed.strip_prefix("depends on ") {
        return Some(KconfigDirective::DependsOn {
            expr: rest.trim().to_string(),
        });
    }
    if let Some(rest) = trimmed.strip_prefix("select ") {
        return parse_kconfig_target_directive(rest, true);
    }
    if let Some(rest) = trimmed.strip_prefix("imply ") {
        return parse_kconfig_target_directive(rest, false);
    }
    if let Some(rest) = trimmed.strip_prefix("visible if ") {
        return Some(KconfigDirective::VisibleIf {
            expr: rest.trim().to_string(),
        });
    }
    if let Some(rest) = trimmed.strip_prefix("if ") {
        let expr = rest.trim();
        if expr.is_empty() {
            return None;
        }
        return Some(KconfigDirective::If {
            expr: expr.to_string(),
        });
    }
    if let Some(rest) = trimmed.strip_prefix("default ") {
        return parse_kconfig_default_directive(rest);
    }
    if let Some(source) = parse_kconfig_source_directive(trimmed) {
        return Some(KconfigDirective::Source { source });
    }

    None
}

pub(super) fn strip_kconfig_keyword<'a>(trimmed: &'a str, keyword: &str) -> Option<&'a str> {
    let rest = trimmed.strip_prefix(keyword)?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    Some(rest.trim_start())
}

fn parse_kconfig_entry(rest: &str, kind: KconfigEntryKind) -> Option<KconfigDirective> {
    let symbol = rest.split_whitespace().next()?.trim();
    if symbol.is_empty() {
        return None;
    }

    Some(KconfigDirective::Entry {
        kind,
        symbol: symbol.to_string(),
    })
}

fn parse_kconfig_target_directive(rest: &str, is_select: bool) -> Option<KconfigDirective> {
    let (symbol, condition) = split_kconfig_if_clause(rest);
    let symbol = symbol.trim();
    if symbol.is_empty() {
        return None;
    }

    let condition = condition.map(str::trim).filter(|expr| !expr.is_empty());
    Some(if is_select {
        KconfigDirective::Select {
            symbol: symbol.to_string(),
            condition: condition.map(str::to_string),
        }
    } else {
        KconfigDirective::Imply {
            symbol: symbol.to_string(),
            condition: condition.map(str::to_string),
        }
    })
}

fn parse_kconfig_default_directive(rest: &str) -> Option<KconfigDirective> {
    let (value, condition) = split_kconfig_if_clause(rest);
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    Some(KconfigDirective::Default {
        value: value.to_string(),
        condition: condition
            .map(str::trim)
            .filter(|expr| !expr.is_empty())
            .map(str::to_string),
    })
}

fn parse_kconfig_source_directive(trimmed: &str) -> Option<KconfigSource> {
    let (keyword, rest) = trimmed.split_once(char::is_whitespace)?;

    let (optional, relative) = match keyword {
        "source" => (false, false),
        "rsource" => (false, true),
        "osource" => (true, false),
        "orsource" => (true, true),
        _ => return None,
    };

    let rest = rest.trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let path = rest[1..].split('"').next()?.trim();
    if path.is_empty() {
        return None;
    }

    Some(KconfigSource {
        path: path.to_string(),
        optional,
        relative,
    })
}

pub(super) fn split_kconfig_if_clause(input: &str) -> (&str, Option<&str>) {
    let mut in_quotes = false;
    let mut escape = false;
    let mut last_if = None;

    for (idx, ch) in input.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match ch {
            '\\' if in_quotes => escape = true,
            '"' => in_quotes = !in_quotes,
            _ if !in_quotes && input[idx..].starts_with(" if ") => last_if = Some(idx),
            _ => {}
        }
    }

    match last_if {
        Some(idx) => (&input[..idx], Some(&input[idx + 4..])),
        None => (input, None),
    }
}

pub(super) fn split_kconfig_trailing_comment(line: &str) -> (&str, &str) {
    let mut in_quotes = false;
    let mut escape = false;
    let mut comment_idx = None;

    for (idx, ch) in line.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match ch {
            '\\' if in_quotes => escape = true,
            '"' => in_quotes = !in_quotes,
            '#' if !in_quotes => {
                comment_idx = Some(idx);
                break;
            }
            _ => {}
        }
    }

    let Some(comment_idx) = comment_idx else {
        return (line, "");
    };

    let body_end = line[..comment_idx].trim_end().len();
    (&line[..body_end], &line[body_end..])
}
