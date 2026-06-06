//! Shared metadata/report/result value models.
//!
//! This facade keeps the historic `crate::model::*` API while domain-specific
//! model modules own validation and data shapes.

mod content_paths;
mod hardware;
mod identity;
mod kernel;
mod module_identity;
mod report;
mod runtime;
mod test_targets;
mod validation;

pub use content_paths::{DocumentationPath, SamplePath, ToolPath};
pub use hardware::{AcpiId, DeviceCompatible, FirmwarePath, PciId, UsbId};
pub use identity::{
    GitCommitId, MetadataFingerprint, MetadataSchemaVersion, OutputBranchName, PlanFingerprint,
    SnapshotId, ToolVersion, TreeFingerprint, CURRENT_METADATA_SCHEMA_VERSION,
};
pub use kernel::{
    ArchName, GeneratedArtifactPath, HeaderPath, KbuildObject, KconfigSymbol, SourceFilePath,
    UapiPath,
};
pub use module_identity::{ModuleAlias, ModuleName};
pub(crate) use report::ReportPath;
pub use report::{ReducerReportSummary, SelftestReportSummary};
pub use runtime::{ExportedSymbol, Initcall, RuntimeRegistrationSurface};
pub use test_targets::{KselftestTarget, KunitSuite};

#[cfg(test)]
mod tests;
