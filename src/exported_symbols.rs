//! Conservative exported-symbol removal proof.
//!
//! Removing a file that provides an `EXPORT_SYMBOL*()` entry can break live
//! consumers outside the removed subtree. This scanner is intentionally simple:
//! it proves absence of live textual C/ASM/C++ translation-unit consumers, and
//! fails closed when a removed provider uses an unsupported export form.

use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::model::ExportedSymbol;
use crate::path_policy::normalized_relative_path_covers;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ExportedSymbolRemovalProof {
    pub symbol: ExportedSymbol,
    pub provider: PathBuf,
    pub export_macro: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ExportedSymbolDefinition {
    symbol: ExportedSymbol,
    provider: PathBuf,
    export_macro: String,
    line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MalformedExport {
    file: PathBuf,
    line: usize,
    export_macro: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LiveSymbolConsumer {
    file: PathBuf,
    line: usize,
    symbol: ExportedSymbol,
}

pub(crate) fn prove_removed_exports_have_no_live_consumers(
    root: &Path,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> Result<BTreeSet<ExportedSymbolRemovalProof>> {
    let source_files = source_files(root)?;
    let mut removed_exports = BTreeSet::new();
    let mut live_sources = Vec::new();

    for relative in &source_files {
        if path_is_removed(relative, removed_paths, removed_dirs, removed_files) {
            let scan = scan_exported_symbols_in_file(root, relative)?;
            if let Some(malformed) = scan.malformed.into_iter().next() {
                anyhow::bail!(
                    "exported symbol provider removal requires parsable EXPORT_SYMBOL proof; unsupported {} invocation in {}:{}",
                    malformed.export_macro,
                    malformed.file.display(),
                    malformed.line,
                );
            }
            removed_exports.extend(scan.definitions);
        } else {
            live_sources.push(relative.clone());
        }
    }

    let removed_symbols = removed_exports
        .iter()
        .map(|export| export.symbol.clone())
        .collect::<BTreeSet<_>>();
    let removed_arch_macros = removed_arch_preprocessor_macros(removed_paths, removed_dirs);
    let shadowed_symbols = live_shadowed_removed_symbols(
        root,
        &removed_symbols,
        removed_paths,
        removed_dirs,
        removed_files,
        &removed_arch_macros,
    )?;
    let live_consumers = live_consumers_for_symbols(
        root,
        &live_sources,
        &removed_symbols,
        &shadowed_symbols,
        &removed_arch_macros,
    )?;

    let mut proofs = BTreeSet::new();
    for export in removed_exports {
        if let Some(consumers) = live_consumers.get(&export.symbol) {
            anyhow::bail!(
                "exported symbol provider removal requires proof that no live consumer remains for '{}' exported by {}:{}; live consumer(s): {}",
                export.symbol.as_str(),
                export.provider.display(),
                export.line,
                render_consumers(consumers),
            );
        }
        proofs.insert(ExportedSymbolRemovalProof {
            symbol: export.symbol,
            provider: export.provider,
            export_macro: export.export_macro,
            line: export.line,
        });
    }

    Ok(proofs)
}

#[derive(Debug, Default)]
struct ExportScan {
    definitions: BTreeSet<ExportedSymbolDefinition>,
    malformed: BTreeSet<MalformedExport>,
}

fn scan_exported_symbols_in_file(root: &Path, relative: &Path) -> Result<ExportScan> {
    let content = std::fs::read_to_string(root.join(relative)).with_context(|| {
        format!(
            "failed to read exported-symbol provider {}",
            relative.display()
        )
    })?;
    Ok(scan_exported_symbols_in_content(relative, &content))
}

fn scan_exported_symbols_in_content(relative: &Path, content: &str) -> ExportScan {
    let source = mask_c_comments_and_literals(content);
    let mut scan = ExportScan::default();
    let mut offset = 0usize;
    let mut line = 1usize;

    while let Some((start, token, token_line)) = next_identifier(&source, offset, line) {
        line = token_line;
        offset = start + token.len();
        if !is_export_macro(token) {
            continue;
        }

        let after_token = skip_ascii_whitespace(&source, offset);
        let Some(after_open) = source[after_token..]
            .strip_prefix('(')
            .map(|_| after_token + 1)
        else {
            scan.malformed.insert(MalformedExport {
                file: relative.to_path_buf(),
                line: token_line,
                export_macro: token.to_string(),
            });
            continue;
        };
        let first_arg = skip_ascii_whitespace(&source, after_open);
        let Some((symbol, _end)) = parse_exported_symbol_token(&source, first_arg) else {
            scan.malformed.insert(MalformedExport {
                file: relative.to_path_buf(),
                line: token_line,
                export_macro: token.to_string(),
            });
            continue;
        };
        scan.definitions.insert(ExportedSymbolDefinition {
            symbol: ExportedSymbol::new(symbol)
                .expect("parse_exported_symbol_token should return valid exported symbol"),
            provider: relative.to_path_buf(),
            export_macro: token.to_string(),
            line: token_line,
        });
    }

    scan
}

fn live_consumers_for_symbols(
    root: &Path,
    live_sources: &[PathBuf],
    removed_symbols: &BTreeSet<ExportedSymbol>,
    shadowed_symbols: &BTreeSet<ExportedSymbol>,
    removed_arch_macros: &BTreeSet<String>,
) -> Result<BTreeMap<ExportedSymbol, BTreeSet<LiveSymbolConsumer>>> {
    let mut consumers = BTreeMap::<ExportedSymbol, BTreeSet<LiveSymbolConsumer>>::new();
    if removed_symbols.is_empty() {
        return Ok(consumers);
    }

    for relative in live_sources {
        let content = std::fs::read_to_string(root.join(relative)).with_context(|| {
            format!(
                "failed to read live source while proving no consumers for removed exported symbols: {}",
                relative.display(),
            )
        })?;
        for (symbol, lines) in identifier_occurrence_lines_for_symbols(
            &content,
            removed_symbols,
            shadowed_symbols,
            removed_arch_macros,
        ) {
            let symbol_consumers = consumers.entry(symbol.clone()).or_default();
            for line in lines {
                symbol_consumers.insert(LiveSymbolConsumer {
                    file: relative.clone(),
                    line,
                    symbol: symbol.clone(),
                });
            }
        }
    }
    Ok(consumers)
}

fn live_shadowed_removed_symbols(
    root: &Path,
    removed_symbols: &BTreeSet<ExportedSymbol>,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
    removed_arch_macros: &BTreeSet<String>,
) -> Result<BTreeSet<ExportedSymbol>> {
    let mut shadowed_symbols = BTreeSet::new();
    if removed_symbols.is_empty() {
        return Ok(shadowed_symbols);
    }

    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry.path().strip_prefix(root).with_context(|| {
            format!(
                "failed to derive root-relative exported-symbol macro scan path for {}",
                entry.path().display()
            )
        })?;
        if path_is_removed(relative, removed_paths, removed_dirs, removed_files)
            || !is_header_or_source_path(relative)
        {
            continue;
        }

        let content = std::fs::read_to_string(entry.path()).with_context(|| {
            format!(
                "failed to read live file while scanning exported-symbol shadows: {}",
                relative.display(),
            )
        })?;
        for symbol in
            macro_defined_removed_symbols_in_content(&content, removed_symbols, removed_arch_macros)
        {
            shadowed_symbols.insert(symbol);
        }
        for symbol in function_defined_removed_symbols_in_content(
            &content,
            removed_symbols,
            removed_arch_macros,
        ) {
            shadowed_symbols.insert(symbol);
        }
        if is_c_or_asm_source_path(relative) {
            for definition in scan_exported_symbols_in_content(relative, &content).definitions {
                if removed_symbols.contains(&definition.symbol) {
                    shadowed_symbols.insert(definition.symbol);
                }
            }
        }
    }

    Ok(shadowed_symbols)
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
                "failed to derive root-relative exported-symbol scan path for {}",
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

fn is_header_or_source_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("h" | "c" | "S" | "s" | "cc" | "cpp" | "cxx")
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
    if local_static_symbols.contains(symbol) {
        return BTreeSet::new();
    }
    let mut lines = BTreeSet::new();
    let mut offset = 0usize;
    let mut line = 1usize;

    while let Some((start, token, token_line)) = next_identifier(&source, offset, line) {
        line = token_line;
        offset = start + token.len();
        if token == symbol && !identifier_is_member_access(&source, start) {
            lines.insert(token_line);
        }
    }

    lines
}

fn identifier_occurrence_lines_for_symbols(
    content: &str,
    removed_symbols: &BTreeSet<ExportedSymbol>,
    shadowed_symbols: &BTreeSet<ExportedSymbol>,
    removed_arch_macros: &BTreeSet<String>,
) -> BTreeMap<ExportedSymbol, BTreeSet<usize>> {
    let source = mask_c_comments_and_literals(&mask_removed_arch_preprocessor_blocks(
        content,
        removed_arch_macros,
    ));
    let local_static_symbols = file_local_static_function_symbols(&source);
    let mut lines = BTreeMap::<ExportedSymbol, BTreeSet<usize>>::new();
    let mut offset = 0usize;
    let mut line = 1usize;

    while let Some((start, token, token_line)) = next_identifier(&source, offset, line) {
        line = token_line;
        offset = start + token.len();
        if !removed_symbols.contains(token)
            || local_static_symbols.contains(token)
            || identifier_is_member_access(&source, start)
        {
            continue;
        }
        let symbol = ExportedSymbol::new(token)
            .expect("next_identifier should only emit valid exported symbol tokens");
        if shadowed_symbols.contains(&symbol) {
            continue;
        }
        lines.entry(symbol).or_default().insert(token_line);
    }

    lines
}

fn macro_defined_removed_symbols_in_content(
    content: &str,
    removed_symbols: &BTreeSet<ExportedSymbol>,
    removed_arch_macros: &BTreeSet<String>,
) -> BTreeSet<ExportedSymbol> {
    let content = mask_removed_arch_preprocessor_blocks(content, removed_arch_macros);
    let mut symbols = BTreeSet::new();
    for line in content.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix('#') else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix("define") else {
            continue;
        };
        let rest = rest.trim_start();
        let Some((name, _end)) = parse_macro_name(rest) else {
            continue;
        };
        if let Ok(symbol) = ExportedSymbol::new(name) {
            if removed_symbols.contains(&symbol) && exported_symbol_looks_macro_like(&symbol) {
                symbols.insert(symbol);
            }
        }
    }
    symbols
}

fn function_defined_removed_symbols_in_content(
    content: &str,
    removed_symbols: &BTreeSet<ExportedSymbol>,
    removed_arch_macros: &BTreeSet<String>,
) -> BTreeSet<ExportedSymbol> {
    let source = mask_c_comments_and_literals(&mask_removed_arch_preprocessor_blocks(
        content,
        removed_arch_macros,
    ));
    let mut symbols = BTreeSet::new();
    let mut offset = 0usize;
    let mut line = 1usize;

    while let Some((start, token, token_line)) = next_identifier(&source, offset, line) {
        line = token_line;
        offset = start + token.len();
        if !removed_symbols.contains(token) || identifier_is_member_access(&source, start) {
            continue;
        }
        let after_name = skip_ascii_whitespace(&source, offset);
        let Some(after_open) = source[after_name..].strip_prefix('(').map(|_| after_name + 1) else {
            continue;
        };
        let Some(after_close) = skip_balanced_parens(&source, after_open) else {
            continue;
        };
        let after_sig = skip_ascii_whitespace(&source, after_close);
        if !source[after_sig..].starts_with('{') {
            continue;
        }
        if let Ok(symbol) = ExportedSymbol::new(token) {
            symbols.insert(symbol);
        }
    }

    symbols
}

fn parse_macro_name(source: &str) -> Option<(&str, usize)> {
    let mut chars = source.char_indices();
    let (_, first) = chars.next()?;
    if !first.is_ascii_uppercase() && first != '_' {
        return None;
    }

    let mut end = first.len_utf8();
    for (idx, ch) in chars {
        if !(ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_') {
            break;
        }
        end = idx + ch.len_utf8();
    }
    Some((&source[..end], end))
}

fn exported_symbol_looks_macro_like(symbol: &ExportedSymbol) -> bool {
    let text = symbol.as_str();
    text.chars().any(|ch| ch.is_ascii_alphabetic())
        && text
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
}

fn skip_balanced_parens(source: &str, mut offset: usize) -> Option<usize> {
    let mut depth = 1usize;
    while offset < source.len() {
        let ch = source[offset..].chars().next()?;
        offset += ch.len_utf8();
        match ch {
            '(' => depth += 1,
            ')' => {
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

fn removed_arch_preprocessor_macros(
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
) -> BTreeSet<String> {
    let mut macros = BTreeSet::new();
    for path in removed_paths.iter().chain(removed_dirs.iter()) {
        let mut components = path.components();
        let Some(first) = components.next() else {
            continue;
        };
        let Some(second) = components.next() else {
            continue;
        };
        if first.as_os_str() != "arch" {
            continue;
        }
        let Some(arch) = second.as_os_str().to_str() else {
            continue;
        };
        for macro_name in arch_guard_macros(arch) {
            macros.insert((*macro_name).to_string());
        }
    }
    macros
}

fn arch_guard_macros(arch: &str) -> &'static [&'static str] {
    match arch {
        "alpha" => &["__alpha__"],
        "arc" => &["__arc__"],
        "arm" => &["__arm__", "CONFIG_ARM"],
        "csky" => &["__csky__", "CONFIG_CSKY"],
        "hexagon" => &["__hexagon__", "CONFIG_HEXAGON"],
        "loongarch" => &["__loongarch__", "CONFIG_LOONGARCH"],
        "m68k" => &["__m68k__", "CONFIG_M68K"],
        "microblaze" => &["__microblaze__", "CONFIG_MICROBLAZE"],
        "mips" => &["__mips__", "CONFIG_MIPS"],
        "nios2" => &["__nios2__", "CONFIG_NIOS2"],
        "openrisc" => &["__or1k__", "CONFIG_OPENRISC"],
        "parisc" => &["__hppa__", "CONFIG_PARISC"],
        "powerpc" => &["__powerpc__", "CONFIG_PPC", "CONFIG_PPC32", "CONFIG_PPC64"],
        "s390" => &["__s390__", "CONFIG_S390"],
        "sh" => &["__sh__", "CONFIG_SUPERH"],
        "sparc" => &["__sparc__", "CONFIG_SPARC", "CONFIG_SPARC32", "CONFIG_SPARC64"],
        "um" => &["CONFIG_UML"],
        "xtensa" => &["__XTENSA__", "CONFIG_XTENSA"],
        _ => &[],
    }
}

fn mask_removed_arch_preprocessor_blocks(
    content: &str,
    removed_arch_macros: &BTreeSet<String>,
) -> String {
    if removed_arch_macros.is_empty() {
        return content.to_string();
    }

    #[derive(Clone, Copy)]
    struct Frame {
        parent_active: bool,
        current_active: bool,
        known: bool,
        branch_taken: bool,
    }

    let mut out = String::with_capacity(content.len());
    let mut stack: Vec<Frame> = Vec::new();
    let mut active = true;

    for segment in content.split_inclusive('\n') {
        let trimmed = segment.trim_start();
        if let Some(rest) = trimmed.strip_prefix("#ifdef") {
            let token = rest.trim();
            let (known, branch_active, branch_taken) = if removed_arch_macros.contains(token) {
                (true, false, false)
            } else {
                (false, active, true)
            };
            stack.push(Frame {
                parent_active: active,
                current_active: branch_active && active,
                known,
                branch_taken,
            });
            active = branch_active && stack.last().unwrap().parent_active;
            out.push_str(segment);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#ifndef") {
            let token = rest.trim();
            let (known, branch_active, branch_taken) = if removed_arch_macros.contains(token) {
                (true, active, true)
            } else {
                (false, active, true)
            };
            stack.push(Frame {
                parent_active: active,
                current_active: branch_active,
                known,
                branch_taken,
            });
            active = branch_active;
            out.push_str(segment);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#if") {
            let expr = rest.trim();
            let eval = simple_removed_arch_if_eval(expr, removed_arch_macros);
            let (known, branch_active, branch_taken) = match eval {
                Some(true) => (true, false, false),
                Some(false) => (true, active, true),
                None => (false, active, true),
            };
            stack.push(Frame {
                parent_active: active,
                current_active: branch_active,
                known,
                branch_taken,
            });
            active = branch_active;
            out.push_str(segment);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#elif") {
            let expr = rest.trim();
            if let Some(frame) = stack.last_mut() {
                if frame.known {
                    if frame.branch_taken {
                        frame.current_active = false;
                    } else {
                        match simple_removed_arch_if_eval(expr, removed_arch_macros) {
                            Some(true) => frame.current_active = false,
                            Some(false) => {
                                frame.current_active = frame.parent_active;
                                frame.branch_taken = true;
                            }
                            None => frame.current_active = frame.parent_active,
                        }
                    }
                    active = frame.parent_active && frame.current_active;
                }
            }
            out.push_str(segment);
            continue;
        }
        if trimmed.starts_with("#else") {
            if let Some(frame) = stack.last_mut() {
                if frame.known {
                    frame.current_active = frame.parent_active && !frame.branch_taken;
                    frame.branch_taken = true;
                    active = frame.current_active;
                }
            }
            out.push_str(segment);
            continue;
        }
        if trimmed.starts_with("#endif") {
            if let Some(frame) = stack.pop() {
                active = frame.parent_active;
            }
            out.push_str(segment);
            continue;
        }

        if active {
            out.push_str(segment);
        } else {
            for ch in segment.chars() {
                out.push(if ch == '\n' { '\n' } else { ' ' });
            }
        }
    }

    out
}

fn simple_removed_arch_if_eval(
    expr: &str,
    removed_arch_macros: &BTreeSet<String>,
) -> Option<bool> {
    let expr = expr.trim();
    if let Some(macro_name) = simple_defined_macro_name(expr) {
        return Some(removed_arch_macros.contains(macro_name));
    }
    if let Some(rest) = expr.strip_prefix('!') {
        let rest = rest.trim();
        if let Some(macro_name) = simple_defined_macro_name(rest) {
            return Some(!removed_arch_macros.contains(macro_name));
        }
    }
    None
}

fn simple_defined_macro_name(expr: &str) -> Option<&str> {
    let expr = expr.trim();
    if let Some(rest) = expr.strip_prefix("defined") {
        let rest = rest.trim();
        if let Some(rest) = rest.strip_prefix('(') {
            let end = rest.find(')')?;
            return Some(rest[..end].trim());
        }
        return Some(rest);
    }
    None
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

fn parse_exported_symbol_token(source: &str, offset: usize) -> Option<(&str, usize)> {
    let mut chars = source[offset..].char_indices();
    let (_, first) = chars.next()?;
    if !is_exported_symbol_start(first) {
        return None;
    }

    let mut end = offset + first.len_utf8();
    for (idx, ch) in chars {
        if !is_exported_symbol_continue(ch) {
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

fn is_export_macro(token: &str) -> bool {
    matches!(
        token,
        "EXPORT_SYMBOL"
            | "EXPORT_SYMBOL_GPL"
            | "EXPORT_SYMBOL_GPL_FUTURE"
            | "EXPORT_SYMBOL_NS"
            | "EXPORT_SYMBOL_NS_GPL"
    )
}

fn is_exported_symbol_start(ch: char) -> bool {
    ch == '$' || ch == '_' || ch.is_ascii_alphabetic()
}

fn is_exported_symbol_continue(ch: char) -> bool {
    is_exported_symbol_start(ch) || ch.is_ascii_digit()
}

fn is_c_identifier_start(ch: char) -> bool {
    is_exported_symbol_start(ch)
}

fn is_c_identifier_continue(ch: char) -> bool {
    is_exported_symbol_continue(ch)
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

fn render_consumers(consumers: &BTreeSet<LiveSymbolConsumer>) -> String {
    consumers
        .iter()
        .take(8)
        .map(|consumer| format!("{}:{}", consumer.file.display(), consumer.line,))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_exported_symbols_ignores_comments_and_strings() {
        let scan = scan_exported_symbols_in_content(
            Path::new("drivers/foo/provider.c"),
            concat!(
                "// EXPORT_SYMBOL(commented_out)\n",
                "const char *s = \"EXPORT_SYMBOL(in_string)\";\n",
                "void real(void) {}\n",
                "EXPORT_SYMBOL_GPL(real);\n",
            ),
        );

        assert!(scan.malformed.is_empty());
        assert_eq!(
            scan.definitions
                .iter()
                .map(|definition| (definition.symbol.as_str(), definition.line))
                .collect::<Vec<_>>(),
            vec![("real", 4)]
        );
    }

    #[test]
    fn test_scan_exported_symbols_accepts_parisc_dollar_symbols() {
        let scan = scan_exported_symbols_in_content(
            Path::new("arch/parisc/kernel/parisc_ksyms.c"),
            concat!(
                "extern int $global$;\n",
                "EXPORT_SYMBOL($global$);\n",
                "extern void $$divI(void);\n",
                "EXPORT_SYMBOL($$divI);\n",
            ),
        );

        assert!(scan.malformed.is_empty());
        assert_eq!(
            scan.definitions
                .iter()
                .map(|definition| (definition.symbol.as_str(), definition.line))
                .collect::<Vec<_>>(),
            vec![("$$divI", 4), ("$global$", 2)]
        );
    }

    #[test]
    fn test_prove_removed_exports_rejects_live_consumer() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "drivers/foo/provider.c",
            "void foo_api(void) {}\nEXPORT_SYMBOL(foo_api);\n",
        );
        write(
            root,
            "drivers/live/user.c",
            "extern void foo_api(void);\nvoid user(void) { foo_api(); }\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("drivers/foo/provider.c")]);
        let removed_files = removed_paths.clone();

        let err = format!(
            "{:#}",
            prove_removed_exports_have_no_live_consumers(
                root,
                &removed_paths,
                &BTreeSet::new(),
                &removed_files,
            )
            .unwrap_err()
        );

        assert!(err.contains("exported symbol provider removal requires proof"));
        assert!(err.contains("foo_api"));
        assert!(err.contains("drivers/live/user.c"));
    }

    #[test]
    fn test_prove_removed_exports_allows_only_removed_consumers() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "drivers/foo/provider.c",
            "void foo_api(void) {}\nEXPORT_SYMBOL_NS(foo_api, NS);\n",
        );
        write(
            root,
            "drivers/foo/user.c",
            "extern void foo_api(void);\nvoid user(void) { foo_api(); }\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("drivers/foo")]);
        let removed_dirs = removed_paths.clone();

        let proofs = prove_removed_exports_have_no_live_consumers(
            root,
            &removed_paths,
            &removed_dirs,
            &BTreeSet::new(),
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([ExportedSymbolRemovalProof {
                symbol: ExportedSymbol::new("foo_api").unwrap(),
                provider: PathBuf::from("drivers/foo/provider.c"),
                export_macro: String::from("EXPORT_SYMBOL_NS"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_ignores_live_header_only_mentions() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "net/sunrpc/xdr.c",
            "void __xdr_commit_encode(void) {}\nEXPORT_SYMBOL_GPL(__xdr_commit_encode);\n",
        );
        write(
            root,
            "include/linux/sunrpc/xdr.h",
            concat!(
                "extern void __xdr_commit_encode(void);\n",
                "static inline void xdr_commit_encode(void)\n",
                "{\n",
                "\t__xdr_commit_encode();\n",
                "}\n",
            ),
        );
        let removed_paths = BTreeSet::from([PathBuf::from("net/sunrpc")]);
        let removed_dirs = removed_paths.clone();

        let proofs = prove_removed_exports_have_no_live_consumers(
            root,
            &removed_paths,
            &removed_dirs,
            &BTreeSet::new(),
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([ExportedSymbolRemovalProof {
                symbol: ExportedSymbol::new("__xdr_commit_encode").unwrap(),
                provider: PathBuf::from("net/sunrpc/xdr.c"),
                export_macro: String::from("EXPORT_SYMBOL_GPL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_ignores_struct_field_name_collisions() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "net/sunrpc/cache.c",
            "void cache_flush(void) {}\nEXPORT_SYMBOL_GPL(cache_flush);\n",
        );
        write(
            root,
            "include/linux/ops.h",
            "struct ops { void (*cache_flush)(void); };\n",
        );
        write(
            root,
            "drivers/live/user.c",
            concat!(
                "#include <linux/ops.h>\n",
                "static void local_flush(void) {}\n",
                "void user(struct ops *ops)\n",
                "{\n",
                "\tstruct ops defaults = { .cache_flush = local_flush };\n",
                "\tops->cache_flush();\n",
                "\tdefaults.cache_flush();\n",
                "}\n",
            ),
        );
        let removed_paths = BTreeSet::from([PathBuf::from("net/sunrpc")]);
        let removed_dirs = removed_paths.clone();

        let proofs = prove_removed_exports_have_no_live_consumers(
            root,
            &removed_paths,
            &removed_dirs,
            &BTreeSet::new(),
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([ExportedSymbolRemovalProof {
                symbol: ExportedSymbol::new("cache_flush").unwrap(),
                provider: PathBuf::from("net/sunrpc/cache.c"),
                export_macro: String::from("EXPORT_SYMBOL_GPL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_ignores_live_macro_shadow_for_uppercase_symbol() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "arch/powerpc/kernel/setup_32.c",
            "unsigned int DMA_MODE_READ;\nEXPORT_SYMBOL(DMA_MODE_READ);\n",
        );
        write(root, "arch/x86/include/asm/dma.h", "#define DMA_MODE_READ 0x44\n");
        write(
            root,
            "drivers/live/user.c",
            "#include <asm/dma.h>\nvoid user(int chan) { set_dma_mode(chan, DMA_MODE_READ); }\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("arch/powerpc")]);
        let removed_dirs = removed_paths.clone();

        let proofs = prove_removed_exports_have_no_live_consumers(
            root,
            &removed_paths,
            &removed_dirs,
            &BTreeSet::new(),
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([ExportedSymbolRemovalProof {
                symbol: ExportedSymbol::new("DMA_MODE_READ").unwrap(),
                provider: PathBuf::from("arch/powerpc/kernel/setup_32.c"),
                export_macro: String::from("EXPORT_SYMBOL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_ignores_live_function_definition_shadow() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "arch/arm/xen/enlighten.c",
            "int HYPERVISOR_console_io(int cmd, int count, char *str) { return 0; }\nEXPORT_SYMBOL_GPL(HYPERVISOR_console_io);\n",
        );
        write(
            root,
            "arch/x86/include/asm/xen/hypercall.h",
            concat!(
                "static inline int\n",
                "HYPERVISOR_console_io(int cmd, int count, char *str)\n",
                "{\n",
                "\treturn 0;\n",
                "}\n",
            ),
        );
        write(
            root,
            "drivers/tty/hvc/hvc_xen.c",
            "#include <asm/xen/hypercall.h>\nint user(char *buf) { return HYPERVISOR_console_io(0, 1, buf); }\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("arch/arm")]);
        let removed_dirs = removed_paths.clone();

        let proofs = prove_removed_exports_have_no_live_consumers(
            root,
            &removed_paths,
            &removed_dirs,
            &BTreeSet::new(),
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([ExportedSymbolRemovalProof {
                symbol: ExportedSymbol::new("HYPERVISOR_console_io").unwrap(),
                provider: PathBuf::from("arch/arm/xen/enlighten.c"),
                export_macro: String::from("EXPORT_SYMBOL_GPL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_ignores_removed_arch_preprocessor_guarded_consumer() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "arch/sparc/mm/init_64.c",
            "unsigned long _PAGE_CACHE;\nEXPORT_SYMBOL(_PAGE_CACHE);\n",
        );
        write(
            root,
            "drivers/video/fbdev/aty/atyfb_base.c",
            concat!(
                "#ifdef __sparc__\n",
                "void setup(void) { unsigned long prot = _PAGE_CACHE; }\n",
                "#endif\n",
                "void generic(void) {}\n",
            ),
        );
        let removed_paths = BTreeSet::from([PathBuf::from("arch/sparc")]);
        let removed_dirs = removed_paths.clone();

        let proofs = prove_removed_exports_have_no_live_consumers(
            root,
            &removed_paths,
            &removed_dirs,
            &BTreeSet::new(),
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([ExportedSymbolRemovalProof {
                symbol: ExportedSymbol::new("_PAGE_CACHE").unwrap(),
                provider: PathBuf::from("arch/sparc/mm/init_64.c"),
                export_macro: String::from("EXPORT_SYMBOL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_ignores_live_alternate_export_provider() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "arch/s390/kernel/entry.S",
            "SYM_FUNC_START(__WARN_trap)\nSYM_FUNC_END(__WARN_trap)\nEXPORT_SYMBOL(__WARN_trap)\n",
        );
        write(
            root,
            "arch/x86/entry/entry.S",
            "SYM_FUNC_START(__WARN_trap)\nSYM_FUNC_END(__WARN_trap)\nEXPORT_SYMBOL(__WARN_trap)\n",
        );
        write(
            root,
            "arch/x86/kernel/traps.c",
            "extern void __WARN_trap(void);\nvoid use_trap(void) { __WARN_trap(); }\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("arch/s390")]);
        let removed_dirs = removed_paths.clone();

        let proofs = prove_removed_exports_have_no_live_consumers(
            root,
            &removed_paths,
            &removed_dirs,
            &BTreeSet::new(),
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([ExportedSymbolRemovalProof {
                symbol: ExportedSymbol::new("__WARN_trap").unwrap(),
                provider: PathBuf::from("arch/s390/kernel/entry.S"),
                export_macro: String::from("EXPORT_SYMBOL"),
                line: 3,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_ignores_file_local_static_function_collisions() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "net/sunrpc/stats.c",
            "int svc_seq_show(void) { return 0; }\nEXPORT_SYMBOL_GPL(svc_seq_show);\n",
        );
        write(
            root,
            "drivers/live/user.c",
            concat!(
                "struct ops { int (*show)(void); };\n",
                "static int svc_seq_show(void) { return 1; }\n",
                "static struct ops ops = { .show = svc_seq_show };\n",
            ),
        );
        let removed_paths = BTreeSet::from([PathBuf::from("net/sunrpc")]);
        let removed_dirs = removed_paths.clone();

        let proofs = prove_removed_exports_have_no_live_consumers(
            root,
            &removed_paths,
            &removed_dirs,
            &BTreeSet::new(),
        )
        .unwrap();

        assert_eq!(
            proofs,
            BTreeSet::from([ExportedSymbolRemovalProof {
                symbol: ExportedSymbol::new("svc_seq_show").unwrap(),
                provider: PathBuf::from("net/sunrpc/stats.c"),
                export_macro: String::from("EXPORT_SYMBOL_GPL"),
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
