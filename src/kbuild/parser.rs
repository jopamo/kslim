//! Kbuild logical-line and assignment parsing.
//!
//! This module owns Makefile logical-line construction, protected line
//! detection for recipes and define blocks, and Kbuild assignment
//! classification. Graph indexing and rewriting consume this parser surface.

use std::collections::HashSet;

use super::{CompositeKind, KbuildAssignment, KbuildAssignmentKind, LogicalLine, ObjListKind};

pub(crate) fn logical_lines(content: &str) -> Vec<LogicalLine> {
    let mut out = Vec::new();
    let mut current = Vec::new();
    let mut joined = String::new();
    let mut start_line = 1usize;

    for (idx, line) in content.lines().enumerate() {
        if current.is_empty() {
            start_line = idx + 1;
        }

        current.push(line.to_string());

        let trimmed = line.trim_end();
        let continued = trimmed.ends_with('\\');
        let segment = if continued {
            trimmed.trim_end_matches('\\').trim_end()
        } else {
            trimmed
        };

        if !joined.is_empty() {
            joined.push(' ');
        }
        joined.push_str(segment);

        if !continued {
            out.push(LogicalLine {
                start_line,
                original: current.clone(),
                joined: joined.clone(),
            });
            current.clear();
            joined.clear();
        }
    }

    if !current.is_empty() {
        out.push(LogicalLine {
            start_line,
            original: current,
            joined,
        });
    }

    out
}

pub(crate) fn protected_make_logical_line_starts(lines: &[LogicalLine]) -> HashSet<usize> {
    let mut protected = HashSet::new();
    let mut in_define_block = false;

    for line in lines {
        if in_define_block {
            protected.insert(line.start_line);
            if is_make_define_end(line) {
                in_define_block = false;
            }
            continue;
        }

        if is_make_recipe_line(line) {
            protected.insert(line.start_line);
            continue;
        }

        if is_make_define_start(line) {
            protected.insert(line.start_line);
            in_define_block = true;
        }
    }

    protected
}

fn is_make_recipe_line(line: &LogicalLine) -> bool {
    line.original
        .first()
        .is_some_and(|raw| raw.starts_with('\t'))
}

fn is_make_define_start(line: &LogicalLine) -> bool {
    if is_make_recipe_line(line) {
        return false;
    }

    let text = line.joined.trim_start();
    if starts_with_make_directive(text, "define") {
        return true;
    }

    let Some(after_override) = text.strip_prefix("override") else {
        return false;
    };
    starts_with_make_directive(after_override.trim_start(), "define")
}

fn is_make_define_end(line: &LogicalLine) -> bool {
    line.joined.trim() == "endef"
}

fn starts_with_make_directive(text: &str, directive: &str) -> bool {
    let Some(rest) = text.strip_prefix(directive) else {
        return false;
    };

    rest.is_empty() || rest.chars().next().is_some_and(|ch| ch.is_whitespace())
}

pub(crate) fn parse_make_assignment(line: &str) -> Option<(&str, &str, &str)> {
    if line.starts_with('\t') {
        return None;
    }

    let body = line.split('#').next().unwrap_or("").trim();
    if body.is_empty() {
        return None;
    }

    for op in [":=", "+=", "?=", "="] {
        if let Some((lhs, rhs)) = body.split_once(op) {
            return Some((lhs.trim(), op, rhs.trim()));
        }
    }

    None
}

pub(crate) fn parse_kbuild_assignment(line: &str) -> Option<KbuildAssignment<'_>> {
    let (lhs, op, rhs) = parse_make_assignment(line)?;
    let kind = parse_kbuild_assignment_kind(lhs)?;
    Some(KbuildAssignment { lhs, op, rhs, kind })
}

pub(in crate::kbuild) fn parse_kbuild_assignment_kind(lhs: &str) -> Option<KbuildAssignmentKind<'_>> {
    if lhs == "subdir-y" {
        return Some(KbuildAssignmentKind::SubdirList);
    }
    if is_flag_assignment(lhs) {
        return Some(KbuildAssignmentKind::CcFlags);
    }
    if lhs == "obj-y" {
        return Some(KbuildAssignmentKind::ObjList(ObjListKind::BuiltIn));
    }
    if lhs == "obj-m" {
        return Some(KbuildAssignmentKind::ObjList(ObjListKind::Module));
    }
    if let Some(family) = lhs.strip_suffix("-y") {
        if is_object_list_family(family) {
            return Some(KbuildAssignmentKind::ObjList(ObjListKind::BuiltIn));
        }
    }
    if let Some(family) = lhs.strip_suffix("-m") {
        if is_object_list_family(family) {
            return Some(KbuildAssignmentKind::ObjList(ObjListKind::Module));
        }
    }
    if let Some((base, symbol)) = config_gated_assignment(lhs) {
        if is_object_list_family(base) {
            return Some(KbuildAssignmentKind::ObjList(ObjListKind::Config(symbol)));
        }
        return Some(KbuildAssignmentKind::CompositeMembers(
            CompositeKind::Config {
                target: base,
                symbol,
            },
        ));
    }
    if let Some(target) = lhs.strip_suffix("-objs") {
        if !target.is_empty() {
            return Some(KbuildAssignmentKind::CompositeMembers(
                CompositeKind::Objs { target },
            ));
        }
    }
    if let Some(target) = lhs.strip_suffix("-y") {
        if !target.is_empty() {
            return Some(KbuildAssignmentKind::CompositeMembers(
                CompositeKind::BuiltIn { target },
            ));
        }
    }
    if let Some(target) = lhs.strip_suffix("-m") {
        if !target.is_empty() {
            return Some(KbuildAssignmentKind::CompositeMembers(
                CompositeKind::Module { target },
            ));
        }
    }
    None
}

fn is_flag_assignment(lhs: &str) -> bool {
    matches!(
        lhs,
        "ccflags-y"
            | "asflags-y"
            | "cppflags-y"
            | "ldflags-y"
            | "subdir-ccflags-y"
            | "subdir-asflags-y"
            | "cflags-y"
            | "aflags-y"
    )
}

fn is_object_list_family(base: &str) -> bool {
    matches!(base, "obj" | "lib" | "always" | "extra" | "head" | "init")
}


fn config_gated_assignment(lhs: &str) -> Option<(&str, &str)> {
    let (base, suffix) = lhs.split_once("-$(")?;
    if base.is_empty() {
        return None;
    }
    let symbol = suffix.strip_prefix("CONFIG_")?.strip_suffix(')')?;
    if symbol.is_empty() {
        return None;
    }
    Some((base, symbol))
}
