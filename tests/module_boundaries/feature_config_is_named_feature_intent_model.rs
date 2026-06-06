use super::common::*;

fn section_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let (_, rest) = source
        .split_once(start)
        .unwrap_or_else(|| panic!("missing section start marker {start:?}"));
    let (section, _) = rest
        .split_once(end)
        .unwrap_or_else(|| panic!("missing section end marker {end:?}"));
    section
}

#[test]
fn feature_config_is_named_feature_intent_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_model = production_source(&root.join("src/config/model.rs"));
    let config_validate = production_source(&root.join("src/config/validate.rs"));
    let config_templates = production_source(&root.join("src/config/templates.rs"));
    let generate_state = state_source(root);
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);
    let architecture_flat = architecture.replace("\n  ", " ");

    assert!(
        config_model.contains("pub struct FeatureConfig {"),
        "config/model.rs should define the [features] named-intent model"
    );
    let profile_config = section_between(
        &config_model,
        "pub struct ProfileConfig",
        "impl ProfileConfig",
    );
    assert!(
        profile_config.contains("pub features: FeatureConfig,"),
        "ProfileConfig should carry FeatureConfig"
    );

    let feature_config = section_between(
        &config_model,
        "pub struct FeatureConfig",
        "impl FeatureConfig",
    );
    for required in [
        "pub remove: BTreeMap<String, FeatureIntentConfig>,",
        "pub preserve: BTreeMap<String, FeatureIntentConfig>,",
    ] {
        assert!(
            feature_config.contains(required),
            "FeatureConfig should own named feature intent map {required}"
        );
    }
    assert!(
        config_model.contains("pub fn is_empty(&self) -> bool")
            && config_model.contains("self.remove.is_empty() && self.preserve.is_empty()"),
        "FeatureConfig should expose empty intent detection"
    );
    assert!(
        config_model.contains("pub fn effective_removal_input(&self) -> Option<SlimConfig>")
            && config_model.contains("pub fn effective_preservation_input(&self)")
            && config_model.contains("self.features.remove.values()")
            && config_model.contains("self.features.preserve.values()")
            && config_model.contains("slim.remove_paths.extend(intent.roots.iter().cloned())")
            && config_model.contains("slim.remove_paths\n                .extend(intent.remove_paths.iter().cloned())")
            && config_model.contains("input.preserve_paths.extend(intent.roots.iter().cloned())")
            && config_model.contains("slim.remove_configs.extend(intent.configs.iter().cloned())")
            && config_model.contains("slim.remove_configs\n                .extend(intent.remove_configs.iter().cloned())")
            && config_model.contains("input\n                .preserve_configs")
            && config_model.contains(".extend(intent.configs.iter().cloned())")
            && config_model.contains("pub fn effective_abi_policy(&self) -> AbiPolicyConfig")
            && config_model
                .contains("policy.allow_public_header_removal |= intent.allow_public_header_removal")
            && config_model
                .contains("policy.allow_uapi_header_removal |= intent.allow_uapi_header_removal")
            && config_model.contains("pub fn effective_feature_safety_levels(&self)")
            && config_model.contains("intent.safety.unwrap_or_default()")
            && config_model.contains("pub fn effective_feature_arch_scopes(&self)")
            && config_model.contains("scopes.insert(name.clone(), intent.arch_scope.clone())")
            && config_model.contains("pub fn effective_feature_test_matrices(&self)")
            && config_model.contains("matrices.insert(name.clone(), matrix)")
            && config_model.contains("pub fn effective_feature_report_modes(&self)")
            && config_model.contains("modes.insert(name.clone(), mode)"),
        "ProfileConfig should resolve supported named feature remove/preserve intent into effective inputs"
    );

    let feature_intent = section_between(
        &config_model,
        "pub struct FeatureIntentConfig",
        "pub struct ArchPolicyConfig",
    );
    for required in [
        "pub kind: Option<String>,",
        "pub roots: Vec<String>,",
        "pub remove_paths: Vec<String>,",
        "pub configs: Vec<String>,",
        "pub remove_configs: Vec<String>,",
        "pub exported_symbols: Vec<String>,",
        "pub remove_exported_symbols: Vec<String>,",
        "pub module_names: Vec<String>,",
        "pub remove_module_names: Vec<String>,",
        "pub module_aliases: Vec<String>,",
        "pub remove_module_aliases: Vec<String>,",
        "pub device_compatibles: Vec<String>,",
        "pub remove_device_compatibles: Vec<String>,",
        "pub acpi_ids: Vec<String>,",
        "pub remove_acpi_ids: Vec<String>,",
        "pub pci_ids: Vec<String>,",
        "pub remove_pci_ids: Vec<String>,",
        "pub usb_ids: Vec<String>,",
        "pub remove_usb_ids: Vec<String>,",
        "pub firmware_paths: Vec<String>,",
        "pub remove_firmware_paths: Vec<String>,",
        "pub initcalls: Vec<String>,",
        "pub remove_initcalls: Vec<String>,",
        "pub runtime_registrations: Vec<String>,",
        "pub remove_runtime_registrations: Vec<String>,",
        "pub docs: Vec<String>,",
        "pub remove_docs: Vec<String>,",
        "pub tools: Vec<String>,",
        "pub remove_tools: Vec<String>,",
        "pub samples: Vec<String>,",
        "pub remove_samples: Vec<String>,",
        "pub kunit_suites: Vec<String>,",
        "pub remove_kunit_suites: Vec<String>,",
        "pub kselftest_targets: Vec<String>,",
        "pub remove_kselftest_targets: Vec<String>,",
        "pub allow_public_header_removal: bool,",
        "pub allow_uapi_header_removal: bool,",
        "pub arch_scope: Vec<String>,",
        "pub safety: Option<FeatureSafetyLevel>,",
        "pub preserve_uapi: bool,",
        "pub preserve_module_aliases: bool,",
        "pub require_clean_boot: bool,",
        "pub report_only: bool,",
    ] {
        assert!(
            feature_intent.contains(required),
            "FeatureIntentConfig should own named feature intent field {required}"
        );
    }
    assert!(
        config_model.contains("pub enum FeatureSafetyLevel")
            && config_model.contains("Conservative")
            && config_model.contains("Normal")
            && config_model.contains("Aggressive")
            && config_model.contains("Surgical")
            && config_model.contains("Unsafe")
            && config_model.contains("pub fn as_str(self) -> &'static str"),
        "FeatureSafetyLevel should normalize supported per-feature safety values"
    );
    assert!(
        config_model.contains("pub struct FeatureTestMatrixConfig")
            && config_model.contains("pub require_clean_boot: bool,")
            && config_model.contains("pub fn is_default(&self) -> bool"),
        "FeatureTestMatrixConfig should normalize supported per-feature test matrix policy"
    );
    assert!(
        config_model.contains("pub struct FeatureReportModeConfig")
            && config_model.contains("pub report_only: bool,")
            && config_model.contains("pub fn is_default(&self) -> bool"),
        "FeatureReportModeConfig should normalize supported per-feature report mode policy"
    );

    for forbidden in [
        "KslimConfig",
        "OutputConfig",
        "ReducerConfig",
        "SelfTestConfig",
        "PatchConfig",
        "IntegrationsConfig",
        "FeatureResolutionState",
        "PrunePlan",
        "RemovalManifest",
        "CandidateTreeState",
        "PublishedSnapshotState",
        "LockfilePath",
    ] {
        assert!(
            !feature_config.contains(forbidden) && !feature_intent.contains(forbidden),
            "FeatureConfig must stay raw named intent, not resolved policy/state {forbidden}"
        );
    }

    assert!(
        config_templates.contains("features: FeatureConfig::default()"),
        "default_profile_config should construct empty FeatureConfig"
    );
    assert!(
        config_validate
            .contains("fn validate_feature_config(config: &FeatureConfig) -> Result<()>")
            && config_validate.contains("validate_named_feature_intent")
            && config_validate.contains("feature names must not be empty")
            && config_validate
                .contains("cannot be declared in both features.remove and features.preserve")
            && config_validate.contains("roots, configs, exported_symbols, module_names")
            && config_validate.contains("module_aliases")
            && config_validate.contains("device_compatibles")
            && config_validate.contains("acpi_ids")
            && config_validate.contains("pci_ids")
            && config_validate.contains("usb_ids")
            && config_validate.contains("firmware_paths")
            && config_validate.contains("initcalls")
            && config_validate.contains("runtime_registrations")
            && config_validate.contains("docs")
            && config_validate.contains("tools")
            && config_validate.contains("samples")
            && config_validate.contains("kunit_suites")
            && config_validate.contains("kselftest_targets")
            && config_validate.contains("remove_paths is removal-only")
            && config_validate.contains("remove_configs is removal-only")
            && config_validate.contains("remove_exported_symbols is removal-only")
            && config_validate.contains("remove_module_names is removal-only")
            && config_validate.contains("remove_module_aliases is removal-only")
            && config_validate.contains("remove_device_compatibles is removal-only")
            && config_validate.contains("remove_acpi_ids is removal-only")
            && config_validate.contains("remove_pci_ids is removal-only")
            && config_validate.contains("remove_usb_ids is removal-only")
            && config_validate.contains("remove_firmware_paths is removal-only")
            && config_validate.contains("remove_initcalls is removal-only")
            && config_validate.contains("remove_runtime_registrations is removal-only")
            && config_validate.contains("remove_docs is removal-only")
            && config_validate.contains("remove_tools is removal-only")
            && config_validate.contains("remove_samples is removal-only")
            && config_validate.contains("remove_kunit_suites is removal-only")
            && config_validate.contains("remove_kselftest_targets is removal-only")
            && config_validate.contains("allow_public_header_removal is removal-only")
            && config_validate.contains("allow_uapi_header_removal is removal-only")
            && config_validate.contains("safety is removal-only")
            && config_validate.contains("&format!(\"{section}.{name}.arch_scope\")")
            && config_validate.contains("validate_arch_name_list")
            && config_validate.contains("validate_scoped_abi_policy(profile)?")
            && config_validate.contains("features.preserve")
            && config_validate.contains("from_slim_config_with_abi_policy_and_preservation")
            && !config_validate.contains("per-feature safety support lands later")
            && !config_validate.contains("per-feature arch scope support lands later")
            && !config_validate.contains("per-feature test matrix support lands")
            && !config_validate.contains("per-feature report-only support lands")
            && !config_validate.contains("report_only is parsed but not yet supported")
            && config_validate.contains("validate_feature_config(&profile.features)?"),
        "profile validation should support named removals/preservations and fail closed for unsupported feature policy"
    );
    assert!(
        generate_state.contains("FeatureResolutionState::from_profile(profile)")
            && generate_state.contains("FeatureResolutionSource::NamedFeatureRemove")
            && generate_state.contains(".features\n            .remove\n            .values()")
            && generate_state.contains("intent.declares_removal_input()")
            && generate_state.contains("profile.effective_removal_input()")
            && generate_state.contains("profile.effective_preservation_input()")
            && generate_state.contains("profile.effective_abi_policy()")
            && generate_state.contains("profile.effective_feature_safety_levels()")
            && generate_state.contains("feature_safety_levels")
            && generate_state.contains(".effective_feature_arch_scopes()")
            && generate_state.contains("feature_arch_scopes")
            && generate_state.contains("profile.effective_feature_test_matrices()")
            && generate_state.contains("feature_test_matrices")
            && generate_state.contains("profile.effective_feature_report_modes()")
            && generate_state.contains("feature_report_modes")
            && generate_state.contains("preserved_paths()"),
        "generate state should resolve named feature removals and preservations before planning"
    );
    assert!(
        architecture_flat
            .contains("`FeatureConfig` is the `[features]` profile named-intent model")
            && architecture_flat.contains("remove/preserve maps keyed by feature name")
            && architecture_flat.contains("exported symbols")
            && architecture_flat.contains("module names")
            && architecture_flat.contains("module aliases")
            && architecture_flat.contains("devicetree compatibles")
            && architecture_flat.contains("ACPI IDs")
            && architecture_flat.contains("PCI IDs")
            && architecture_flat.contains("USB IDs")
            && architecture_flat.contains("firmware references")
            && architecture_flat.contains("initcalls")
            && architecture_flat.contains("runtime registration surfaces")
            && architecture_flat.contains("documentation paths")
            && architecture_flat.contains("tool paths")
            && architecture_flat.contains("sample paths")
            && architecture_flat.contains("KUnit suites")
            && architecture_flat.contains("kselftest targets")
            && architecture_flat.contains("Named feature removals resolve")
            && architecture_flat.contains("Exported-symbol intent is resolved")
            && architecture_flat.contains("module-name intent")
            && architecture_flat.contains("module-alias intent")
            && architecture_flat.contains("devicetree-compatible intent")
            && architecture_flat.contains("ACPI ID intent")
            && architecture_flat.contains("PCI ID intent")
            && architecture_flat.contains("USB ID intent")
            && architecture_flat.contains("firmware-path intent")
            && architecture_flat.contains("initcall intent")
            && architecture_flat.contains("runtime-registration intent")
            && architecture_flat.contains("documentation intent")
            && architecture_flat.contains("tool intent")
            && architecture_flat.contains("sample intent")
            && architecture_flat.contains("KUnit suite intent")
            && architecture_flat.contains("kselftest target intent")
            && architecture_flat.contains("scoped ABI/UAPI approval flags")
            && architecture_flat.contains("safety levels are normalized")
            && architecture_flat.contains("architecture scopes are validated")
            && architecture_flat.contains("clean-boot test requirements are validated")
            && architecture_flat.contains("report-only modes are validated")
            && architecture_flat.contains("Named feature preservations resolve")
            && architecture_flat.contains("Per-feature UAPI/module preservation"),
        "architecture docs should describe FeatureConfig ownership and supported named removals/preservations"
    );
    assert!(
        kernel_build_guide.contains("[features.remove.bluetooth]")
            && kernel_build_guide.contains("[features.preserve.netfilter]")
            && kernel_build_guide.contains("module names")
            && kernel_build_guide.contains("module aliases")
            && kernel_build_guide.contains("devicetree compatibles")
            && kernel_build_guide.contains("ACPI IDs")
            && kernel_build_guide.contains("PCI IDs")
            && kernel_build_guide.contains("USB IDs")
            && kernel_build_guide.contains("firmware paths")
            && kernel_build_guide.contains("initcalls")
            && kernel_build_guide.contains("runtime registrations")
            && kernel_build_guide.contains("docs")
            && kernel_build_guide.contains("tools")
            && kernel_build_guide.contains("samples")
            && kernel_build_guide.contains("KUnit suites")
            && kernel_build_guide.contains("kselftest targets")
            && kernel_build_guide.contains("`remove_paths`, `remove_configs`")
            && kernel_build_guide.contains("`remove_exported_symbols`")
            && kernel_build_guide.contains("`remove_module_names`")
            && kernel_build_guide.contains("`remove_module_aliases`")
            && kernel_build_guide.contains("`remove_device_compatibles`")
            && kernel_build_guide.contains("`remove_acpi_ids`")
            && kernel_build_guide.contains("`remove_pci_ids`")
            && kernel_build_guide.contains("`remove_usb_ids`")
            && kernel_build_guide.contains("`remove_firmware_paths`")
            && kernel_build_guide.contains("`remove_initcalls`")
            && kernel_build_guide.contains("`remove_runtime_registrations`")
            && kernel_build_guide.contains("`remove_docs`")
            && kernel_build_guide.contains("`remove_tools`")
            && kernel_build_guide.contains("`remove_samples`")
            && kernel_build_guide.contains("`remove_kunit_suites`")
            && kernel_build_guide.contains("`remove_kselftest_targets`")
            && kernel_build_guide.contains("exported-symbol facts")
            && kernel_build_guide.contains("module-name facts")
            && kernel_build_guide.contains("module-alias facts")
            && kernel_build_guide.contains("devicetree-compatible facts")
            && kernel_build_guide.contains("ACPI ID facts")
            && kernel_build_guide.contains("PCI ID facts")
            && kernel_build_guide.contains("USB ID facts")
            && kernel_build_guide.contains("firmware-path facts")
            && kernel_build_guide.contains("initcall facts")
            && kernel_build_guide.contains("runtime-registration facts")
            && kernel_build_guide.contains("documentation facts")
            && kernel_build_guide.contains("tool facts")
            && kernel_build_guide.contains("sample facts")
            && kernel_build_guide.contains("KUnit suite facts")
            && kernel_build_guide.contains("kselftest target facts")
            && kernel_build_guide.contains("allow_uapi_header_removal = true")
            && kernel_build_guide.contains("safety = \"surgical\"")
            && kernel_build_guide.contains("arch_scope = [\"x86\"]")
            && kernel_build_guide.contains("require_clean_boot = true")
            && kernel_build_guide.contains("report_only = true")
            && kernel_build_guide.contains("Per-feature safety levels are normalized")
            && kernel_build_guide.contains("Per-feature arch scopes are validated")
            && kernel_build_guide.contains("Per-feature clean-boot")
            && kernel_build_guide.contains("Per-feature report-only modes")
            && kernel_build_guide.contains("UAPI/module preservation policy"),
        "kernel build iteration docs should explain supported and unsupported FeatureConfig policy"
    );
}
