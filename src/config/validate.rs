use anyhow::{Context, Result};
use std::collections::BTreeSet;

use super::{
    AbiPolicyConfig, ArchPolicyConfig, BuildMatrixConfig, FeatureConfig, FeatureIntentConfig,
    KslimConfig, OutputConfig, PerformanceConfig, ProfileConfig, ReducerConfig, ReportConfig,
    RuntimeMatrixConfig, SlimConfig,
};
use crate::model::{
    AcpiId, ArchName, DeviceCompatible, DocumentationPath, ExportedSymbol, FirmwarePath, Initcall,
    KselftestTarget, KunitSuite, ModuleAlias, ModuleName, PciId, RuntimeRegistrationSurface,
    SamplePath, ToolPath, UsbId,
};
use crate::paths::OutputRepoPath;
use crate::removal_manifest::RemovalManifest;

const BUILD_MATRIX_PRESETS: &[&str] = &["default", "extended", "hardening"];
const REPORT_FORMATS: &[&str] = &["text", "markdown", "json"];

fn is_default_reducer_config(config: &ReducerConfig) -> bool {
    config == &ReducerConfig::default()
}

fn validate_named_feature_intent(
    section: &str,
    name: &str,
    intent: &FeatureIntentConfig,
) -> Result<()> {
    if name.trim().is_empty() {
        anyhow::bail!("{section} feature names must not be empty");
    }
    if intent
        .kind
        .as_deref()
        .is_some_and(|kind| kind.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.kind must not be empty when specified");
    }
    if intent.roots.iter().any(|root| root.trim().is_empty()) {
        anyhow::bail!("{section}.{name}.roots must not contain empty values");
    }
    if intent
        .remove_paths
        .iter()
        .any(|path| path.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.remove_paths must not contain empty values");
    }
    if intent.configs.iter().any(|config| config.trim().is_empty()) {
        anyhow::bail!("{section}.{name}.configs must not contain empty values");
    }
    if intent
        .remove_configs
        .iter()
        .any(|config| config.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.remove_configs must not contain empty values");
    }
    if intent
        .exported_symbols
        .iter()
        .any(|symbol| symbol.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.exported_symbols must not contain empty values");
    }
    if intent
        .remove_exported_symbols
        .iter()
        .any(|symbol| symbol.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.remove_exported_symbols must not contain empty values");
    }
    if intent
        .module_names
        .iter()
        .any(|module| module.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.module_names must not contain empty values");
    }
    if intent
        .remove_module_names
        .iter()
        .any(|module| module.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.remove_module_names must not contain empty values");
    }
    if intent
        .module_aliases
        .iter()
        .any(|alias| alias.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.module_aliases must not contain empty values");
    }
    if intent
        .remove_module_aliases
        .iter()
        .any(|alias| alias.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.remove_module_aliases must not contain empty values");
    }
    if intent
        .device_compatibles
        .iter()
        .any(|compatible| compatible.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.device_compatibles must not contain empty values");
    }
    if intent
        .remove_device_compatibles
        .iter()
        .any(|compatible| compatible.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.remove_device_compatibles must not contain empty values");
    }
    if intent.acpi_ids.iter().any(|id| id.trim().is_empty()) {
        anyhow::bail!("{section}.{name}.acpi_ids must not contain empty values");
    }
    if intent.remove_acpi_ids.iter().any(|id| id.trim().is_empty()) {
        anyhow::bail!("{section}.{name}.remove_acpi_ids must not contain empty values");
    }
    if intent.pci_ids.iter().any(|id| id.trim().is_empty()) {
        anyhow::bail!("{section}.{name}.pci_ids must not contain empty values");
    }
    if intent.remove_pci_ids.iter().any(|id| id.trim().is_empty()) {
        anyhow::bail!("{section}.{name}.remove_pci_ids must not contain empty values");
    }
    if intent.usb_ids.iter().any(|id| id.trim().is_empty()) {
        anyhow::bail!("{section}.{name}.usb_ids must not contain empty values");
    }
    if intent.remove_usb_ids.iter().any(|id| id.trim().is_empty()) {
        anyhow::bail!("{section}.{name}.remove_usb_ids must not contain empty values");
    }
    if intent
        .firmware_paths
        .iter()
        .any(|path| path.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.firmware_paths must not contain empty values");
    }
    if intent
        .remove_firmware_paths
        .iter()
        .any(|path| path.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.remove_firmware_paths must not contain empty values");
    }
    if intent
        .initcalls
        .iter()
        .any(|initcall| initcall.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.initcalls must not contain empty values");
    }
    if intent
        .remove_initcalls
        .iter()
        .any(|initcall| initcall.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.remove_initcalls must not contain empty values");
    }
    if intent
        .runtime_registrations
        .iter()
        .any(|surface| surface.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.runtime_registrations must not contain empty values");
    }
    if intent
        .remove_runtime_registrations
        .iter()
        .any(|surface| surface.trim().is_empty())
    {
        anyhow::bail!(
            "{section}.{name}.remove_runtime_registrations must not contain empty values"
        );
    }
    if intent.docs.iter().any(|path| path.trim().is_empty()) {
        anyhow::bail!("{section}.{name}.docs must not contain empty values");
    }
    if intent.remove_docs.iter().any(|path| path.trim().is_empty()) {
        anyhow::bail!("{section}.{name}.remove_docs must not contain empty values");
    }
    if intent.tools.iter().any(|path| path.trim().is_empty()) {
        anyhow::bail!("{section}.{name}.tools must not contain empty values");
    }
    if intent
        .remove_tools
        .iter()
        .any(|path| path.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.remove_tools must not contain empty values");
    }
    if intent.samples.iter().any(|path| path.trim().is_empty()) {
        anyhow::bail!("{section}.{name}.samples must not contain empty values");
    }
    if intent
        .remove_samples
        .iter()
        .any(|path| path.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.remove_samples must not contain empty values");
    }
    if intent
        .kunit_suites
        .iter()
        .any(|suite| suite.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.kunit_suites must not contain empty values");
    }
    if intent
        .remove_kunit_suites
        .iter()
        .any(|suite| suite.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.remove_kunit_suites must not contain empty values");
    }
    if intent
        .kselftest_targets
        .iter()
        .any(|target| target.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.kselftest_targets must not contain empty values");
    }
    if intent
        .remove_kselftest_targets
        .iter()
        .any(|target| target.trim().is_empty())
    {
        anyhow::bail!("{section}.{name}.remove_kselftest_targets must not contain empty values");
    }
    for symbol in &intent.exported_symbols {
        ExportedSymbol::new(symbol.as_str()).with_context(|| {
            format!("{section}.{name}.exported_symbols contains invalid exported symbol")
        })?;
    }
    for symbol in &intent.remove_exported_symbols {
        ExportedSymbol::new(symbol.as_str()).with_context(|| {
            format!("{section}.{name}.remove_exported_symbols contains invalid exported symbol")
        })?;
    }
    for module in &intent.module_names {
        ModuleName::new(module.as_str()).with_context(|| {
            format!("{section}.{name}.module_names contains invalid module name")
        })?;
    }
    for module in &intent.remove_module_names {
        ModuleName::new(module.as_str()).with_context(|| {
            format!("{section}.{name}.remove_module_names contains invalid module name")
        })?;
    }
    for alias in &intent.module_aliases {
        ModuleAlias::new(alias.as_str()).with_context(|| {
            format!("{section}.{name}.module_aliases contains invalid module alias")
        })?;
    }
    for alias in &intent.remove_module_aliases {
        ModuleAlias::new(alias.as_str()).with_context(|| {
            format!("{section}.{name}.remove_module_aliases contains invalid module alias")
        })?;
    }
    for compatible in &intent.device_compatibles {
        DeviceCompatible::new(compatible.as_str()).with_context(|| {
            format!("{section}.{name}.device_compatibles contains invalid device compatible")
        })?;
    }
    for compatible in &intent.remove_device_compatibles {
        DeviceCompatible::new(compatible.as_str()).with_context(|| {
            format!("{section}.{name}.remove_device_compatibles contains invalid device compatible")
        })?;
    }
    for id in &intent.acpi_ids {
        AcpiId::new(id.as_str())
            .with_context(|| format!("{section}.{name}.acpi_ids contains invalid ACPI ID"))?;
    }
    for id in &intent.remove_acpi_ids {
        AcpiId::new(id.as_str()).with_context(|| {
            format!("{section}.{name}.remove_acpi_ids contains invalid ACPI ID")
        })?;
    }
    for id in &intent.pci_ids {
        PciId::new(id.as_str())
            .with_context(|| format!("{section}.{name}.pci_ids contains invalid PCI ID"))?;
    }
    for id in &intent.remove_pci_ids {
        PciId::new(id.as_str())
            .with_context(|| format!("{section}.{name}.remove_pci_ids contains invalid PCI ID"))?;
    }
    for id in &intent.usb_ids {
        UsbId::new(id.as_str())
            .with_context(|| format!("{section}.{name}.usb_ids contains invalid USB ID"))?;
    }
    for id in &intent.remove_usb_ids {
        UsbId::new(id.as_str())
            .with_context(|| format!("{section}.{name}.remove_usb_ids contains invalid USB ID"))?;
    }
    for path in &intent.firmware_paths {
        FirmwarePath::new(path.as_str()).with_context(|| {
            format!("{section}.{name}.firmware_paths contains invalid firmware path")
        })?;
    }
    for path in &intent.remove_firmware_paths {
        FirmwarePath::new(path.as_str()).with_context(|| {
            format!("{section}.{name}.remove_firmware_paths contains invalid firmware path")
        })?;
    }
    for initcall in &intent.initcalls {
        Initcall::new(initcall.as_str())
            .with_context(|| format!("{section}.{name}.initcalls contains invalid initcall"))?;
    }
    for initcall in &intent.remove_initcalls {
        Initcall::new(initcall.as_str()).with_context(|| {
            format!("{section}.{name}.remove_initcalls contains invalid initcall")
        })?;
    }
    for surface in &intent.runtime_registrations {
        RuntimeRegistrationSurface::new(surface.as_str()).with_context(|| {
            format!("{section}.{name}.runtime_registrations contains invalid runtime registration")
        })?;
    }
    for surface in &intent.remove_runtime_registrations {
        RuntimeRegistrationSurface::new(surface.as_str()).with_context(|| {
            format!(
                "{section}.{name}.remove_runtime_registrations contains invalid runtime registration"
            )
        })?;
    }
    for path in &intent.docs {
        DocumentationPath::new(path.as_str()).with_context(|| {
            format!("{section}.{name}.docs contains invalid documentation path")
        })?;
    }
    for path in &intent.remove_docs {
        DocumentationPath::new(path.as_str()).with_context(|| {
            format!("{section}.{name}.remove_docs contains invalid documentation path")
        })?;
    }
    for path in &intent.tools {
        ToolPath::new(path.as_str())
            .with_context(|| format!("{section}.{name}.tools contains invalid tool path"))?;
    }
    for path in &intent.remove_tools {
        ToolPath::new(path.as_str())
            .with_context(|| format!("{section}.{name}.remove_tools contains invalid tool path"))?;
    }
    for path in &intent.samples {
        SamplePath::new(path.as_str())
            .with_context(|| format!("{section}.{name}.samples contains invalid sample path"))?;
    }
    for path in &intent.remove_samples {
        SamplePath::new(path.as_str()).with_context(|| {
            format!("{section}.{name}.remove_samples contains invalid sample path")
        })?;
    }
    for suite in &intent.kunit_suites {
        KunitSuite::new(suite.as_str()).with_context(|| {
            format!("{section}.{name}.kunit_suites contains invalid KUnit suite")
        })?;
    }
    for suite in &intent.remove_kunit_suites {
        KunitSuite::new(suite.as_str()).with_context(|| {
            format!("{section}.{name}.remove_kunit_suites contains invalid KUnit suite")
        })?;
    }
    for target in &intent.kselftest_targets {
        KselftestTarget::new(target.as_str()).with_context(|| {
            format!("{section}.{name}.kselftest_targets contains invalid kselftest target")
        })?;
    }
    for target in &intent.remove_kselftest_targets {
        KselftestTarget::new(target.as_str()).with_context(|| {
            format!("{section}.{name}.remove_kselftest_targets contains invalid kselftest target")
        })?;
    }
    if intent.arch_scope.iter().any(|arch| arch.trim().is_empty()) {
        anyhow::bail!("{section}.{name}.arch_scope must not contain empty values");
    }
    validate_arch_name_list(&format!("{section}.{name}.arch_scope"), &intent.arch_scope)?;
    Ok(())
}

fn validate_feature_config(config: &FeatureConfig) -> Result<()> {
    for (name, intent) in &config.remove {
        validate_named_feature_intent("features.remove", name, intent)?;
    }
    for (name, intent) in &config.preserve {
        validate_named_feature_intent("features.preserve", name, intent)?;
    }
    if let Some(name) = config
        .remove
        .keys()
        .find(|name| config.preserve.contains_key(*name))
    {
        anyhow::bail!(
            "feature '{name}' cannot be declared in both features.remove and features.preserve"
        );
    }
    for (section, intents) in [
        ("features.remove", &config.remove),
        ("features.preserve", &config.preserve),
    ] {
        for (name, intent) in intents {
            validate_supported_feature_intent(section, name, intent)?;
        }
    }
    Ok(())
}

fn validate_supported_feature_intent(
    section: &str,
    name: &str,
    intent: &FeatureIntentConfig,
) -> Result<()> {
    if section != "features.remove" && !intent.remove_paths.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_paths is removal-only; use features.remove for explicit path removal"
        );
    }
    if section != "features.remove" && !intent.remove_configs.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_configs is removal-only; use features.remove for explicit Kconfig symbol removal"
        );
    }
    if section != "features.remove" && !intent.remove_exported_symbols.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_exported_symbols is removal-only; use features.remove for explicit exported-symbol removal"
        );
    }
    if section != "features.remove" && !intent.remove_module_names.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_module_names is removal-only; use features.remove for explicit module-name removal"
        );
    }
    if section != "features.remove" && !intent.remove_module_aliases.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_module_aliases is removal-only; use features.remove for explicit module-alias removal"
        );
    }
    if section != "features.remove" && !intent.remove_device_compatibles.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_device_compatibles is removal-only; use features.remove for explicit devicetree-compatible removal"
        );
    }
    if section != "features.remove" && !intent.remove_acpi_ids.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_acpi_ids is removal-only; use features.remove for explicit ACPI ID removal"
        );
    }
    if section != "features.remove" && !intent.remove_pci_ids.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_pci_ids is removal-only; use features.remove for explicit PCI ID removal"
        );
    }
    if section != "features.remove" && !intent.remove_usb_ids.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_usb_ids is removal-only; use features.remove for explicit USB ID removal"
        );
    }
    if section != "features.remove" && !intent.remove_firmware_paths.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_firmware_paths is removal-only; use features.remove for explicit firmware path removal"
        );
    }
    if section != "features.remove" && !intent.remove_initcalls.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_initcalls is removal-only; use features.remove for explicit initcall removal"
        );
    }
    if section != "features.remove" && !intent.remove_runtime_registrations.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_runtime_registrations is removal-only; use features.remove for explicit runtime registration removal"
        );
    }
    if section != "features.remove" && !intent.remove_docs.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_docs is removal-only; use features.remove for explicit documentation removal"
        );
    }
    if section != "features.remove" && !intent.remove_tools.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_tools is removal-only; use features.remove for explicit tool removal"
        );
    }
    if section != "features.remove" && !intent.remove_samples.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_samples is removal-only; use features.remove for explicit sample removal"
        );
    }
    if section != "features.remove" && !intent.remove_kunit_suites.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_kunit_suites is removal-only; use features.remove for explicit KUnit suite removal"
        );
    }
    if section != "features.remove" && !intent.remove_kselftest_targets.is_empty() {
        anyhow::bail!(
            "{section}.{name}.remove_kselftest_targets is removal-only; use features.remove for explicit kselftest target removal"
        );
    }
    if section != "features.remove" && intent.allow_public_header_removal {
        anyhow::bail!(
            "{section}.{name}.allow_public_header_removal is removal-only; use features.remove for explicit ABI removal approval"
        );
    }
    if section != "features.remove" && intent.allow_uapi_header_removal {
        anyhow::bail!(
            "{section}.{name}.allow_uapi_header_removal is removal-only; use features.remove for explicit UAPI removal approval"
        );
    }
    if section != "features.remove" && intent.safety.is_some() {
        anyhow::bail!(
            "{section}.{name}.safety is removal-only; use features.remove for per-feature removal safety"
        );
    }
    if intent.roots.is_empty()
        && intent.configs.is_empty()
        && intent.remove_paths.is_empty()
        && intent.remove_configs.is_empty()
        && intent.exported_symbols.is_empty()
        && intent.remove_exported_symbols.is_empty()
        && intent.module_names.is_empty()
        && intent.remove_module_names.is_empty()
        && intent.module_aliases.is_empty()
        && intent.remove_module_aliases.is_empty()
        && intent.device_compatibles.is_empty()
        && intent.remove_device_compatibles.is_empty()
        && intent.acpi_ids.is_empty()
        && intent.remove_acpi_ids.is_empty()
        && intent.pci_ids.is_empty()
        && intent.remove_pci_ids.is_empty()
        && intent.usb_ids.is_empty()
        && intent.remove_usb_ids.is_empty()
        && intent.firmware_paths.is_empty()
        && intent.remove_firmware_paths.is_empty()
        && intent.initcalls.is_empty()
        && intent.remove_initcalls.is_empty()
        && intent.runtime_registrations.is_empty()
        && intent.remove_runtime_registrations.is_empty()
        && intent.docs.is_empty()
        && intent.remove_docs.is_empty()
        && intent.tools.is_empty()
        && intent.remove_tools.is_empty()
        && intent.samples.is_empty()
        && intent.remove_samples.is_empty()
        && intent.kunit_suites.is_empty()
        && intent.remove_kunit_suites.is_empty()
        && intent.kselftest_targets.is_empty()
        && intent.remove_kselftest_targets.is_empty()
    {
        let declaration_hint = if section == "features.remove" {
            "roots, configs, exported_symbols, module_names, module_aliases, device_compatibles, acpi_ids, pci_ids, usb_ids, firmware_paths, initcalls, runtime_registrations, docs, tools, samples, kunit_suites, kselftest_targets, remove_paths, remove_configs, remove_exported_symbols, remove_module_names, remove_module_aliases, remove_device_compatibles, remove_acpi_ids, remove_pci_ids, remove_usb_ids, remove_firmware_paths, remove_initcalls, remove_runtime_registrations, remove_docs, remove_tools, remove_samples, remove_kunit_suites, or remove_kselftest_targets"
        } else {
            "roots, configs, exported_symbols, module_names, module_aliases, device_compatibles, acpi_ids, pci_ids, usb_ids, firmware_paths, initcalls, runtime_registrations, docs, tools, samples, kunit_suites, or kselftest_targets"
        };
        anyhow::bail!("{section}.{name} must declare {declaration_hint}");
    }
    if intent.preserve_uapi {
        anyhow::bail!(
                "{section}.{name}.preserve_uapi is parsed but not yet supported; declare UAPI roots under features.preserve until per-feature UAPI preservation policy lands"
            );
    }
    if intent.preserve_module_aliases {
        anyhow::bail!("{section}.{name}.preserve_module_aliases is parsed but not yet supported");
    }
    Ok(())
}

fn removal_input_from_feature_intent(intent: &FeatureIntentConfig) -> SlimConfig {
    let mut slim = SlimConfig::default();
    slim.remove_paths.extend(intent.roots.iter().cloned());
    slim.remove_paths
        .extend(intent.remove_paths.iter().cloned());
    slim.remove_configs.extend(intent.configs.iter().cloned());
    slim.remove_configs
        .extend(intent.remove_configs.iter().cloned());
    slim
}

fn abi_policy_for_feature_intent(
    base: &AbiPolicyConfig,
    intent: &FeatureIntentConfig,
) -> AbiPolicyConfig {
    let mut policy = base.clone();
    policy.allow_public_header_removal |= intent.allow_public_header_removal;
    policy.allow_uapi_header_removal |= intent.allow_uapi_header_removal;
    policy
}

fn validate_scoped_abi_policy(profile: &ProfileConfig) -> Result<()> {
    if let Some(slim) = profile.removal_input().filter(|slim| !slim.is_noop()) {
        RemovalManifest::from_slim_config_with_abi_policy(slim, &profile.abi)?;
    }

    for (name, intent) in &profile.features.remove {
        let slim = removal_input_from_feature_intent(intent);
        if slim.is_noop() {
            continue;
        }
        let policy = abi_policy_for_feature_intent(&profile.abi, intent);
        RemovalManifest::from_slim_config_with_abi_policy(&slim, &policy)
            .with_context(|| format!("invalid features.remove.{name} ABI/UAPI removal policy"))?;
    }
    Ok(())
}

fn validate_arch_name_field(field: &str, arch: &str) -> Result<()> {
    ArchName::new(arch).map_err(|err| anyhow::anyhow!("{field} is invalid: {:#}", err))?;
    Ok(())
}

fn validate_arch_name_list(field: &str, arches: &[String]) -> Result<BTreeSet<String>> {
    let mut seen = BTreeSet::new();
    for arch in arches {
        validate_arch_name_field(field, arch)?;
        if !seen.insert(arch.clone()) {
            anyhow::bail!("{field} must not contain duplicate architecture '{arch}'");
        }
    }
    Ok(seen)
}

fn validate_nonempty_unique_string_list(
    field: &str,
    values: &[String],
) -> Result<BTreeSet<String>> {
    let mut seen = BTreeSet::new();
    for value in values {
        if value.trim().is_empty() {
            anyhow::bail!("{field} must not contain empty values");
        }
        if !seen.insert(value.clone()) {
            anyhow::bail!("{field} must not contain duplicate value '{value}'");
        }
    }
    Ok(seen)
}

fn validate_arch_policy_config(config: &ArchPolicyConfig) -> Result<()> {
    if let Some(primary_arch) = config.primary_arch.as_deref() {
        validate_arch_name_field("arch.primary_arch", primary_arch)?;
    }
    let secondary = validate_arch_name_list("arch.secondary_arches", &config.secondary_arches)?;
    let disabled = validate_arch_name_list("arch.disabled_arches", &config.disabled_arches)?;

    if let Some(primary_arch) = config.primary_arch.as_deref() {
        if secondary.contains(primary_arch) {
            anyhow::bail!(
                "arch.primary_arch '{primary_arch}' must not also appear in arch.secondary_arches"
            );
        }
        if disabled.contains(primary_arch) {
            anyhow::bail!(
                "arch.primary_arch '{primary_arch}' must not also appear in arch.disabled_arches"
            );
        }
    }
    if let Some(arch) = secondary.iter().find(|arch| disabled.contains(*arch)) {
        anyhow::bail!(
            "architecture '{arch}' cannot be declared in both arch.secondary_arches and arch.disabled_arches"
        );
    }
    if config.allow_arch_local_removal && config.primary_arch.is_none() {
        anyhow::bail!("arch.allow_arch_local_removal requires arch.primary_arch");
    }
    if config.allow_arch_local_removal {
        anyhow::bail!(
            "arch.allow_arch_local_removal is not yet supported; declare explicit slim.remove_paths for arch-local removals"
        );
    }
    if !config.preserve_arch_shared {
        anyhow::bail!(
            "arch.preserve_arch_shared=false is not yet supported; shared-arch preservation must remain enabled"
        );
    }
    Ok(())
}

fn validate_build_matrix_config(config: &BuildMatrixConfig) -> Result<()> {
    let presets = validate_nonempty_unique_string_list("build_matrix.presets", &config.presets)?;
    for preset in presets {
        if !BUILD_MATRIX_PRESETS.contains(&preset.as_str()) {
            anyhow::bail!(
                "build_matrix.presets contains unsupported preset '{preset}' (expected default, extended, or hardening)"
            );
        }
    }
    validate_arch_name_list("build_matrix.arches", &config.arches)?;
    validate_nonempty_unique_string_list("build_matrix.config_targets", &config.config_targets)?;
    validate_nonempty_unique_string_list("build_matrix.targets", &config.targets)?;
    if config
        .randconfig_seed
        .as_deref()
        .is_some_and(|seed| seed.trim().is_empty())
    {
        anyhow::bail!("build_matrix.randconfig_seed must not be empty when specified");
    }
    if config.jobs == Some(0) {
        anyhow::bail!("build_matrix.jobs must be greater than zero when specified");
    }
    if !config.is_default() {
        anyhow::bail!(
            "build matrix config is parsed but not yet supported; use [[selftests.kernel_builds]] until build matrix support lands"
        );
    }
    Ok(())
}

fn validate_runtime_matrix_config(config: &RuntimeMatrixConfig) -> Result<()> {
    validate_arch_name_list("runtime_matrix.boot_arches", &config.boot_arches)?;
    validate_nonempty_unique_string_list("runtime_matrix.qemu_machines", &config.qemu_machines)?;
    validate_nonempty_unique_string_list("runtime_matrix.kunit_suites", &config.kunit_suites)?;
    validate_nonempty_unique_string_list(
        "runtime_matrix.kselftest_targets",
        &config.kselftest_targets,
    )?;
    if config.boot_timeout_seconds == Some(0) {
        anyhow::bail!(
            "runtime_matrix.boot_timeout_seconds must be greater than zero when specified"
        );
    }
    if !config.is_default() {
        anyhow::bail!(
            "runtime matrix config is parsed but not yet supported; use [selftests].commands until runtime matrix support lands"
        );
    }
    Ok(())
}

fn validate_report_config(config: &ReportConfig) -> Result<()> {
    let formats = validate_nonempty_unique_string_list("reports.formats", &config.formats)?;
    for format in formats {
        if !REPORT_FORMATS.contains(&format.as_str()) {
            anyhow::bail!(
                "reports.formats contains unsupported format '{format}' (expected text, markdown, or json)"
            );
        }
    }
    if config.include_raw_logs {
        anyhow::bail!(
            "reports.include_raw_logs is not supported for committed reports; raw logs must remain attempt metadata or CI artifacts"
        );
    }
    if !config.is_default() {
        anyhow::bail!(
            "report config is parsed but not yet supported; committed report artifacts are fixed until report planning lands"
        );
    }
    Ok(())
}

fn validate_performance_config(config: &PerformanceConfig) -> Result<()> {
    if config.max_worker_threads == Some(0) {
        anyhow::bail!("performance.max_worker_threads must be greater than zero when specified");
    }
    if config.max_io_threads == Some(0) {
        anyhow::bail!("performance.max_io_threads must be greater than zero when specified");
    }
    if !config.fail_on_regression {
        anyhow::bail!(
            "performance.fail_on_regression cannot be disabled; performance policy must fail closed"
        );
    }
    if !config.is_default() {
        anyhow::bail!(
            "performance config is parsed but not yet supported; hot-path work shape is fixed until performance planning lands"
        );
    }
    Ok(())
}

fn validate_output_branch_name(field: &str, value: &str) -> Result<()> {
    if value != value.trim() {
        anyhow::bail!("{field} must not have leading or trailing whitespace");
    }
    if value.starts_with('/') || value.ends_with('/') || value.contains("//") {
        anyhow::bail!("{field} must not contain empty branch path components");
    }
    if value.split('/').any(|component| component == ".") {
        anyhow::bail!("{field} must not contain '.' branch path components");
    }
    if value.split('/').any(|component| component == "..") || value.contains("..") {
        anyhow::bail!("{field} must not contain '..'");
    }
    if value.ends_with(".lock")
        || value
            .split('/')
            .any(|component| component.ends_with(".lock"))
    {
        anyhow::bail!("{field} must not contain branch path components ending in '.lock'");
    }
    Ok(())
}

fn validate_output_config(config: &OutputConfig) -> Result<()> {
    if config.path.trim().is_empty() {
        anyhow::bail!("output.path must not be empty");
    }
    OutputRepoPath::new(config.path.as_str())
        .map_err(|err| anyhow::anyhow!("output.path is invalid: {:#}", err))?;
    if config.branch_prefix.trim().is_empty() {
        anyhow::bail!("output.branch_prefix must not be empty");
    }
    validate_output_branch_name("output.branch_prefix", &config.branch_prefix)?;
    if let Some(branch) = config.branch.as_deref() {
        if branch.trim().is_empty() {
            anyhow::bail!("output.branch must not be empty when specified");
        }
        if config.has_explicit_branch() {
            validate_output_branch_name("output.branch", branch)?;
        }
    }
    Ok(())
}

pub fn validate_config(config: &KslimConfig) -> Result<()> {
    if config.project.name.trim().is_empty() {
        anyhow::bail!("project.name must not be empty");
    }
    if config.upstream.name.trim().is_empty() {
        anyhow::bail!("upstream.name must not be empty");
    }
    if config.upstream.url.trim().is_empty() {
        anyhow::bail!("upstream.url must not be empty");
    }
    if config
        .upstream
        .mode
        .as_deref()
        .is_some_and(|mode| mode.trim().is_empty())
    {
        anyhow::bail!("upstream.mode must not be empty when specified");
    }
    if let Some(mode) = config.upstream.mode.as_deref() {
        if mode != "direct" {
            anyhow::bail!(
                "upstream.mode '{}' is not supported; kslim now operates only in direct read-only mode",
                mode
            );
        }
    }
    if config.upstream.cache.is_some() {
        anyhow::bail!(
            "upstream.cache is no longer supported; remove it and point upstream.url at a local read-only git tree"
        );
    }
    validate_output_config(&config.output)?;
    if config.git.user_email.trim().is_empty() {
        anyhow::bail!("git.user_email must not be empty");
    }
    if config.git.user_name.trim().is_empty() {
        anyhow::bail!("git.user_name must not be empty");
    }
    if config.git.remote_name.trim().is_empty() {
        anyhow::bail!("git.remote_name must not be empty");
    }
    Ok(())
}

pub fn validate_profile(profile: &ProfileConfig) -> Result<()> {
    if profile.profile.name.trim().is_empty() {
        anyhow::bail!("profile.name must not be empty");
    }
    if let Some(parent_profile) = profile.profile.inherits.as_deref() {
        if parent_profile.trim().is_empty() {
            anyhow::bail!("profile.inherits must not be empty when specified");
        }
        anyhow::bail!(
            "profile.inherits is parsed but not yet supported; keep inherited intent explicit in this profile until profile inheritance planning lands"
        );
    }
    if profile.base.r#ref.trim().is_empty() {
        anyhow::bail!("base.ref must not be empty");
    }
    validate_feature_config(&profile.features)?;
    validate_arch_policy_config(&profile.arch)?;
    validate_build_matrix_config(&profile.build_matrix)?;
    validate_runtime_matrix_config(&profile.runtime_matrix)?;
    validate_report_config(&profile.reports)?;
    crate::security::validate_security_config(&profile.security)?;
    validate_performance_config(&profile.performance)?;
    validate_scoped_abi_policy(profile)?;
    if let Some(patches) = &profile.patches {
        let sources = patches.sources();
        if sources.is_empty() {
            anyhow::bail!("patches must contain at least one source");
        }
        for (idx, source) in sources.iter().enumerate() {
            if source.source != "worktree" {
                anyhow::bail!(
                    "patches.sources[{}].source '{}' is not supported (expected 'worktree')",
                    idx,
                    source.source
                );
            }
            if source.path.trim().is_empty() {
                anyhow::bail!("patches.sources[{}].path must not be empty", idx);
            }
            if source.base_remote.trim().is_empty() {
                anyhow::bail!("patches.sources[{}].base_remote must not be empty", idx);
            }
            if source.base_ref.trim().is_empty() {
                anyhow::bail!("patches.sources[{}].base_ref must not be empty", idx);
            }
        }
    }
    if let Some(rtlmq) = &profile.integrations.rtlmq {
        if rtlmq.source.trim().is_empty() {
            anyhow::bail!("integrations.rtlmq.source must not be empty");
        }
        if rtlmq
            .tests_source
            .as_deref()
            .is_some_and(|path| path.trim().is_empty())
        {
            anyhow::bail!("integrations.rtlmq.tests_source must not be empty when specified");
        }
    }
    let removal_input = profile.effective_removal_input();
    let preservation_input = profile.effective_preservation_input();
    if removal_input.is_some() || preservation_input.is_some() {
        let removal_input_for_validation = removal_input.clone().unwrap_or_default();
        let abi_policy = profile.effective_abi_policy();
        RemovalManifest::from_slim_config_with_abi_policy_and_preservation(
            &removal_input_for_validation,
            preservation_input.as_ref(),
            &abi_policy,
        )?;
    }
    let reducer_has_effective_input = removal_input.is_some();
    if !reducer_has_effective_input && !is_default_reducer_config(&profile.reducer) {
        anyhow::bail!(
            "reducer settings may only be customized when [slim] or [features.remove] declares removal input"
        );
    }
    if profile
        .selftests
        .commands
        .iter()
        .any(|command| command.trim().is_empty())
    {
        anyhow::bail!("selftests.commands must not contain empty commands");
    }
    for (idx, build) in profile.selftests.kernel_builds.iter().enumerate() {
        let label = build
            .name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("#{}", idx + 1));

        if build
            .config_target
            .as_deref()
            .is_some_and(|target| target.trim().is_empty())
        {
            anyhow::bail!(
                "selftests.kernel_builds[{}] ({}): config_target must not be empty",
                idx,
                label
            );
        }
        if build.targets.iter().any(|target| target.trim().is_empty()) {
            anyhow::bail!(
                "selftests.kernel_builds[{}] ({}): targets must not contain empty values",
                idx,
                label
            );
        }
        if build.make_args.iter().any(|arg| arg.trim().is_empty()) {
            anyhow::bail!(
                "selftests.kernel_builds[{}] ({}): make_args must not contain empty values",
                idx,
                label
            );
        }
        if build
            .make_program
            .as_deref()
            .is_some_and(|program| program.trim().is_empty())
        {
            anyhow::bail!(
                "selftests.kernel_builds[{}] ({}): make_program must not be empty",
                idx,
                label
            );
        }
        if build
            .output_dir
            .as_deref()
            .is_some_and(|dir| dir.trim().is_empty())
        {
            anyhow::bail!(
                "selftests.kernel_builds[{}] ({}): output_dir must not be empty",
                idx,
                label
            );
        }
        if build
            .env
            .iter()
            .any(|(key, value)| key.trim().is_empty() || value.trim().is_empty())
        {
            anyhow::bail!(
                "selftests.kernel_builds[{}] ({}): env must not contain empty keys or values",
                idx,
                label
            );
        }
        if let Some(arch) = build.env.get("ARCH") {
            ArchName::new(arch.as_str()).map_err(|err| {
                anyhow::anyhow!(
                    "selftests.kernel_builds[{}] ({}): ARCH env is invalid: {:#}",
                    idx,
                    label,
                    err
                )
            })?;
        }
        if build.config_target.is_none() && build.targets.is_empty() {
            anyhow::bail!(
                "selftests.kernel_builds[{}] ({}): specify config_target, targets, or both",
                idx,
                label
            );
        }
    }
    Ok(())
}
