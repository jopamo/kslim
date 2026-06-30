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

#[derive(Debug, Default)]
struct LiveSymbolShadows {
    global_symbols: BTreeSet<ExportedSymbol>,
    file_local_symbols: BTreeMap<PathBuf, BTreeSet<ExportedSymbol>>,
    alternate_export_providers: BTreeSet<ExportedSymbol>,
}

#[derive(Debug, Default)]
struct ScopedSymbols {
    global: BTreeSet<ExportedSymbol>,
    file_local: BTreeSet<ExportedSymbol>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct PreprocessorMacroFacts {
    definitely_undefined: BTreeSet<String>,
    definitely_defined: BTreeSet<String>,
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
    let preprocessor_macro_facts =
        preprocessor_macro_facts(root, removed_paths, removed_dirs, removed_files)?;
    let shadowed_symbols = live_shadowed_removed_symbols(
        root,
        &removed_symbols,
        removed_paths,
        removed_dirs,
        removed_files,
        &preprocessor_macro_facts,
    )?;
    let live_consumers = live_consumers_for_symbols(
        root,
        &live_sources,
        &removed_symbols,
        &shadowed_symbols.file_local_symbols,
        &shadowed_symbols.global_symbols,
        &shadowed_symbols.alternate_export_providers,
        &preprocessor_macro_facts,
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
    file_local_shadowed_symbols: &BTreeMap<PathBuf, BTreeSet<ExportedSymbol>>,
    global_shadowed_symbols: &BTreeSet<ExportedSymbol>,
    alternate_export_providers: &BTreeSet<ExportedSymbol>,
    preprocessor_macro_facts: &PreprocessorMacroFacts,
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
        let empty_shadows = BTreeSet::new();
        let local_shadows = file_local_shadowed_symbols
            .get(relative)
            .unwrap_or(&empty_shadows);
        for (symbol, lines) in identifier_occurrence_lines_for_symbols(
            &content,
            removed_symbols,
            local_shadows,
            global_shadowed_symbols,
            alternate_export_providers,
            preprocessor_macro_facts,
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
    preprocessor_macro_facts: &PreprocessorMacroFacts,
) -> Result<LiveSymbolShadows> {
    let mut shadowed_symbols = LiveSymbolShadows::default();
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
            || !is_kernel_export_scan_path(relative)
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
        let file_local_shadows = shadowed_symbols
            .file_local_symbols
            .entry(relative.to_path_buf())
            .or_default();
        let global_shadows = &mut shadowed_symbols.global_symbols;
        let is_header = matches!(
            relative.extension().and_then(|ext| ext.to_str()),
            Some("h")
        );
        for symbol in
            macro_defined_removed_symbols_in_content(
                &content,
                removed_symbols,
                preprocessor_macro_facts,
            )
        {
            if is_header {
                global_shadows.insert(symbol);
            } else {
                file_local_shadows.insert(symbol);
            }
        }
        let function_symbols = function_defined_removed_symbols_in_content(
            &content,
            removed_symbols,
            preprocessor_macro_facts,
        );
        for symbol in function_symbols.global {
            if is_header {
                global_shadows.insert(symbol);
            } else {
                global_shadows.insert(symbol);
            }
        }
        for symbol in function_symbols.file_local {
            if is_header {
                global_shadows.insert(symbol);
            } else {
                file_local_shadows.insert(symbol);
            }
        }
        let variable_symbols = variable_defined_removed_symbols_in_content(
            &content,
            removed_symbols,
            preprocessor_macro_facts,
        );
        for symbol in variable_symbols.global {
            if is_header {
                global_shadows.insert(symbol);
            } else {
                global_shadows.insert(symbol);
            }
        }
        for symbol in variable_symbols.file_local {
            if is_header {
                global_shadows.insert(symbol);
            } else {
                file_local_shadows.insert(symbol);
            }
        }
        if is_c_or_asm_source_path(relative) {
            for definition in scan_exported_symbols_in_content(relative, &content).definitions {
                if removed_symbols.contains(&definition.symbol) {
                    shadowed_symbols
                        .alternate_export_providers
                        .insert(definition.symbol);
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
        if is_kernel_export_scan_path(relative) && is_c_or_asm_source_path(relative) {
            files.push(relative.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn is_kernel_export_scan_path(path: &Path) -> bool {
    !matches!(
        path.components().next().and_then(|component| component.as_os_str().to_str()),
        Some("tools" | "scripts")
    )
}

fn is_c_or_asm_source_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("c" | "S" | "s" | "cc" | "cpp" | "cxx" | "c_shipped" | "S_shipped" | "s_shipped" | "cc_shipped" | "cpp_shipped" | "cxx_shipped")
    )
}

fn is_header_or_source_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("h" | "h_shipped" | "c" | "S" | "s" | "cc" | "cpp" | "cxx" | "c_shipped" | "S_shipped" | "s_shipped" | "cc_shipped" | "cpp_shipped" | "cxx_shipped")
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
    let source = mask_c_comments_and_literals(&mask_preprocessor_directive_lines(content));
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
            && !identifier_is_member_access(&source, start)
            && !identifier_is_goto_target(&source, start)
            && !identifier_is_label_definition(&source, offset)
        {
            lines.insert(token_line);
        }
    }

    lines
}

fn identifier_occurrence_lines_for_symbols(
    content: &str,
    removed_symbols: &BTreeSet<ExportedSymbol>,
    local_shadowed_symbols: &BTreeSet<ExportedSymbol>,
    global_shadowed_symbols: &BTreeSet<ExportedSymbol>,
    alternate_export_providers: &BTreeSet<ExportedSymbol>,
    preprocessor_macro_facts: &PreprocessorMacroFacts,
) -> BTreeMap<ExportedSymbol, BTreeSet<usize>> {
    let source = mask_c_comments_and_literals(&mask_preprocessor_directive_lines(
        &mask_known_preprocessor_blocks(content, preprocessor_macro_facts),
    ));
    let local_static_symbols = file_local_static_function_symbols(&source);
    let type_body_ranges = type_definition_body_ranges(&source);
    let mut lines = BTreeMap::<ExportedSymbol, BTreeSet<usize>>::new();
    let mut offset = 0usize;
    let mut line = 1usize;

    while let Some((start, token, token_line)) = next_identifier(&source, offset, line) {
        line = token_line;
        offset = start + token.len();
        if !removed_symbols.contains(token)
            || local_static_symbols.contains(token)
            || offset_in_ranges(start, &type_body_ranges)
            || identifier_is_member_access(&source, start)
            || identifier_is_goto_target(&source, start)
            || identifier_is_label_definition(&source, offset)
        {
            continue;
        }
        let symbol = ExportedSymbol::new(token)
            .expect("next_identifier should only emit valid exported symbol tokens");
        if local_shadowed_symbols.contains(&symbol)
            || global_shadowed_symbols.contains(&symbol)
            || alternate_export_providers.contains(&symbol)
        {
            continue;
        }
        lines.entry(symbol).or_default().insert(token_line);
    }

    lines
}

fn macro_defined_removed_symbols_in_content(
    content: &str,
    removed_symbols: &BTreeSet<ExportedSymbol>,
    preprocessor_macro_facts: &PreprocessorMacroFacts,
) -> BTreeSet<ExportedSymbol> {
    let content = mask_known_preprocessor_blocks(content, preprocessor_macro_facts);
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
            if removed_symbols.contains(&symbol) {
                symbols.insert(symbol);
            }
        }
    }
    symbols
}

fn function_defined_removed_symbols_in_content(
    content: &str,
    removed_symbols: &BTreeSet<ExportedSymbol>,
    preprocessor_macro_facts: &PreprocessorMacroFacts,
) -> ScopedSymbols {
    let source = mask_c_comments_and_literals(&mask_preprocessor_directive_lines(
        &mask_known_preprocessor_blocks(content, preprocessor_macro_facts),
    ));
    let mut symbols = ScopedSymbols::default();
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
            if definition_is_static(&source, start) {
                symbols.file_local.insert(symbol);
            } else {
                symbols.global.insert(symbol);
            }
        }
    }

    symbols
}

fn variable_defined_removed_symbols_in_content(
    content: &str,
    removed_symbols: &BTreeSet<ExportedSymbol>,
    preprocessor_macro_facts: &PreprocessorMacroFacts,
) -> ScopedSymbols {
    let source = mask_c_comments_and_literals(&mask_preprocessor_directive_lines(
        &mask_known_preprocessor_blocks(content, preprocessor_macro_facts),
    ));
    let mut symbols = ScopedSymbols::default();
    let mut offset = 0usize;
    let mut line = 1usize;

    while let Some((start, token, token_line)) = next_identifier(&source, offset, line) {
        line = token_line;
        offset = start + token.len();
        if !removed_symbols.contains(token) || identifier_is_member_access(&source, start) {
            continue;
        }
        if !identifier_is_variable_definition(&source, start, offset) {
            continue;
        }
        if let Ok(symbol) = ExportedSymbol::new(token) {
            if definition_is_static(&source, start) {
                symbols.file_local.insert(symbol);
            } else {
                symbols.global.insert(symbol);
            }
        }
    }

    symbols
}

fn parse_macro_name(source: &str) -> Option<(&str, usize)> {
    parse_exported_symbol_token(source, 0)
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

fn skip_balanced_parens(source: &str, offset: usize) -> Option<usize> {
    skip_balanced_delimiters(source, offset, '(', ')')
}

fn preprocessor_macro_facts(
    root: &Path,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> Result<PreprocessorMacroFacts> {
    let removed_arches = removed_arch_names(removed_paths, removed_dirs);
    let mut definitely_undefined = BTreeSet::new();
    for arch in &removed_arches {
        for macro_name in arch_guard_macros(arch) {
            definitely_undefined.insert((*macro_name).to_string());
        }
    }
    definitely_undefined.extend(removed_local_config_macros(
        root,
        removed_paths,
        removed_dirs,
        removed_files,
    )?);
    Ok(PreprocessorMacroFacts {
        definitely_undefined,
        definitely_defined: live_arch_root_selected_config_macros(
            root,
            removed_paths,
            removed_dirs,
            removed_files,
        )?,
    })
}

fn removed_arch_names(
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
) -> BTreeSet<String> {
    removed_paths
        .iter()
        .chain(removed_dirs.iter())
        .filter_map(|path| arch_path_name(path).map(str::to_string))
        .collect()
}

fn arch_path_name(path: &Path) -> Option<&str> {
    let mut components = path.components();
    let first = components.next()?;
    let second = components.next()?;
    if first.as_os_str() != "arch" {
        return None;
    }
    second.as_os_str().to_str()
}

fn removed_local_config_macros(
    root: &Path,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> Result<BTreeSet<String>> {
    if removed_paths.is_empty() && removed_dirs.is_empty() && removed_files.is_empty() {
        return Ok(BTreeSet::new());
    }

    let mut symbols = BTreeMap::<String, (bool, bool)>::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(file_name) = entry.file_name().to_str() else {
            continue;
        };
        if !file_name.starts_with("Kconfig") {
            continue;
        }
        let relative = entry.path().strip_prefix(root).with_context(|| {
            format!(
                "failed to derive root-relative removed-path Kconfig scan path for {}",
                entry.path().display()
            )
        })?;
        let in_removed_path = path_is_removed(relative, removed_paths, removed_dirs, removed_files);
        let content = std::fs::read_to_string(entry.path()).with_context(|| {
            format!(
                "failed to read Kconfig while deriving removed-path preprocessor macros: {}",
                relative.display()
            )
        })?;
        for symbol in kconfig_symbol_definitions(&content) {
            let seen = symbols.entry(symbol.to_string()).or_default();
            if in_removed_path {
                seen.0 = true;
            } else {
                seen.1 = true;
            }
        }
    }

    Ok(symbols
        .into_iter()
        .filter_map(|(symbol, (seen_in_removed_path, seen_outside_removed_path))| {
            if seen_in_removed_path && !seen_outside_removed_path {
                Some(format!("CONFIG_{symbol}"))
            } else {
                None
            }
        })
        .collect())
}

fn kconfig_symbol_definitions(content: &str) -> impl Iterator<Item = &str> {
    content.lines().filter_map(kconfig_symbol_definition)
}

fn kconfig_symbol_definition(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("config ")
        .or_else(|| trimmed.strip_prefix("menuconfig "))?;
    let symbol = rest.split_whitespace().next()?;
    if symbol.is_empty()
        || !symbol
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        return None;
    }
    Some(symbol)
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

fn live_arch_root_selected_config_macros(
    root: &Path,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> Result<BTreeSet<String>> {
    let live_arches = live_arch_names(root, removed_paths, removed_dirs, removed_files)?;
    let mut always_selected = None::<BTreeSet<String>>;

    for arch in live_arches {
        let kconfig = root.join("arch").join(&arch).join("Kconfig");
        let content = match std::fs::read_to_string(&kconfig) {
            Ok(content) => content,
            Err(_) => return Ok(BTreeSet::new()),
        };
        let selected = unconditional_arch_root_selected_symbols(
            &content,
            &arch_root_config_symbol(&arch),
        )
        .into_iter()
        .map(|symbol| format!("CONFIG_{symbol}"))
        .collect::<BTreeSet<_>>();
        always_selected = Some(match always_selected {
            None => selected,
            Some(existing) => existing.intersection(&selected).cloned().collect(),
        });
        if always_selected.as_ref().is_some_and(BTreeSet::is_empty) {
            break;
        }
    }

    Ok(always_selected.unwrap_or_default())
}

fn live_arch_names(
    root: &Path,
    removed_paths: &BTreeSet<PathBuf>,
    removed_dirs: &BTreeSet<PathBuf>,
    removed_files: &BTreeSet<PathBuf>,
) -> Result<BTreeSet<String>> {
    let arch_root = root.join("arch");
    let mut live_arches = BTreeSet::new();
    if !arch_root.is_dir() {
        return Ok(live_arches);
    }
    for entry in std::fs::read_dir(&arch_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let arch_name = entry.file_name();
        let Some(arch) = arch_name.to_str() else {
            continue;
        };
        let relative = Path::new("arch").join(arch);
        if !path_is_removed(&relative, removed_paths, removed_dirs, removed_files) {
            live_arches.insert(arch.to_string());
        }
    }
    Ok(live_arches)
}

fn arch_root_config_symbol(arch: &str) -> String {
    match arch {
        "um" => String::from("UML"),
        "sh" => String::from("SUPERH"),
        "powerpc" => String::from("PPC"),
        _ => arch
            .chars()
            .map(|ch| ch.to_ascii_uppercase())
            .collect(),
    }
}

fn unconditional_arch_root_selected_symbols(
    content: &str,
    root_symbol: &str,
) -> BTreeSet<String> {
    let mut in_root_config = false;
    let mut symbols = BTreeSet::new();

    for line in content.lines() {
        let trimmed = line.trim_start();
        if !in_root_config {
            if kconfig_symbol_definition(line) == Some(root_symbol) {
                in_root_config = true;
            }
            continue;
        }

        if !line.chars().next().is_some_and(char::is_whitespace)
            && !trimmed.is_empty()
            && !trimmed.starts_with('#')
        {
            break;
        }

        let Some(rest) = trimmed.strip_prefix("select ") else {
            continue;
        };
        if rest.contains(" if ") {
            continue;
        }
        let Some(symbol) = rest.split_whitespace().next() else {
            continue;
        };
        if symbol.is_empty()
            || !symbol
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
        {
            continue;
        }
        symbols.insert(symbol.to_string());
    }

    symbols
}

fn mask_known_preprocessor_blocks(
    content: &str,
    preprocessor_macro_facts: &PreprocessorMacroFacts,
) -> String {
    if preprocessor_macro_facts.definitely_undefined.is_empty()
        && preprocessor_macro_facts.definitely_defined.is_empty()
    {
        return content.to_string();
    }

    #[derive(Clone, Copy)]
    struct Frame {
        parent_active: bool,
        current_active: bool,
        known: bool,
        branch_taken: bool,
        parent_certain_active: bool,
        current_certain_active: bool,
    }

    let mut out = String::with_capacity(content.len());
    let mut stack: Vec<Frame> = Vec::new();
    let mut active = true;
    let mut certain_active = true;
    let mut seen_local_define_names = BTreeSet::<String>::new();
    let mut definitely_defined_macros = BTreeSet::<String>::new();
    let mut uncertain_defined_macros = BTreeSet::<String>::new();

    for segment in content.split_inclusive('\n') {
        let trimmed = segment.trim_start();
        if let Some(rest) = trimmed.strip_prefix("#ifdef") {
            let token = rest.trim();
            let (known, selected, branch_taken) =
                if preprocessor_macro_facts.definitely_undefined.contains(token) {
                (true, false, false)
            } else if preprocessor_macro_facts.definitely_defined.contains(token) {
                (true, true, true)
            } else if definitely_defined_macros.contains(token) {
                (true, true, true)
            } else if uncertain_defined_macros.contains(token) {
                (false, true, true)
            } else if seen_local_define_names.contains(token) {
                (true, false, false)
            } else {
                (false, active, true)
            };
            stack.push(Frame {
                parent_active: active,
                current_active: active && selected,
                known,
                branch_taken,
                parent_certain_active: certain_active,
                current_certain_active: certain_active && known && selected,
            });
            active = stack.last().unwrap().current_active;
            certain_active = stack.last().unwrap().current_certain_active;
            out.push_str(segment);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#ifndef") {
            let token = rest.trim();
            let (known, selected, branch_taken) =
                if preprocessor_macro_facts.definitely_undefined.contains(token) {
                (true, true, true)
            } else if preprocessor_macro_facts.definitely_defined.contains(token) {
                (true, false, false)
            } else if definitely_defined_macros.contains(token) {
                (true, false, false)
            } else if uncertain_defined_macros.contains(token) {
                (false, true, true)
            } else if seen_local_define_names.contains(token) {
                (true, true, true)
            } else {
                (false, active, true)
            };
            stack.push(Frame {
                parent_active: active,
                current_active: active && selected,
                known,
                branch_taken,
                parent_certain_active: certain_active,
                current_certain_active: certain_active && known && selected,
            });
            active = stack.last().unwrap().current_active;
            certain_active = stack.last().unwrap().current_certain_active;
            out.push_str(segment);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#if") {
            let expr = rest.trim();
            let eval = simple_known_macro_if_eval(expr, preprocessor_macro_facts);
            let (known, selected, branch_taken) = match eval {
                Some(true) => (true, false, false),
                Some(false) => (true, true, true),
                None => (false, true, true),
            };
            stack.push(Frame {
                parent_active: active,
                current_active: active && selected,
                known,
                branch_taken,
                parent_certain_active: certain_active,
                current_certain_active: certain_active && known && selected,
            });
            active = stack.last().unwrap().current_active;
            certain_active = stack.last().unwrap().current_certain_active;
            out.push_str(segment);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#elif") {
            let expr = rest.trim();
            if let Some(frame) = stack.last_mut() {
                if frame.known {
                    if frame.branch_taken {
                        frame.current_active = false;
                        frame.current_certain_active = false;
                    } else {
                        match simple_known_macro_if_eval(expr, preprocessor_macro_facts) {
                            Some(true) => {
                                frame.current_active = false;
                                frame.current_certain_active = false;
                            }
                            Some(false) => {
                                frame.current_active = frame.parent_active;
                                frame.current_certain_active = frame.parent_certain_active;
                                frame.branch_taken = true;
                            }
                            None => {
                                frame.current_active = frame.parent_active;
                                frame.current_certain_active = false;
                            }
                        }
                    }
                    active = frame.parent_active && frame.current_active;
                    certain_active = frame.parent_certain_active && frame.current_certain_active;
                } else {
                    match simple_known_macro_if_eval(expr, preprocessor_macro_facts) {
                        Some(true) => {
                            frame.current_active = false;
                            frame.current_certain_active = false;
                            active = false;
                            certain_active = false;
                        }
                        Some(false) | None => {
                            frame.current_active = frame.parent_active;
                            frame.current_certain_active = false;
                            active = frame.parent_active;
                            certain_active = false;
                        }
                    }
                }
            }
            out.push_str(segment);
            continue;
        }
        if trimmed.starts_with("#else") {
            if let Some(frame) = stack.last_mut() {
                if frame.known {
                    frame.current_active = frame.parent_active && !frame.branch_taken;
                    frame.current_certain_active =
                        frame.parent_certain_active && !frame.branch_taken;
                    frame.branch_taken = true;
                    active = frame.current_active;
                    certain_active = frame.current_certain_active;
                } else {
                    frame.current_active = frame.parent_active;
                    frame.current_certain_active = false;
                    active = frame.parent_active;
                    certain_active = false;
                }
            }
            out.push_str(segment);
            continue;
        }
        if trimmed.starts_with("#endif") {
            if let Some(frame) = stack.pop() {
                active = frame.parent_active;
                certain_active = frame.parent_certain_active;
            }
            out.push_str(segment);
            continue;
        }

        if let Some(macro_name) = directive_macro_name(trimmed, "define") {
            seen_local_define_names.insert(macro_name.to_string());
            if active {
                if certain_active || definitely_defined_macros.contains(macro_name) {
                    definitely_defined_macros.insert(macro_name.to_string());
                    uncertain_defined_macros.remove(macro_name);
                } else {
                    uncertain_defined_macros.insert(macro_name.to_string());
                }
            }
        } else if let Some(macro_name) = directive_macro_name(trimmed, "undef") {
            if active {
                if certain_active {
                    definitely_defined_macros.remove(macro_name);
                    uncertain_defined_macros.remove(macro_name);
                } else if definitely_defined_macros.remove(macro_name) {
                    uncertain_defined_macros.insert(macro_name.to_string());
                }
            }
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

fn directive_macro_name<'a>(line: &'a str, directive: &str) -> Option<&'a str> {
    let rest = line.strip_prefix('#')?.trim_start();
    let rest = rest.strip_prefix(directive)?.trim_start();
    parse_macro_name(rest).map(|(name, _)| name)
}

fn simple_known_macro_if_eval(
    expr: &str,
    preprocessor_macro_facts: &PreprocessorMacroFacts,
) -> Option<bool> {
    match KnownMacroExprParser::new(expr, preprocessor_macro_facts).parse() {
        RemovedMacroTruth::True => Some(false),
        RemovedMacroTruth::False => Some(true),
        RemovedMacroTruth::Unknown => None,
    }
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

fn simple_macro_predicate_name<'a>(expr: &'a str, predicate: &str) -> Option<&'a str> {
    let expr = expr.trim();
    let rest = expr.strip_prefix(predicate)?.trim();
    let rest = rest.strip_prefix('(')?;
    let end = rest.find(')')?;
    simple_plain_config_macro_name(rest[..end].trim())
}

fn simple_plain_config_macro_name(expr: &str) -> Option<&str> {
    let expr = expr.trim();
    if expr.is_empty()
        || !expr.starts_with("CONFIG_")
        || !expr
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        return None;
    }
    Some(expr)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RemovedMacroTruth {
    True,
    False,
    Unknown,
}

impl RemovedMacroTruth {
    fn not(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }

    fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }

    fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }
}

struct KnownMacroExprParser<'a> {
    source: &'a str,
    offset: usize,
    preprocessor_macro_facts: &'a PreprocessorMacroFacts,
}

impl<'a> KnownMacroExprParser<'a> {
    fn new(source: &'a str, preprocessor_macro_facts: &'a PreprocessorMacroFacts) -> Self {
        Self {
            source,
            offset: 0,
            preprocessor_macro_facts,
        }
    }

    fn parse(mut self) -> RemovedMacroTruth {
        let value = self.parse_or();
        self.skip_ws();
        if self.offset == self.source.len() {
            value
        } else {
            RemovedMacroTruth::Unknown
        }
    }

    fn parse_or(&mut self) -> RemovedMacroTruth {
        let mut value = self.parse_and();
        loop {
            self.skip_ws();
            if !self.consume("||") {
                return value;
            }
            value = value.or(self.parse_and());
        }
    }

    fn parse_and(&mut self) -> RemovedMacroTruth {
        let mut value = self.parse_unary();
        loop {
            self.skip_ws();
            if !self.consume("&&") {
                return value;
            }
            value = value.and(self.parse_unary());
        }
    }

    fn parse_unary(&mut self) -> RemovedMacroTruth {
        self.skip_ws();
        if self.consume("!") {
            return self.parse_unary().not();
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> RemovedMacroTruth {
        self.skip_ws();
        if self.consume("(") {
            let value = self.parse_or();
            self.skip_ws();
            if !self.consume(")") {
                return RemovedMacroTruth::Unknown;
            }
            return value;
        }

        if let Some(value) = self.parse_literal_truth() {
            return value;
        }

        if let Some(macro_name) = self.parse_macro_expr_name() {
            return if self.preprocessor_macro_facts.definitely_defined.contains(macro_name) {
                RemovedMacroTruth::True
            } else if self
                .preprocessor_macro_facts
                .definitely_undefined
                .contains(macro_name)
            {
                RemovedMacroTruth::False
            } else {
                RemovedMacroTruth::Unknown
            };
        }

        self.consume_unknown_atom();
        RemovedMacroTruth::Unknown
    }

    fn parse_literal_truth(&mut self) -> Option<RemovedMacroTruth> {
        let start = self.offset;
        while let Some(ch) = self.peek_char() {
            if !ch.is_ascii_digit() {
                break;
            }
            self.offset += ch.len_utf8();
        }
        if self.offset == start {
            return None;
        }
        let value = self.source[start..self.offset].parse::<u64>().ok()?;
        Some(if value == 0 {
            RemovedMacroTruth::False
        } else {
            RemovedMacroTruth::True
        })
    }

    fn parse_macro_expr_name(&mut self) -> Option<&'a str> {
        let rest = &self.source[self.offset..];

        if let Some(macro_name) = parse_defined_macro_prefix(rest) {
            self.offset += defined_macro_len(rest)?;
            return Some(macro_name);
        }
        for predicate in ["IS_ENABLED", "IS_REACHABLE"] {
            if let Some(macro_name) = parse_macro_predicate_prefix(rest, predicate) {
                self.offset += macro_predicate_len(rest, predicate)?;
                return Some(macro_name);
            }
        }
        if let Some(macro_name) = parse_plain_config_macro_prefix(rest) {
            self.offset += macro_name.len();
            return Some(macro_name);
        }
        None
    }

    fn consume_unknown_atom(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() || matches!(ch, '!' | '(' | ')' | '&' | '|') {
                break;
            }
            self.offset += ch.len_utf8();
        }
    }

    fn skip_ws(&mut self) {
        while let Some(ch) = self.peek_char() {
            if !ch.is_whitespace() {
                break;
            }
            self.offset += ch.len_utf8();
        }
    }

    fn consume(&mut self, token: &str) -> bool {
        if self.source[self.offset..].starts_with(token) {
            self.offset += token.len();
            true
        } else {
            false
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }
}

fn parse_defined_macro_prefix(expr: &str) -> Option<&str> {
    simple_defined_macro_name(prefix_balanced_macro_expr(expr)?)
}

fn parse_macro_predicate_prefix<'a>(expr: &'a str, predicate: &str) -> Option<&'a str> {
    simple_macro_predicate_name(prefix_balanced_macro_expr(expr)?, predicate)
}

fn parse_plain_config_macro_prefix(expr: &str) -> Option<&str> {
    let end = expr
        .char_indices()
        .take_while(|&(_, ch)| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
        .last()
        .map_or(0, |(index, ch)| index + ch.len_utf8());
    if end == 0 {
        return None;
    }
    simple_plain_config_macro_name(&expr[..end])
}

fn defined_macro_len(expr: &str) -> Option<usize> {
    prefix_balanced_macro_expr(expr).map(str::len)
}

fn macro_predicate_len(expr: &str, predicate: &str) -> Option<usize> {
    let prefix = prefix_balanced_macro_expr(expr)?;
    if prefix.trim_start().starts_with(predicate) {
        Some(prefix.len())
    } else {
        None
    }
}

fn prefix_balanced_macro_expr(expr: &str) -> Option<&str> {
    let base = expr;
    let mut offset = 0usize;
    if base.starts_with("defined") {
        offset += "defined".len();
        offset = skip_ascii_whitespace(base, offset);
        if base[offset..].starts_with('(') {
            offset += 1;
            offset = skip_balanced_parens(base, offset)?;
        } else {
            let (_, end) = parse_exported_symbol_token(base, offset)?;
            offset = end;
        }
        return Some(&base[..offset]);
    }
    for predicate in ["IS_ENABLED", "IS_REACHABLE"] {
        if base.starts_with(predicate) {
            offset += predicate.len();
            offset = skip_ascii_whitespace(base, offset);
            if !base[offset..].starts_with('(') {
                return None;
            }
            offset += 1;
            offset = skip_balanced_parens(base, offset)?;
            return Some(&base[..offset]);
        }
    }
    let (_, end) = parse_exported_symbol_token(base, 0)?;
    Some(&base[..end])
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
        let Some((_, next)) = parse_exported_symbol_token(source, offset) else {
            return false;
        };
        offset = skip_ascii_whitespace(source, next);
        if source[offset..].starts_with('(') {
            let Some(next) = skip_balanced_parens(source, offset + 1) else {
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

fn identifier_is_goto_target(source: &str, start: usize) -> bool {
    statement_prefix(source, start)
        .split_whitespace()
        .next_back()
        .is_some_and(|token| token == "goto")
}

fn identifier_is_label_definition(source: &str, end: usize) -> bool {
    let after = skip_ascii_whitespace(source, end);
    source[after..].starts_with(':')
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

fn mask_preprocessor_directive_lines(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    for line in content.split_inclusive('\n') {
        if line.trim_start().starts_with('#') {
            for ch in line.chars() {
                out.push(if ch == '\n' { '\n' } else { ' ' });
            }
        } else {
            out.push_str(line);
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
    fn test_prove_removed_exports_ignores_struct_function_pointer_field_declaration() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "arch/um/drivers/harddog_user_exp.c",
            "void stop_watchdog(int in_fd, int out_fd) {}\nEXPORT_SYMBOL(stop_watchdog);\n",
        );
        write(
            root,
            "drivers/firmware/cirrus/cs_dsp.c",
            concat!(
                "struct cs_dsp;\n",
                "struct cs_dsp_ops {\n",
                "\tvoid (*stop_watchdog)(struct cs_dsp *dsp);\n",
                "};\n",
            ),
        );
        let removed_paths = BTreeSet::from([PathBuf::from("arch/um")]);
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
                symbol: ExportedSymbol::new("stop_watchdog").unwrap(),
                provider: PathBuf::from("arch/um/drivers/harddog_user_exp.c"),
                export_macro: String::from("EXPORT_SYMBOL"),
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
    fn test_prove_removed_exports_ignores_live_macro_shadow_for_lowercase_symbol() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "arch/alpha/lib/copy_user.S",
            "SYM_FUNC_START(__copy_user)\nSYM_FUNC_END(__copy_user)\nEXPORT_SYMBOL(__copy_user)\n",
        );
        write(
            root,
            "arch/x86/lib/usercopy_32.c",
            "#define __copy_user(to, from, size) do { } while (0)\nvoid user(void *to, const void *from, unsigned long n) { __copy_user(to, from, n); }\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("arch/alpha")]);
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
                symbol: ExportedSymbol::new("__copy_user").unwrap(),
                provider: PathBuf::from("arch/alpha/lib/copy_user.S"),
                export_macro: String::from("EXPORT_SYMBOL"),
                line: 3,
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
    fn test_prove_removed_exports_ignores_live_variable_definition_shadow() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "arch/loongarch/kernel/smp.c",
            "int __cpu_logical_map[4];\nEXPORT_SYMBOL(__cpu_logical_map);\n",
        );
        write(
            root,
            "arch/arm64/kernel/setup.c",
            concat!(
                "unsigned long __cpu_logical_map[4] = { 0 };\n",
                "unsigned long cpu_logical_map(unsigned int cpu)\n",
                "{\n",
                "\treturn __cpu_logical_map[cpu];\n",
                "}\n",
            ),
        );
        let removed_paths = BTreeSet::from([PathBuf::from("arch/loongarch")]);
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
                symbol: ExportedSymbol::new("__cpu_logical_map").unwrap(),
                provider: PathBuf::from("arch/loongarch/kernel/smp.c"),
                export_macro: String::from("EXPORT_SYMBOL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_ignores_live_non_exported_function_provider() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "arch/arm/xen/p2m.c",
            "bool __set_phys_to_machine(unsigned long pfn, unsigned long mfn) { return true; }\nEXPORT_SYMBOL_GPL(__set_phys_to_machine);\n",
        );
        write(
            root,
            "arch/x86/xen/p2m.c",
            "bool __set_phys_to_machine(unsigned long pfn, unsigned long mfn) { return true; }\n",
        );
        write(
            root,
            "drivers/xen/mem-reservation.c",
            "bool live(unsigned long pfn) { return __set_phys_to_machine(pfn, 0); }\n",
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
                symbol: ExportedSymbol::new("__set_phys_to_machine").unwrap(),
                provider: PathBuf::from("arch/arm/xen/p2m.c"),
                export_macro: String::from("EXPORT_SYMBOL_GPL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_does_not_treat_local_macro_shadow_as_global() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "arch/alpha/lib/copy_user.S",
            "SYM_FUNC_START(__copy_user)\nSYM_FUNC_END(__copy_user)\nEXPORT_SYMBOL(__copy_user)\n",
        );
        write(
            root,
            "arch/x86/lib/usercopy_32.c",
            "#define __copy_user(to, from, size) do { } while (0)\nvoid local(void *to, const void *from, unsigned long n) { __copy_user(to, from, n); }\n",
        );
        write(
            root,
            "drivers/live/user.c",
            "extern void __copy_user(void *, const void *, unsigned long);\nvoid live(void *to, const void *from, unsigned long n) { __copy_user(to, from, n); }\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("arch/alpha")]);
        let removed_dirs = removed_paths.clone();

        let err = format!(
            "{:#}",
            prove_removed_exports_have_no_live_consumers(
                root,
                &removed_paths,
                &removed_dirs,
                &BTreeSet::new(),
            )
            .unwrap_err()
        );

        assert!(err.contains("__copy_user"));
        assert!(err.contains("drivers/live/user.c"));
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
    fn test_prove_removed_exports_ignores_removed_arch_local_config_guarded_consumer() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "arch/arm/Kconfig", "config ARM_DMA_USE_IOMMU\n\tbool\n");
        write(
            root,
            "arch/arm/mm/dma-mapping.c",
            "void arm_iommu_detach_device(void) {}\nEXPORT_SYMBOL(arm_iommu_detach_device);\n",
        );
        write(
            root,
            "drivers/gpu/drm/tegra/drm.c",
            concat!(
                "#if IS_ENABLED(CONFIG_ARM_DMA_USE_IOMMU)\n",
                "void live(void) { arm_iommu_detach_device(); }\n",
                "#endif\n",
                "void generic(void) {}\n",
            ),
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
                symbol: ExportedSymbol::new("arm_iommu_detach_device").unwrap(),
                provider: PathBuf::from("arch/arm/mm/dma-mapping.c"),
                export_macro: String::from("EXPORT_SYMBOL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_ignores_removed_non_arch_local_config_guarded_consumer() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "drivers/macintosh/Kconfig", "config PMAC_BACKLIGHT\n\tbool\n");
        write(
            root,
            "arch/powerpc/platforms/powermac/backlight.c",
            "int pmac_backlight;\nEXPORT_SYMBOL_GPL(pmac_backlight);\n",
        );
        write(
            root,
            "drivers/video/backlight/backlight.c",
            concat!(
                "#ifdef CONFIG_PMAC_BACKLIGHT\n",
                "void live(void) { pmac_backlight = 1; }\n",
                "#endif\n",
                "void generic(void) {}\n",
            ),
        );
        let removed_paths = BTreeSet::from([
            PathBuf::from("arch/powerpc"),
            PathBuf::from("drivers/macintosh"),
        ]);
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
                symbol: ExportedSymbol::new("pmac_backlight").unwrap(),
                provider: PathBuf::from("arch/powerpc/platforms/powermac/backlight.c"),
                export_macro: String::from("EXPORT_SYMBOL_GPL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_ignores_compound_removed_config_guarded_consumer() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "arch/powerpc/platforms/powermac/Kconfig", "config PPC_PMAC\n\tbool\n");
        write(
            root,
            "arch/powerpc/platforms/powermac/feature.c",
            "void pmac_set_early_video_resume(void) {}\nEXPORT_SYMBOL(pmac_set_early_video_resume);\n",
        );
        write(
            root,
            "drivers/video/fbdev/aty/radeon_pm.c",
            concat!(
                "#if defined(CONFIG_PM) && defined(CONFIG_PPC_PMAC)\n",
                "void live(void) { pmac_set_early_video_resume(); }\n",
                "#endif\n",
            ),
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
                symbol: ExportedSymbol::new("pmac_set_early_video_resume").unwrap(),
                provider: PathBuf::from("arch/powerpc/platforms/powermac/feature.c"),
                export_macro: String::from("EXPORT_SYMBOL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_ignores_removed_arch_elif_branch_after_unknown_branch() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "arch/nios2/mm/cacheflush.c",
            "void flush_dcache_range(unsigned long start, unsigned long end) {}\nEXPORT_SYMBOL(flush_dcache_range);\n",
        );
        write(
            root,
            "drivers/gpu/drm/drm_cache.c",
            concat!(
                "#if defined(CONFIG_X86)\n",
                "void x86(void) {}\n",
                "#elif defined(__powerpc__)\n",
                "void ppc(void) { flush_dcache_range(0, 0); }\n",
                "#else\n",
                "void other(void) {}\n",
                "#endif\n",
            ),
        );
        let removed_paths = BTreeSet::from([
            PathBuf::from("arch/nios2"),
            PathBuf::from("arch/powerpc"),
        ]);
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
                symbol: ExportedSymbol::new("flush_dcache_range").unwrap(),
                provider: PathBuf::from("arch/nios2/mm/cacheflush.c"),
                export_macro: String::from("EXPORT_SYMBOL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_ignores_local_macro_defined_only_in_removed_guard() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "arch/arm/common/Kconfig", "config SA1111\n\tbool\n");
        write(
            root,
            "arch/arm/common/sa1111.c",
            "int sa1111_driver_register(void) { return 0; }\nEXPORT_SYMBOL(sa1111_driver_register);\n",
        );
        write(
            root,
            "drivers/usb/host/ohci-hcd.c",
            concat!(
                "#if defined(CONFIG_ARCH_SA1100) && defined(CONFIG_SA1111)\n",
                "#define SA1111_DRIVER ohci_hcd_sa1111_driver\n",
                "#endif\n",
                "#ifdef SA1111_DRIVER\n",
                "int live(void) { return sa1111_driver_register(); }\n",
                "#endif\n",
            ),
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
                symbol: ExportedSymbol::new("sa1111_driver_register").unwrap(),
                provider: PathBuf::from("arch/arm/common/sa1111.c"),
                export_macro: String::from("EXPORT_SYMBOL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_does_not_mask_config_still_defined_on_live_arch() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "arch/arm/Kconfig", "config ARCH_EXYNOS\n\tbool\n");
        write(root, "arch/arm64/Kconfig.platforms", "config ARCH_EXYNOS\n\tbool\n");
        write(
            root,
            "arch/arm/mach-exynos/exynos.c",
            "void exynos_legacy(void) {}\nEXPORT_SYMBOL(exynos_legacy);\n",
        );
        write(
            root,
            "drivers/live/user.c",
            concat!(
                "#if IS_ENABLED(CONFIG_ARCH_EXYNOS)\n",
                "void live(void) { exynos_legacy(); }\n",
                "#endif\n",
            ),
        );
        let removed_paths = BTreeSet::from([PathBuf::from("arch/arm")]);
        let removed_dirs = removed_paths.clone();

        let err = format!(
            "{:#}",
            prove_removed_exports_have_no_live_consumers(
                root,
                &removed_paths,
                &removed_dirs,
                &BTreeSet::new(),
            )
            .unwrap_err()
        );

        assert!(err.contains("exynos_legacy"));
        assert!(err.contains("drivers/live/user.c"));
    }

    #[test]
    fn test_prove_removed_exports_ignores_live_source_fallback_macro_shadow() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "arch/s390/lib/spinlock.c",
            "void arch_spin_relax(void) {}\nEXPORT_SYMBOL(arch_spin_relax);\n",
        );
        write(
            root,
            "kernel/locking/spinlock.c",
            concat!(
                "#ifndef arch_spin_relax\n",
                "# define arch_spin_relax(l) cpu_relax()\n",
                "#endif\n",
                "void generic(int *lock) { arch_spin_relax(lock); }\n",
            ),
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
                symbol: ExportedSymbol::new("arch_spin_relax").unwrap(),
                provider: PathBuf::from("arch/s390/lib/spinlock.c"),
                export_macro: String::from("EXPORT_SYMBOL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_ignores_live_shipped_source_definition() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "arch/m68k/amiga/config.c",
            "unsigned short *key_maps[1];\nEXPORT_SYMBOL_GPL(key_maps);\n",
        );
        write(
            root,
            "drivers/tty/vt/defkeymap.c_shipped",
            "unsigned short *key_maps[1] = { 0 };\n",
        );
        write(
            root,
            "drivers/tty/vt/keyboard.c",
            "void live(void) { (void)key_maps[0]; }\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("arch/m68k")]);
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
                symbol: ExportedSymbol::new("key_maps").unwrap(),
                provider: PathBuf::from("arch/m68k/amiga/config.c"),
                export_macro: String::from("EXPORT_SYMBOL_GPL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_ignores_inactive_preprocessor_directive_reference() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "arch/s390/Kconfig", "config GENERIC_LOCKBREAK\n\tbool\n");
        write(
            root,
            "arch/s390/lib/spinlock.c",
            "void arch_spin_relax(void) {}\nEXPORT_SYMBOL(arch_spin_relax);\n",
        );
        write(
            root,
            "kernel/locking/spinlock.c",
            concat!(
                "#if !defined(CONFIG_GENERIC_LOCKBREAK) || defined(CONFIG_DEBUG_LOCK_ALLOC)\n",
                "void generic(void) {}\n",
                "#else\n",
                "#ifndef arch_spin_relax\n",
                "# define arch_spin_relax(l) cpu_relax()\n",
                "#endif\n",
                "#endif\n",
            ),
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
                symbol: ExportedSymbol::new("arch_spin_relax").unwrap(),
                provider: PathBuf::from("arch/s390/lib/spinlock.c"),
                export_macro: String::from("EXPORT_SYMBOL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_ignores_local_goto_label_name_collision() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "arch/s390/kernel/debug.c",
            "void debug_unregister(void) {}\nEXPORT_SYMBOL(debug_unregister);\n",
        );
        write(
            root,
            "drivers/net/wireless/ath/ath11k/spectral.c",
            concat!(
                "int live(int fail) {\n",
                "\tif (fail)\n",
                "\t\tgoto debug_unregister;\n",
                "\treturn 0;\n",
                "debug_unregister:\n",
                "\treturn -1;\n",
                "}\n",
            ),
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
                symbol: ExportedSymbol::new("debug_unregister").unwrap(),
                provider: PathBuf::from("arch/s390/kernel/debug.c"),
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

    #[test]
    fn test_prove_removed_exports_ignores_tools_selftest_consumer() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "fs/hostfs/hostfs_user_exp.c",
            "int read_dir(void) { return 0; }\nEXPORT_SYMBOL(read_dir);\n",
        );
        write(
            root,
            "tools/testing/selftests/landlock/fs_test.c",
            "int user(void) { return read_dir(); }\n",
        );
        let removed_paths = BTreeSet::from([PathBuf::from("fs/hostfs")]);
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
                symbol: ExportedSymbol::new("read_dir").unwrap(),
                provider: PathBuf::from("fs/hostfs/hostfs_user_exp.c"),
                export_macro: String::from("EXPORT_SYMBOL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_ignores_branch_guarded_by_config_selected_on_all_live_arches() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "arch/arc/kernel/stacktrace.c",
            "void save_stack_trace(struct stack_trace *trace) {}\nEXPORT_SYMBOL_GPL(save_stack_trace);\n",
        );
        write(
            root,
            "arch/x86/Kconfig",
            "config X86\n\tdef_bool y\n\tselect ARCH_STACKWALK\n",
        );
        write(
            root,
            "arch/arm64/Kconfig",
            "config ARM64\n\tdef_bool y\n\tselect ARCH_STACKWALK\n",
        );
        write(
            root,
            "arch/riscv/Kconfig",
            "config RISCV\n\tdef_bool y\n\tselect ARCH_STACKWALK\n",
        );
        write(
            root,
            "kernel/stacktrace.c",
            concat!(
                "#ifdef CONFIG_ARCH_STACKWALK\n",
                "void live(void) {}\n",
                "#else\n",
                "void generic(struct stack_trace *trace) { save_stack_trace(trace); }\n",
                "#endif\n",
            ),
        );
        let removed_paths = BTreeSet::from([PathBuf::from("arch/arc")]);
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
                symbol: ExportedSymbol::new("save_stack_trace").unwrap(),
                provider: PathBuf::from("arch/arc/kernel/stacktrace.c"),
                export_macro: String::from("EXPORT_SYMBOL_GPL"),
                line: 2,
            }])
        );
    }

    #[test]
    fn test_prove_removed_exports_does_not_mask_config_selected_on_only_some_live_arches() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "arch/arc/kernel/stacktrace.c",
            "void save_stack_trace(struct stack_trace *trace) {}\nEXPORT_SYMBOL_GPL(save_stack_trace);\n",
        );
        write(
            root,
            "arch/x86/Kconfig",
            "config X86\n\tdef_bool y\n\tselect ARCH_STACKWALK\n",
        );
        write(root, "arch/arm64/Kconfig", "config ARM64\n\tdef_bool y\n");
        write(
            root,
            "kernel/stacktrace.c",
            concat!(
                "#ifdef CONFIG_ARCH_STACKWALK\n",
                "void live(void) {}\n",
                "#else\n",
                "void generic(struct stack_trace *trace) { save_stack_trace(trace); }\n",
                "#endif\n",
            ),
        );
        let removed_paths = BTreeSet::from([PathBuf::from("arch/arc")]);
        let removed_dirs = removed_paths.clone();

        let err = format!(
            "{:#}",
            prove_removed_exports_have_no_live_consumers(
                root,
                &removed_paths,
                &removed_dirs,
                &BTreeSet::new(),
            )
            .unwrap_err()
        );

        assert!(err.contains("save_stack_trace"));
        assert!(err.contains("kernel/stacktrace.c"));
    }

    fn write(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }
}
