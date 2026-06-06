use super::common::*;

#[test]
fn model_module_owns_shared_value_models() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let model = production_source(&root.join("src/model/mod.rs"));
    let generate_plan = plan_source(root);
    let generate_state = state_source(root);
    let verify = production_source(&root.join("src/generate/verify.rs"));
    let output_metadata = production_source(&root.join("src/output_repo/metadata.rs"));
    let abi_policy = production_source(&root.join("src/abi/policy.rs"));
    let exported_symbols = production_source(&root.join("src/exported_symbols.rs"));
    let device_bindings = production_source(&root.join("src/hardware/devicetree.rs"));
    let content_paths = production_source(&root.join("src/model/content_paths.rs"));
    let hardware = production_source(&root.join("src/model/hardware.rs"));
    let identity = production_source(&root.join("src/model/identity.rs"));
    let kernel = production_source(&root.join("src/model/kernel.rs"));
    let module_identity = production_source(&root.join("src/model/module_identity.rs"));
    let report = production_source(&root.join("src/model/report.rs"));
    let runtime = production_source(&root.join("src/model/runtime.rs"));
    let test_targets = production_source(&root.join("src/model/test_targets.rs"));
    let validation = production_source(&root.join("src/model/validation.rs"));

    assert!(
        main.contains("mod model;"),
        "main.rs should register the shared model module"
    );
    for module in [
        "mod content_paths;",
        "mod hardware;",
        "mod identity;",
        "mod kernel;",
        "mod module_identity;",
        "mod report;",
        "mod runtime;",
        "mod test_targets;",
        "mod validation;",
    ] {
        assert!(
            model.contains(module),
            "src/model/mod.rs should be a facade that registers domain model module {module}"
        );
    }

    let module_models = [
        (
            "src/model/report.rs",
            report.as_str(),
            &[
                "pub(crate) struct ReportPath",
                "pub struct ReducerReportSummary",
                "pub struct SelftestReportSummary",
            ][..],
        ),
        (
            "src/model/identity.rs",
            identity.as_str(),
            &[
                "pub struct MetadataSchemaVersion",
                "CURRENT_METADATA_SCHEMA_VERSION",
                "pub struct PlanFingerprint",
                "pub struct TreeFingerprint",
                "pub struct MetadataFingerprint",
                "pub struct SnapshotId",
                "pub struct GitCommitId",
                "pub struct OutputBranchName",
                "pub struct ToolVersion",
            ][..],
        ),
        (
            "src/model/kernel.rs",
            kernel.as_str(),
            &[
                "pub struct ArchName",
                "pub struct KconfigSymbol",
                "pub struct KbuildObject",
                "pub struct SourceFilePath",
                "pub struct HeaderPath",
                "pub struct UapiPath",
                "pub struct GeneratedArtifactPath",
            ][..],
        ),
        (
            "src/model/content_paths.rs",
            content_paths.as_str(),
            &[
                "pub struct DocumentationPath",
                "pub struct ToolPath",
                "pub struct SamplePath",
            ][..],
        ),
        (
            "src/model/test_targets.rs",
            test_targets.as_str(),
            &["pub struct KunitSuite", "pub struct KselftestTarget"][..],
        ),
        (
            "src/model/runtime.rs",
            runtime.as_str(),
            &[
                "pub struct ExportedSymbol",
                "pub struct Initcall",
                "pub struct RuntimeRegistrationSurface",
            ][..],
        ),
        (
            "src/model/module_identity.rs",
            module_identity.as_str(),
            &["pub struct ModuleName", "pub struct ModuleAlias"][..],
        ),
        (
            "src/model/hardware.rs",
            hardware.as_str(),
            &[
                "pub struct DeviceCompatible",
                "pub struct AcpiId",
                "pub struct PciId",
                "pub struct UsbId",
                "pub struct FirmwarePath",
            ][..],
        ),
    ];
    for (module_name, source, required_models) in module_models {
        for required in required_models {
            assert!(
                source.contains(required),
                "{module_name} should own domain value model {required}"
            );
            assert!(
                model.contains(required.trim_start_matches("pub(crate) struct ").trim_start_matches("pub struct "))
                    || *required == "CURRENT_METADATA_SCHEMA_VERSION",
                "src/model/mod.rs should re-export domain value model {required}"
            );
        }
    }

    for forbidden in [
        "pub struct MetadataSchemaVersion",
        "pub struct KconfigSymbol",
        "pub struct KbuildObject",
        "pub struct ExportedSymbol",
        "pub struct DeviceCompatible",
        "pub struct ReducerReportSummary",
    ] {
        assert!(
            !model.contains(forbidden),
            "src/model/mod.rs should remain a facade and not define model item {forbidden}"
        );
    }
    for required in [
        "pub(super) fn non_empty_model_value",
        "pub(super) fn normalized_relative_model_path_parts",
        "pub(super) fn is_c_identifier",
    ] {
        assert!(
            validation.contains(required),
            "src/model/validation.rs should own shared validation helper {required}"
        );
    }

    for source in [
        ("plan/mod.rs", generate_plan.as_str()),
        ("generate/state.rs", generate_state.as_str()),
        ("generate/verify.rs", verify.as_str()),
        ("output_repo/metadata.rs", output_metadata.as_str()),
        ("exported_symbols.rs", exported_symbols.as_str()),
        ("hardware/devicetree.rs", device_bindings.as_str()),
    ] {
        for duplicated in [
            "pub(crate) struct ToolVersion",
            "pub struct TreeFingerprint",
            "pub struct MetadataFingerprint",
            "pub struct GitCommitId",
            "pub struct OutputBranchName",
            "pub struct ArchName",
            "pub struct KconfigSymbol",
            "pub struct KbuildObject",
            "pub struct SourceFilePath",
            "pub struct HeaderPath",
            "pub struct UapiPath",
            "pub struct GeneratedArtifactPath",
            "pub struct DocumentationPath",
            "pub struct ToolPath",
            "pub struct SamplePath",
            "pub struct KunitSuite",
            "pub struct KselftestTarget",
            "pub struct ExportedSymbol",
            "pub struct Initcall",
            "pub struct RuntimeRegistrationSurface",
            "pub struct ModuleName",
            "pub struct ModuleAlias",
            "pub struct DeviceCompatible",
            "pub struct AcpiId",
            "pub struct PciId",
            "pub struct UsbId",
            "pub struct FirmwarePath",
            "pub struct ReducerReportSummary",
            "pub struct SelftestReportSummary",
        ] {
            assert!(
                !source.1.contains(duplicated),
                "{} must not redefine shared model item {duplicated}",
                source.0
            );
        }
    }

    assert!(
        generate_state.contains("pub(crate) remove_configs: Vec<KconfigSymbol>"),
        "generate/state.rs should store resolved removal config symbols as KconfigSymbol"
    );
    assert!(
        generate_state.contains("pub(crate) set_defaults: BTreeMap<KconfigSymbol, String>"),
        "generate/state.rs should store resolved default override symbols as KconfigSymbol"
    );

    let removal_manifest_model = production_source(&root.join("src/removal_manifest/model.rs"));
    assert!(
        !removal_manifest_model.contains("pub type KconfigSymbol = String"),
        "removal_manifest/model.rs must not redefine KconfigSymbol as a raw string alias"
    );
    assert!(
        !removal_manifest_model.contains("pub type KbuildObject = String"),
        "removal_manifest/model.rs must not redefine KbuildObject as a raw string alias"
    );
    assert!(
        !removal_manifest_model.contains("pub type SourceFilePath ="),
        "removal_manifest/model.rs must not redefine SourceFilePath"
    );
    assert!(
        !removal_manifest_model.contains("pub type HeaderPath = String"),
        "removal_manifest/model.rs must not redefine HeaderPath as a raw string alias"
    );
    assert!(
        !removal_manifest_model.contains("pub type UapiPath ="),
        "removal_manifest/model.rs must not redefine UapiPath"
    );
    assert!(
        !removal_manifest_model.contains("pub type GeneratedArtifactPath ="),
        "removal_manifest/model.rs must not redefine GeneratedArtifactPath"
    );
    assert!(
        removal_manifest_model.contains("pub removed_headers: BTreeSet<HeaderPath>"),
        "removal_manifest/model.rs should store removed headers as HeaderPath"
    );
    assert!(
        removal_manifest_model.contains("pub removed_kbuild_objects: BTreeSet<KbuildObject>"),
        "removal_manifest/model.rs should store removed kbuild objects as KbuildObject"
    );
    assert!(
        abi_policy.contains("UapiPath::matches_path") && abi_policy.contains("UapiPath::new"),
        "src/abi/policy.rs should use UapiPath as the UAPI path boundary"
    );
    assert!(
        exported_symbols.contains("use crate::model::ExportedSymbol;")
            && exported_symbols.contains("pub symbol: ExportedSymbol")
            && exported_symbols.contains("ExportedSymbol::new"),
        "exported_symbols.rs should publish exported symbol proof with ExportedSymbol"
    );
    assert!(
        device_bindings.contains("use crate::model::DeviceCompatible;")
            && device_bindings.contains("pub compatible_strings: Vec<DeviceCompatible>")
            && device_bindings.contains("DeviceCompatible::new"),
        "src/hardware/devicetree.rs should publish compatible-string proof with DeviceCompatible"
    );
}
