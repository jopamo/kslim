//! Stable output repository names.
//!
//! This module owns deterministic names for output branches, tags, commit
//! message labels, snapshot identifiers, and metadata file-name groups. It must
//! not inspect workspaces or candidate trees.

use crate::config::{KslimConfig, ProfileConfig};
use crate::lockfile::ResolvedBase;

use super::{metadata, report};

pub const COMMIT_SUBJECT_IMPORT_PREFIX: &str = "kslim: import linux";
pub const COMMIT_SECTION_UPSTREAM: &str = "Upstream";
pub const COMMIT_SECTION_BASE_REF: &str = "Base-ref";
pub const COMMIT_SECTION_BASE_COMMIT: &str = "Base-commit";
pub const COMMIT_SECTION_PROFILE: &str = "Profile";
pub const COMMIT_SECTION_MODE: &str = "Mode";
pub const COMMIT_SECTION_PLAN_FINGERPRINT: &str = "Plan-fingerprint";
pub const COMMIT_SECTION_REDUCER_SUMMARY: &str = "Reducer-summary";
pub const COMMIT_SECTION_SELFTEST_SUMMARY: &str = "Selftest-summary";
pub const COMMIT_MESSAGE_HOST_PATH_REDACTION: &str = "<host-path>";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitMessageDetails<'a> {
    pub plan_fingerprint: &'a str,
    pub reducer_summary: &'a str,
    pub selftest_summary: &'a str,
}

impl<'a> CommitMessageDetails<'a> {
    pub fn new(
        plan_fingerprint: &'a str,
        reducer_summary: &'a str,
        selftest_summary: &'a str,
    ) -> Self {
        Self {
            plan_fingerprint,
            reducer_summary,
            selftest_summary,
        }
    }
}

pub(crate) fn sanitize_commit_message_value(value: &str) -> String {
    if contains_host_specific_absolute_path(value) {
        COMMIT_MESSAGE_HOST_PATH_REDACTION.to_string()
    } else {
        value.to_string()
    }
}

fn contains_host_specific_absolute_path(value: &str) -> bool {
    if host_path_token(value) {
        return true;
    }

    value
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '"' | '\'' | ',' | ';'))
        .any(host_path_token)
}

fn host_path_token(token: &str) -> bool {
    let token = trim_commit_message_path_candidate(token);
    if token.is_empty() {
        return false;
    }
    if crate::security::is_host_specific_absolute_path(token) {
        return true;
    }
    if let Some((_, value)) = token.rsplit_once('=') {
        let value = trim_commit_message_path_candidate(value);
        if crate::security::is_host_specific_absolute_path(value) {
            return true;
        }
    }
    if let Some(file_url) = token.find("file:").map(|index| &token[index..]) {
        if crate::security::is_host_specific_absolute_path(file_url) {
            return true;
        }
    }
    if !token.contains("://") {
        if let Some((_, value)) = token.rsplit_once(':') {
            let value = trim_commit_message_path_candidate(value);
            if crate::security::is_host_specific_absolute_path(value) {
                return true;
            }
        }
    }
    false
}

fn trim_commit_message_path_candidate(value: &str) -> &str {
    value.trim_matches(|ch: char| {
        matches!(
            ch,
            '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | ',' | ';'
        )
    })
}

#[allow(dead_code)]
pub const STABLE_METADATA_FILE_NAMES: &[&str] = &[
    metadata::BASE_METADATA_FILE,
    metadata::GENERATED_METADATA_FILE,
    metadata::PATCH_METADATA_FILE,
    metadata::REPORT_FILE,
    metadata::CANDIDATE_METADATA_FILE,
    metadata::PUBLISHED_METADATA_FILE,
    report::REDUCER_REMOVAL_MANIFEST,
    report::REDUCER_REPORT_MD,
    report::REDUCER_REPORT_JSON,
    report::REDUCER_DIAGNOSTICS_JSON,
    report::REDUCER_EDIT_SUMMARY_JSON,
    report::REDUCER_KCONFIG_SOLVER_REPORT_JSON,
    report::REDUCER_KCONFIG_REWRITE_REPORT_JSON,
    report::REDUCER_SKIPPED_SITES_JSON,
    report::MATRIX_REPORT_JSON,
    report::GENERATE_REPORT_JSON,
];

pub fn branch_name(
    config: &KslimConfig,
    profile: &ProfileConfig,
    resolved: &ResolvedBase,
) -> String {
    if let Some(branch) = &config.output.branch {
        return branch.clone();
    }
    format!(
        "{}/{}/{}",
        config.output.branch_prefix, &resolved.r#ref, profile.profile.name
    )
}

pub fn tag_name(
    config: &KslimConfig,
    profile: &ProfileConfig,
    resolved: &ResolvedBase,
    run: u32,
) -> String {
    if let Some(branch) = &config.output.branch {
        return format!("{}-r{}", branch, run);
    }
    format!(
        "{}-{}-{}-r{}",
        config.output.branch_prefix, &resolved.r#ref, profile.profile.name, run
    )
}

pub(crate) fn initial_branch(config: &KslimConfig) -> String {
    config
        .output
        .branch
        .clone()
        .unwrap_or_else(|| "master".to_string())
}

#[allow(dead_code)]
pub fn snapshot_id(
    config: &KslimConfig,
    profile: &ProfileConfig,
    resolved: &ResolvedBase,
    run: u32,
) -> String {
    format!(
        "{}@{}#r{}",
        branch_name(config, profile, resolved),
        resolved.commit,
        run
    )
}

pub fn commit_message(
    config: &KslimConfig,
    profile: &ProfileConfig,
    resolved: &ResolvedBase,
    mode: &str,
    details: &CommitMessageDetails<'_>,
) -> String {
    format!(
        concat!(
            "{} {}\n\n",
            "{}: {}\n",
            "{}: {}\n",
            "{}: {}\n",
            "{}: {}\n",
            "{}: {}\n",
            "{}: {}\n",
            "{}: {}\n",
            "{}: {}"
        ),
        COMMIT_SUBJECT_IMPORT_PREFIX,
        sanitize_commit_message_value(&resolved.r#ref),
        COMMIT_SECTION_UPSTREAM,
        sanitize_commit_message_value(&metadata::committed_upstream_label(config)),
        COMMIT_SECTION_BASE_REF,
        sanitize_commit_message_value(&resolved.r#ref),
        COMMIT_SECTION_BASE_COMMIT,
        sanitize_commit_message_value(&resolved.commit),
        COMMIT_SECTION_PROFILE,
        sanitize_commit_message_value(&profile.profile.name),
        COMMIT_SECTION_MODE,
        sanitize_commit_message_value(mode),
        COMMIT_SECTION_PLAN_FINGERPRINT,
        sanitize_commit_message_value(details.plan_fingerprint),
        COMMIT_SECTION_REDUCER_SUMMARY,
        sanitize_commit_message_value(details.reducer_summary),
        COMMIT_SECTION_SELFTEST_SUMMARY,
        sanitize_commit_message_value(details.selftest_summary),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_and_profile() -> (KslimConfig, ProfileConfig, ResolvedBase) {
        let mut config = crate::config::default_kslim_config("demo", "/tmp/output");
        config.upstream.url = String::from("https://example.com/linux.git");
        let profile = crate::config::default_profile_config("v1.0");
        let resolved = ResolvedBase {
            upstream: String::from("linux"),
            url: String::from("https://example.com/linux.git"),
            r#ref: String::from("v1.0"),
            commit: String::from("0123456789abcdef"),
            resolved_at: String::from("2026-01-01T00:00:00Z"),
        };
        (config, profile, resolved)
    }

    #[test]
    fn branch_tag_and_snapshot_names_are_stable_for_default_output() {
        let (config, profile, resolved) = config_and_profile();

        assert_eq!(
            branch_name(&config, &profile, &resolved),
            "kslim/v1.0/default"
        );
        assert_eq!(
            tag_name(&config, &profile, &resolved, 7),
            "kslim-v1.0-default-r7"
        );
        assert_eq!(initial_branch(&config), "master");
        assert_eq!(
            snapshot_id(&config, &profile, &resolved, 7),
            "kslim/v1.0/default@0123456789abcdef#r7"
        );
    }

    #[test]
    fn explicit_output_branch_controls_branch_tag_and_initial_branch() {
        let (mut config, profile, resolved) = config_and_profile();
        config.output.branch = Some(String::from("release/linux-min"));

        assert_eq!(
            branch_name(&config, &profile, &resolved),
            "release/linux-min"
        );
        assert_eq!(
            tag_name(&config, &profile, &resolved, 2),
            "release/linux-min-r2"
        );
        assert_eq!(initial_branch(&config), "release/linux-min");
        assert_eq!(
            snapshot_id(&config, &profile, &resolved, 2),
            "release/linux-min@0123456789abcdef#r2"
        );
    }

    #[test]
    fn commit_message_uses_stable_subject_and_section_labels() {
        let (config, profile, resolved) = config_and_profile();

        let details = CommitMessageDetails::new(
            "fingerprint-plan",
            "ran=true files_removed=1 dirs_removed=2 edits=3",
            "enabled=true built_in_checks=2 kernel_builds=1 commands=0",
        );
        let message = commit_message(&config, &profile, &resolved, "slimmed", &details);

        assert!(message.starts_with("kslim: import linux v1.0\n\n"));
        assert!(message.contains("Upstream: https://example.com/linux.git"));
        assert!(message.contains("Base-ref: v1.0"));
        assert!(message.contains("Base-commit: 0123456789abcdef"));
        assert!(message.contains("Profile: default"));
        assert!(message.contains("Mode: slimmed"));
        assert!(message.contains("Plan-fingerprint: fingerprint-plan"));
        assert!(
            message.contains("Reducer-summary: ran=true files_removed=1 dirs_removed=2 edits=3")
        );
        assert!(message.contains(
            "Selftest-summary: enabled=true built_in_checks=2 kernel_builds=1 commands=0"
        ));
        assert!(!message.contains("/tmp/output"));
    }

    #[test]
    fn commit_message_redacts_host_paths_from_published_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let host_path = tmp.path().join("candidate-tree");
        let host_path = host_path.to_str().unwrap();
        let mut config = crate::config::default_kslim_config("demo", host_path);
        config.upstream.url = format!("file://{host_path}");
        let mut profile = crate::config::default_profile_config("v1.0");
        profile.profile.name = format!("profile={host_path}");
        let resolved = ResolvedBase {
            upstream: String::from("linux"),
            url: format!("file://{host_path}"),
            r#ref: host_path.to_string(),
            commit: format!("commit={host_path}"),
            resolved_at: String::from("2026-01-01T00:00:00Z"),
        };
        let plan_fingerprint = format!("plan={host_path}");
        let reducer_summary = format!("ran=true candidate:{host_path}");
        let selftest_summary = format!("enabled=true log=file://{host_path}");
        let details =
            CommitMessageDetails::new(&plan_fingerprint, &reducer_summary, &selftest_summary);

        let message = commit_message(&config, &profile, &resolved, host_path, &details);

        assert!(!message.contains(host_path));
        assert!(message.contains(COMMIT_MESSAGE_HOST_PATH_REDACTION));
        assert!(message.contains("Upstream: local-upstream:linux"));
        assert!(message.contains("Base-ref: <host-path>"));
        assert!(message.contains("Reducer-summary: <host-path>"));
        assert!(message.contains("Selftest-summary: <host-path>"));
    }

    #[test]
    fn sanitize_commit_message_value_redacts_embedded_host_paths_only() {
        assert_eq!(
            sanitize_commit_message_value("https://example.com/linux.git"),
            "https://example.com/linux.git"
        );
        assert_eq!(
            sanitize_commit_message_value("git@example.com:linux/kernel.git"),
            "git@example.com:linux/kernel.git"
        );
        assert_eq!(
            sanitize_commit_message_value("candidate:/tmp/kslim-tree"),
            COMMIT_MESSAGE_HOST_PATH_REDACTION
        );
        assert_eq!(
            sanitize_commit_message_value("candidate=file:///tmp/kslim-tree"),
            COMMIT_MESSAGE_HOST_PATH_REDACTION
        );
        assert_eq!(
            sanitize_commit_message_value("C:\\tmp\\kslim-tree"),
            COMMIT_MESSAGE_HOST_PATH_REDACTION
        );
    }

    #[test]
    fn stable_metadata_file_names_cover_metadata_and_report_files() {
        for required in [
            metadata::BASE_METADATA_FILE,
            metadata::GENERATED_METADATA_FILE,
            metadata::PATCH_METADATA_FILE,
            metadata::REPORT_FILE,
            metadata::CANDIDATE_METADATA_FILE,
            metadata::PUBLISHED_METADATA_FILE,
            report::REDUCER_REPORT_JSON,
            report::REDUCER_DIAGNOSTICS_JSON,
            report::REDUCER_KCONFIG_SOLVER_REPORT_JSON,
            report::REDUCER_KCONFIG_REWRITE_REPORT_JSON,
            report::GENERATE_REPORT_JSON,
        ] {
            assert!(
                STABLE_METADATA_FILE_NAMES.contains(&required),
                "missing stable metadata file name {required}"
            );
        }
        assert!(!STABLE_METADATA_FILE_NAMES.contains(&report::LAST_ATTEMPT_JSON));
    }
}
