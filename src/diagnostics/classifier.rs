//! Diagnostic classifier for selftest and build failures.
//!
//! This module owns deterministic parsing of supported selftest, compiler,
//! linker, and make failure shapes into `ClassifiedDiagnostic` values.

use std::path::{Path, PathBuf};

use crate::selftest::SelfTestFailure;

use super::command_capture::{
    capture_selftest_failure, CapturedBuiltInDiagnostic, CapturedCommandDiagnostic,
    CapturedDiagnostic,
};
use super::ClassifiedDiagnostic;

pub fn classify_selftest_failure(root: &Path, failure: &SelfTestFailure) -> ClassifiedDiagnostic {
    match capture_selftest_failure(failure) {
        CapturedDiagnostic::Command(details) => classify_command_failure(root, &details),
        CapturedDiagnostic::BuiltIn(details) => classify_builtin_failure(root, &details),
    }
}

pub(in crate::diagnostics) fn classify_builtin_failure(
    root: &Path,
    details: &CapturedBuiltInDiagnostic<'_>,
) -> ClassifiedDiagnostic {
    if details.check == "kconfig-sources" {
        if let Some((kconfig_file, line, source)) =
            parse_missing_kconfig_source_message(details.message)
        {
            return ClassifiedDiagnostic::MissingKconfigSource {
                kconfig_file: normalize_source_path(root, kconfig_file),
                line,
                source: source.to_string(),
            };
        }
    }

    ClassifiedDiagnostic::Unknown
}

pub(in crate::diagnostics) fn classify_command_failure(
    root: &Path,
    details: &CapturedCommandDiagnostic<'_>,
) -> ClassifiedDiagnostic {
    for line in details.stderr.lines() {
        if let Some((source_file, line, header)) = parse_missing_header_line(line) {
            let normalized = normalize_source_path(root, source_file);
            return ClassifiedDiagnostic::MissingHeader {
                source_file: normalized,
                line,
                header: header.to_string(),
                build_target: details.target.map(str::to_string),
                arch: details.arch.map(str::to_string),
                config: details.config.map(str::to_string),
            };
        }
        if let Some(path) = parse_make_missing_directory_line(line) {
            return ClassifiedDiagnostic::MissingMakeDirectory {
                path: path.to_string(),
                build_target: details.target.map(str::to_string),
                arch: details.arch.map(str::to_string),
                config: details.config.map(str::to_string),
            };
        }
        if let Some(target) = parse_make_missing_target_line(line) {
            return ClassifiedDiagnostic::MissingMakeTarget {
                target: target.to_string(),
                build_target: details.target.map(str::to_string),
                arch: details.arch.map(str::to_string),
                config: details.config.map(str::to_string),
            };
        }
        if let Some((source_file, line, symbol)) = parse_gcc_undeclared_identifier_line(line) {
            return ClassifiedDiagnostic::UndeclaredIdentifier {
                source_file: normalize_source_path(root, source_file),
                line,
                symbol: symbol.to_string(),
                build_target: details.target.map(str::to_string),
                arch: details.arch.map(str::to_string),
                config: details.config.map(str::to_string),
            };
        }
        if let Some((source_file, line, symbol)) = parse_clang_undeclared_identifier_line(line) {
            return ClassifiedDiagnostic::UndeclaredIdentifier {
                source_file: normalize_source_path(root, source_file),
                line,
                symbol: symbol.to_string(),
                build_target: details.target.map(str::to_string),
                arch: details.arch.map(str::to_string),
                config: details.config.map(str::to_string),
            };
        }
        if let Some((source_file, line, symbol)) = parse_gcc_implicit_declaration_line(line) {
            return ClassifiedDiagnostic::ImplicitDeclaration {
                source_file: normalize_source_path(root, source_file),
                line,
                symbol: symbol.to_string(),
                build_target: details.target.map(str::to_string),
                arch: details.arch.map(str::to_string),
                config: details.config.map(str::to_string),
            };
        }
        if let Some((source_file, line, symbol)) = parse_clang_implicit_declaration_line(line) {
            return ClassifiedDiagnostic::ImplicitDeclaration {
                source_file: normalize_source_path(root, source_file),
                line,
                symbol: symbol.to_string(),
                build_target: details.target.map(str::to_string),
                arch: details.arch.map(str::to_string),
                config: details.config.map(str::to_string),
            };
        }
        if let Some((source_file, symbol)) = parse_gcc_undefined_reference_line(line) {
            return ClassifiedDiagnostic::UndefinedReference {
                source_file: normalize_source_path(root, source_file),
                symbol: symbol.to_string(),
                build_target: details.target.map(str::to_string),
                arch: details.arch.map(str::to_string),
                config: details.config.map(str::to_string),
            };
        }
    }

    ClassifiedDiagnostic::Unknown
}

pub(in crate::diagnostics) fn normalize_source_path(root: &Path, source_file: &str) -> PathBuf {
    let path = Path::new(source_file);
    if path.is_absolute() {
        path.strip_prefix(root).unwrap_or(path).to_path_buf()
    } else {
        PathBuf::from(source_file)
    }
}

pub(in crate::diagnostics) fn parse_missing_header_line(line: &str) -> Option<(&str, usize, &str)> {
    let marker = ": fatal error: ";
    let (location, rest) = line.split_once(marker)?;
    let header = parse_missing_header_detail(rest)?;
    let (source_file, line) = parse_source_file_and_line(location)?;
    if source_file.is_empty() || header.trim().is_empty() {
        return None;
    }
    Some((source_file, line, header.trim()))
}

pub(in crate::diagnostics) fn parse_missing_header_detail(rest: &str) -> Option<&str> {
    if let Some(header) = rest.strip_suffix(": No such file or directory") {
        return Some(header.trim());
    }

    for (open, close) in [('\'', '\''), ('"', '"')] {
        let Some(body) = rest.strip_prefix(open) else {
            continue;
        };
        let Some((header, suffix)) = body.split_once(close) else {
            continue;
        };
        if header.trim().is_empty() || suffix.trim() != "file not found" {
            continue;
        }
        return Some(header);
    }

    None
}

pub(in crate::diagnostics) fn parse_make_missing_target_line(line: &str) -> Option<&str> {
    let marker = "No rule to make target ";
    let (_, rest) = line.split_once(marker)?;
    let (target, _) = parse_quoted_symbol(rest)?;
    if target.trim().is_empty() || target.ends_with('/') {
        return None;
    }
    Some(target)
}

pub(in crate::diagnostics) fn parse_make_missing_directory_line(line: &str) -> Option<&str> {
    let marker = "No rule to make target ";
    let (_, rest) = line.split_once(marker)?;
    let (target, _) = parse_quoted_symbol(rest)?;
    if target.trim().is_empty() || !target.ends_with('/') {
        return None;
    }
    Some(target)
}

pub(in crate::diagnostics) fn parse_missing_kconfig_source_message(message: &str) -> Option<(&str, usize, &str)> {
    let body = message.strip_prefix("selftest failed: ")?;
    let (location, rest) = body.split_once(" references missing Kconfig source ")?;
    let (kconfig_file, line) = parse_file_and_line(location)?;
    let (source, _) = parse_quoted_symbol(rest)?;
    Some((kconfig_file, line, source))
}

pub(in crate::diagnostics) fn parse_gcc_undeclared_identifier_line(line: &str) -> Option<(&str, usize, &str)> {
    let (source_file, line, rest) = split_gcc_error_or_warning_line(line)?;

    parse_quoted_undeclared_symbol(rest).map(|symbol| (source_file, line, symbol))
}

pub(in crate::diagnostics) fn parse_clang_undeclared_identifier_line(line: &str) -> Option<(&str, usize, &str)> {
    let (source_file, line, rest) = split_gcc_error_or_warning_line(line)?;
    let body = rest.strip_prefix("use of undeclared identifier ")?;
    let (symbol, _) = parse_quoted_symbol(body)?;
    Some((source_file, line, symbol))
}

pub(in crate::diagnostics) fn parse_gcc_implicit_declaration_line(line: &str) -> Option<(&str, usize, &str)> {
    let (source_file, line, rest) = split_gcc_error_or_warning_line(line)?;
    let body = rest.strip_prefix("implicit declaration of function ")?;
    let (symbol, _) = parse_quoted_symbol(body)?;
    Some((source_file, line, symbol))
}

pub(in crate::diagnostics) fn parse_clang_implicit_declaration_line(line: &str) -> Option<(&str, usize, &str)> {
    let (source_file, line, rest) = split_gcc_error_or_warning_line(line)?;
    let body = rest.strip_prefix("call to undeclared function ")?;
    let (symbol, _) = parse_quoted_symbol(body)?;
    Some((source_file, line, symbol))
}

pub(in crate::diagnostics) fn parse_gcc_undefined_reference_line(line: &str) -> Option<(&str, &str)> {
    let marker = ": undefined reference to ";
    let (location, rest) = line.split_once(marker)?;
    let source_file = source_file_from_location(location)?;
    let (symbol, _) = parse_quoted_symbol(rest)?;
    Some((source_file, symbol))
}

pub(in crate::diagnostics) fn split_gcc_error_or_warning_line(line: &str) -> Option<(&str, usize, &str)> {
    for marker in [": error: ", ": warning: "] {
        let Some((location, rest)) = line.split_once(marker) else {
            continue;
        };
        let (source_file, line) = parse_source_file_and_line(location)?;
        if source_file.is_empty() {
            return None;
        }
        return Some((source_file, line, rest));
    }

    None
}

pub(in crate::diagnostics) fn parse_quoted_undeclared_symbol(rest: &str) -> Option<&str> {
    let (symbol, suffix) = parse_quoted_symbol(rest)?;
    if !suffix.trim_start().starts_with("undeclared") {
        return None;
    }

    Some(symbol)
}

pub(in crate::diagnostics) fn parse_quoted_symbol(rest: &str) -> Option<(&str, &str)> {
    for (open, close) in [('`', '\''), ('‘', '’'), ('\'', '\'')] {
        let Some(body) = rest.strip_prefix(open) else {
            continue;
        };
        let Some((symbol, suffix)) = body.split_once(close) else {
            continue;
        };
        if symbol.trim().is_empty() {
            continue;
        }
        return Some((symbol, suffix));
    }

    None
}

pub(in crate::diagnostics) fn source_file_from_location(location: &str) -> Option<&str> {
    let trimmed = location
        .rsplit_once(": ")
        .map(|(_, suffix)| suffix)
        .unwrap_or(location);
    trimmed.split(':').next()
}

pub(in crate::diagnostics) fn parse_source_file_and_line(location: &str) -> Option<(&str, usize)> {
    let trimmed = location
        .rsplit_once(": ")
        .map(|(_, suffix)| suffix)
        .unwrap_or(location);
    let (before_column, _) = trimmed.rsplit_once(':')?;
    parse_file_and_line(before_column)
}

pub(in crate::diagnostics) fn parse_file_and_line(location: &str) -> Option<(&str, usize)> {
    let (file, line) = location.rsplit_once(':')?;
    let line = line.parse().ok()?;
    Some((file, line))
}
