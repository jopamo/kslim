use super::*;
use crate::config::{self, FeatureIntentConfig, KernelBuildConfig, SlimConfig};
use crate::feature::FeatureKind;
use crate::state::{
    CliOverrides, FeatureGraphFingerprint, FeatureResolutionSource, OutputPlanMode, ProfileName,
    RequestedGenerateState, ResolvedCandidateState,
};
use crate::lockfile::ResolvedBase;
use crate::model::ToolVersion;
use crate::paths::RequestedConfigPath;
use std::collections::BTreeMap;

fn requested_state() -> RequestedGenerateState {
    requested_state_with_cli_overrides(default_cli_overrides())
}

fn requested_state_for_profile(profile: &str) -> RequestedGenerateState {
    RequestedGenerateState::new(
        RequestedConfigPath::new("/project/kslim.toml").unwrap(),
        ProfileName::new(profile).unwrap(),
        default_cli_overrides(),
    )
}

fn requested_state_with_cli_overrides(cli_overrides: CliOverrides) -> RequestedGenerateState {
    RequestedGenerateState::new(
        RequestedConfigPath::new("/project/kslim.toml").unwrap(),
        ProfileName::new("default").unwrap(),
        cli_overrides,
    )
}

fn default_cli_overrides() -> CliOverrides {
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
    }
}

fn resolved_state() -> ResolvedCandidateState {
    let profile = config::default_profile_config("v1.0");
    resolved_state_from_profile(&profile)
}

fn resolved_state_with_base_commit(commit: &str) -> ResolvedCandidateState {
    let profile = config::default_profile_config("v1.0");
    let config = config::default_kslim_config("demo", "/output");
    ResolvedCandidateState::from_resolved_inputs(
        &config,
        &profile,
        ResolvedBase {
            upstream: String::from("linux"),
            url: String::from("/upstream/linux.git"),
            r#ref: String::from("v1.0"),
            commit: commit.to_string(),
            resolved_at: String::from("2026-01-01T00:00:00Z"),
        },
        None,
        OutputPlanMode::from_profile(&profile).stable_name(),
        "kslim/v1.0/default",
    )
    .unwrap()
}

fn resolved_state_from_profile(profile: &config::ProfileConfig) -> ResolvedCandidateState {
    let config = config::default_kslim_config("demo", "/output");
    let mode = OutputPlanMode::from_profile(profile).stable_name();
    ResolvedCandidateState::from_resolved_inputs(
        &config,
        profile,
        ResolvedBase {
            upstream: String::from("linux"),
            url: String::from("/upstream/linux.git"),
            r#ref: String::from("v1.0"),
            commit: String::from("deadbeef"),
            resolved_at: String::from("2026-01-01T00:00:00Z"),
        },
        None,
        mode,
        "kslim/v1.0/default",
    )
    .unwrap()
}

fn mapped_profile(reverse_insert: bool) -> config::ProfileConfig {
    let mut profile = config::default_profile_config("v1.0");
    let set_defaults = map_from_pairs(
        [
            (String::from("CONFIG_ZETA"), String::from("n")),
            (String::from("CONFIG_ALPHA"), String::from("y")),
        ],
        reverse_insert,
    );
    profile.slim = Some(SlimConfig {
        remove_paths: vec![String::from("drivers/demo")],
        remove_configs: Vec::new(),
        set_defaults,
        unsafe_allow_root_path_removal: false,
    });

    let features = [
        (
            String::from("zeta"),
            FeatureIntentConfig {
                roots: vec![String::from("drivers/zeta")],
                safety: Some(config::FeatureSafetyLevel::Aggressive),
                arch_scope: vec![String::from("x86")],
                require_clean_boot: true,
                report_only: true,
                ..FeatureIntentConfig::default()
            },
        ),
        (
            String::from("alpha"),
            FeatureIntentConfig {
                roots: vec![String::from("drivers/alpha")],
                safety: Some(config::FeatureSafetyLevel::Conservative),
                arch_scope: vec![String::from("arm64")],
                require_clean_boot: true,
                report_only: true,
                ..FeatureIntentConfig::default()
            },
        ),
    ];
    insert_pairs(&mut profile.features.remove, features, reverse_insert);

    let env = map_from_pairs(
        [
            (
                String::from("CROSS_COMPILE"),
                String::from("x86_64-linux-gnu-"),
            ),
            (String::from("ARCH"), String::from("x86")),
        ],
        reverse_insert,
    );
    profile.selftests.kernel_builds = vec![KernelBuildConfig {
        name: Some(String::from("map-order")),
        config_target: None,
        targets: Vec::new(),
        output_dir: None,
        jobs: None,
        clean: true,
        make_program: None,
        make_args: Vec::new(),
        env,
    }];

    profile
}

fn array_profile(reverse_order: bool) -> config::ProfileConfig {
    let mut profile = config::default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: string_values(
            reverse_order,
            [
                "drivers/zeta",
                "drivers/alpha",
                "include/linux/zeta.h",
                "include/linux/alpha.h",
                "include/uapi/linux/zeta.h",
                "include/uapi/linux/alpha.h",
            ],
        ),
        remove_configs: string_values(reverse_order, ["CONFIG_ZETA", "CONFIG_ALPHA"]),
        set_defaults: BTreeMap::new(),
        unsafe_allow_root_path_removal: false,
    });
    profile.abi.allow_public_header_removal = true;
    profile.abi.allow_uapi_header_removal = true;
    profile.arch.secondary_arches = string_values(reverse_order, ["x86", "arm64"]);
    profile.arch.disabled_arches = string_values(reverse_order, ["s390", "riscv"]);

    profile.features.remove.insert(
        String::from("arrayed"),
        FeatureIntentConfig {
            roots: string_values(reverse_order, ["net/zeta", "net/alpha"]),
            remove_paths: string_values(
                reverse_order,
                ["drivers/extra-zeta", "drivers/extra-alpha"],
            ),
            configs: string_values(
                reverse_order,
                ["CONFIG_FEATURE_ZETA", "CONFIG_FEATURE_ALPHA"],
            ),
            remove_configs: string_values(
                reverse_order,
                ["CONFIG_REMOVE_ZETA", "CONFIG_REMOVE_ALPHA"],
            ),
            arch_scope: string_values(reverse_order, ["x86", "arm64"]),
            ..FeatureIntentConfig::default()
        },
    );
    profile.features.preserve.insert(
        String::from("keep"),
        FeatureIntentConfig {
            roots: string_values(reverse_order, ["sound/zeta", "sound/alpha"]),
            configs: string_values(reverse_order, ["CONFIG_KEEP_ZETA", "CONFIG_KEEP_ALPHA"]),
            ..FeatureIntentConfig::default()
        },
    );

    profile
}

fn string_values<const N: usize>(reverse_order: bool, values: [&str; N]) -> Vec<String> {
    let mut values = values.into_iter().map(String::from).collect::<Vec<_>>();
    if reverse_order {
        values.reverse();
    }
    values
}

fn map_from_pairs<const N: usize>(
    pairs: [(String, String); N],
    reverse_insert: bool,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    insert_pairs(&mut out, pairs, reverse_insert);
    out
}

fn insert_pairs<K: Ord, V, const N: usize>(
    out: &mut BTreeMap<K, V>,
    pairs: [(K, V); N],
    reverse_insert: bool,
) {
    if reverse_insert {
        for (key, value) in pairs.into_iter().rev() {
            out.insert(key, value);
        }
    } else {
        for (key, value) in pairs {
            out.insert(key, value);
        }
    }
}

fn plan_for_resolved_state(resolved: ResolvedCandidateState) -> GeneratePlan {
    GeneratePlan::from_parts(
        requested_state(),
        resolved,
        ConfigContentHash::new("config-test").unwrap(),
        ToolVersion::new("test-tool").unwrap(),
    )
    .unwrap()
}

fn plan_for_profile(profile: &config::ProfileConfig) -> GeneratePlan {
    let resolved = resolved_state_from_profile(profile);
    let config_content_hash = ConfigContentHash::from_resolved_state(&resolved).unwrap();
    GeneratePlan::from_parts(
        requested_state(),
        resolved,
        config_content_hash,
        ToolVersion::new("test-tool").unwrap(),
    )
    .unwrap()
}

fn plan_for_cli_overrides(cli_overrides: CliOverrides) -> GeneratePlan {
    GeneratePlan::from_parts(
        requested_state_with_cli_overrides(cli_overrides),
        resolved_state(),
        ConfigContentHash::new("config-test").unwrap(),
        ToolVersion::new("test-tool").unwrap(),
    )
    .unwrap()
}

#[test]
fn stable_plan_fingerprint_serialization_has_fixed_line_order_and_digest_source() {
    let plan = GeneratePlan::from_parts(
        requested_state(),
        resolved_state(),
        ConfigContentHash::new("config-test").unwrap(),
        ToolVersion::new("test-tool").unwrap(),
    )
    .unwrap();

    let serialization = plan.fingerprint.stable_serialization();
    assert!(serialization.ends_with('\n'));
    assert!(!serialization.contains("\r\n"));
    assert_eq!(
        plan.fingerprint.as_str(),
        format!("fingerprint-{}", sha256_hex(serialization))
    );

    let keys = serialization
        .lines()
        .map(|line| {
            let (key, _) = line
                .split_once('=')
                .unwrap_or_else(|| panic!("fingerprint line missing '=' delimiter: {line}"));
            key
        })
        .collect::<Vec<_>>();
    assert_eq!(
        keys,
        vec![
            "format",
            "version",
            "tool_version",
            "config_content_hash",
            "source_map.available",
            "requested.selected_profile",
            "requested.cli_overrides.dry_run",
            "requested.cli_overrides.deep_dry_run",
            "requested.cli_overrides.report_only",
            "requested.cli_overrides.force",
            "requested.cli_overrides.offline",
            "requested.cli_overrides.strict",
            "requested.cli_overrides.no_strict",
            "requested.cli_overrides.base_ref",
            "requested.cli_overrides.feature",
            "requested.cli_overrides.remove_feature",
            "requested.cli_overrides.preserve_feature",
            "requested.cli_overrides.arch",
            "requested.cli_overrides.primary_arch",
            "requested.cli_overrides.secondary_arch",
            "requested.cli_overrides.safety",
            "requested.cli_overrides.matrix",
            "requested.cli_overrides.max_fixup_passes",
            "requested.cli_overrides.run_selftests",
            "resolved.base.upstream",
            "resolved.base.ref",
            "resolved.base.commit",
            "resolved.base.resolved_at",
            "resolved.patch_plan.source_count",
            "resolved.patch_plan.total_patch_count",
            "resolved.integration_plan.entry_count",
            "resolved.integration_plan.rtlmq",
            "resolved.feature_graph_fingerprint",
            "resolved.removal_manifest_fingerprint",
            "resolved.abi_policy_fingerprint",
            "resolved.arch_policy_fingerprint",
            "resolved.feature_intent_plan.intent_count",
            "resolved.feature_resolution.source",
            "resolved.feature_resolution.abi_policy.allow_public_header_removal",
            "resolved.feature_resolution.abi_policy.allow_uapi_header_removal",
            "resolved.feature_resolution.unsafe_allow_root_path_removal",
            "resolved.feature_conflicts.total",
            "resolved.feature_conflicts.blocking",
            "resolved.abi_decision.allow_public_header_removal",
            "resolved.abi_decision.allow_uapi_header_removal",
            "resolved.prune_plan.abi_policy.allow_public_header_removal",
            "resolved.prune_plan.abi_policy.allow_uapi_header_removal",
            "resolved.prune_plan.unsafe_allow_root_path_removal",
            "resolved.reducer_plan.max_fixup_passes",
            "resolved.reducer_plan.report_unsupported_expressions",
            "resolved.reducer_plan.fail_on_unknown_diagnostics",
            "resolved.reducer_plan.reject_unproven_fixups",
            "resolved.reducer_plan.reject_unreasoned_edits",
            "resolved.reducer_plan.reject_speculative_fallout_edits",
            "resolved.reducer_plan.fail_on_missing_prune_paths",
            "resolved.reducer_plan.ignore_unsupported_special_removals",
            "resolved.build_matrix_plan.enabled",
            "resolved.build_matrix_plan.preset_count",
            "resolved.build_matrix_plan.arch_count",
            "resolved.build_matrix_plan.config_target_count",
            "resolved.build_matrix_plan.target_count",
            "resolved.build_matrix_plan.randconfig_seed",
            "resolved.build_matrix_plan.jobs",
            "resolved.build_matrix_plan.fail_on_error",
            "resolved.selftest_plan.enabled",
            "resolved.selftest_plan.check_kconfig_sources",
            "resolved.selftest_plan.check_makefiles",
            "resolved.output_plan.branch",
            "resolved.output_plan.mode",
            "resolved.output_plan.naming.project_name",
            "resolved.output_plan.naming.profile_name",
            "resolved.output_plan.naming.branch_prefix",
            "resolved.output_plan.naming.explicit_branch",
            "resolved.output_plan.naming.base_ref",
            "resolved.output_plan.naming.base_commit",
        ]
    );

    assert!(serialization.starts_with(&format!(
        "format={PLAN_FINGERPRINT_SERIALIZATION_FORMAT}\nversion={PLAN_FINGERPRINT_SCHEMA_VERSION}\ntool_version=test-tool\n"
    )));
}

#[test]
fn stable_plan_fingerprint_is_reproducible_for_identical_inputs() {
    let requested = requested_state();
    let resolved = resolved_state();
    let content_hash = ConfigContentHash::new("config-test").unwrap();
    let tool = ToolVersion::new("test-tool").unwrap();
    let mut config_sources = config::ConfigSourceMap::default();
    config_sources.insert(
        "project.name",
        config::ConfigSourceKind::ConfigFile,
        "/project/kslim.toml",
    );
    let mut profile_sources = config::ConfigSourceMap::default();
    profile_sources.insert(
        "base.ref",
        config::ConfigSourceKind::Profile,
        "/project/profiles/default.toml",
    );
    let mut override_sources = config::ConfigSourceMap::default();
    override_sources.insert_cli_override("base.ref", "cli --base");
    let source_maps =
        GeneratePlanSourceMaps::new(config_sources, profile_sources, override_sources);

    let first = GeneratePlan::from_parts(
        requested.clone(),
        resolved.clone(),
        content_hash.clone(),
        tool.clone(),
    )
    .unwrap()
    .with_source_maps(source_maps.clone())
    .unwrap();
    let second = GeneratePlan::from_parts(requested, resolved, content_hash, tool)
        .unwrap()
        .with_source_maps(source_maps)
        .unwrap();

    assert_eq!(first.fingerprint, second.fingerprint);
    assert_eq!(first.plan_id, second.plan_id);
    assert_eq!(
        first.fingerprint.as_str(),
        format!(
            "fingerprint-{}",
            sha256_hex(first.fingerprint.stable_serialization())
        )
    );
    assert_eq!(
        first.fingerprint.stable_serialization(),
        second.fingerprint.stable_serialization()
    );
    assert!(first
        .fingerprint
        .stable_serialization()
        .contains("source_map.available=true"));
}

#[test]
fn stable_plan_fingerprint_includes_tool_version_as_digest_input() {
    let first = GeneratePlan::from_parts(
        requested_state(),
        resolved_state(),
        ConfigContentHash::new("config-test").unwrap(),
        ToolVersion::new("test-tool-a").unwrap(),
    )
    .unwrap();
    let second = GeneratePlan::from_parts(
        requested_state(),
        resolved_state(),
        ConfigContentHash::new("config-test").unwrap(),
        ToolVersion::new("test-tool-b").unwrap(),
    )
    .unwrap();

    assert_ne!(first.fingerprint, second.fingerprint);
    assert_line_before(
        first.fingerprint.stable_serialization(),
        "version=1",
        "tool_version=test-tool-a",
    );
    assert_line_before(
        second.fingerprint.stable_serialization(),
        "version=1",
        "tool_version=test-tool-b",
    );
}

#[test]
fn stable_plan_fingerprint_includes_selected_profile_as_digest_input() {
    let resolved = resolved_state();
    let default = GeneratePlan::from_parts(
        requested_state_for_profile("default"),
        resolved.clone(),
        ConfigContentHash::new("config-test").unwrap(),
        ToolVersion::new("test-tool").unwrap(),
    )
    .unwrap();
    let alternate = GeneratePlan::from_parts(
        requested_state_for_profile("alternate"),
        resolved,
        ConfigContentHash::new("config-test").unwrap(),
        ToolVersion::new("test-tool").unwrap(),
    )
    .unwrap();

    assert_ne!(default.fingerprint, alternate.fingerprint);
    assert_line_before(
        default.fingerprint.stable_serialization(),
        "source_map.available=false",
        "requested.selected_profile=default",
    );
    assert_line_before(
        alternate.fingerprint.stable_serialization(),
        "source_map.available=false",
        "requested.selected_profile=alternate",
    );
}

#[test]
fn stable_plan_fingerprint_includes_normalized_cli_overrides_as_digest_input() {
    let raw_overrides = CliOverrides {
        dry_run: true,
        deep_dry_run: false,
        report_only: false,
        force: true,
        offline: true,
        base_ref: Some(String::from(" HEAD ")),
        feature: Some(String::from(" bluetooth ")),
        remove_feature: None,
        preserve_feature: None,
        arch: Some(String::from(" x86 ")),
        primary_arch: None,
        secondary_arch: None,
        safety: Some(String::from(" surgical ")),
        max_fixup_passes: Some(7),
        matrix: Some(String::from(" HARDENING ")),
        strict: true,
        no_strict: false,
        run_selftests: false,
    };
    let mut normalized_overrides = raw_overrides.clone();
    normalized_overrides.base_ref = Some(String::from("HEAD"));
    normalized_overrides.feature = Some(String::from("bluetooth"));
    normalized_overrides.arch = Some(String::from("x86"));
    normalized_overrides.safety = Some(String::from("surgical"));
    normalized_overrides.matrix = Some(String::from("hardening"));

    let raw = GeneratePlan::from_parts(
        requested_state_with_cli_overrides(raw_overrides),
        resolved_state(),
        ConfigContentHash::new("config-test").unwrap(),
        ToolVersion::new("test-tool").unwrap(),
    )
    .unwrap();
    let normalized = GeneratePlan::from_parts(
        requested_state_with_cli_overrides(normalized_overrides.clone()),
        resolved_state(),
        ConfigContentHash::new("config-test").unwrap(),
        ToolVersion::new("test-tool").unwrap(),
    )
    .unwrap();
    let mut changed_overrides = normalized_overrides;
    changed_overrides.matrix = Some(String::from("runtime"));
    let changed = GeneratePlan::from_parts(
        requested_state_with_cli_overrides(changed_overrides),
        resolved_state(),
        ConfigContentHash::new("config-test").unwrap(),
        ToolVersion::new("test-tool").unwrap(),
    )
    .unwrap();

    assert_eq!(raw.fingerprint, normalized.fingerprint);
    assert_ne!(normalized.fingerprint, changed.fingerprint);
    let serialization = normalized.fingerprint.stable_serialization();
    for line in [
        "requested.cli_overrides.dry_run=true",
        "requested.cli_overrides.deep_dry_run=false",
        "requested.cli_overrides.report_only=false",
        "requested.cli_overrides.force=true",
        "requested.cli_overrides.offline=true",
        "requested.cli_overrides.strict=true",
        "requested.cli_overrides.no_strict=false",
        "requested.cli_overrides.base_ref=HEAD",
        "requested.cli_overrides.feature=bluetooth",
        "requested.cli_overrides.remove_feature=<none>",
        "requested.cli_overrides.preserve_feature=<none>",
        "requested.cli_overrides.arch=x86",
        "requested.cli_overrides.primary_arch=<none>",
        "requested.cli_overrides.secondary_arch=<none>",
        "requested.cli_overrides.safety=surgical",
        "requested.cli_overrides.matrix=hardening",
        "requested.cli_overrides.max_fixup_passes=7",
        "requested.cli_overrides.run_selftests=false",
    ] {
        assert!(
            serialization.lines().any(|candidate| candidate == line),
            "missing normalized CLI override fingerprint line: {line}"
        );
    }
}

#[test]
fn stable_plan_fingerprint_changes_for_each_cli_override_change() {
    let default = plan_for_cli_overrides(default_cli_overrides());
    let cases = [
        (
            "dry_run",
            CliOverrides {
                dry_run: true,
                ..default_cli_overrides()
            },
            "requested.cli_overrides.dry_run=true",
        ),
        (
            "deep_dry_run",
            CliOverrides {
                deep_dry_run: true,
                ..default_cli_overrides()
            },
            "requested.cli_overrides.deep_dry_run=true",
        ),
        (
            "report_only",
            CliOverrides {
                report_only: true,
                ..default_cli_overrides()
            },
            "requested.cli_overrides.report_only=true",
        ),
        (
            "force",
            CliOverrides {
                force: true,
                ..default_cli_overrides()
            },
            "requested.cli_overrides.force=true",
        ),
        (
            "offline",
            CliOverrides {
                offline: true,
                ..default_cli_overrides()
            },
            "requested.cli_overrides.offline=true",
        ),
        (
            "strict",
            CliOverrides {
                strict: true,
                ..default_cli_overrides()
            },
            "requested.cli_overrides.strict=true",
        ),
        (
            "no_strict",
            CliOverrides {
                no_strict: true,
                ..default_cli_overrides()
            },
            "requested.cli_overrides.no_strict=true",
        ),
        (
            "base_ref",
            CliOverrides {
                base_ref: Some(String::from("v6.10")),
                ..default_cli_overrides()
            },
            "requested.cli_overrides.base_ref=v6.10",
        ),
        (
            "feature",
            CliOverrides {
                feature: Some(String::from("bluetooth")),
                ..default_cli_overrides()
            },
            "requested.cli_overrides.feature=bluetooth",
        ),
        (
            "remove_feature",
            CliOverrides {
                remove_feature: Some(String::from("wifi")),
                ..default_cli_overrides()
            },
            "requested.cli_overrides.remove_feature=wifi",
        ),
        (
            "preserve_feature",
            CliOverrides {
                preserve_feature: Some(String::from("usb")),
                ..default_cli_overrides()
            },
            "requested.cli_overrides.preserve_feature=usb",
        ),
        (
            "arch",
            CliOverrides {
                arch: Some(String::from("x86")),
                ..default_cli_overrides()
            },
            "requested.cli_overrides.arch=x86",
        ),
        (
            "primary_arch",
            CliOverrides {
                primary_arch: Some(String::from("arm64")),
                ..default_cli_overrides()
            },
            "requested.cli_overrides.primary_arch=arm64",
        ),
        (
            "secondary_arch",
            CliOverrides {
                secondary_arch: Some(String::from("riscv")),
                ..default_cli_overrides()
            },
            "requested.cli_overrides.secondary_arch=riscv",
        ),
        (
            "safety",
            CliOverrides {
                safety: Some(String::from("surgical")),
                ..default_cli_overrides()
            },
            "requested.cli_overrides.safety=surgical",
        ),
        (
            "matrix",
            CliOverrides {
                matrix: Some(String::from("hardening")),
                ..default_cli_overrides()
            },
            "requested.cli_overrides.matrix=hardening",
        ),
        (
            "max_fixup_passes",
            CliOverrides {
                max_fixup_passes: Some(11),
                ..default_cli_overrides()
            },
            "requested.cli_overrides.max_fixup_passes=11",
        ),
        (
            "run_selftests",
            CliOverrides {
                run_selftests: false,
                ..default_cli_overrides()
            },
            "requested.cli_overrides.run_selftests=false",
        ),
    ];

    for (field, overrides, expected_line) in cases {
        let changed = plan_for_cli_overrides(overrides);
        assert_ne!(
            default.fingerprint, changed.fingerprint,
            "fingerprint did not change when cli override {field} changed"
        );
        assert_ne!(
            default.plan_id, changed.plan_id,
            "plan id did not change when cli override {field} changed"
        );
        let serialization = changed.fingerprint.stable_serialization();
        assert!(
            serialization
                .lines()
                .any(|candidate| candidate == expected_line),
            "missing changed CLI override fingerprint line for {field}: {expected_line}"
        );
    }
}

#[test]
fn stable_plan_fingerprint_changes_for_profile_config_changes() {
    let default_profile = config::default_profile_config("v1.0");
    let default = plan_for_profile(&default_profile);

    let mut direct_slim = config::default_profile_config("v1.0");
    direct_slim.slim = Some(SlimConfig {
        remove_paths: vec![String::from("drivers/demo")],
        remove_configs: vec![String::from("CONFIG_DEMO")],
        set_defaults: map_from_pairs(
            [(String::from("CONFIG_DEMO_DEBUG"), String::from("n"))],
            false,
        ),
        unsafe_allow_root_path_removal: false,
    });

    let mut named_preserve = config::default_profile_config("v1.0");
    named_preserve.features.preserve.insert(
        String::from("keep-net"),
        FeatureIntentConfig {
            roots: vec![String::from("net/keep")],
            configs: vec![String::from("CONFIG_KEEP_NET")],
            ..FeatureIntentConfig::default()
        },
    );

    let mut reducer_policy = config::default_profile_config("v1.0");
    reducer_policy.reducer.max_fixup_passes = 9;
    reducer_policy.reducer.fail_on_missing_prune_paths = true;

    let mut selftests = config::default_profile_config("v1.0");
    selftests.selftests.enabled = false;
    selftests
        .selftests
        .commands
        .push(String::from("echo profile-smoke"));

    let mut integration = config::default_profile_config("v1.0");
    integration.integrations.rtlmq = Some(config::RtlmqIntegrationConfig {
        source: String::from("/project/integrations/rtlmq"),
        tests_source: Some(String::from("/project/integrations/rtlmq-tests")),
    });

    let mut renamed = config::default_profile_config("v1.0");
    renamed.profile.name = String::from("alternate");

    let cases: Vec<(&str, config::ProfileConfig, Vec<&str>)> = vec![
        (
            "direct slim removal",
            direct_slim,
            vec![
                "resolved.feature_resolution.source=direct_slim_input",
                "resolved.feature_resolution.remove_paths=drivers/demo",
                "resolved.feature_resolution.remove_configs=CONFIG_DEMO",
                "resolved.feature_resolution.set_defaults.symbol=CONFIG_DEMO_DEBUG",
                "resolved.feature_resolution.set_defaults.value=n",
                "resolved.output_plan.mode=slimmed",
            ],
        ),
        (
            "named preserve intent",
            named_preserve,
            vec![
                "resolved.feature_intent_plan.intent_count=1",
                "resolved.feature_intent_plan.intents.0.action=preserve",
                "resolved.feature_intent_plan.intents.0.name=keep-net",
                "resolved.feature_resolution.preserve_paths=net/keep",
                "resolved.feature_resolution.preserve_configs=CONFIG_KEEP_NET",
            ],
        ),
        (
            "reducer policy",
            reducer_policy,
            vec![
                "resolved.reducer_plan.max_fixup_passes=9",
                "resolved.reducer_plan.fail_on_missing_prune_paths=true",
            ],
        ),
        (
            "selftest policy",
            selftests,
            vec![
                "resolved.selftest_plan.enabled=false",
                "resolved.selftest_plan.commands=echo profile-smoke",
            ],
        ),
        (
            "integration policy",
            integration,
            vec![
                "resolved.integration_plan.entry_count=1",
                "resolved.integration_plan.entries.0.kind=rtlmq",
            ],
        ),
        (
            "profile naming",
            renamed,
            vec!["resolved.output_plan.naming.profile_name=alternate"],
        ),
    ];

    for (case, profile, expected_lines) in cases {
        let changed = plan_for_profile(&profile);
        assert_ne!(
            default.config_content_hash, changed.config_content_hash,
            "config content hash did not change for profile change case: {case}"
        );
        assert_ne!(
            default.fingerprint, changed.fingerprint,
            "fingerprint did not change for profile change case: {case}"
        );
        assert_ne!(
            default.plan_id, changed.plan_id,
            "plan id did not change for profile change case: {case}"
        );
        let serialization = changed.fingerprint.stable_serialization();
        for expected_line in expected_lines {
            assert!(
                serialization
                    .lines()
                    .any(|candidate| candidate == expected_line),
                "missing profile-derived fingerprint line for {case}: {expected_line}"
            );
        }
    }
}

#[test]
fn stable_plan_fingerprint_includes_resolved_base_commit_as_digest_input() {
    let first = plan_for_resolved_state(resolved_state_with_base_commit("deadbeef"));
    let second = plan_for_resolved_state(resolved_state_with_base_commit("feedface"));

    assert_ne!(first.fingerprint, second.fingerprint);
    assert_line_before(
        first.fingerprint.stable_serialization(),
        "resolved.base.ref=v1.0",
        "resolved.base.commit=deadbeef",
    );
    assert_line_before(
        first.fingerprint.stable_serialization(),
        "resolved.base.commit=deadbeef",
        "resolved.base.resolved_at=2026-01-01T00:00:00Z",
    );
    assert_line_before(
        second.fingerprint.stable_serialization(),
        "resolved.base.ref=v1.0",
        "resolved.base.commit=feedface",
    );
}

#[test]
fn stable_plan_fingerprint_includes_feature_graph_fingerprint_as_digest_input() {
    let mut first_resolved = resolved_state();
    first_resolved.feature_graph_fingerprint =
        FeatureGraphFingerprint::new("feature-graph-first").unwrap();
    let mut second_resolved = first_resolved.clone();
    second_resolved.feature_graph_fingerprint =
        FeatureGraphFingerprint::new("feature-graph-second").unwrap();

    let first = plan_for_resolved_state(first_resolved);
    let second = plan_for_resolved_state(second_resolved);

    assert_ne!(first.fingerprint, second.fingerprint);
    assert_line_before(
        first.fingerprint.stable_serialization(),
        "resolved.integration_plan.rtlmq=<none>",
        "resolved.feature_graph_fingerprint=feature-graph-first",
    );
    assert_line_containing_before(
        first.fingerprint.stable_serialization(),
        "resolved.feature_graph_fingerprint=feature-graph-first",
        "resolved.removal_manifest_fingerprint=removal-manifest-",
    );
    assert_line_before(
        second.fingerprint.stable_serialization(),
        "resolved.integration_plan.rtlmq=<none>",
        "resolved.feature_graph_fingerprint=feature-graph-second",
    );
}

#[test]
fn stable_plan_fingerprint_includes_abi_policy_as_digest_input() {
    let fail_closed = plan_for_resolved_state(resolved_state());
    let mut allowed_profile = config::default_profile_config("v1.0");
    allowed_profile.abi.allow_public_header_removal = true;
    allowed_profile.abi.allow_uapi_header_removal = true;
    let allowed = plan_for_resolved_state(resolved_state_from_profile(&allowed_profile));

    assert_ne!(fail_closed.fingerprint, allowed.fingerprint);
    let serialization = allowed.fingerprint.stable_serialization();
    assert_line_containing_before(
        serialization,
        "resolved.abi_policy_fingerprint=abi-policy-",
        "resolved.arch_policy_fingerprint=arch-policy-",
    );
    for line in [
        "resolved.feature_resolution.abi_policy.allow_public_header_removal=true",
        "resolved.feature_resolution.abi_policy.allow_uapi_header_removal=true",
        "resolved.abi_decision.allow_public_header_removal=true",
        "resolved.abi_decision.allow_uapi_header_removal=true",
        "resolved.prune_plan.abi_policy.allow_public_header_removal=true",
        "resolved.prune_plan.abi_policy.allow_uapi_header_removal=true",
    ] {
        assert!(
            serialization.lines().any(|candidate| candidate == line),
            "missing ABI policy fingerprint line: {line}"
        );
    }

    let fail_closed_serialization = fail_closed.fingerprint.stable_serialization();
    for line in [
        "resolved.feature_resolution.abi_policy.allow_public_header_removal=false",
        "resolved.feature_resolution.abi_policy.allow_uapi_header_removal=false",
        "resolved.abi_decision.allow_public_header_removal=false",
        "resolved.abi_decision.allow_uapi_header_removal=false",
        "resolved.prune_plan.abi_policy.allow_public_header_removal=false",
        "resolved.prune_plan.abi_policy.allow_uapi_header_removal=false",
    ] {
        assert!(
            fail_closed_serialization
                .lines()
                .any(|candidate| candidate == line),
            "missing fail-closed ABI policy fingerprint line: {line}"
        );
    }
}

#[test]
fn stable_plan_fingerprint_includes_build_matrix_as_digest_input() {
    let default = plan_for_resolved_state(resolved_state());
    let mut matrix_profile = config::default_profile_config("v1.0");
    matrix_profile.build_matrix.enabled = true;
    matrix_profile.build_matrix.presets = string_values(false, ["hardening", "default"]);
    matrix_profile.build_matrix.arches = string_values(false, ["x86", "arm64"]);
    matrix_profile.build_matrix.config_targets =
        string_values(false, ["defconfig", "allmodconfig"]);
    matrix_profile.build_matrix.targets = string_values(false, ["vmlinux", "modules"]);
    matrix_profile.build_matrix.randconfig_seed = Some(String::from("seed-1"));
    matrix_profile.build_matrix.jobs = Some(16);
    matrix_profile.build_matrix.fail_on_error = false;

    let matrix = plan_for_resolved_state(resolved_state_from_profile(&matrix_profile));

    assert_ne!(default.fingerprint, matrix.fingerprint);
    let serialization = matrix.fingerprint.stable_serialization();
    for line in [
        "resolved.build_matrix_plan.enabled=true",
        "resolved.build_matrix_plan.preset_count=2",
        "resolved.build_matrix_plan.arch_count=2",
        "resolved.build_matrix_plan.config_target_count=2",
        "resolved.build_matrix_plan.target_count=2",
        "resolved.build_matrix_plan.randconfig_seed=seed-1",
        "resolved.build_matrix_plan.jobs=16",
        "resolved.build_matrix_plan.fail_on_error=false",
    ] {
        assert!(
            serialization.lines().any(|candidate| candidate == line),
            "missing build matrix fingerprint line: {line}"
        );
    }
    assert_line_before(
        serialization,
        "resolved.build_matrix_plan.presets=default",
        "resolved.build_matrix_plan.presets=hardening",
    );
    assert_line_before(
        serialization,
        "resolved.build_matrix_plan.arches=arm64",
        "resolved.build_matrix_plan.arches=x86",
    );
    assert_line_before(
        serialization,
        "resolved.build_matrix_plan.config_targets=allmodconfig",
        "resolved.build_matrix_plan.config_targets=defconfig",
    );
    assert_line_before(
        serialization,
        "resolved.build_matrix_plan.targets=modules",
        "resolved.build_matrix_plan.targets=vmlinux",
    );
}

#[test]
fn stable_fingerprint_line_serialization_escapes_control_characters() {
    let mut out = String::new();

    append_fingerprint_line(&mut out, "field", "slash\\line\ncarriage\rtab\tend");

    assert_eq!(out, "field=slash\\\\line\\ncarriage\\rtab\\tend\n");
}

#[test]
fn stable_plan_fingerprint_serializes_maps_in_key_order() {
    let forward = plan_for_resolved_state(resolved_state_from_profile(&mapped_profile(false)));
    let reverse = plan_for_resolved_state(resolved_state_from_profile(&mapped_profile(true)));

    assert_eq!(forward.fingerprint, reverse.fingerprint);
    let serialization = forward.fingerprint.stable_serialization();
    assert_line_before(
        serialization,
        "resolved.feature_resolution.set_defaults.symbol=CONFIG_ALPHA",
        "resolved.feature_resolution.set_defaults.symbol=CONFIG_ZETA",
    );
    assert_line_before(
        serialization,
        "resolved.feature_resolution.feature_safety_levels.feature=alpha",
        "resolved.feature_resolution.feature_safety_levels.feature=zeta",
    );
    assert_line_before(
        serialization,
        "resolved.feature_resolution.feature_arch_scopes.feature=alpha",
        "resolved.feature_resolution.feature_arch_scopes.feature=zeta",
    );
    assert_line_before(
        serialization,
        "resolved.feature_resolution.feature_test_matrices.feature=alpha",
        "resolved.feature_resolution.feature_test_matrices.feature=zeta",
    );
    assert_line_before(
        serialization,
        "resolved.feature_resolution.feature_report_modes.feature=alpha",
        "resolved.feature_resolution.feature_report_modes.feature=zeta",
    );
    assert_line_before(
        serialization,
        "resolved.prune_plan.set_defaults.symbol=CONFIG_ALPHA",
        "resolved.prune_plan.set_defaults.symbol=CONFIG_ZETA",
    );
    assert_line_before(
        serialization,
        "resolved.selftest_plan.kernel_builds.0.env.name=ARCH",
        "resolved.selftest_plan.kernel_builds.0.env.name=CROSS_COMPILE",
    );
}

#[test]
fn stable_plan_fingerprint_serializes_set_like_arrays_in_sorted_order() {
    let forward = plan_for_resolved_state(resolved_state_from_profile(&array_profile(false)));
    let reverse = plan_for_resolved_state(resolved_state_from_profile(&array_profile(true)));

    assert_eq!(forward.fingerprint, reverse.fingerprint);
    let serialization = forward.fingerprint.stable_serialization();
    assert_line_containing_before(serialization, ".roots=net/alpha", ".roots=net/zeta");
    assert_line_containing_before(
        serialization,
        ".remove_paths=drivers/extra-alpha",
        ".remove_paths=drivers/extra-zeta",
    );
    assert_line_containing_before(
        serialization,
        ".configs=CONFIG_FEATURE_ALPHA",
        ".configs=CONFIG_FEATURE_ZETA",
    );
    assert_line_containing_before(
        serialization,
        ".remove_configs=CONFIG_REMOVE_ALPHA",
        ".remove_configs=CONFIG_REMOVE_ZETA",
    );
    assert_line_containing_before(serialization, ".arch_scope=arm64", ".arch_scope=x86");
    assert_line_before(
        serialization,
        "resolved.feature_resolution.remove_paths=drivers/alpha",
        "resolved.feature_resolution.remove_paths=drivers/zeta",
    );
    assert_line_before(
        serialization,
        "resolved.feature_resolution.remove_configs=CONFIG_ALPHA",
        "resolved.feature_resolution.remove_configs=CONFIG_ZETA",
    );
    assert_line_before(
        serialization,
        "resolved.feature_resolution.preserve_paths=sound/alpha",
        "resolved.feature_resolution.preserve_paths=sound/zeta",
    );
    assert_line_before(
        serialization,
        "resolved.feature_resolution.preserve_configs=CONFIG_KEEP_ALPHA",
        "resolved.feature_resolution.preserve_configs=CONFIG_KEEP_ZETA",
    );
    assert_line_before(
        serialization,
        "resolved.feature_resolution.feature_arch_scopes.arch=arm64",
        "resolved.feature_resolution.feature_arch_scopes.arch=x86",
    );
    assert_line_before(
        serialization,
        "resolved.abi_decision.approved_public_headers=include/linux/alpha.h",
        "resolved.abi_decision.approved_public_headers=include/linux/zeta.h",
    );
    assert_line_before(
        serialization,
        "resolved.abi_decision.approved_uapi_paths=include/uapi/linux/alpha.h",
        "resolved.abi_decision.approved_uapi_paths=include/uapi/linux/zeta.h",
    );
    assert_line_before(
        serialization,
        "resolved.prune_plan.remove_paths=drivers/alpha",
        "resolved.prune_plan.remove_paths=drivers/zeta",
    );
    assert_line_before(
        serialization,
        "resolved.prune_plan.remove_configs=CONFIG_ALPHA",
        "resolved.prune_plan.remove_configs=CONFIG_ZETA",
    );
    assert_line_before(
        serialization,
        "resolved.prune_plan.preserve_paths=sound/alpha",
        "resolved.prune_plan.preserve_paths=sound/zeta",
    );
    assert_line_before(
        serialization,
        "resolved.prune_plan.preserve_configs=CONFIG_KEEP_ALPHA",
        "resolved.prune_plan.preserve_configs=CONFIG_KEEP_ZETA",
    );
}

#[test]
fn stable_plan_fingerprint_serializes_enum_values_as_stable_tokens() {
    let mut profile = config::default_profile_config("v1.0");
    profile.slim = Some(SlimConfig {
        remove_paths: vec![String::from("drivers/demo")],
        remove_configs: Vec::new(),
        set_defaults: BTreeMap::new(),
        unsafe_allow_root_path_removal: false,
    });
    profile.features.remove.insert(
        String::from("wireless"),
        FeatureIntentConfig {
            kind: Some(String::from("network protocol")),
            roots: vec![String::from("net/wireless")],
            safety: Some(config::FeatureSafetyLevel::Aggressive),
            ..FeatureIntentConfig::default()
        },
    );
    let mut config_sources = config::ConfigSourceMap::default();
    config_sources.insert(
        "defaulted",
        config::ConfigSourceKind::Default,
        "built-in default",
    );
    config_sources.insert(
        "config_file",
        config::ConfigSourceKind::ConfigFile,
        "/project/kslim.toml",
    );
    config_sources.insert(
        "profile",
        config::ConfigSourceKind::Profile,
        "/project/profiles/default.toml",
    );
    config_sources.insert(
        "include_file",
        config::ConfigSourceKind::IncludeFile,
        "profiles/common.toml",
    );
    config_sources.insert(
        "environment",
        config::ConfigSourceKind::Environment,
        "KSLIM_PROFILE",
    );
    config_sources.insert("cli", config::ConfigSourceKind::Cli, "cli --base");

    let plan = plan_for_resolved_state(resolved_state_from_profile(&profile))
        .with_source_maps(GeneratePlanSourceMaps::new(
            config_sources,
            config::ConfigSourceMap::default(),
            config::ConfigSourceMap::default(),
        ))
        .unwrap();

    let serialization = plan.fingerprint.stable_serialization();
    assert_eq!(
        OutputPlanMode::ALL
            .into_iter()
            .map(OutputPlanMode::stable_name)
            .collect::<Vec<_>>(),
        vec!["unmodified-upstream", "slimmed"]
    );
    assert_eq!(
        FeatureResolutionSource::ALL
            .into_iter()
            .map(FeatureResolutionSource::stable_name)
            .collect::<Vec<_>>(),
        vec![
            "no_removal_input",
            "direct_slim_input",
            "named_feature_remove_input",
            "combined_slim_and_named_feature_input",
        ]
    );
    assert_eq!(
        FeatureKind::ALL
            .into_iter()
            .map(FeatureKind::stable_name)
            .collect::<Vec<_>>(),
        vec![
            "subsystem",
            "driver",
            "bus",
            "filesystem",
            "network_protocol",
            "crypto_algorithm",
            "scheduler_feature",
            "security_feature",
            "tracing_feature",
            "bpf_feature",
            "arch_feature",
            "soc_platform",
            "board_platform_support",
            "firmware_loader_feature",
            "module_only_feature",
            "userspace_abi_feature",
            "generated_artifact_family",
            "docs_tests_tools_only_feature",
        ]
    );
    for (line, rust_name) in [
        ("source_map.config.defaulted.kind=default", "Default"),
        (
            "source_map.config.config_file.kind=config_file",
            "ConfigFile",
        ),
        ("source_map.config.profile.kind=profile", "Profile"),
        (
            "source_map.config.include_file.kind=include_file",
            "IncludeFile",
        ),
        (
            "source_map.config.environment.kind=environment",
            "Environment",
        ),
        ("source_map.config.cli.kind=cli", "Cli"),
        (
            "resolved.feature_resolution.source=combined_slim_and_named_feature_input",
            "CombinedSlimAndNamedFeature",
        ),
        (
            "resolved.feature_resolution.feature_safety_levels.level=aggressive",
            "Aggressive",
        ),
        (
            "resolved.feature_intent_plan.intents.0.kind=network_protocol",
            "NetworkProtocol",
        ),
        ("resolved.output_plan.mode=slimmed", "Slimmed"),
    ] {
        assert!(
            serialization.contains(line),
            "missing stable enum fingerprint line: {line}"
        );
        assert!(
            !serialization.contains(&format!("={rust_name}")),
            "fingerprint must not serialize Rust enum variant name {rust_name}"
        );
    }
    assert_line_containing(serialization, ".action=remove");
    assert_line_containing(serialization, ".safety=aggressive");

    let config_model = config::default_kslim_config("demo", "/output");
    let invalid_mode = ResolvedCandidateState::from_resolved_inputs(
        &config_model,
        &config::default_profile_config("v1.0"),
        ResolvedBase {
            upstream: String::from("linux"),
            url: String::from("/upstream/linux.git"),
            r#ref: String::from("v1.0"),
            commit: String::from("deadbeef"),
            resolved_at: String::from("2026-01-01T00:00:00Z"),
        },
        None,
        "Slimmed",
        "kslim/v1.0/default",
    )
    .unwrap_err();
    assert!(format!("{invalid_mode:#}").contains("stable token"));
}

#[test]
fn stable_plan_fingerprint_serializes_source_map_entries_in_key_order() {
    let resolved = resolved_state();
    let mut forward = config::ConfigSourceMap::default();
    forward.insert("alpha.value", config::ConfigSourceKind::Default, "default");
    forward.insert("zeta.value", config::ConfigSourceKind::Default, "default");
    let mut reverse = config::ConfigSourceMap::default();
    reverse.insert("zeta.value", config::ConfigSourceKind::Default, "default");
    reverse.insert("alpha.value", config::ConfigSourceKind::Default, "default");

    let forward = plan_for_resolved_state(resolved.clone())
        .with_source_maps(GeneratePlanSourceMaps::new(
            forward,
            config::ConfigSourceMap::default(),
            config::ConfigSourceMap::default(),
        ))
        .unwrap();
    let reverse = plan_for_resolved_state(resolved)
        .with_source_maps(GeneratePlanSourceMaps::new(
            reverse,
            config::ConfigSourceMap::default(),
            config::ConfigSourceMap::default(),
        ))
        .unwrap();

    assert_eq!(forward.fingerprint, reverse.fingerprint);
    assert_line_before(
        forward.fingerprint.stable_serialization(),
        "source_map.config.alpha.value.kind=default",
        "source_map.config.zeta.value.kind=default",
    );
}

#[test]
fn stable_plan_fingerprint_excludes_temporary_paths_from_source_map_inputs() {
    let temp_root = std::path::PathBuf::from("/var/tmp");
    let first_temp = temp_root
        .join("kslim-first-source")
        .join("profile.toml")
        .to_string_lossy()
        .into_owned();
    let second_temp = temp_root
        .join("kslim-second-source")
        .join("profile.toml")
        .to_string_lossy()
        .into_owned();

    let mut first_sources = config::ConfigSourceMap::default();
    first_sources.insert(
        "temp.value",
        config::ConfigSourceKind::IncludeFile,
        &first_temp,
    );
    let mut second_sources = config::ConfigSourceMap::default();
    second_sources.insert(
        "temp.value",
        config::ConfigSourceKind::IncludeFile,
        &second_temp,
    );

    let first = plan_for_resolved_state(resolved_state())
        .with_source_maps(GeneratePlanSourceMaps::new(
            first_sources,
            config::ConfigSourceMap::default(),
            config::ConfigSourceMap::default(),
        ))
        .unwrap();
    let second = plan_for_resolved_state(resolved_state())
        .with_source_maps(GeneratePlanSourceMaps::new(
            second_sources,
            config::ConfigSourceMap::default(),
            config::ConfigSourceMap::default(),
        ))
        .unwrap();

    assert_eq!(first.fingerprint, second.fingerprint);
    let serialization = first.fingerprint.stable_serialization();
    assert!(serialization.contains("source_map.config.temp.value.source=<temporary-path>"));
    assert!(!serialization.contains(&first_temp));
    assert!(!second
        .fingerprint
        .stable_serialization()
        .contains(&second_temp));
}

fn assert_line_containing(serialization: &str, needle: &str) {
    assert!(
        serialization.lines().any(|line| line.contains(needle)),
        "missing fingerprint line containing: {needle}"
    );
}

fn assert_line_containing_before(serialization: &str, first: &str, second: &str) {
    let first_index = serialization
        .lines()
        .position(|line| line.contains(first))
        .unwrap_or_else(|| panic!("missing fingerprint line containing: {first}"));
    let second_index = serialization
        .lines()
        .position(|line| line.contains(second))
        .unwrap_or_else(|| panic!("missing fingerprint line containing: {second}"));
    assert!(
        first_index < second_index,
        "expected line containing {first:?} before line containing {second:?} in fingerprint serialization"
    );
}

fn assert_line_before(serialization: &str, first: &str, second: &str) {
    let first_index = serialization
        .lines()
        .position(|line| line == first)
        .unwrap_or_else(|| panic!("missing fingerprint line: {first}"));
    let second_index = serialization
        .lines()
        .position(|line| line == second)
        .unwrap_or_else(|| panic!("missing fingerprint line: {second}"));
    assert!(
        first_index < second_index,
        "expected {first:?} before {second:?} in fingerprint serialization"
    );
}
