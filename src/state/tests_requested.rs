use super::*;
use crate::config;

#[test]
fn test_requested_generate_state_captures_request_inputs() {
    let opts = GenerateOptions {
        dry_run: true,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: Some(5),
        matrix: Some(String::from(" HARDENING ")),
        offline: true,
        frozen_plan: None,
        force: true,
        base_ref: Some(String::from("HEAD")),
        feature: None,
        remove_feature: None,
        preserve_feature: None,
        arch: None,
        primary_arch: None,
        secondary_arch: None,
        safety: None,
        strict: true,
        no_strict: false,
        run_selftests: false,
    };
    let profile = config::default_profile_config("v1.0");
    let config_path = PathBuf::from("/tmp/project/kslim.toml");

    let requested =
        RequestedGenerateState::from_inputs(config_path.clone(), &profile, &opts).unwrap();

    assert_eq!(requested.config_path.as_path(), config_path.as_path());
    assert_eq!(requested.selected_profile.as_str(), "default");
    assert!(requested.cli_overrides.dry_run);
    assert!(requested.cli_overrides.force);
    assert!(requested.cli_overrides.offline);
    assert!(requested.cli_overrides.strict);
    assert!(!requested.cli_overrides.no_strict);
    assert_eq!(requested.cli_overrides.base_ref.as_deref(), Some("HEAD"));
    assert_eq!(requested.cli_overrides.max_fixup_passes, Some(5));
    assert_eq!(requested.cli_overrides.matrix.as_deref(), Some("hardening"));
    assert!(!requested.cli_overrides.run_selftests);
    let mut loose_profile = profile.clone();
    loose_profile.reducer.fail_on_unknown_diagnostics = false;
    loose_profile.selftests.check_makefiles = false;
    let overridden = requested
        .cli_overrides
        .apply_profile_overrides(loose_profile)
        .unwrap();
    assert!(overridden.reducer.strict_mode());
    assert_eq!(overridden.reducer.max_fixup_passes, 5);
    assert!(overridden.selftests.check_makefiles);
    let identity = requested.identity().unwrap();
    assert_eq!(identity.phase, GenerateStatePhase::Requested);
    assert!(identity.key.contains("config=/tmp/project/kslim.toml"));
    assert!(identity.key.contains("profile=default"));
    assert!(identity.key.contains("base_override=HEAD"));
    assert!(identity.key.contains("offline=true"));
    assert!(identity.key.contains("max_fixup_passes_override=5"));
    assert!(identity.key.contains("matrix_override=hardening"));
}

#[test]
fn test_requested_generate_state_no_strict_disables_publication_gates() {
    let opts = GenerateOptions {
        dry_run: false,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: None,
        matrix: None,
        offline: false,
        frozen_plan: None,
        force: false,
        base_ref: None,
        feature: None,
        remove_feature: None,
        preserve_feature: None,
        arch: None,
        primary_arch: None,
        secondary_arch: None,
        safety: None,
        strict: false,
        no_strict: true,
        run_selftests: true,
    };
    let profile = config::default_profile_config("v1.0");
    let requested =
        RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts).unwrap();

    assert_eq!(
        requested.cli_overrides.strictness_cli_source(),
        Some("cli --no-strict")
    );
    let profile = requested
        .cli_overrides
        .apply_profile_overrides(profile)
        .unwrap();
    assert!(!profile.reducer.report_unsupported_expressions);
    assert!(!profile.reducer.fail_on_unknown_diagnostics);
    assert!(!profile.reducer.reject_unproven_fixups);
    assert!(!profile.reducer.reject_unreasoned_edits);
    assert!(!profile.reducer.reject_speculative_fallout_edits);
    assert!(!profile.reducer.strict_mode());
    assert!(requested.identity().unwrap().key.contains("no_strict=true"));
}

#[test]
fn test_requested_generate_state_rejects_conflicting_strictness_flags() {
    let opts = GenerateOptions {
        dry_run: false,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: None,
        matrix: None,
        offline: false,
        frozen_plan: None,
        force: false,
        base_ref: None,
        feature: None,
        remove_feature: None,
        preserve_feature: None,
        arch: None,
        primary_arch: None,
        secondary_arch: None,
        safety: None,
        strict: true,
        no_strict: true,
        run_selftests: true,
    };
    let profile = config::default_profile_config("v1.0");

    let err = RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
        .unwrap_err()
        .to_string();

    assert!(err.contains("cli --strict and --no-strict are mutually exclusive"));
}

#[test]
fn test_requested_generate_state_normalizes_cli_base_override() {
    let opts = GenerateOptions {
        dry_run: true,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: None,
        matrix: None,
        offline: false,
        frozen_plan: None,
        force: false,
        base_ref: Some(String::from("  refs/heads/main  ")),
        feature: None,
        remove_feature: None,
        preserve_feature: None,
        arch: None,
        primary_arch: None,
        secondary_arch: None,
        safety: None,
        strict: false,
        no_strict: false,
        run_selftests: true,
    };
    let profile = config::default_profile_config("v1.0");
    let requested =
        RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts).unwrap();

    assert_eq!(
        requested.cli_overrides.base_ref.as_deref(),
        Some("refs/heads/main")
    );
    assert!(requested
        .identity()
        .unwrap()
        .key
        .contains("base_override=refs/heads/main"));
}

#[test]
fn test_requested_generate_state_rejects_empty_cli_base_override() {
    let opts = GenerateOptions {
        dry_run: true,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: None,
        matrix: None,
        offline: false,
        frozen_plan: None,
        force: false,
        base_ref: Some(String::from(" \t ")),
        feature: None,
        remove_feature: None,
        preserve_feature: None,
        arch: None,
        primary_arch: None,
        secondary_arch: None,
        safety: None,
        strict: false,
        no_strict: false,
        run_selftests: true,
    };
    let profile = config::default_profile_config("v1.0");

    let err = RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
        .unwrap_err()
        .to_string();

    assert!(err.contains("cli --base must not be empty"));
}

#[test]
fn test_requested_generate_state_normalizes_cli_feature_override() {
    let opts = GenerateOptions {
        dry_run: true,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: None,
        matrix: None,
        offline: false,
        frozen_plan: None,
        force: false,
        base_ref: None,
        feature: Some(String::from(" bluetooth ")),
        remove_feature: None,
        preserve_feature: None,
        arch: None,
        primary_arch: None,
        secondary_arch: None,
        safety: None,
        strict: false,
        no_strict: false,
        run_selftests: true,
    };
    let profile = config::default_profile_config("v1.0");
    let requested =
        RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts).unwrap();

    assert_eq!(
        requested.cli_overrides.feature.as_deref(),
        Some("bluetooth")
    );
    assert!(requested
        .identity()
        .unwrap()
        .key
        .contains("feature_override=bluetooth"));
}

#[test]
fn test_requested_generate_state_rejects_empty_cli_feature_override() {
    let opts = GenerateOptions {
        dry_run: true,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: None,
        matrix: None,
        offline: false,
        frozen_plan: None,
        force: false,
        base_ref: None,
        feature: Some(String::from(" \t ")),
        remove_feature: None,
        preserve_feature: None,
        arch: None,
        primary_arch: None,
        secondary_arch: None,
        safety: None,
        strict: false,
        no_strict: false,
        run_selftests: true,
    };
    let profile = config::default_profile_config("v1.0");

    let err = RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
        .unwrap_err()
        .to_string();

    assert!(err.contains("cli --feature must not be empty"));
}

#[test]
fn test_requested_generate_state_normalizes_cli_remove_feature_override() {
    let opts = GenerateOptions {
        dry_run: true,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: None,
        matrix: None,
        offline: false,
        frozen_plan: None,
        force: false,
        base_ref: None,
        feature: None,
        remove_feature: Some(String::from(" bluetooth ")),
        preserve_feature: None,
        arch: None,
        primary_arch: None,
        secondary_arch: None,
        safety: None,
        strict: false,
        no_strict: false,
        run_selftests: true,
    };
    let profile = config::default_profile_config("v1.0");
    let requested =
        RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts).unwrap();

    assert_eq!(
        requested.cli_overrides.remove_feature.as_deref(),
        Some("bluetooth")
    );
    assert!(requested
        .identity()
        .unwrap()
        .key
        .contains("remove_feature_override=bluetooth"));
}

#[test]
fn test_requested_generate_state_rejects_empty_cli_remove_feature_override() {
    let opts = GenerateOptions {
        dry_run: true,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: None,
        matrix: None,
        offline: false,
        frozen_plan: None,
        force: false,
        base_ref: None,
        feature: None,
        remove_feature: Some(String::from(" \t ")),
        preserve_feature: None,
        arch: None,
        primary_arch: None,
        secondary_arch: None,
        safety: None,
        strict: false,
        no_strict: false,
        run_selftests: true,
    };
    let profile = config::default_profile_config("v1.0");

    let err = RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
        .unwrap_err()
        .to_string();

    assert!(err.contains("cli --remove-feature must not be empty"));
}

#[test]
fn test_requested_generate_state_normalizes_cli_preserve_feature_override() {
    let opts = GenerateOptions {
        dry_run: true,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: None,
        matrix: None,
        offline: false,
        frozen_plan: None,
        force: false,
        base_ref: None,
        feature: None,
        remove_feature: None,
        preserve_feature: Some(String::from(" netfilter ")),
        arch: None,
        primary_arch: None,
        secondary_arch: None,
        safety: None,
        strict: false,
        no_strict: false,
        run_selftests: true,
    };
    let profile = config::default_profile_config("v1.0");
    let requested =
        RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts).unwrap();

    assert_eq!(
        requested.cli_overrides.preserve_feature.as_deref(),
        Some("netfilter")
    );
    assert!(requested
        .identity()
        .unwrap()
        .key
        .contains("preserve_feature_override=netfilter"));
}

#[test]
fn test_requested_generate_state_rejects_empty_cli_preserve_feature_override() {
    let opts = GenerateOptions {
        dry_run: true,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: None,
        matrix: None,
        offline: false,
        frozen_plan: None,
        force: false,
        base_ref: None,
        feature: None,
        remove_feature: None,
        preserve_feature: Some(String::from(" \t ")),
        arch: None,
        primary_arch: None,
        secondary_arch: None,
        safety: None,
        strict: false,
        no_strict: false,
        run_selftests: true,
    };
    let profile = config::default_profile_config("v1.0");

    let err = RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
        .unwrap_err()
        .to_string();

    assert!(err.contains("cli --preserve-feature must not be empty"));
}

#[test]
fn test_requested_generate_state_normalizes_cli_safety_override() {
    let opts = GenerateOptions {
        dry_run: true,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: None,
        matrix: None,
        offline: false,
        frozen_plan: None,
        force: false,
        base_ref: None,
        feature: None,
        remove_feature: None,
        preserve_feature: None,
        arch: None,
        primary_arch: None,
        secondary_arch: None,
        safety: Some(String::from(" surgical ")),
        strict: false,
        no_strict: false,
        run_selftests: true,
    };
    let profile = config::default_profile_config("v1.0");
    let requested =
        RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts).unwrap();

    assert_eq!(requested.cli_overrides.safety.as_deref(), Some("surgical"));
    assert!(requested
        .identity()
        .unwrap()
        .key
        .contains("safety_override=surgical"));
}

#[test]
fn test_requested_generate_state_rejects_invalid_cli_safety_override() {
    let mut opts = GenerateOptions {
        dry_run: true,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: None,
        matrix: None,
        offline: false,
        frozen_plan: None,
        force: false,
        base_ref: None,
        feature: None,
        remove_feature: None,
        preserve_feature: None,
        arch: None,
        primary_arch: None,
        secondary_arch: None,
        safety: Some(String::from(" \t ")),
        strict: false,
        no_strict: false,
        run_selftests: true,
    };
    let profile = config::default_profile_config("v1.0");

    let err = RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
        .unwrap_err()
        .to_string();

    assert!(err.contains("cli --safety must not be empty"));

    opts.safety = Some(String::from("reckless"));
    let err = RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
        .unwrap_err()
        .to_string();

    assert!(err.contains("cli --safety is invalid"));
}

#[test]
fn test_requested_generate_state_normalizes_cli_arch_override() {
    let mut opts = GenerateOptions {
        dry_run: true,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: None,
        matrix: None,
        offline: false,
        frozen_plan: None,
        force: false,
        base_ref: None,
        feature: None,
        remove_feature: None,
        preserve_feature: None,
        arch: Some(String::from(" x86 ")),
        primary_arch: None,
        secondary_arch: None,
        safety: None,
        strict: false,
        no_strict: false,
        run_selftests: true,
    };
    let profile = config::default_profile_config("v1.0");
    let requested =
        RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts).unwrap();

    assert_eq!(requested.cli_overrides.arch.as_deref(), Some("x86"));
    assert!(requested
        .identity()
        .unwrap()
        .key
        .contains("arch_override=x86"));

    opts.arch = None;
    opts.primary_arch = Some(String::from(" arm64 "));
    let requested =
        RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts).unwrap();

    assert_eq!(
        requested.cli_overrides.primary_arch.as_deref(),
        Some("arm64")
    );
    assert!(requested
        .identity()
        .unwrap()
        .key
        .contains("primary_arch_override=arm64"));

    opts.primary_arch = None;
    opts.secondary_arch = Some(String::from(" riscv "));
    let requested =
        RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts).unwrap();

    assert_eq!(
        requested.cli_overrides.secondary_arch.as_deref(),
        Some("riscv")
    );
    assert!(requested
        .identity()
        .unwrap()
        .key
        .contains("secondary_arch_override=riscv"));
}

#[test]
fn test_requested_generate_state_rejects_invalid_cli_arch_override() {
    let mut opts = GenerateOptions {
        dry_run: true,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: None,
        matrix: None,
        offline: false,
        frozen_plan: None,
        force: false,
        base_ref: None,
        feature: None,
        remove_feature: None,
        preserve_feature: None,
        arch: Some(String::from(" \t ")),
        primary_arch: None,
        secondary_arch: None,
        safety: None,
        strict: false,
        no_strict: false,
        run_selftests: true,
    };
    let profile = config::default_profile_config("v1.0");

    let err = RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
        .unwrap_err()
        .to_string();

    assert!(err.contains("cli --arch must not be empty"));

    opts.arch = Some(String::from("x86/../../host"));
    let err = RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
        .unwrap_err()
        .to_string();

    assert!(err.contains("cli --arch is invalid"));

    opts.arch = None;
    opts.primary_arch = Some(String::from(" \t "));
    let err = RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
        .unwrap_err()
        .to_string();

    assert!(err.contains("cli --primary-arch must not be empty"));

    opts.primary_arch = Some(String::from("x86/../../host"));
    let err = RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
        .unwrap_err()
        .to_string();

    assert!(err.contains("cli --primary-arch is invalid"));

    opts.primary_arch = None;
    opts.secondary_arch = Some(String::from(" \t "));
    let err = RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
        .unwrap_err()
        .to_string();

    assert!(err.contains("cli --secondary-arch must not be empty"));

    opts.secondary_arch = Some(String::from("x86/../../host"));
    let err = RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
        .unwrap_err()
        .to_string();

    assert!(err.contains("cli --secondary-arch is invalid"));

    opts.arch = Some(String::from("x86"));
    opts.primary_arch = Some(String::from("arm64"));
    opts.secondary_arch = None;
    let err = RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
        .unwrap_err()
        .to_string();

    assert!(err.contains("cli --arch and --primary-arch are mutually exclusive"));

    opts.primary_arch = None;
    opts.secondary_arch = Some(String::from("arm64"));
    let err = RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
        .unwrap_err()
        .to_string();

    assert!(err.contains("cli --arch and --secondary-arch are mutually exclusive"));
}

#[test]
fn test_requested_generate_state_rejects_conflicting_cli_feature_overrides() {
    let opts = GenerateOptions {
        dry_run: true,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: None,
        matrix: None,
        offline: false,
        frozen_plan: None,
        force: false,
        base_ref: None,
        feature: Some(String::from("bluetooth")),
        remove_feature: Some(String::from("wifi")),
        preserve_feature: None,
        arch: None,
        primary_arch: None,
        secondary_arch: None,
        safety: None,
        strict: false,
        no_strict: false,
        run_selftests: true,
    };
    let profile = config::default_profile_config("v1.0");

    let err = RequestedGenerateState::from_inputs("/tmp/project/kslim.toml", &profile, &opts)
        .unwrap_err()
        .to_string();

    assert!(err.contains(
        "cli --feature, --remove-feature, and --preserve-feature are mutually exclusive"
    ));
}

#[test]
fn test_requested_generate_state_rejects_invalid_request_identity_parts() {
    let err = RequestedConfigPath::new(PathBuf::new())
        .unwrap_err()
        .to_string();
    assert!(err.contains("requested config path is empty"));

    let err = ProfileName::new("   ").unwrap_err().to_string();
    assert!(err.contains("profile name must not be empty"));
}
