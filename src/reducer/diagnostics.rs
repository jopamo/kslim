use crate::diagnostics::ClassifiedDiagnostic;
use crate::selftest;

use super::ReducerStats;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RawDiagnosticExcerpt {
    pub command_context: String,
    pub build_target: Option<String>,
    pub raw_excerpt: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct NonConvergenceReport {
    pub pass_count: usize,
    pub remaining_diagnostics: Vec<ClassifiedDiagnostic>,
    pub fixers_skipped: Vec<String>,
    pub publishable: bool,
}

pub(crate) fn raw_diagnostic_excerpt_from_failure(
    failure: &selftest::SelfTestFailure,
) -> RawDiagnosticExcerpt {
    match failure {
        selftest::SelfTestFailure::BuiltIn { check, message } => RawDiagnosticExcerpt {
            command_context: format!("built-in selftest: {check}"),
            build_target: None,
            raw_excerpt: message.clone(),
        },
        selftest::SelfTestFailure::KernelBuild { label, details, .. } => RawDiagnosticExcerpt {
            command_context: format!("kernel build selftest '{label}': {}", details.command),
            build_target: details.target.clone(),
            raw_excerpt: captured_command_raw_excerpt(details),
        },
        selftest::SelfTestFailure::Command { details } => RawDiagnosticExcerpt {
            command_context: format!("selftest command: {}", details.command),
            build_target: details.target.clone(),
            raw_excerpt: captured_command_raw_excerpt(details),
        },
    }
}

fn captured_command_raw_excerpt(details: &selftest::CapturedCommandFailure) -> String {
    let mut excerpt = String::new();
    if let Some(exit_status) = details.exit_status {
        excerpt.push_str(&format!("exit_status: {exit_status}\n"));
    }
    if let Some(arch) = &details.arch {
        excerpt.push_str(&format!("arch: {arch}\n"));
    }
    if let Some(config) = &details.config {
        excerpt.push_str(&format!("config: {config}\n"));
    }
    if !details.stdout.is_empty() {
        excerpt.push_str("stdout:\n");
        excerpt.push_str(&details.stdout);
        if !details.stdout.ends_with('\n') {
            excerpt.push('\n');
        }
    }
    if !details.stderr.is_empty() {
        excerpt.push_str("stderr:\n");
        excerpt.push_str(&details.stderr);
        if !details.stderr.ends_with('\n') {
            excerpt.push('\n');
        }
    }
    if excerpt.is_empty() {
        excerpt.push_str("<no command output>\n");
    }
    excerpt
}

pub(crate) fn render_raw_diagnostic_excerpt(excerpt: &RawDiagnosticExcerpt) -> String {
    let mut rendered = format!("command context: {}\n", excerpt.command_context);
    if let Some(target) = &excerpt.build_target {
        rendered.push_str(&format!("build target: {target}\n"));
    }
    rendered.push_str("raw diagnostic excerpt:\n");
    rendered.push_str(excerpt.raw_excerpt.trim_end());
    rendered
}

pub(crate) fn non_convergence_report(
    stats: &ReducerStats,
    mut remaining_diagnostics: Vec<ClassifiedDiagnostic>,
    pass_count: usize,
) -> NonConvergenceReport {
    let mut fixers_skipped = stats
        .skipped_fixups
        .iter()
        .map(|skipped| {
            let fixer = skipped.fixer_name.unwrap_or("<none>");
            format!("{fixer}: {}", skipped.reason)
        })
        .collect::<Vec<_>>();
    fixers_skipped.sort();
    fixers_skipped.dedup();
    sort_classified_diagnostics(&mut remaining_diagnostics);

    NonConvergenceReport {
        pass_count,
        remaining_diagnostics,
        fixers_skipped,
        publishable: false,
    }
}

pub(crate) fn record_selftest_failure_diagnostic(
    reducer_stats: &mut ReducerStats,
    failure: &selftest::SelfTestFailure,
    classified: ClassifiedDiagnostic,
) {
    reducer_stats.classified_diagnostics.push(classified);
    reducer_stats
        .raw_diagnostic_excerpts
        .push(raw_diagnostic_excerpt_from_failure(failure));
}

fn sort_classified_diagnostics(diagnostics: &mut Vec<ClassifiedDiagnostic>) {
    diagnostics.sort_by(|left, right| {
        classified_diagnostic_sort_key(left).cmp(&classified_diagnostic_sort_key(right))
    });
    diagnostics.dedup_by(|left, right| {
        classified_diagnostic_sort_key(left) == classified_diagnostic_sort_key(right)
    });
}

fn classified_diagnostic_sort_key(
    diagnostic: &ClassifiedDiagnostic,
) -> (u8, String, Option<usize>, String, String, String, String) {
    (
        classified_diagnostic_rank(diagnostic),
        diagnostic
            .file()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        diagnostic.line(),
        diagnostic.subject().unwrap_or("").to_string(),
        diagnostic.build_target().unwrap_or("").to_string(),
        diagnostic.arch().unwrap_or("").to_string(),
        diagnostic.config().unwrap_or("").to_string(),
    )
}

fn classified_diagnostic_rank(diagnostic: &ClassifiedDiagnostic) -> u8 {
    match diagnostic {
        ClassifiedDiagnostic::MissingHeader { .. } => 0,
        ClassifiedDiagnostic::MissingKconfigSource { .. } => 1,
        ClassifiedDiagnostic::MissingMakeDirectory { .. } => 2,
        ClassifiedDiagnostic::MissingMakeTarget { .. } => 3,
        ClassifiedDiagnostic::UndeclaredIdentifier { .. } => 4,
        ClassifiedDiagnostic::ImplicitDeclaration { .. } => 5,
        ClassifiedDiagnostic::UndefinedReference { .. } => 6,
        ClassifiedDiagnostic::Unknown => 7,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn missing_header(path: &str, line: usize, header: &str) -> ClassifiedDiagnostic {
        ClassifiedDiagnostic::MissingHeader {
            source_file: PathBuf::from(path),
            line,
            header: header.to_string(),
            build_target: Some(String::from("modules")),
            arch: Some(String::from("arm64")),
            config: Some(String::from("defconfig")),
        }
    }

    #[test]
    fn non_convergence_report_sorts_remaining_diagnostics() {
        let duplicate = missing_header("drivers/z.c", 7, "z.h");
        let report = non_convergence_report(
            &ReducerStats::default(),
            vec![
                ClassifiedDiagnostic::Unknown,
                duplicate.clone(),
                missing_header("drivers/a.c", 9, "a.h"),
                duplicate,
            ],
            3,
        );

        assert_eq!(
            report.remaining_diagnostics,
            vec![
                missing_header("drivers/a.c", 9, "a.h"),
                missing_header("drivers/z.c", 7, "z.h"),
                ClassifiedDiagnostic::Unknown,
            ]
        );
        assert_eq!(report.pass_count, 3);
        assert!(!report.publishable);
    }
}
