use super::*;
use crate::config;
use crate::state::{
    CliOverrides, ProfileName, RequestedGenerateState, ResolvedCandidateState,
};
use crate::lockfile::ResolvedBase;
use crate::model::ToolVersion;
use crate::paths::RequestedConfigPath;

fn requested_state() -> RequestedGenerateState {
    RequestedGenerateState::new(
        RequestedConfigPath::new("/tmp/project/kslim.toml").unwrap(),
        ProfileName::new("default").unwrap(),
        CliOverrides {
            dry_run: false,
            deep_dry_run: false,
            report_only: false,
            force: false,
            offline: false,
            base_ref: None,
            feature: None,
            remove_feature: None,
            preserve_feature: None,
            arch: None,
            primary_arch: None,
            secondary_arch: None,
            safety: None,
            max_fixup_passes: None,
            matrix: None,
            strict: false,
            no_strict: false,
            run_selftests: true,
        },
    )
}

fn resolved_state(max_fixup_passes: usize) -> ResolvedCandidateState {
    let config = config::default_kslim_config("demo", "/tmp/output");
    let mut profile = config::default_profile_config("v1.0");
    profile.reducer.max_fixup_passes = max_fixup_passes;
    ResolvedCandidateState::from_resolved_inputs(
        &config,
        &profile,
        ResolvedBase {
            upstream: String::from("linux"),
            url: String::from("/tmp/linux.git"),
            r#ref: String::from("v1.0"),
            commit: String::from("deadbeef"),
            resolved_at: String::from("2026-01-01T00:00:00Z"),
        },
        None,
        "unmodified-upstream",
        "kslim/v1.0/default",
    )
    .unwrap()
}

#[test]
fn config_content_hash_tracks_resolved_state_and_feeds_plan_fingerprint() {
    let first_resolved = resolved_state(5);
    let identical_resolved = resolved_state(5);
    let changed_resolved = resolved_state(7);

    let first_hash = ConfigContentHash::from_resolved_state(&first_resolved).unwrap();
    let identical_hash = ConfigContentHash::from_resolved_state(&identical_resolved).unwrap();
    let changed_hash = ConfigContentHash::from_resolved_state(&changed_resolved).unwrap();

    assert!(first_hash.as_str().starts_with("config-"));
    assert_eq!(first_hash, identical_hash);
    assert_ne!(first_hash, changed_hash);

    let plan = GeneratePlan::from_parts(
        requested_state(),
        first_resolved,
        first_hash.clone(),
        ToolVersion::new("test-tool").unwrap(),
    )
    .unwrap();

    assert_eq!(plan.config_content_hash, first_hash);
    assert!(plan
        .fingerprint
        .stable_serialization()
        .contains(&format!("config_content_hash={}", first_hash.as_str())));
}
