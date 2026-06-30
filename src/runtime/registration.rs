//! Conservative runtime-registration removal proof.
//!
//! Runtime registration macros and calls publish entry points into kernel
//! runtime dispatch tables. Removing a provider while a live entry point remains
//! is treated as unsafe. This scanner proves the registered identifiers are not
//! referenced by live C/ASM sources, and fails closed for malformed recognized
//! registration invocations in removed providers.

use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::model::RuntimeRegistrationSurface;
use crate::path_policy::normalized_relative_path_covers;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RuntimeRegistrationRemovalProof {
    pub provider: PathBuf,
    pub registration_macro: String,
    pub entry_points: Vec<String>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RuntimeRegistration {
    provider: PathBuf,
    registration_macro: String,
    entry_points: Vec<String>,
    line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MalformedRegistration {
    file: PathBuf,
    line: usize,
    registration_macro: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LiveEntryPointReference {
    file: PathBuf,
    line: usize,
    entry_point: String,
}

#[derive(Debug, Default)]
struct LiveEntryPointShadows {
    global: BTreeSet<String>,
    file_local: BTreeMap<PathBuf, BTreeSet<String>>,
}

#[derive(Debug, Default)]
struct ScopedEntryPoints {
    global: BTreeSet<String>,
    file_local: BTreeSet<String>,
}

pub(crate) fn prove_removed_runtime_registrations_have_no_live_entry_points(
    root: &Path,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> Result<BTreeSet<RuntimeRegistrationRemovalProof>> {
    let source_files = source_files(root)?;
    let mut removed_registrations = BTreeSet::new();
    let mut live_sources = Vec::new();

    for relative in &source_files {
        if path_is_removed(relative, removed_paths, removed_dirs, removed_files) {
            let scan = scan_runtime_registrations_in_file(root, relative)?;
            if let Some(malformed) = scan.malformed.into_iter().next() {
                anyhow::bail!(
                    "runtime registration removal requires parsable entry-point proof; unsupported {} invocation in {}:{}",
                    malformed.registration_macro,
                    malformed.file.display(),
                    malformed.line,
                );
            }
            removed_registrations.extend(scan.registrations);
        } else {
            live_sources.push(relative.clone());
        }
    }

    let tracked_entry_points = removed_registrations
        .iter()
        .flat_map(|registration| registration.entry_points.iter().cloned())
        .collect::<BTreeSet<_>>();
    let shadowed_entry_points = live_shadowed_entry_points(
        root,
        &tracked_entry_points,
        removed_paths,
        removed_dirs,
        removed_files,
    )?;
    let live_references = live_references_for_entry_points(
        root,
        &live_sources,
        &tracked_entry_points,
        &shadowed_entry_points.file_local,
        &shadowed_entry_points.global,
    )?;

    let mut proofs = BTreeSet::new();
    for registration in removed_registrations {
        let registration_live_references = registration
            .entry_points
            .iter()
            .filter_map(|entry_point| live_references.get(entry_point))
            .flat_map(|references| references.iter().cloned())
            .collect::<BTreeSet<_>>();
        if !registration_live_references.is_empty() {
            anyhow::bail!(
                "runtime registration removal requires proof that no live entry point remains for {} in {}:{}; live reference(s): {}",
                registration.registration_macro,
                registration.provider.display(),
                registration.line,
                render_live_references(&registration_live_references),
            );
        }
        proofs.insert(RuntimeRegistrationRemovalProof {
            provider: registration.provider,
            registration_macro: registration.registration_macro,
            entry_points: registration.entry_points,
            line: registration.line,
        });
    }

    Ok(proofs)
}

#[derive(Debug, Default)]
struct RuntimeRegistrationScan {
    registrations: BTreeSet<RuntimeRegistration>,
    malformed: BTreeSet<MalformedRegistration>,
}

fn scan_runtime_registrations_in_file(
    root: &Path,
    relative: &Path,
) -> Result<RuntimeRegistrationScan> {
    let content = std::fs::read_to_string(root.join(relative)).with_context(|| {
        format!(
            "failed to read removed runtime-registration provider {}",
            relative.display()
        )
    })?;
    Ok(scan_runtime_registrations_in_content(relative, &content))
}

fn scan_runtime_registrations_in_content(
    relative: &Path,
    content: &str,
) -> RuntimeRegistrationScan {
    let source = mask_c_comments_and_literals(content);
    let mut scan = RuntimeRegistrationScan::default();
    let mut offset = 0usize;
    let mut line = 1usize;

    while let Some((start, token, token_line)) = next_identifier(&source, offset, line) {
        line = token_line;
        offset = start + token.len();
        if !RuntimeRegistrationSurface::is_known_registration_macro(token) {
            continue;
        }

        let Some((entry_points, end)) = parse_registration_entry_points(&source, offset) else {
            scan.malformed.insert(MalformedRegistration {
                file: relative.to_path_buf(),
                line: token_line,
                registration_macro: token.to_string(),
            });
            continue;
        };
        offset = end;
        scan.registrations.insert(RuntimeRegistration {
            provider: relative.to_path_buf(),
            registration_macro: token.to_string(),
            entry_points,
            line: token_line,
        });
    }

    scan
}

fn parse_registration_entry_points(source: &str, offset: usize) -> Option<(Vec<String>, usize)> {
    let mut cursor = skip_ascii_whitespace(source, offset);
    if !source[cursor..].starts_with('(') {
        return None;
    }
    cursor += 1;

    let mut entry_points = BTreeSet::new();
    loop {
        cursor = skip_ascii_whitespace(source, cursor);
        if source[cursor..].starts_with(')') {
            cursor += 1;
            break;
        }
        while source[cursor..].starts_with('&') || source[cursor..].starts_with('*') {
            cursor += 1;
            cursor = skip_ascii_whitespace(source, cursor);
        }
        let Some((identifier, end)) = parse_c_identifier(source, cursor) else {
            return None;
        };
        entry_points.insert(identifier.to_string());
        cursor = end;
        loop {
            cursor = skip_ascii_whitespace(source, cursor);
            if source[cursor..].starts_with('[') {
                cursor += 1;
                cursor = skip_balanced_delimiters(source, cursor, '[', ']')?;
                continue;
            }
            if source[cursor..].starts_with('.') {
                cursor += 1;
                cursor = skip_ascii_whitespace(source, cursor);
                let Some((_, end)) = parse_c_identifier(source, cursor) else {
                    return None;
                };
                cursor = end;
                continue;
            }
            if source[cursor..].starts_with("->") {
                cursor += 2;
                cursor = skip_ascii_whitespace(source, cursor);
                let Some((_, end)) = parse_c_identifier(source, cursor) else {
                    return None;
                };
                cursor = end;
                continue;
            }
            break;
        }
        cursor = skip_ascii_whitespace(source, cursor);
        let ch = source[cursor..].chars().next()?;
        cursor += ch.len_utf8();
        match ch {
            ',' => continue,
            ')' => break,
            _ => return None,
        }
    }

    if entry_points.is_empty() {
        return None;
    }
    Some((entry_points.into_iter().collect(), cursor))
}

fn live_references_for_entry_points(
    root: &Path,
    live_sources: &[PathBuf],
    entry_points: &BTreeSet<String>,
    file_local_shadowed_entry_points: &BTreeMap<PathBuf, BTreeSet<String>>,
    global_shadowed_entry_points: &BTreeSet<String>,
) -> Result<BTreeMap<String, BTreeSet<LiveEntryPointReference>>> {
    let mut references = BTreeMap::<String, BTreeSet<LiveEntryPointReference>>::new();
    if entry_points.is_empty() {
        return Ok(references);
    }

    for relative in live_sources {
        let content = std::fs::read_to_string(root.join(relative)).with_context(|| {
            format!(
                "failed to read live source while proving no runtime entry point remains: {}",
                relative.display(),
            )
        })?;
        let empty_shadows = BTreeSet::new();
        let local_shadows = file_local_shadowed_entry_points
            .get(relative)
            .unwrap_or(&empty_shadows);
        for (entry_point, lines) in identifier_occurrence_lines_for_symbols(
            &content,
            entry_points,
            local_shadows,
            global_shadowed_entry_points,
        ) {
            let entry_point_references = references.entry(entry_point.clone()).or_default();
            for line in lines {
                entry_point_references.insert(LiveEntryPointReference {
                    file: relative.clone(),
                    line,
                    entry_point: entry_point.clone(),
                });
            }
        }
    }
    Ok(references)
}

fn live_shadowed_entry_points(
    root: &Path,
    entry_points: &BTreeSet<String>,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> Result<LiveEntryPointShadows> {
    let mut shadowed = LiveEntryPointShadows::default();
    if entry_points.is_empty() {
        return Ok(shadowed);
    }

    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry.path().strip_prefix(root).with_context(|| {
            format!(
                "failed to derive root-relative runtime-registration shadow scan path for {}",
                entry.path().display()
            )
        })?;
        if path_is_removed(relative, removed_paths, removed_dirs, removed_files)
            || !is_c_or_asm_source_path(relative)
        {
            continue;
        }

        let content = std::fs::read_to_string(entry.path()).with_context(|| {
            format!(
                "failed to read live source while scanning runtime-registration shadows: {}",
                relative.display(),
            )
        })?;
        let function_symbols = function_defined_entry_points_in_content(&content, entry_points);
        for symbol in function_symbols.global {
            shadowed.global.insert(symbol);
        }
        let file_local_shadows = shadowed
            .file_local
            .entry(relative.to_path_buf())
            .or_default();
        for symbol in function_symbols.file_local {
            file_local_shadows.insert(symbol);
        }

        let variable_symbols = variable_defined_entry_points_in_content(&content, entry_points);
        for symbol in variable_symbols.global {
            shadowed.global.insert(symbol);
        }
        let file_local_shadows = shadowed
            .file_local
            .entry(relative.to_path_buf())
            .or_default();
        for symbol in variable_symbols.file_local {
            file_local_shadows.insert(symbol);
        }
    }

    Ok(shadowed)
}

fn source_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry.path().strip_prefix(root).with_context(|| {
            format!(
                "failed to derive root-relative runtime-registration scan path for {}",
                entry.path().display()
            )
        })?;
        if is_c_or_asm_source_path(relative) {
            files.push(relative.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn is_c_or_asm_source_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("c" | "S" | "s" | "cc" | "cpp" | "cxx")
    )
}

fn path_is_removed(
    path: &Path,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> bool {
    removed_paths.contains(path)
        || removed_files.contains(path)
        || removed_dirs
            .iter()
            .any(|dir| normalized_relative_path_covers(dir, path))
}

#[allow(dead_code)]
fn identifier_occurrence_lines(content: &str, symbol: &str) -> BTreeSet<usize> {
    let source = mask_c_comments_and_literals(content);
    let local_static_symbols = file_local_static_function_symbols(&source);
    let type_body_ranges = type_definition_body_ranges(&source);
    if local_static_symbols.contains(symbol) {
        return BTreeSet::new();
    }
    let mut lines = BTreeSet::new();
    let mut offset = 0usize;
    let mut line = 1usize;

    while let Some((start, token, token_line)) = next_identifier(&source, offset, line) {
        line = token_line;
        offset = start + token.len();
        if token == symbol
            && !offset_in_ranges(start, &type_body_ranges)
            && !identifier_is_type_tag_reference(&source, start)
            && !identifier_is_member_access(&source, start)
        {
            lines.insert(token_line);
        }
    }

    lines
}

fn identifier_occurrence_lines_for_symbols(
    content: &str,
    entry_points: &BTreeSet<String>,
    local_shadowed_entry_points: &BTreeSet<String>,
    global_shadowed_entry_points: &BTreeSet<String>,
) -> BTreeMap<String, BTreeSet<usize>> {
    let source = mask_c_comments_and_literals(content);
    let local_static_symbols = file_local_static_function_symbols(&source);
    let type_body_ranges = type_definition_body_ranges(&source);
    let mut lines = BTreeMap::<String, BTreeSet<usize>>::new();
    let mut offset = 0usize;
    let mut line = 1usize;

    while let Some((start, token, token_line)) = next_identifier(&source, offset, line) {
        line = token_line;
        offset = start + token.len();
        if !entry_points.contains(token)
            || local_static_symbols.contains(token)
            || local_shadowed_entry_points.contains(token)
            || global_shadowed_entry_points.contains(token)
            || offset_in_ranges(start, &type_body_ranges)
            || identifier_is_type_tag_reference(&source, start)
            || identifier_is_member_access(&source, start)
        {
            continue;
        }
        lines
            .entry(token.to_string())
            .or_default()
            .insert(token_line);
    }

    lines
}

fn function_defined_entry_points_in_content(
    content: &str,
    entry_points: &BTreeSet<String>,
) -> ScopedEntryPoints {
    let source = mask_c_comments_and_literals(content);
    let type_body_ranges = type_definition_body_ranges(&source);
    let mut symbols = ScopedEntryPoints::default();
    let mut offset = 0usize;
    let mut line = 1usize;

    while let Some((start, token, token_line)) = next_identifier(&source, offset, line) {
        line = token_line;
        offset = start + token.len();
        if !entry_points.contains(token)
            || offset_in_ranges(start, &type_body_ranges)
            || identifier_is_member_access(&source, start)
        {
            continue;
        }
        let after_name = skip_ascii_whitespace(&source, offset);
        let Some(after_open) = source[after_name..].strip_prefix('(').map(|_| after_name + 1)
        else {
            continue;
        };
        let Some(after_close) = skip_balanced_delimiters(&source, after_open, '(', ')') else {
            continue;
        };
        let after_sig = skip_ascii_whitespace(&source, after_close);
        if !source[after_sig..].starts_with('{') {
            continue;
        }
        if definition_is_static(&source, start) {
            symbols.file_local.insert(token.to_string());
        } else {
            symbols.global.insert(token.to_string());
        }
    }

    symbols
}

fn variable_defined_entry_points_in_content(
    content: &str,
    entry_points: &BTreeSet<String>,
) -> ScopedEntryPoints {
    let source = mask_c_comments_and_literals(content);
    let type_body_ranges = type_definition_body_ranges(&source);
    let mut symbols = ScopedEntryPoints::default();
    let mut offset = 0usize;
    let mut line = 1usize;

    while let Some((start, token, token_line)) = next_identifier(&source, offset, line) {
        line = token_line;
        offset = start + token.len();
        if !entry_points.contains(token)
            || offset_in_ranges(start, &type_body_ranges)
            || identifier_is_member_access(&source, start)
        {
            continue;
        }
        if !identifier_is_variable_definition(&source, start, offset) {
            continue;
        }
        if definition_is_static(&source, start) {
            symbols.file_local.insert(token.to_string());
        } else {
            symbols.global.insert(token.to_string());
        }
    }

    symbols
}

fn file_local_static_function_symbols(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(static_function_line_symbol)
        .map(String::from)
        .collect()
}

fn static_function_line_symbol(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("static ") {
        return None;
    }

    let Some(open_paren) = trimmed.find('(') else {
        return None;
    };
    let before = &trimmed[..open_paren];
    before
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .filter(|part| !part.is_empty())
        .next_back()
}

fn identifier_is_member_access(source: &str, start: usize) -> bool {
    if start == 0 {
        return false;
    }

    let mut prior = source[..start].chars().rev().skip_while(|ch| ch.is_whitespace());
    let Some(last) = prior.next() else {
        return false;
    };

    if last == '.' {
        return true;
    }

    if last == '>' {
        return prior.next().is_some_and(|ch| ch == '-');
    }

    false
}

fn identifier_is_type_tag_reference(source: &str, start: usize) -> bool {
    if start == 0 {
        return false;
    }
    source[..start]
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .filter(|part| !part.is_empty())
        .next_back()
        .is_some_and(|token| matches!(token, "struct" | "union" | "enum"))
}

fn offset_in_ranges(offset: usize, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|&(start, end)| offset >= start && offset < end)
}

fn type_definition_body_ranges(source: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut offset = 0usize;
    let mut line = 1usize;

    while let Some((start, token, token_line)) = next_identifier(source, offset, line) {
        line = token_line;
        offset = start + token.len();
        if !matches!(token, "struct" | "union" | "enum") {
            continue;
        }

        let mut cursor = offset;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        while cursor < source.len() {
            let Some(ch) = source[cursor..].chars().next() else {
                break;
            };
            match ch {
                '(' => paren_depth += 1,
                ')' => {
                    paren_depth = paren_depth.saturating_sub(1);
                    if paren_depth == 0 && bracket_depth == 0 {
                        break;
                    }
                }
                '[' => bracket_depth += 1,
                ']' => bracket_depth = bracket_depth.saturating_sub(1),
                ';' | '=' if paren_depth == 0 && bracket_depth == 0 => break,
                '{' if paren_depth == 0 && bracket_depth == 0 => {
                    let open = cursor;
                    if let Some(close) = skip_balanced_delimiters(source, cursor + 1, '{', '}') {
                        ranges.push((open + 1, close.saturating_sub(1)));
                        offset = close;
                    }
                    break;
                }
                _ => {}
            }
            cursor += ch.len_utf8();
        }
    }

    ranges
}

fn next_identifier(
    source: &str,
    mut offset: usize,
    mut line: usize,
) -> Option<(usize, &str, usize)> {
    while offset < source.len() {
        let ch = source[offset..].chars().next()?;
        if ch == '\n' {
            line += 1;
            offset += 1;
            continue;
        }
        if is_c_identifier_start(ch) {
            let start = offset;
            offset += ch.len_utf8();
            while offset < source.len() {
                let ch = source[offset..].chars().next()?;
                if !is_c_identifier_continue(ch) {
                    break;
                }
                offset += ch.len_utf8();
            }
            return Some((start, &source[start..offset], line));
        }
        offset += ch.len_utf8();
    }
    None
}

fn parse_c_identifier(source: &str, offset: usize) -> Option<(&str, usize)> {
    let mut chars = source[offset..].char_indices();
    let (_, first) = chars.next()?;
    if !is_c_identifier_start(first) {
        return None;
    }

    let mut end = offset + first.len_utf8();
    for (idx, ch) in chars {
        if !is_c_identifier_continue(ch) {
            break;
        }
        end = offset + idx + ch.len_utf8();
    }
    Some((&source[offset..end], end))
}

fn skip_ascii_whitespace(source: &str, mut offset: usize) -> usize {
    while offset < source.len() {
        let byte = source.as_bytes()[offset];
        if !byte.is_ascii_whitespace() {
            break;
        }
        offset += 1;
    }
    offset
}

fn skip_balanced_delimiters(
    source: &str,
    mut offset: usize,
    open: char,
    close: char,
) -> Option<usize> {
    let mut depth = 1usize;
    while offset < source.len() {
        let ch = source[offset..].chars().next()?;
        offset += ch.len_utf8();
        match ch {
            ch if ch == open => depth += 1,
            ch if ch == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn identifier_is_variable_definition(source: &str, start: usize, end: usize) -> bool {
    let prefix = statement_prefix(source, start);
    if prefix.is_empty() || prefix.contains('=') || prefix.ends_with(':') {
        return false;
    }
    if prefix
        .split_whitespace()
        .any(|token| matches!(token, "extern" | "return" | "goto"))
    {
        return false;
    }
    if matches!(
        first_identifier(prefix),
        Some("if" | "for" | "while" | "switch" | "sizeof" | "typeof" | "defined")
    ) {
        return false;
    }

    let mut offset = skip_ascii_whitespace(source, end);
    while source[offset..].starts_with('[') {
        let Some(next) = skip_balanced_delimiters(source, offset + 1, '[', ']') else {
            return false;
        };
        offset = skip_ascii_whitespace(source, next);
    }

    if source[offset..].starts_with('(') {
        return false;
    }

    loop {
        if source[offset..].starts_with([';', '=', ',']) {
            return true;
        }
        let Some((_, next)) = parse_c_identifier(source, offset) else {
            return false;
        };
        offset = skip_ascii_whitespace(source, next);
        if source[offset..].starts_with('(') {
            let Some(next) = skip_balanced_delimiters(source, offset + 1, '(', ')') else {
                return false;
            };
            offset = skip_ascii_whitespace(source, next);
            continue;
        }
    }
}

fn definition_is_static(source: &str, start: usize) -> bool {
    statement_prefix(source, start)
        .split_whitespace()
        .any(|token| token == "static")
}

fn statement_prefix(source: &str, start: usize) -> &str {
    let statement_start = source[..start]
        .rfind([';', '{', '}', '\n'])
        .map_or(0, |index| index + 1);
    source[statement_start..start].trim()
}

fn first_identifier(source: &str) -> Option<&str> {
    next_identifier(source, 0, 1).map(|(_, token, _)| token)
}

fn is_c_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_c_identifier_continue(ch: char) -> bool {
    is_c_identifier_start(ch) || ch.is_ascii_digit()
}

fn mask_c_comments_and_literals(content: &str) -> String {
    #[derive(Clone, Copy)]
    enum State {
        Normal,
        LineComment,
        BlockComment,
        StringLiteral,
        CharLiteral,
    }

    let mut out = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    let mut state = State::Normal;

    while let Some(ch) = chars.next() {
        match state {
            State::Normal if ch == '/' && chars.peek() == Some(&'/') => {
                out.push(' ');
                out.push(' ');
                chars.next();
                state = State::LineComment;
            }
            State::Normal if ch == '/' && chars.peek() == Some(&'*') => {
                out.push(' ');
                out.push(' ');
                chars.next();
                state = State::BlockComment;
            }
            State::Normal if ch == '"' => {
                out.push(' ');
                state = State::StringLiteral;
            }
            State::Normal if ch == '\'' => {
                out.push(' ');
                state = State::CharLiteral;
            }
            State::Normal => out.push(ch),
            State::LineComment if ch == '\n' => {
                out.push('\n');
                state = State::Normal;
            }
            State::LineComment => out.push(' '),
            State::BlockComment if ch == '*' && chars.peek() == Some(&'/') => {
                out.push(' ');
                out.push(' ');
                chars.next();
                state = State::Normal;
            }
            State::BlockComment if ch == '\n' => out.push('\n'),
            State::BlockComment => out.push(' '),
            State::StringLiteral if ch == '\\' => {
                out.push(' ');
                if let Some(escaped) = chars.next() {
                    out.push(if escaped == '\n' { '\n' } else { ' ' });
                }
            }
            State::StringLiteral if ch == '"' => {
                out.push(' ');
                state = State::Normal;
            }
            State::StringLiteral if ch == '\n' => {
                out.push('\n');
                state = State::Normal;
            }
            State::StringLiteral => out.push(' '),
            State::CharLiteral if ch == '\\' => {
                out.push(' ');
                if let Some(escaped) = chars.next() {
                    out.push(if escaped == '\n' { '\n' } else { ' ' });
                }
            }
            State::CharLiteral if ch == '\'' => {
                out.push(' ');
                state = State::Normal;
            }
            State::CharLiteral if ch == '\n' => {
                out.push('\n');
                state = State::Normal;
            }
            State::CharLiteral => out.push(' '),
        }
    }

    out
}

fn render_live_references(references: &BTreeSet<LiveEntryPointReference>) -> String {
    references
        .iter()
        .take(8)
        .map(|reference| {
            format!(
                "{}:{}:{}",
                reference.file.display(),
                reference.line,
                reference.entry_point,
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_runtime_registrations_ignores_comments_and_strings() {
        let scan = scan_runtime_registrations_in_content(
            Path::new("drivers/foo/provider.c"),
            concat!(
                "// module_init(commented_out)\n",
                "const char *s = \"module_init(in_string)\";\n",
                "static int real_init(void) { return 0; }\n",
                "module_init(real_init);\n",
            ),
        );

        assert!(scan.malformed.is_empty());
        assert_eq!(
            scan.registrations
                .iter()
                .map(|registration| {
                    (
                        registration.registration_macro.as_str(),
                        registration.entry_points.clone(),
                        registration.line,
                    )
                })
                .collect::<Vec<_>>(),
            vec![("module_init", vec![String::from("real_init")], 4)]
        );
    }

    #[test]
    fn test_scan_runtime_registrations_collects_multiple_entry_points() {
        let scan = scan_runtime_registrations_in_content(
            Path::new("drivers/foo/provider.c"),
            "module_platform_driver_probe(foo_driver, foo_probe);\nplatform_driver_register(&bar_driver);\n",
        );

        assert!(scan.malformed.is_empty());
        assert_eq!(
            scan.registrations
                .iter()
                .map(|registration| registration.entry_points.clone())
                .collect::<Vec<_>>(),
            vec![
                vec![String::from("foo_driver"), String::from("foo_probe")],
                vec![String::from("bar_driver")],
            ]
        );
    }

    #[test]
    fn test_prove_removed_runtime_registration_rejects_live_entry_point() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "drivers/foo/provider.c",
            "static int foo_init(void) { return 0; }\nmodule_init(foo_init);\n",
        );
        write(
            root,
            "drivers/live/user.c",
            "extern int foo_init(void);\nint call(void) { return foo_init(); }\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("drivers/foo/provider.c")]);
        let removed_files = removed_paths.clone();

        let err = format!(
            "{:#}",
            prove_removed_runtime_registrations_have_no_live_entry_points(
                root,
                &removed_paths,
                &BTreeSet::new(),
                &removed_files,
            )
            .unwrap_err()
        );

        assert!(err.contains("runtime registration removal requires proof"));
        assert!(err.contains("drivers/live/user.c"));
        assert!(err.contains("foo_init"));
    }

    #[test]
    fn test_prove_removed_runtime_registration_allows_only_removed_entry_points() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "drivers/foo/provider.c",
            "static int foo_init(void) { return 0; }\nmodule_init(foo_init);\n",
        );
        write(
            root,
            "drivers/foo/internal.c",
            "extern int foo_init(void);\nint call(void) { return foo_init(); }\n",
        );
        write(root, "drivers/live/user.c", "int live;\n");
        let removed_paths = BTreeSet::from([PathBuf::from("drivers/foo")]);
        let removed_dirs = removed_paths.clone();

        let proofs = prove_removed_runtime_registrations_have_no_live_entry_points(
            root,
            &removed_paths,
            &removed_dirs,
            &BTreeSet::new(),
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([RuntimeRegistrationRemovalProof {
                provider: PathBuf::from("drivers/foo/provider.c"),
                registration_macro: String::from("module_init"),
                entry_points: vec![String::from("foo_init")],
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_runtime_registration_rejects_malformed_entry_point() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "drivers/foo/provider.c", "module_init();\n");
        let removed_paths = BTreeSet::from([PathBuf::from("drivers/foo/provider.c")]);
        let removed_files = removed_paths.clone();

        let err = format!(
            "{:#}",
            prove_removed_runtime_registrations_have_no_live_entry_points(
                root,
                &removed_paths,
                &BTreeSet::new(),
                &removed_files,
            )
            .unwrap_err()
        );

        assert!(err.contains("parsable entry-point proof"));
        assert!(err.contains("drivers/foo/provider.c:1"));
    }

    #[test]
    fn test_prove_removed_runtime_registration_rejects_unsupported_expression() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "drivers/foo/provider.c",
            "module_init(foo_init + 1);\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("drivers/foo/provider.c")]);
        let removed_files = removed_paths.clone();

        let err = format!(
            "{:#}",
            prove_removed_runtime_registrations_have_no_live_entry_points(
                root,
                &removed_paths,
                &BTreeSet::new(),
                &removed_files,
            )
            .unwrap_err()
        );

        assert!(err.contains("parsable entry-point proof"));
        assert!(err.contains("drivers/foo/provider.c:1"));
    }

    #[test]
    fn test_prove_removed_runtime_registration_accepts_member_expression_entry_point() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "drivers/foo/provider.c",
            concat!(
                "static struct { int i2c_driver; } foo_driver;\n",
                "static int foo_init(void) { return i2c_add_driver(&foo_driver.i2c_driver); }\n",
            ),
        );
        let removed_paths = BTreeSet::from([PathBuf::from("drivers/foo/provider.c")]);
        let removed_files = removed_paths.clone();

        let proofs = prove_removed_runtime_registrations_have_no_live_entry_points(
            root,
            &removed_paths,
            &BTreeSet::new(),
            &removed_files,
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([RuntimeRegistrationRemovalProof {
                provider: PathBuf::from("drivers/foo/provider.c"),
                registration_macro: String::from("i2c_add_driver"),
                entry_points: vec![String::from("foo_driver")],
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_runtime_registration_accepts_indexed_member_expression_entry_point() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "drivers/foo/provider.c",
            concat!(
                "struct miscdevice { int x; };\n",
                "struct wrapper { struct miscdevice misc; };\n",
                "struct state { struct wrapper devs[3]; };\n",
                "static int foo(void) {\n",
                "\tstruct state *p;\n",
                "\tint i = 0;\n",
                "\treturn misc_register(&p->devs[i].misc);\n",
                "}\n",
            ),
        );
        let removed_paths = BTreeSet::from([PathBuf::from("drivers/foo/provider.c")]);
        let removed_files = removed_paths.clone();

        let proofs = prove_removed_runtime_registrations_have_no_live_entry_points(
            root,
            &removed_paths,
            &BTreeSet::new(),
            &removed_files,
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([RuntimeRegistrationRemovalProof {
                provider: PathBuf::from("drivers/foo/provider.c"),
                registration_macro: String::from("misc_register"),
                entry_points: vec![String::from("p")],
                line: 7,
            }])
        );
    }

    #[test]
    fn test_prove_removed_runtime_registration_ignores_file_local_static_collision() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "drivers/foo/provider.c",
            "static int foo_init(void) { return 0; }\nmodule_init(foo_init);\n",
        );
        write(
            root,
            "drivers/live/user.c",
            "static int foo_init(void) { return 1; }\nint keep = 1;\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("drivers/foo/provider.c")]);
        let removed_files = removed_paths.clone();

        let proofs = prove_removed_runtime_registrations_have_no_live_entry_points(
            root,
            &removed_paths,
            &BTreeSet::new(),
            &removed_files,
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([RuntimeRegistrationRemovalProof {
                provider: PathBuf::from("drivers/foo/provider.c"),
                registration_macro: String::from("module_init"),
                entry_points: vec![String::from("foo_init")],
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_runtime_registration_ignores_live_global_definition_shadow() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "arch/alpha/kernel/pci.c",
            "static int pcibios_init(void) { return 0; }\nsubsys_initcall(pcibios_init);\n",
        );
        write(
            root,
            "arch/x86/pci/common.c",
            "int pcibios_init(void) { return 0; }\n",
        );
        write(
            root,
            "arch/x86/pci/legacy.c",
            "int live(void) { return pcibios_init(); }\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("arch/alpha")]);
        let removed_dirs = removed_paths.clone();

        let proofs = prove_removed_runtime_registrations_have_no_live_entry_points(
            root,
            &removed_paths,
            &removed_dirs,
            &BTreeSet::new(),
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([RuntimeRegistrationRemovalProof {
                provider: PathBuf::from("arch/alpha/kernel/pci.c"),
                registration_macro: String::from("subsys_initcall"),
                entry_points: vec![String::from("pcibios_init")],
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_runtime_registration_ignores_struct_tag_name_references() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "sound/soc/renesas/fsi.c",
            "static struct platform_driver fsi_driver = { 0 };\nmodule_platform_driver(fsi_driver);\n",
        );
        write(
            root,
            "drivers/fsi/fsi-core.c",
            "int fsi_driver_register(struct fsi_driver *fsi_drv) { return 0; }\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("sound/soc/renesas/fsi.c")]);
        let removed_files = removed_paths.clone();

        let proofs = prove_removed_runtime_registrations_have_no_live_entry_points(
            root,
            &removed_paths,
            &BTreeSet::new(),
            &removed_files,
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([RuntimeRegistrationRemovalProof {
                provider: PathBuf::from("sound/soc/renesas/fsi.c"),
                registration_macro: String::from("module_platform_driver"),
                entry_points: vec![String::from("fsi_driver")],
                line: 2,
            }])
        );
    }

    fn write(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }
}
