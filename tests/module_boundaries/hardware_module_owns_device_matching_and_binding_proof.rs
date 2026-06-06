use super::common::*;

#[test]
fn hardware_module_owns_device_matching_and_binding_proof() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let hardware_mod = production_source(&root.join("src/hardware/mod.rs"));
    let devicetree = production_source(&root.join("src/hardware/devicetree.rs"));
    let matching = production_source(&root.join("src/hardware/matching.rs"));
    let device_bindings_facade = production_source(&root.join("src/device_bindings.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        main.contains("mod hardware;") && main.contains("mod device_bindings;"),
        "main.rs should register hardware ownership and legacy device_bindings facade modules"
    );

    for required in [
        "//! Hardware identity matching and devicetree proof gates.",
        "mod devicetree;",
        "mod matching;",
        "pub(crate) use devicetree::{",
        "prove_removed_device_bindings_have_no_live_references",
        "DeviceBindingRemovalProof",
        "pub(crate) use matching::{HardwareMatchKind, HardwareMatchSubject, PlatformMatchName}",
    ] {
        assert!(
            hardware_mod.contains(required),
            "src/hardware/mod.rs should own hardware module declaration/export item {required}"
        );
    }

    for required in [
        "pub(crate) struct DeviceBindingRemovalProof",
        "pub(crate) fn prove_removed_device_bindings_have_no_live_references",
        "const BINDING_ROOT: &str = \"Documentation/devicetree/bindings\"",
        "fn is_device_binding_file(path: &Path) -> bool",
        "fn is_dts_source_path(path: &Path) -> bool",
        "fn compatible_strings_in_content(content: &str) -> BTreeSet<DeviceCompatible>",
        "fn schema_reference_tokens(binding: &Path) -> Vec<String>",
    ] {
        assert!(
            devicetree.contains(required),
            "src/hardware/devicetree.rs should own devicetree binding proof item {required}"
        );
    }

    for required in [
        "pub(crate) enum HardwareMatchKind",
        "DevicetreeCompatible",
        "Modalias",
        "FirmwarePath",
        "PciId",
        "UsbId",
        "AcpiId",
        "Platform",
        "pub(crate) struct PlatformMatchName",
        "pub(crate) enum HardwareMatchSubject",
        "pub(crate) fn stable_key(&self) -> String",
        "devicetree_compatible",
        "modalias",
        "firmware_path",
        "pci_id",
        "usb_id",
        "acpi_id",
        "platform",
    ] {
        assert!(
            matching.contains(required),
            "src/hardware/matching.rs should own hardware matching item {required}"
        );
    }

    assert!(
        device_bindings_facade.contains("pub(crate) use crate::hardware::{")
            && !device_bindings_facade.contains("struct RemovedDeviceBinding"),
        "src/device_bindings.rs should be a compatibility facade over src/hardware"
    );

    assert!(
        architecture.contains("`hardware/*`")
            && architecture.contains("Devicetree binding proof")
            && architecture.contains("modalias/module alias matching")
            && architecture.contains("firmware path matching")
            && architecture.contains("PCI, USB, ACPI, and platform matching")
            && architecture.contains("`device_bindings.rs` is only the compatibility facade"),
        "docs/architecture.md should document hardware ownership and device_bindings facade"
    );
}
