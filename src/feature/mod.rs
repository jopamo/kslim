//! Feature semantics, graph resolution, and conflict models.
//!
//! This module owns typed feature intent after profile parsing, the semantic
//! feature graph, ownership resolution slices, and conflict reports. It does
//! not own candidate state, reducer edits, published metadata, or lockfile
//! truth.

use anyhow::Result;

use crate::config::{
    normalize_feature_name, FeatureIntentConfig, FeatureSafetyLevel, ProfileConfig,
};
use crate::model::{
    AcpiId, ArchName, DeviceCompatible, DocumentationPath, ExportedSymbol, FirmwarePath,
    GeneratedArtifactPath, HeaderPath, Initcall, KbuildObject, KconfigSymbol, KselftestTarget,
    KunitSuite, ModuleAlias, ModuleName, PciId, RuntimeRegistrationSurface, SamplePath,
    SourceFilePath, ToolPath, UapiPath, UsbId,
};
use crate::paths::RelativeKernelPath;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod acpi_id_resolution;
mod conflict_detection;
mod device_compatible_resolution;
mod documentation_resolution;
mod exported_symbol_resolution;
mod firmware_path_resolution;
mod generated_artifact_resolution;
mod initcall_resolution;
mod kbuild_resolution;
mod kconfig_resolution;
mod kselftest_target_resolution;
mod kunit_suite_resolution;
mod module_alias_resolution;
mod module_name_resolution;
mod path_resolution;
mod pci_id_resolution;
mod private_header_resolution;
mod public_header_resolution;
mod runtime_registration_resolution;
mod sample_resolution;
mod source_resolution;
mod tool_resolution;
mod uapi_header_resolution;
mod usb_id_resolution;
#[allow(unused_imports)]
pub(crate) use acpi_id_resolution::{
    FeatureAcpiIdResolution, FeatureResolvedAcpiId, FeatureResolvedAcpiIdKind,
};
#[allow(unused_imports)]
pub(crate) use conflict_detection::FeatureDeviceId;
#[allow(unused_imports)]
pub(crate) use conflict_detection::FeatureDeviceIdTableReference;
#[allow(unused_imports)]
pub(crate) use conflict_detection::FeatureExportedSymbolConsumer;
#[allow(unused_imports)]
pub(crate) use conflict_detection::FeatureKbuildReference;
#[allow(unused_imports)]
pub(crate) use conflict_detection::FeatureKconfigSelection;
#[allow(unused_imports)]
pub(crate) use conflict_detection::FeatureRuntimeRegistrationReachability;
#[allow(unused_imports)]
pub(crate) use conflict_detection::FeatureUserspaceUapiReference;
#[allow(unused_imports)]
pub(crate) use device_compatible_resolution::{
    FeatureDeviceCompatibleResolution, FeatureResolvedDeviceCompatible,
    FeatureResolvedDeviceCompatibleKind,
};
#[allow(unused_imports)]
pub(crate) use documentation_resolution::{
    FeatureDocumentationResolution, FeatureResolvedDocumentation, FeatureResolvedDocumentationKind,
};
#[allow(unused_imports)]
pub(crate) use exported_symbol_resolution::{
    FeatureExportedSymbolResolution, FeatureResolvedExportedSymbol,
    FeatureResolvedExportedSymbolKind,
};
#[allow(unused_imports)]
pub(crate) use firmware_path_resolution::{
    FeatureFirmwarePathResolution, FeatureResolvedFirmwarePath, FeatureResolvedFirmwarePathKind,
};
#[allow(unused_imports)]
pub(crate) use generated_artifact_resolution::{
    FeatureGeneratedArtifactResolution, FeatureResolvedGeneratedArtifact,
    FeatureResolvedGeneratedArtifactKind,
};
#[allow(unused_imports)]
pub(crate) use initcall_resolution::{
    FeatureInitcallResolution, FeatureResolvedInitcall, FeatureResolvedInitcallKind,
};
#[allow(unused_imports)]
pub(crate) use kbuild_resolution::{
    FeatureKbuildResolution, FeatureResolvedKbuildObject, FeatureResolvedKbuildObjectKind,
};
#[allow(unused_imports)]
pub(crate) use kconfig_resolution::{
    FeatureKconfigResolution, FeatureResolvedKconfig, FeatureResolvedKconfigKind,
};
#[allow(unused_imports)]
pub(crate) use kselftest_target_resolution::{
    FeatureKselftestTargetResolution, FeatureResolvedKselftestTarget,
    FeatureResolvedKselftestTargetKind,
};
#[allow(unused_imports)]
pub(crate) use kunit_suite_resolution::{
    FeatureKunitSuiteResolution, FeatureResolvedKunitSuite, FeatureResolvedKunitSuiteKind,
};
#[allow(unused_imports)]
pub(crate) use module_alias_resolution::{
    FeatureModuleAliasResolution, FeatureResolvedModuleAlias, FeatureResolvedModuleAliasKind,
};
#[allow(unused_imports)]
pub(crate) use module_name_resolution::{
    FeatureModuleNameResolution, FeatureResolvedModuleName, FeatureResolvedModuleNameKind,
};
#[allow(unused_imports)]
pub(crate) use path_resolution::{
    FeaturePathResolution, FeatureResolvedPath, FeatureResolvedPathKind,
};
#[allow(unused_imports)]
pub(crate) use pci_id_resolution::{
    FeaturePciIdResolution, FeatureResolvedPciId, FeatureResolvedPciIdKind,
};
#[allow(unused_imports)]
pub(crate) use private_header_resolution::{
    FeaturePrivateHeaderResolution, FeatureResolvedPrivateHeader, FeatureResolvedPrivateHeaderKind,
};
#[allow(unused_imports)]
pub(crate) use public_header_resolution::{
    FeaturePublicHeaderResolution, FeatureResolvedPublicHeader, FeatureResolvedPublicHeaderKind,
};
#[allow(unused_imports)]
pub(crate) use runtime_registration_resolution::{
    FeatureResolvedRuntimeRegistration, FeatureResolvedRuntimeRegistrationKind,
    FeatureRuntimeRegistrationResolution,
};
#[allow(unused_imports)]
pub(crate) use sample_resolution::{
    FeatureResolvedSample, FeatureResolvedSampleKind, FeatureSampleResolution,
};
#[allow(unused_imports)]
pub(crate) use source_resolution::{
    FeatureResolvedSourceFile, FeatureResolvedSourceFileKind, FeatureSourceResolution,
};
#[allow(unused_imports)]
pub(crate) use tool_resolution::{
    FeatureResolvedTool, FeatureResolvedToolKind, FeatureToolResolution,
};
#[allow(unused_imports)]
pub(crate) use uapi_header_resolution::{
    FeatureResolvedUapiHeader, FeatureResolvedUapiHeaderKind, FeatureUapiHeaderResolution,
};
#[allow(unused_imports)]
pub(crate) use usb_id_resolution::{
    FeatureResolvedUsbId, FeatureResolvedUsbIdKind, FeatureUsbIdResolution,
};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FeatureId(String);

#[allow(dead_code)]
impl FeatureId {
    pub(crate) fn new(id: impl Into<String>) -> Result<Self> {
        Ok(Self(normalize_feature_name(&id.into())?))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureKind {
    Subsystem,
    Driver,
    Bus,
    Filesystem,
    NetworkProtocol,
    CryptoAlgorithm,
    SchedulerFeature,
    SecurityFeature,
    TracingFeature,
    BpfFeature,
    ArchFeature,
    SocPlatform,
    BoardPlatformSupport,
    FirmwareLoaderFeature,
    ModuleOnlyFeature,
    UserspaceAbiFeature,
    GeneratedArtifactFamily,
    DocsTestsToolsOnlyFeature,
}

#[allow(dead_code)]
impl FeatureKind {
    pub(crate) const ALL: [Self; 18] = [
        Self::Subsystem,
        Self::Driver,
        Self::Bus,
        Self::Filesystem,
        Self::NetworkProtocol,
        Self::CryptoAlgorithm,
        Self::SchedulerFeature,
        Self::SecurityFeature,
        Self::TracingFeature,
        Self::BpfFeature,
        Self::ArchFeature,
        Self::SocPlatform,
        Self::BoardPlatformSupport,
        Self::FirmwareLoaderFeature,
        Self::ModuleOnlyFeature,
        Self::UserspaceAbiFeature,
        Self::GeneratedArtifactFamily,
        Self::DocsTestsToolsOnlyFeature,
    ];

    pub(crate) fn from_stable_name(value: &str) -> Result<Self> {
        let token = normalize_feature_kind_token(value)?;
        match token.as_str() {
            "subsystem" => Ok(Self::Subsystem),
            "driver" => Ok(Self::Driver),
            "bus" => Ok(Self::Bus),
            "filesystem" => Ok(Self::Filesystem),
            "network_protocol" => Ok(Self::NetworkProtocol),
            "crypto_algorithm" => Ok(Self::CryptoAlgorithm),
            "scheduler_feature" => Ok(Self::SchedulerFeature),
            "security_feature" => Ok(Self::SecurityFeature),
            "tracing_feature" => Ok(Self::TracingFeature),
            "bpf_feature" => Ok(Self::BpfFeature),
            "arch_feature" => Ok(Self::ArchFeature),
            "soc_platform" => Ok(Self::SocPlatform),
            "board_platform_support" => Ok(Self::BoardPlatformSupport),
            "firmware_loader_feature" => Ok(Self::FirmwareLoaderFeature),
            "module_only_feature" => Ok(Self::ModuleOnlyFeature),
            "userspace_abi_feature" => Ok(Self::UserspaceAbiFeature),
            "generated_artifact_family" => Ok(Self::GeneratedArtifactFamily),
            "docs_tests_tools_only_feature" => Ok(Self::DocsTestsToolsOnlyFeature),
            _ => anyhow::bail!("unsupported feature kind: {value}"),
        }
    }

    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::Subsystem => "subsystem",
            Self::Driver => "driver",
            Self::Bus => "bus",
            Self::Filesystem => "filesystem",
            Self::NetworkProtocol => "network_protocol",
            Self::CryptoAlgorithm => "crypto_algorithm",
            Self::SchedulerFeature => "scheduler_feature",
            Self::SecurityFeature => "security_feature",
            Self::TracingFeature => "tracing_feature",
            Self::BpfFeature => "bpf_feature",
            Self::ArchFeature => "arch_feature",
            Self::SocPlatform => "soc_platform",
            Self::BoardPlatformSupport => "board_platform_support",
            Self::FirmwareLoaderFeature => "firmware_loader_feature",
            Self::ModuleOnlyFeature => "module_only_feature",
            Self::UserspaceAbiFeature => "userspace_abi_feature",
            Self::GeneratedArtifactFamily => "generated_artifact_family",
            Self::DocsTestsToolsOnlyFeature => "docs_tests_tools_only_feature",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FeatureRoot(RelativeKernelPath);

#[allow(dead_code)]
impl FeatureRoot {
    pub(crate) fn new(root: impl Into<PathBuf>) -> Result<Self> {
        Ok(Self(RelativeKernelPath::new(root)?))
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    pub(crate) fn as_relative_kernel_path(&self) -> &RelativeKernelPath {
        &self.0
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FeatureScope {
    arch_scope: Vec<ArchName>,
}

#[allow(dead_code)]
impl FeatureScope {
    pub(crate) fn from_arch_scope(arches: &[String]) -> Result<Self> {
        Ok(Self {
            arch_scope: sorted_arch_names(arches)?,
        })
    }

    pub(crate) fn unscoped() -> Self {
        Self {
            arch_scope: Vec::new(),
        }
    }

    pub(crate) fn is_unscoped(&self) -> bool {
        self.arch_scope.is_empty()
    }

    pub(crate) fn arch_scope(&self) -> &[ArchName] {
        &self.arch_scope
    }

    pub(crate) fn stable_key(&self) -> String {
        join_arch_names(&self.arch_scope)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureNode {
    intent: FeatureIntent,
}

#[allow(dead_code)]
impl FeatureNode {
    pub(crate) fn from_intent(intent: FeatureIntent) -> Self {
        Self { intent }
    }

    pub(crate) fn id(&self) -> &FeatureId {
        &self.intent.id
    }

    pub(crate) fn intent(&self) -> &FeatureIntent {
        &self.intent
    }

    pub(crate) fn stable_key(&self) -> String {
        format!("feature:{}", self.id().as_str())
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureEdgeKind {
    Dependency,
    Conflict,
    PreservationBoundary,
}

#[allow(dead_code)]
impl FeatureEdgeKind {
    pub(crate) fn from_stable_name(value: &str) -> Result<Self> {
        let token = normalize_feature_edge_kind_token(value)?;
        match token.as_str() {
            "dependency" => Ok(Self::Dependency),
            "conflict" => Ok(Self::Conflict),
            "preservation_boundary" => Ok(Self::PreservationBoundary),
            _ => anyhow::bail!("unsupported feature edge kind: {value}"),
        }
    }

    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::Dependency => "dependency",
            Self::Conflict => "conflict",
            Self::PreservationBoundary => "preservation_boundary",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureEdge {
    from: FeatureId,
    to: FeatureId,
    kind: FeatureEdgeKind,
}

#[allow(dead_code)]
impl FeatureEdge {
    pub(crate) fn new(kind: FeatureEdgeKind, from: FeatureId, to: FeatureId) -> Result<Self> {
        if from == to {
            anyhow::bail!("feature edge endpoints must be distinct: {}", from.as_str());
        }
        Ok(Self { from, to, kind })
    }

    pub(crate) fn from_names(kind: FeatureEdgeKind, from: &str, to: &str) -> Result<Self> {
        Self::new(kind, FeatureId::new(from)?, FeatureId::new(to)?)
    }

    pub(crate) fn from(&self) -> &FeatureId {
        &self.from
    }

    pub(crate) fn to(&self) -> &FeatureId {
        &self.to
    }

    pub(crate) fn kind(&self) -> FeatureEdgeKind {
        self.kind
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}->{}",
            self.kind.stable_name(),
            self.from.as_str(),
            self.to.as_str()
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureGraph {
    nodes: BTreeMap<FeatureId, FeatureNode>,
    edges: BTreeMap<String, FeatureEdge>,
}

#[allow(dead_code)]
impl FeatureGraph {
    pub(crate) fn new(intents: impl IntoIterator<Item = FeatureIntent>) -> Result<Self> {
        Self::with_edges(intents, std::iter::empty())
    }

    pub(crate) fn with_edges(
        intents: impl IntoIterator<Item = FeatureIntent>,
        edges: impl IntoIterator<Item = FeatureEdge>,
    ) -> Result<Self> {
        let mut nodes = BTreeMap::new();
        for intent in intents {
            let node = FeatureNode::from_intent(intent);
            let id = node.id().clone();
            if nodes.insert(id.clone(), node).is_some() {
                anyhow::bail!(
                    "feature graph contains duplicate feature id: {}",
                    id.as_str()
                );
            }
        }

        let mut graph_edges = BTreeMap::new();
        for edge in edges {
            validate_feature_edge_endpoints(&nodes, &edge)?;
            let key = edge.stable_key();
            if graph_edges.insert(key.clone(), edge).is_some() {
                anyhow::bail!("feature graph contains duplicate feature edge: {key}");
            }
        }

        Ok(Self {
            nodes,
            edges: graph_edges,
        })
    }

    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let mut intents = Vec::new();
        for (name, intent) in &profile.features.remove {
            intents.push(FeatureIntent::from_config(
                FeatureIntentAction::Remove,
                name,
                intent,
            )?);
        }
        for (name, intent) in &profile.features.preserve {
            intents.push(FeatureIntent::from_config(
                FeatureIntentAction::Preserve,
                name,
                intent,
            )?);
        }
        Self::new(intents)
    }

    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub(crate) fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub(crate) fn get(&self, id: &FeatureId) -> Option<&FeatureNode> {
        self.nodes.get(id)
    }

    pub(crate) fn nodes(&self) -> impl Iterator<Item = &FeatureNode> {
        self.nodes.values()
    }

    pub(crate) fn intents(&self) -> impl Iterator<Item = &FeatureIntent> {
        self.nodes.values().map(FeatureNode::intent)
    }

    pub(crate) fn edges(&self) -> impl Iterator<Item = &FeatureEdge> {
        self.edges.values()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureIntentAction {
    Remove,
    Preserve,
}

#[allow(dead_code)]
impl FeatureIntentAction {
    pub(crate) fn from_stable_name(value: &str) -> Result<Self> {
        match value {
            "remove" => Ok(Self::Remove),
            "preserve" => Ok(Self::Preserve),
            _ => anyhow::bail!("feature intent action must be remove or preserve: {value}"),
        }
    }

    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::Remove => "remove",
            Self::Preserve => "preserve",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureIntent {
    pub(crate) action: FeatureIntentAction,
    pub(crate) id: FeatureId,
    pub(crate) kind: Option<FeatureKind>,
    pub(crate) roots: Vec<FeatureRoot>,
    pub(crate) remove_paths: Vec<RelativeKernelPath>,
    pub(crate) configs: Vec<KconfigSymbol>,
    pub(crate) remove_configs: Vec<KconfigSymbol>,
    pub(crate) exported_symbols: Vec<ExportedSymbol>,
    pub(crate) remove_exported_symbols: Vec<ExportedSymbol>,
    pub(crate) module_names: Vec<ModuleName>,
    pub(crate) remove_module_names: Vec<ModuleName>,
    pub(crate) module_aliases: Vec<ModuleAlias>,
    pub(crate) remove_module_aliases: Vec<ModuleAlias>,
    pub(crate) device_compatibles: Vec<DeviceCompatible>,
    pub(crate) remove_device_compatibles: Vec<DeviceCompatible>,
    pub(crate) acpi_ids: Vec<AcpiId>,
    pub(crate) remove_acpi_ids: Vec<AcpiId>,
    pub(crate) pci_ids: Vec<PciId>,
    pub(crate) remove_pci_ids: Vec<PciId>,
    pub(crate) usb_ids: Vec<UsbId>,
    pub(crate) remove_usb_ids: Vec<UsbId>,
    pub(crate) firmware_paths: Vec<FirmwarePath>,
    pub(crate) remove_firmware_paths: Vec<FirmwarePath>,
    pub(crate) initcalls: Vec<Initcall>,
    pub(crate) remove_initcalls: Vec<Initcall>,
    pub(crate) runtime_registrations: Vec<RuntimeRegistrationSurface>,
    pub(crate) remove_runtime_registrations: Vec<RuntimeRegistrationSurface>,
    pub(crate) docs: Vec<DocumentationPath>,
    pub(crate) remove_docs: Vec<DocumentationPath>,
    pub(crate) tools: Vec<ToolPath>,
    pub(crate) remove_tools: Vec<ToolPath>,
    pub(crate) samples: Vec<SamplePath>,
    pub(crate) remove_samples: Vec<SamplePath>,
    pub(crate) kunit_suites: Vec<KunitSuite>,
    pub(crate) remove_kunit_suites: Vec<KunitSuite>,
    pub(crate) kselftest_targets: Vec<KselftestTarget>,
    pub(crate) remove_kselftest_targets: Vec<KselftestTarget>,
    pub(crate) allow_public_header_removal: bool,
    pub(crate) allow_uapi_header_removal: bool,
    pub(crate) scope: FeatureScope,
    pub(crate) safety: Option<FeatureSafetyLevel>,
    pub(crate) preserve_uapi: bool,
    pub(crate) preserve_module_aliases: bool,
    pub(crate) require_clean_boot: bool,
    pub(crate) report_only: bool,
}

#[allow(dead_code)]
impl FeatureIntent {
    pub(crate) fn from_config(
        action: FeatureIntentAction,
        name: &str,
        config: &FeatureIntentConfig,
    ) -> Result<Self> {
        let id = FeatureId::new(name)?;
        let kind = config
            .kind
            .as_deref()
            .map(FeatureKind::from_stable_name)
            .transpose()?;
        let roots = sorted_feature_roots(&config.roots)?;
        let remove_paths = sorted_relative_kernel_paths(&config.remove_paths)?;
        let configs = sorted_kconfig_symbols(&config.configs)?;
        let remove_configs = sorted_kconfig_symbols(&config.remove_configs)?;
        let exported_symbols = sorted_exported_symbols(&config.exported_symbols)?;
        let remove_exported_symbols = sorted_exported_symbols(&config.remove_exported_symbols)?;
        let module_names = sorted_module_names(&config.module_names)?;
        let remove_module_names = sorted_module_names(&config.remove_module_names)?;
        let module_aliases = sorted_module_aliases(&config.module_aliases)?;
        let remove_module_aliases = sorted_module_aliases(&config.remove_module_aliases)?;
        let device_compatibles = sorted_device_compatibles(&config.device_compatibles)?;
        let remove_device_compatibles =
            sorted_device_compatibles(&config.remove_device_compatibles)?;
        let acpi_ids = sorted_acpi_ids(&config.acpi_ids)?;
        let remove_acpi_ids = sorted_acpi_ids(&config.remove_acpi_ids)?;
        let pci_ids = sorted_pci_ids(&config.pci_ids)?;
        let remove_pci_ids = sorted_pci_ids(&config.remove_pci_ids)?;
        let usb_ids = sorted_usb_ids(&config.usb_ids)?;
        let remove_usb_ids = sorted_usb_ids(&config.remove_usb_ids)?;
        let firmware_paths = sorted_firmware_paths(&config.firmware_paths)?;
        let remove_firmware_paths = sorted_firmware_paths(&config.remove_firmware_paths)?;
        let initcalls = sorted_initcalls(&config.initcalls)?;
        let remove_initcalls = sorted_initcalls(&config.remove_initcalls)?;
        let runtime_registrations =
            sorted_runtime_registration_surfaces(&config.runtime_registrations)?;
        let remove_runtime_registrations =
            sorted_runtime_registration_surfaces(&config.remove_runtime_registrations)?;
        let docs = sorted_documentation_paths(&config.docs)?;
        let remove_docs = sorted_documentation_paths(&config.remove_docs)?;
        let tools = sorted_tool_paths(&config.tools)?;
        let remove_tools = sorted_tool_paths(&config.remove_tools)?;
        let samples = sorted_sample_paths(&config.samples)?;
        let remove_samples = sorted_sample_paths(&config.remove_samples)?;
        let kunit_suites = sorted_kunit_suites(&config.kunit_suites)?;
        let remove_kunit_suites = sorted_kunit_suites(&config.remove_kunit_suites)?;
        let kselftest_targets = sorted_kselftest_targets(&config.kselftest_targets)?;
        let remove_kselftest_targets = sorted_kselftest_targets(&config.remove_kselftest_targets)?;
        let scope = FeatureScope::from_arch_scope(&config.arch_scope)?;

        validate_action_specific_fields(
            action,
            id.as_str(),
            config,
            &roots,
            &remove_paths,
            &configs,
            &remove_configs,
            &exported_symbols,
            &remove_exported_symbols,
            &module_names,
            &remove_module_names,
            &module_aliases,
            &remove_module_aliases,
            &device_compatibles,
            &remove_device_compatibles,
            &acpi_ids,
            &remove_acpi_ids,
            &pci_ids,
            &remove_pci_ids,
            &usb_ids,
            &remove_usb_ids,
            &firmware_paths,
            &remove_firmware_paths,
            &initcalls,
            &remove_initcalls,
            &runtime_registrations,
            &remove_runtime_registrations,
            &docs,
            &remove_docs,
            &tools,
            &remove_tools,
            &samples,
            &remove_samples,
            &kunit_suites,
            &remove_kunit_suites,
            &kselftest_targets,
            &remove_kselftest_targets,
        )?;

        Ok(Self {
            action,
            id,
            kind,
            roots,
            remove_paths,
            configs,
            remove_configs,
            exported_symbols,
            remove_exported_symbols,
            module_names,
            remove_module_names,
            module_aliases,
            remove_module_aliases,
            device_compatibles,
            remove_device_compatibles,
            acpi_ids,
            remove_acpi_ids,
            pci_ids,
            remove_pci_ids,
            usb_ids,
            remove_usb_ids,
            firmware_paths,
            remove_firmware_paths,
            initcalls,
            remove_initcalls,
            runtime_registrations,
            remove_runtime_registrations,
            docs,
            remove_docs,
            tools,
            remove_tools,
            samples,
            remove_samples,
            kunit_suites,
            remove_kunit_suites,
            kselftest_targets,
            remove_kselftest_targets,
            allow_public_header_removal: config.allow_public_header_removal,
            allow_uapi_header_removal: config.allow_uapi_header_removal,
            scope,
            safety: config.safety,
            preserve_uapi: config.preserve_uapi,
            preserve_module_aliases: config.preserve_module_aliases,
            require_clean_boot: config.require_clean_boot,
            report_only: config.report_only,
        })
    }

    pub(crate) fn roots_key(&self) -> String {
        join_feature_roots(&self.roots)
    }

    pub(crate) fn remove_paths_key(&self) -> String {
        join_relative_kernel_paths(&self.remove_paths)
    }

    pub(crate) fn configs_key(&self) -> String {
        join_kconfig_symbols(&self.configs)
    }

    pub(crate) fn remove_configs_key(&self) -> String {
        join_kconfig_symbols(&self.remove_configs)
    }

    pub(crate) fn exported_symbols_key(&self) -> String {
        join_exported_symbols(&self.exported_symbols)
    }

    pub(crate) fn remove_exported_symbols_key(&self) -> String {
        join_exported_symbols(&self.remove_exported_symbols)
    }

    pub(crate) fn module_names_key(&self) -> String {
        join_module_names(&self.module_names)
    }

    pub(crate) fn remove_module_names_key(&self) -> String {
        join_module_names(&self.remove_module_names)
    }

    pub(crate) fn module_aliases_key(&self) -> String {
        join_module_aliases(&self.module_aliases)
    }

    pub(crate) fn remove_module_aliases_key(&self) -> String {
        join_module_aliases(&self.remove_module_aliases)
    }

    pub(crate) fn device_compatibles_key(&self) -> String {
        join_device_compatibles(&self.device_compatibles)
    }

    pub(crate) fn remove_device_compatibles_key(&self) -> String {
        join_device_compatibles(&self.remove_device_compatibles)
    }

    pub(crate) fn acpi_ids_key(&self) -> String {
        join_acpi_ids(&self.acpi_ids)
    }

    pub(crate) fn remove_acpi_ids_key(&self) -> String {
        join_acpi_ids(&self.remove_acpi_ids)
    }

    pub(crate) fn pci_ids_key(&self) -> String {
        join_pci_ids(&self.pci_ids)
    }

    pub(crate) fn remove_pci_ids_key(&self) -> String {
        join_pci_ids(&self.remove_pci_ids)
    }

    pub(crate) fn usb_ids_key(&self) -> String {
        join_usb_ids(&self.usb_ids)
    }

    pub(crate) fn remove_usb_ids_key(&self) -> String {
        join_usb_ids(&self.remove_usb_ids)
    }

    pub(crate) fn firmware_paths_key(&self) -> String {
        join_firmware_paths(&self.firmware_paths)
    }

    pub(crate) fn remove_firmware_paths_key(&self) -> String {
        join_firmware_paths(&self.remove_firmware_paths)
    }

    pub(crate) fn initcalls_key(&self) -> String {
        join_initcalls(&self.initcalls)
    }

    pub(crate) fn remove_initcalls_key(&self) -> String {
        join_initcalls(&self.remove_initcalls)
    }

    pub(crate) fn runtime_registrations_key(&self) -> String {
        join_runtime_registration_surfaces(&self.runtime_registrations)
    }

    pub(crate) fn remove_runtime_registrations_key(&self) -> String {
        join_runtime_registration_surfaces(&self.remove_runtime_registrations)
    }

    pub(crate) fn docs_key(&self) -> String {
        join_documentation_paths(&self.docs)
    }

    pub(crate) fn remove_docs_key(&self) -> String {
        join_documentation_paths(&self.remove_docs)
    }

    pub(crate) fn tools_key(&self) -> String {
        join_tool_paths(&self.tools)
    }

    pub(crate) fn remove_tools_key(&self) -> String {
        join_tool_paths(&self.remove_tools)
    }

    pub(crate) fn samples_key(&self) -> String {
        join_sample_paths(&self.samples)
    }

    pub(crate) fn remove_samples_key(&self) -> String {
        join_sample_paths(&self.remove_samples)
    }

    pub(crate) fn kunit_suites_key(&self) -> String {
        join_kunit_suites(&self.kunit_suites)
    }

    pub(crate) fn remove_kunit_suites_key(&self) -> String {
        join_kunit_suites(&self.remove_kunit_suites)
    }

    pub(crate) fn kselftest_targets_key(&self) -> String {
        join_kselftest_targets(&self.kselftest_targets)
    }

    pub(crate) fn remove_kselftest_targets_key(&self) -> String {
        join_kselftest_targets(&self.remove_kselftest_targets)
    }

    pub(crate) fn arch_scope_key(&self) -> String {
        self.scope.stable_key()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureOwnershipKind {
    ExplicitlyRemoved,
    ExplicitlyPreserved,
    OwnedSolelyByRemovedFeature,
    SharedWithLiveFeature,
    GeneratedByLiveBuild,
    PublicAbiSurface,
    PublicUapiSurface,
    ArchLocal,
    ArchShared,
    RuntimeOnlySurface,
    TestOnlySurface,
    DocumentationOnlySurface,
    UnknownOwnership,
    AmbiguousOwnership,
    UnsupportedOwnership,
}

#[allow(dead_code)]
impl FeatureOwnershipKind {
    pub(crate) const ALL: [Self; 15] = [
        Self::ExplicitlyRemoved,
        Self::ExplicitlyPreserved,
        Self::OwnedSolelyByRemovedFeature,
        Self::SharedWithLiveFeature,
        Self::GeneratedByLiveBuild,
        Self::PublicAbiSurface,
        Self::PublicUapiSurface,
        Self::ArchLocal,
        Self::ArchShared,
        Self::RuntimeOnlySurface,
        Self::TestOnlySurface,
        Self::DocumentationOnlySurface,
        Self::UnknownOwnership,
        Self::AmbiguousOwnership,
        Self::UnsupportedOwnership,
    ];

    pub(crate) fn from_stable_name(value: &str) -> Result<Self> {
        let token = normalize_feature_ownership_kind_token(value)?;
        match token.as_str() {
            "explicitly_removed" => Ok(Self::ExplicitlyRemoved),
            "explicitly_preserved" => Ok(Self::ExplicitlyPreserved),
            "owned_solely_by_removed_feature" => Ok(Self::OwnedSolelyByRemovedFeature),
            "shared_with_live_feature" => Ok(Self::SharedWithLiveFeature),
            "generated_by_live_build" => Ok(Self::GeneratedByLiveBuild),
            "public_abi_surface" => Ok(Self::PublicAbiSurface),
            "public_uapi_surface" => Ok(Self::PublicUapiSurface),
            "arch_local" => Ok(Self::ArchLocal),
            "arch_shared" => Ok(Self::ArchShared),
            "runtime_only_surface" => Ok(Self::RuntimeOnlySurface),
            "test_only_surface" => Ok(Self::TestOnlySurface),
            "documentation_only_surface" => Ok(Self::DocumentationOnlySurface),
            "unknown_ownership" => Ok(Self::UnknownOwnership),
            "ambiguous_ownership" => Ok(Self::AmbiguousOwnership),
            "unsupported_ownership" => Ok(Self::UnsupportedOwnership),
            _ => anyhow::bail!("unsupported feature ownership kind: {value}"),
        }
    }

    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::ExplicitlyRemoved => "explicitly_removed",
            Self::ExplicitlyPreserved => "explicitly_preserved",
            Self::OwnedSolelyByRemovedFeature => "owned_solely_by_removed_feature",
            Self::SharedWithLiveFeature => "shared_with_live_feature",
            Self::GeneratedByLiveBuild => "generated_by_live_build",
            Self::PublicAbiSurface => "public_abi_surface",
            Self::PublicUapiSurface => "public_uapi_surface",
            Self::ArchLocal => "arch_local",
            Self::ArchShared => "arch_shared",
            Self::RuntimeOnlySurface => "runtime_only_surface",
            Self::TestOnlySurface => "test_only_surface",
            Self::DocumentationOnlySurface => "documentation_only_surface",
            Self::UnknownOwnership => "unknown_ownership",
            Self::AmbiguousOwnership => "ambiguous_ownership",
            Self::UnsupportedOwnership => "unsupported_ownership",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FeatureOwnershipSubject(String);

#[allow(dead_code)]
impl FeatureOwnershipSubject {
    pub(crate) fn new(subject: impl Into<String>) -> Result<Self> {
        Ok(Self(normalize_feature_ownership_subject(subject)?))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureOwnership {
    feature: FeatureId,
    subject: FeatureOwnershipSubject,
    kind: FeatureOwnershipKind,
}

#[allow(dead_code)]
impl FeatureOwnership {
    pub(crate) fn new(
        kind: FeatureOwnershipKind,
        feature: FeatureId,
        subject: FeatureOwnershipSubject,
    ) -> Self {
        Self {
            feature,
            subject,
            kind,
        }
    }

    pub(crate) fn from_name(
        kind: FeatureOwnershipKind,
        feature: &str,
        subject: &str,
    ) -> Result<Self> {
        Ok(Self::new(
            kind,
            FeatureId::new(feature)?,
            FeatureOwnershipSubject::new(subject)?,
        ))
    }

    pub(crate) fn kind(&self) -> FeatureOwnershipKind {
        self.kind
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn subject(&self) -> &FeatureOwnershipSubject {
        &self.subject
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.subject.as_str()
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureImpactReport {
    remove_paths: usize,
    remove_configs: usize,
    default_overrides: usize,
    preserve_paths: usize,
    preserve_configs: usize,
    ownerships: Vec<FeatureOwnership>,
}

#[allow(dead_code)]
impl FeatureImpactReport {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Self {
        let removal = profile.effective_removal_input();
        let preservation = profile.effective_preservation_input();
        Self {
            remove_paths: removal.as_ref().map_or(0, |slim| slim.remove_paths.len()),
            remove_configs: removal.as_ref().map_or(0, |slim| slim.remove_configs.len()),
            default_overrides: removal.as_ref().map_or(0, |slim| slim.set_defaults.len()),
            preserve_paths: preservation
                .as_ref()
                .map_or(0, |input| input.preserve_paths.len()),
            preserve_configs: preservation
                .as_ref()
                .map_or(0, |input| input.preserve_configs.len()),
            ownerships: Vec::new(),
        }
    }

    pub(crate) fn for_feature(profile: &ProfileConfig, feature: &str) -> Result<Self> {
        let feature = FeatureId::new(feature)?;
        let mut report = Self::default();
        if let Some(intent) = profile.features.remove.get(feature.as_str()) {
            report.remove_paths += intent.roots.len();
            report.remove_paths += intent.remove_paths.len();
            report.remove_configs += intent.configs.len();
            report.remove_configs += intent.remove_configs.len();
        }
        if let Some(intent) = profile.features.preserve.get(feature.as_str()) {
            report.preserve_paths += intent.roots.len();
            report.preserve_configs += intent.configs.len();
        }
        Ok(report)
    }

    pub(crate) fn with_ownerships(
        mut self,
        ownerships: impl IntoIterator<Item = FeatureOwnership>,
    ) -> Self {
        self.ownerships = ownerships.into_iter().collect();
        self.ownerships
            .sort_by(|left, right| left.stable_key().cmp(&right.stable_key()));
        self
    }

    pub(crate) fn remove_paths(&self) -> usize {
        self.remove_paths
    }

    pub(crate) fn remove_configs(&self) -> usize {
        self.remove_configs
    }

    pub(crate) fn default_overrides(&self) -> usize {
        self.default_overrides
    }

    pub(crate) fn preserve_paths(&self) -> usize {
        self.preserve_paths
    }

    pub(crate) fn preserve_configs(&self) -> usize {
        self.preserve_configs
    }

    pub(crate) fn ownerships(&self) -> &[FeatureOwnership] {
        &self.ownerships
    }

    pub(crate) fn ownership_count(&self) -> usize {
        self.ownerships.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.remove_paths == 0
            && self.remove_configs == 0
            && self.default_overrides == 0
            && self.preserve_paths == 0
            && self.preserve_configs == 0
            && self.ownerships.is_empty()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FeatureConflictKind {
    RemovedFeatureOwnsLiveDependency,
    RemovedFeatureSelectedByLiveKconfig,
    RemovedFeatureReferencedByLiveKbuild,
    RemovedFeatureExportsConsumedSymbol,
    RemovedFeatureDeviceIdReferencedByLiveTable,
    RemovedFeatureUapiReferencedByUserspaceFacingCode,
    RemovedFeatureRuntimeRegistrationReachable,
    SharedFileBetweenRemovedAndPreservedFeatures,
}

#[allow(dead_code)]
impl FeatureConflictKind {
    pub(crate) const ALL: [Self; 8] = [
        Self::RemovedFeatureOwnsLiveDependency,
        Self::RemovedFeatureSelectedByLiveKconfig,
        Self::RemovedFeatureReferencedByLiveKbuild,
        Self::RemovedFeatureExportsConsumedSymbol,
        Self::RemovedFeatureDeviceIdReferencedByLiveTable,
        Self::RemovedFeatureUapiReferencedByUserspaceFacingCode,
        Self::RemovedFeatureRuntimeRegistrationReachable,
        Self::SharedFileBetweenRemovedAndPreservedFeatures,
    ];

    pub(crate) fn from_stable_name(value: &str) -> Result<Self> {
        let token = normalize_feature_conflict_kind_token(value)?;
        match token.as_str() {
            "removed_feature_owns_live_dependency" => Ok(Self::RemovedFeatureOwnsLiveDependency),
            "removed_feature_selected_by_live_kconfig" => {
                Ok(Self::RemovedFeatureSelectedByLiveKconfig)
            }
            "removed_feature_referenced_by_live_kbuild" => {
                Ok(Self::RemovedFeatureReferencedByLiveKbuild)
            }
            "removed_feature_exports_consumed_symbol" => {
                Ok(Self::RemovedFeatureExportsConsumedSymbol)
            }
            "removed_feature_device_id_referenced_by_live_table" => {
                Ok(Self::RemovedFeatureDeviceIdReferencedByLiveTable)
            }
            "removed_feature_uapi_referenced_by_userspace_facing_code" => {
                Ok(Self::RemovedFeatureUapiReferencedByUserspaceFacingCode)
            }
            "removed_feature_runtime_registration_reachable" => {
                Ok(Self::RemovedFeatureRuntimeRegistrationReachable)
            }
            "shared_file_between_removed_and_preserved_features" => {
                Ok(Self::SharedFileBetweenRemovedAndPreservedFeatures)
            }
            _ => anyhow::bail!("unsupported feature conflict kind: {value}"),
        }
    }

    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::RemovedFeatureOwnsLiveDependency => "removed_feature_owns_live_dependency",
            Self::RemovedFeatureSelectedByLiveKconfig => "removed_feature_selected_by_live_kconfig",
            Self::RemovedFeatureReferencedByLiveKbuild => {
                "removed_feature_referenced_by_live_kbuild"
            }
            Self::RemovedFeatureExportsConsumedSymbol => "removed_feature_exports_consumed_symbol",
            Self::RemovedFeatureDeviceIdReferencedByLiveTable => {
                "removed_feature_device_id_referenced_by_live_table"
            }
            Self::RemovedFeatureUapiReferencedByUserspaceFacingCode => {
                "removed_feature_uapi_referenced_by_userspace_facing_code"
            }
            Self::RemovedFeatureRuntimeRegistrationReachable => {
                "removed_feature_runtime_registration_reachable"
            }
            Self::SharedFileBetweenRemovedAndPreservedFeatures => {
                "shared_file_between_removed_and_preserved_features"
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureConflict {
    kind: FeatureConflictKind,
    feature: FeatureId,
    subject: FeatureOwnershipSubject,
    summary: String,
    suggested_action: String,
    strict_blocking: bool,
}

#[allow(dead_code)]
impl FeatureConflict {
    pub(crate) fn new(
        kind: FeatureConflictKind,
        feature: FeatureId,
        subject: FeatureOwnershipSubject,
        summary: impl Into<String>,
        suggested_action: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            kind,
            feature,
            subject,
            summary: normalize_feature_conflict_text("feature conflict summary", summary)?,
            suggested_action: normalize_feature_conflict_text(
                "feature conflict suggested action",
                suggested_action,
            )?,
            strict_blocking: true,
        })
    }

    pub(crate) fn from_name(
        kind: FeatureConflictKind,
        feature: &str,
        subject: &str,
        summary: &str,
        suggested_action: &str,
    ) -> Result<Self> {
        Self::new(
            kind,
            FeatureId::new(feature)?,
            FeatureOwnershipSubject::new(subject)?,
            summary,
            suggested_action,
        )
    }

    pub(crate) fn non_blocking(mut self) -> Self {
        self.strict_blocking = false;
        self
    }

    pub(crate) fn kind(&self) -> FeatureConflictKind {
        self.kind
    }

    pub(crate) fn feature(&self) -> &FeatureId {
        &self.feature
    }

    pub(crate) fn subject(&self) -> &FeatureOwnershipSubject {
        &self.subject
    }

    pub(crate) fn summary(&self) -> &str {
        &self.summary
    }

    pub(crate) fn suggested_action(&self) -> &str {
        &self.suggested_action
    }

    pub(crate) fn strict_blocking(&self) -> bool {
        self.strict_blocking
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.stable_name(),
            self.feature.as_str(),
            self.subject.as_str()
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FeatureConflictReport {
    conflicts: Vec<FeatureConflict>,
}

#[allow(dead_code)]
impl FeatureConflictReport {
    pub(crate) fn new(conflicts: impl IntoIterator<Item = FeatureConflict>) -> Result<Self> {
        let mut conflicts = conflicts.into_iter().collect::<Vec<_>>();
        conflicts.sort_by(|left, right| left.stable_key().cmp(&right.stable_key()));

        let mut previous_key: Option<String> = None;
        for conflict in &conflicts {
            let key = conflict.stable_key();
            if previous_key.as_deref() == Some(key.as_str()) {
                anyhow::bail!("feature conflict report contains duplicate conflict: {key}");
            }
            previous_key = Some(key);
        }

        Ok(Self { conflicts })
    }

    pub(crate) fn len(&self) -> usize {
        self.conflicts.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.conflicts.is_empty()
    }

    pub(crate) fn conflicts(&self) -> &[FeatureConflict] {
        &self.conflicts
    }

    pub(crate) fn blocking_count(&self) -> usize {
        self.conflicts
            .iter()
            .filter(|conflict| conflict.strict_blocking())
            .count()
    }

    pub(crate) fn has_blocking_conflicts(&self) -> bool {
        self.conflicts
            .iter()
            .any(|conflict| conflict.strict_blocking())
    }

    pub(crate) fn reject_blocking_conflicts_in_strict_mode(&self, strict_mode: bool) -> Result<()> {
        if !strict_mode || !self.has_blocking_conflicts() {
            return Ok(());
        }

        let mut message = format!(
            "unresolved feature conflicts block strict mutation ({} blocking conflict(s))",
            self.blocking_count()
        );
        for conflict in self
            .conflicts
            .iter()
            .filter(|conflict| conflict.strict_blocking())
        {
            message.push_str(&format!(
                "\n- {}\n  summary: {}\n  action: {}",
                conflict.stable_key(),
                conflict.summary(),
                conflict.suggested_action()
            ));
        }
        anyhow::bail!(message);
    }
}

fn normalize_feature_kind_token(value: &str) -> Result<String> {
    normalize_feature_token("feature kind", value)
}

fn normalize_feature_edge_kind_token(value: &str) -> Result<String> {
    normalize_feature_token("feature edge kind", value)
}

fn normalize_feature_ownership_kind_token(value: &str) -> Result<String> {
    normalize_feature_token("feature ownership kind", value)
}

fn normalize_feature_conflict_kind_token(value: &str) -> Result<String> {
    normalize_feature_token("feature conflict kind", value)
}

fn normalize_feature_token(label: &str, value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("{label} must not be empty");
    }
    Ok(value
        .chars()
        .map(|ch| match ch {
            '-' | '/' | ' ' => '_',
            _ => ch.to_ascii_lowercase(),
        })
        .collect())
}

fn normalize_feature_conflict_text(label: &str, value: impl Into<String>) -> Result<String> {
    let value = value.into().trim().to_string();
    if value.is_empty() {
        anyhow::bail!("{label} must not be empty");
    }
    if value.chars().any(char::is_control) {
        anyhow::bail!("{label} must not contain control characters");
    }
    Ok(value)
}

fn normalize_feature_ownership_subject(subject: impl Into<String>) -> Result<String> {
    let subject = subject.into().trim().to_string();
    if subject.is_empty() {
        anyhow::bail!("feature ownership subject must not be empty");
    }
    if subject.chars().any(char::is_control) {
        anyhow::bail!("feature ownership subject must not contain control characters");
    }
    Ok(subject)
}

fn validate_feature_edge_endpoints(
    nodes: &BTreeMap<FeatureId, FeatureNode>,
    edge: &FeatureEdge,
) -> Result<()> {
    if !nodes.contains_key(edge.from()) {
        anyhow::bail!(
            "feature edge references unknown source feature: {}",
            edge.from().as_str()
        );
    }
    if !nodes.contains_key(edge.to()) {
        anyhow::bail!(
            "feature edge references unknown target feature: {}",
            edge.to().as_str()
        );
    }
    Ok(())
}

fn validate_action_specific_fields(
    action: FeatureIntentAction,
    name: &str,
    config: &FeatureIntentConfig,
    roots: &[FeatureRoot],
    remove_paths: &[RelativeKernelPath],
    configs: &[KconfigSymbol],
    remove_configs: &[KconfigSymbol],
    exported_symbols: &[ExportedSymbol],
    remove_exported_symbols: &[ExportedSymbol],
    module_names: &[ModuleName],
    remove_module_names: &[ModuleName],
    module_aliases: &[ModuleAlias],
    remove_module_aliases: &[ModuleAlias],
    device_compatibles: &[DeviceCompatible],
    remove_device_compatibles: &[DeviceCompatible],
    acpi_ids: &[AcpiId],
    remove_acpi_ids: &[AcpiId],
    pci_ids: &[PciId],
    remove_pci_ids: &[PciId],
    usb_ids: &[UsbId],
    remove_usb_ids: &[UsbId],
    firmware_paths: &[FirmwarePath],
    remove_firmware_paths: &[FirmwarePath],
    initcalls: &[Initcall],
    remove_initcalls: &[Initcall],
    runtime_registrations: &[RuntimeRegistrationSurface],
    remove_runtime_registrations: &[RuntimeRegistrationSurface],
    docs: &[DocumentationPath],
    remove_docs: &[DocumentationPath],
    tools: &[ToolPath],
    remove_tools: &[ToolPath],
    samples: &[SamplePath],
    remove_samples: &[SamplePath],
    kunit_suites: &[KunitSuite],
    remove_kunit_suites: &[KunitSuite],
    kselftest_targets: &[KselftestTarget],
    remove_kselftest_targets: &[KselftestTarget],
) -> Result<()> {
    let declares_feature_root = !roots.is_empty()
        || !configs.is_empty()
        || !exported_symbols.is_empty()
        || !module_names.is_empty()
        || !module_aliases.is_empty()
        || !device_compatibles.is_empty()
        || !acpi_ids.is_empty()
        || !pci_ids.is_empty()
        || !usb_ids.is_empty()
        || !firmware_paths.is_empty()
        || !initcalls.is_empty()
        || !runtime_registrations.is_empty()
        || !docs.is_empty()
        || !tools.is_empty()
        || !samples.is_empty()
        || !kunit_suites.is_empty()
        || !kselftest_targets.is_empty();
    let declares_removal_root = declares_feature_root
        || !remove_paths.is_empty()
        || !remove_configs.is_empty()
        || !remove_exported_symbols.is_empty()
        || !remove_module_names.is_empty()
        || !remove_module_aliases.is_empty()
        || !remove_device_compatibles.is_empty()
        || !remove_acpi_ids.is_empty()
        || !remove_pci_ids.is_empty()
        || !remove_usb_ids.is_empty()
        || !remove_firmware_paths.is_empty()
        || !remove_initcalls.is_empty()
        || !remove_runtime_registrations.is_empty()
        || !remove_docs.is_empty()
        || !remove_tools.is_empty()
        || !remove_samples.is_empty()
        || !remove_kunit_suites.is_empty()
        || !remove_kselftest_targets.is_empty();

    match action {
        FeatureIntentAction::Remove => {
            if !declares_removal_root {
                anyhow::bail!(
                    "features.remove.{name} must declare roots, configs, exported_symbols, module_names, module_aliases, device_compatibles, acpi_ids, pci_ids, usb_ids, firmware_paths, initcalls, runtime_registrations, docs, tools, samples, kunit_suites, kselftest_targets, remove_paths, remove_configs, remove_exported_symbols, remove_module_names, remove_module_aliases, remove_device_compatibles, remove_acpi_ids, remove_pci_ids, remove_usb_ids, remove_firmware_paths, remove_initcalls, remove_runtime_registrations, remove_docs, remove_tools, remove_samples, remove_kunit_suites, or remove_kselftest_targets"
                );
            }
        }
        FeatureIntentAction::Preserve => {
            if !declares_feature_root {
                anyhow::bail!(
                    "features.preserve.{name} must declare roots, configs, exported_symbols, module_names, module_aliases, device_compatibles, acpi_ids, pci_ids, usb_ids, firmware_paths, initcalls, runtime_registrations, docs, tools, samples, kunit_suites, or kselftest_targets"
                );
            }
            if !remove_paths.is_empty() {
                anyhow::bail!("features.preserve.{name}.remove_paths is removal-only");
            }
            if !remove_configs.is_empty() {
                anyhow::bail!("features.preserve.{name}.remove_configs is removal-only");
            }
            if !remove_exported_symbols.is_empty() {
                anyhow::bail!("features.preserve.{name}.remove_exported_symbols is removal-only");
            }
            if !remove_module_names.is_empty() {
                anyhow::bail!("features.preserve.{name}.remove_module_names is removal-only");
            }
            if !remove_module_aliases.is_empty() {
                anyhow::bail!("features.preserve.{name}.remove_module_aliases is removal-only");
            }
            if !remove_device_compatibles.is_empty() {
                anyhow::bail!("features.preserve.{name}.remove_device_compatibles is removal-only");
            }
            if !remove_acpi_ids.is_empty() {
                anyhow::bail!("features.preserve.{name}.remove_acpi_ids is removal-only");
            }
            if !remove_pci_ids.is_empty() {
                anyhow::bail!("features.preserve.{name}.remove_pci_ids is removal-only");
            }
            if !remove_usb_ids.is_empty() {
                anyhow::bail!("features.preserve.{name}.remove_usb_ids is removal-only");
            }
            if !remove_firmware_paths.is_empty() {
                anyhow::bail!("features.preserve.{name}.remove_firmware_paths is removal-only");
            }
            if !remove_initcalls.is_empty() {
                anyhow::bail!("features.preserve.{name}.remove_initcalls is removal-only");
            }
            if !remove_runtime_registrations.is_empty() {
                anyhow::bail!(
                    "features.preserve.{name}.remove_runtime_registrations is removal-only"
                );
            }
            if !remove_docs.is_empty() {
                anyhow::bail!("features.preserve.{name}.remove_docs is removal-only");
            }
            if !remove_tools.is_empty() {
                anyhow::bail!("features.preserve.{name}.remove_tools is removal-only");
            }
            if !remove_samples.is_empty() {
                anyhow::bail!("features.preserve.{name}.remove_samples is removal-only");
            }
            if !remove_kunit_suites.is_empty() {
                anyhow::bail!("features.preserve.{name}.remove_kunit_suites is removal-only");
            }
            if !remove_kselftest_targets.is_empty() {
                anyhow::bail!("features.preserve.{name}.remove_kselftest_targets is removal-only");
            }
            if config.allow_public_header_removal {
                anyhow::bail!(
                    "features.preserve.{name}.allow_public_header_removal is removal-only"
                );
            }
            if config.allow_uapi_header_removal {
                anyhow::bail!("features.preserve.{name}.allow_uapi_header_removal is removal-only");
            }
            if config.safety.is_some() {
                anyhow::bail!("features.preserve.{name}.safety is removal-only");
            }
        }
    }
    Ok(())
}

fn sorted_feature_roots(values: &[String]) -> Result<Vec<FeatureRoot>> {
    let mut values = values
        .iter()
        .map(|value| FeatureRoot::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_relative_kernel_paths(values: &[String]) -> Result<Vec<RelativeKernelPath>> {
    let mut values = values
        .iter()
        .map(|value| RelativeKernelPath::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_kconfig_symbols(values: &[String]) -> Result<Vec<KconfigSymbol>> {
    let mut values = values
        .iter()
        .map(|value| KconfigSymbol::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_exported_symbols(values: &[String]) -> Result<Vec<ExportedSymbol>> {
    let mut values = values
        .iter()
        .map(|value| ExportedSymbol::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_module_names(values: &[String]) -> Result<Vec<ModuleName>> {
    let mut values = values
        .iter()
        .map(|value| ModuleName::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_module_aliases(values: &[String]) -> Result<Vec<ModuleAlias>> {
    let mut values = values
        .iter()
        .map(|value| ModuleAlias::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_device_compatibles(values: &[String]) -> Result<Vec<DeviceCompatible>> {
    let mut values = values
        .iter()
        .map(|value| DeviceCompatible::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_acpi_ids(values: &[String]) -> Result<Vec<AcpiId>> {
    let mut values = values
        .iter()
        .map(|value| AcpiId::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_pci_ids(values: &[String]) -> Result<Vec<PciId>> {
    let mut values = values
        .iter()
        .map(|value| PciId::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_usb_ids(values: &[String]) -> Result<Vec<UsbId>> {
    let mut values = values
        .iter()
        .map(|value| UsbId::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_firmware_paths(values: &[String]) -> Result<Vec<FirmwarePath>> {
    let mut values = values
        .iter()
        .map(|value| FirmwarePath::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_initcalls(values: &[String]) -> Result<Vec<Initcall>> {
    let mut values = values
        .iter()
        .map(|value| Initcall::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_runtime_registration_surfaces(
    values: &[String],
) -> Result<Vec<RuntimeRegistrationSurface>> {
    let mut values = values
        .iter()
        .map(|value| RuntimeRegistrationSurface::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_documentation_paths(values: &[String]) -> Result<Vec<DocumentationPath>> {
    let mut values = values
        .iter()
        .map(|value| DocumentationPath::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_tool_paths(values: &[String]) -> Result<Vec<ToolPath>> {
    let mut values = values
        .iter()
        .map(|value| ToolPath::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_sample_paths(values: &[String]) -> Result<Vec<SamplePath>> {
    let mut values = values
        .iter()
        .map(|value| SamplePath::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_kunit_suites(values: &[String]) -> Result<Vec<KunitSuite>> {
    let mut values = values
        .iter()
        .map(|value| KunitSuite::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_kselftest_targets(values: &[String]) -> Result<Vec<KselftestTarget>> {
    let mut values = values
        .iter()
        .map(|value| KselftestTarget::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn sorted_arch_names(values: &[String]) -> Result<Vec<ArchName>> {
    let mut values = values
        .iter()
        .map(|value| ArchName::new(value.as_str()))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn join_feature_roots(values: &[FeatureRoot]) -> String {
    values
        .iter()
        .map(|root| root.as_path().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_relative_kernel_paths(values: &[RelativeKernelPath]) -> String {
    values
        .iter()
        .map(|path| path.as_path().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_kconfig_symbols(values: &[KconfigSymbol]) -> String {
    values
        .iter()
        .map(|symbol| symbol.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_exported_symbols(values: &[ExportedSymbol]) -> String {
    values
        .iter()
        .map(|symbol| symbol.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_module_names(values: &[ModuleName]) -> String {
    values
        .iter()
        .map(|module| module.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_module_aliases(values: &[ModuleAlias]) -> String {
    values
        .iter()
        .map(|alias| alias.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_device_compatibles(values: &[DeviceCompatible]) -> String {
    values
        .iter()
        .map(|compatible| compatible.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_acpi_ids(values: &[AcpiId]) -> String {
    values
        .iter()
        .map(|id| id.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_pci_ids(values: &[PciId]) -> String {
    values
        .iter()
        .map(|id| id.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_usb_ids(values: &[UsbId]) -> String {
    values
        .iter()
        .map(|id| id.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_firmware_paths(values: &[FirmwarePath]) -> String {
    values
        .iter()
        .map(|path| path.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_initcalls(values: &[Initcall]) -> String {
    values
        .iter()
        .map(|initcall| initcall.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_runtime_registration_surfaces(values: &[RuntimeRegistrationSurface]) -> String {
    values
        .iter()
        .map(|surface| surface.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_documentation_paths(values: &[DocumentationPath]) -> String {
    values
        .iter()
        .map(|path| path.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_tool_paths(values: &[ToolPath]) -> String {
    values
        .iter()
        .map(|path| path.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_sample_paths(values: &[SamplePath]) -> String {
    values
        .iter()
        .map(|path| path.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_kunit_suites(values: &[KunitSuite]) -> String {
    values
        .iter()
        .map(|suite| suite.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_kselftest_targets(values: &[KselftestTarget]) -> String {
    values
        .iter()
        .map(|target| target.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn join_arch_names(values: &[ArchName]) -> String {
    values
        .iter()
        .map(|arch| arch.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

#[cfg(test)]
mod tests;
