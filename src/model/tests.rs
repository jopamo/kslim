use super::*;
use std::path::{Path, PathBuf};

#[test]
fn test_report_path_rejects_empty_paths() {
    let err = ReportPath::new(PathBuf::new()).unwrap_err().to_string();
    assert!(err.contains("failure report path is empty"));

    let err = ReportPath::new(" ").unwrap_err().to_string();
    assert!(err.contains("failure report path is empty"));
}

#[test]
fn test_arch_name_accepts_kernel_arch_identifiers() {
    assert_eq!(ArchName::new("x86").unwrap().as_str(), "x86");
    assert_eq!(ArchName::new("arm64").unwrap().as_str(), "arm64");
    assert_eq!(ArchName::new("x86_64").unwrap().as_str(), "x86_64");
}

#[test]
fn test_arch_name_rejects_empty_or_path_like_values() {
    let err = ArchName::new(" ").unwrap_err().to_string();
    assert!(err.contains("kernel architecture name must not be empty"));

    let err = ArchName::new("x86/../../host").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));

    let err = ArchName::new("x86 host").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));
}

#[test]
fn test_kconfig_symbol_accepts_kernel_identifiers() {
    assert_eq!(
        KconfigSymbol::new("DRM_AMDGPU").unwrap().as_str(),
        "DRM_AMDGPU"
    );
    assert_eq!(
        KconfigSymbol::new("CONFIG_FOO").unwrap().as_str(),
        "CONFIG_FOO"
    );
    assert_eq!(KconfigSymbol::new("64BIT").unwrap().as_str(), "64BIT");
}

#[test]
fn test_kconfig_symbol_rejects_empty_or_expression_like_values() {
    let err = KconfigSymbol::new("").unwrap_err().to_string();
    assert!(err.contains("Kconfig symbol must not be empty"));

    let err = KconfigSymbol::new("DRM_AMDGPU || DRM_RADEON")
        .unwrap_err()
        .to_string();
    assert!(err.contains("invalid characters"));

    let err = KconfigSymbol::new("drivers/foo").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));
}

#[test]
fn test_exported_symbol_accepts_c_identifiers() {
    assert_eq!(ExportedSymbol::new("foo_api").unwrap().as_str(), "foo_api");
    assert_eq!(
        ExportedSymbol::new("__tracepoint_sched_switch")
            .unwrap()
            .as_str(),
        "__tracepoint_sched_switch"
    );
    assert_eq!(ExportedSymbol::new("$global$").unwrap().as_str(), "$global$");
    assert_eq!(ExportedSymbol::new("$$divI").unwrap().as_str(), "$$divI");
}

#[test]
fn test_exported_symbol_rejects_empty_or_non_c_identifiers() {
    let err = ExportedSymbol::new("").unwrap_err().to_string();
    assert!(err.contains("exported symbol must not be empty"));

    let err = ExportedSymbol::new("1foo").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));

    let err = ExportedSymbol::new("foo-api").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));

    let err = ExportedSymbol::new("foo api").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));
}

#[test]
fn test_initcall_accepts_c_identifiers() {
    assert_eq!(Initcall::new("bt_init").unwrap().as_str(), "bt_init");
    assert_eq!(
        Initcall::new("__initcall_foo").unwrap().as_str(),
        "__initcall_foo"
    );
}

#[test]
fn test_initcall_rejects_empty_or_non_c_identifiers() {
    let err = Initcall::new("").unwrap_err().to_string();
    assert!(err.contains("initcall must not be empty"));

    let err = Initcall::new("1foo").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));

    let err = Initcall::new("foo-init").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));

    let err = Initcall::new("foo init").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));
}

#[test]
fn test_runtime_registration_surface_accepts_known_macro_entry_point_pairs() {
    let surface = RuntimeRegistrationSurface::new("module_platform_driver:btusb_driver").unwrap();

    assert_eq!(surface.as_str(), "module_platform_driver:btusb_driver");
    assert_eq!(surface.registration_macro(), "module_platform_driver");
    assert_eq!(surface.entry_point(), "btusb_driver");
}

#[test]
fn test_runtime_registration_surface_rejects_invalid_values() {
    let err = RuntimeRegistrationSurface::new("").unwrap_err().to_string();
    assert!(err.contains("runtime registration surface must not be empty"));

    let err = RuntimeRegistrationSurface::new("module_init")
        .unwrap_err()
        .to_string();
    assert!(err.contains("registration_macro:entry_point"));

    let err = RuntimeRegistrationSurface::new("module-init:bt_init")
        .unwrap_err()
        .to_string();
    assert!(err.contains("runtime registration macro contains invalid characters"));

    let err = RuntimeRegistrationSurface::new("unknown_register:bt_init")
        .unwrap_err()
        .to_string();
    assert!(err.contains("unsupported runtime registration macro"));

    let err = RuntimeRegistrationSurface::new("module_init:1bad")
        .unwrap_err()
        .to_string();
    assert!(err.contains("runtime registration entry point contains invalid characters"));
}

#[test]
fn test_module_name_accepts_kernel_module_identifiers() {
    assert_eq!(ModuleName::new("amdgpu").unwrap().as_str(), "amdgpu");
    assert_eq!(ModuleName::new("8021q").unwrap().as_str(), "8021q");
    assert_eq!(
        ModuleName::new("snd-hda-intel").unwrap().as_str(),
        "snd_hda_intel"
    );
}

#[test]
fn test_module_name_rejects_paths_suffixes_or_invalid_values() {
    let err = ModuleName::new("").unwrap_err().to_string();
    assert!(err.contains("module name must not be empty"));

    let err = ModuleName::new("foo.ko").unwrap_err().to_string();
    assert!(err.contains("must omit .ko suffix"));

    let err = ModuleName::new("drivers/foo").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));

    let err = ModuleName::new("-foo").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));

    let err = ModuleName::new("foo module").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));
}

#[test]
fn test_module_alias_accepts_kernel_alias_patterns() {
    assert_eq!(
        ModuleAlias::new("pci:v00008086d00001572sv*sd*bc*sc*i*")
            .unwrap()
            .as_str(),
        "pci:v00008086d00001572sv*sd*bc*sc*i*"
    );
    assert_eq!(
        ModuleAlias::new("usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*")
            .unwrap()
            .as_str(),
        "usb:v*p*d*dc*dsc*dp*ic*isc*ip*in*"
    );
    assert_eq!(
        ModuleAlias::new("of:N*T*Cqcom,ipq8064").unwrap().as_str(),
        "of:N*T*Cqcom,ipq8064"
    );
}

#[test]
fn test_module_alias_rejects_empty_file_names_or_invalid_values() {
    let err = ModuleAlias::new("").unwrap_err().to_string();
    assert!(err.contains("module alias must not be empty"));

    let err = ModuleAlias::new("btusb.ko").unwrap_err().to_string();
    assert!(err.contains("must not be a module file name"));

    let err = ModuleAlias::new("pci alias").unwrap_err().to_string();
    assert!(err.contains("must not contain whitespace"));

    let err = ModuleAlias::new("alias\"quoted").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));
}

#[test]
fn test_device_compatible_accepts_vendor_device_tokens() {
    assert_eq!(
        DeviceCompatible::new("vendor,foo").unwrap().as_str(),
        "vendor,foo"
    );
    assert_eq!(
        DeviceCompatible::new("brcm,bcm2835-aux-uart")
            .unwrap()
            .as_str(),
        "brcm,bcm2835-aux-uart"
    );
    assert_eq!(
        DeviceCompatible::new("qcom,ipq8064/ap148")
            .unwrap()
            .as_str(),
        "qcom,ipq8064/ap148"
    );
}

#[test]
fn test_device_compatible_rejects_empty_generic_or_invalid_tokens() {
    let err = DeviceCompatible::new("").unwrap_err().to_string();
    assert!(err.contains("device compatible must not be empty"));

    let err = DeviceCompatible::new("simple-bus").unwrap_err().to_string();
    assert!(err.contains("vendor,device form"));

    let err = DeviceCompatible::new(",foo").unwrap_err().to_string();
    assert!(err.contains("nonempty vendor and device"));

    let err = DeviceCompatible::new("vendor,").unwrap_err().to_string();
    assert!(err.contains("nonempty vendor and device"));

    let err = DeviceCompatible::new("vendor,foo,bar")
        .unwrap_err()
        .to_string();
    assert!(err.contains("vendor,device form"));

    let err = DeviceCompatible::new("vendor foo,bar")
        .unwrap_err()
        .to_string();
    assert!(err.contains("invalid characters"));
}

#[test]
fn test_acpi_id_accepts_kernel_acpi_identifiers() {
    assert_eq!(AcpiId::new("PNP0C09").unwrap().as_str(), "PNP0C09");
    assert_eq!(AcpiId::new("ACPI0003").unwrap().as_str(), "ACPI0003");
    assert_eq!(AcpiId::new("INT33A1").unwrap().as_str(), "INT33A1");
    assert_eq!(AcpiId::new("PRP0001").unwrap().as_str(), "PRP0001");
}

#[test]
fn test_acpi_id_rejects_empty_lowercase_long_or_invalid_values() {
    let err = AcpiId::new("").unwrap_err().to_string();
    assert!(err.contains("ACPI ID must not be empty"));

    let err = AcpiId::new("pnp0c09").unwrap_err().to_string();
    assert!(err.contains("uppercase ASCII"));

    let err = AcpiId::new("PNP").unwrap_err().to_string();
    assert!(err.contains("4 to 16"));

    let err = AcpiId::new("ABCDEFGHIJKLMNOPQ").unwrap_err().to_string();
    assert!(err.contains("4 to 16"));

    let err = AcpiId::new("PNP0C09*").unwrap_err().to_string();
    assert!(err.contains("uppercase ASCII"));
}

#[test]
fn test_pci_id_accepts_kernel_pci_identifiers() {
    assert_eq!(PciId::new("8086:1572").unwrap().as_str(), "8086:1572");
    assert_eq!(PciId::new("10EC:8168").unwrap().as_str(), "10EC:8168");
    assert_eq!(PciId::new("1AF4:1000").unwrap().as_str(), "1AF4:1000");
    assert_eq!(PciId::new("14E4:43A0").unwrap().as_str(), "14E4:43A0");
}

#[test]
fn test_pci_id_rejects_empty_lowercase_wrong_form_or_invalid_values() {
    let err = PciId::new("").unwrap_err().to_string();
    assert!(err.contains("PCI ID must not be empty"));

    let err = PciId::new("10ec:8168").unwrap_err().to_string();
    assert!(err.contains("uppercase hexadecimal"));

    let err = PciId::new("8086").unwrap_err().to_string();
    assert!(err.contains("VVVV:DDDD"));

    let err = PciId::new("8086:1572:0001").unwrap_err().to_string();
    assert!(err.contains("VVVV:DDDD"));

    let err = PciId::new("8086:157G").unwrap_err().to_string();
    assert!(err.contains("uppercase hexadecimal"));
}

#[test]
fn test_usb_id_accepts_kernel_usb_identifiers() {
    assert_eq!(UsbId::new("0BDA:8153").unwrap().as_str(), "0BDA:8153");
    assert_eq!(UsbId::new("046D:C52B").unwrap().as_str(), "046D:C52B");
    assert_eq!(UsbId::new("1D6B:0002").unwrap().as_str(), "1D6B:0002");
    assert_eq!(UsbId::new("8087:0024").unwrap().as_str(), "8087:0024");
}

#[test]
fn test_usb_id_rejects_empty_lowercase_wrong_form_or_invalid_values() {
    let err = UsbId::new("").unwrap_err().to_string();
    assert!(err.contains("USB ID must not be empty"));

    let err = UsbId::new("0bda:8153").unwrap_err().to_string();
    assert!(err.contains("uppercase hexadecimal"));

    let err = UsbId::new("0BDA").unwrap_err().to_string();
    assert!(err.contains("VVVV:PPPP"));

    let err = UsbId::new("0BDA:8153:0001").unwrap_err().to_string();
    assert!(err.contains("VVVV:PPPP"));

    let err = UsbId::new("0BDA:815G").unwrap_err().to_string();
    assert!(err.contains("uppercase hexadecimal"));
}

#[test]
fn test_firmware_path_accepts_relative_firmware_names() {
    assert_eq!(
        FirmwarePath::new("amdgpu/polaris10_mc.bin")
            .unwrap()
            .as_str(),
        "amdgpu/polaris10_mc.bin"
    );
    assert_eq!(
        FirmwarePath::new("./iwlwifi-7260-17.ucode")
            .unwrap()
            .as_str(),
        "iwlwifi-7260-17.ucode"
    );
    assert_eq!(
        FirmwarePath::new("qcom//venus-5.2/venus.mbn")
            .unwrap()
            .as_path(),
        Path::new("qcom/venus-5.2/venus.mbn")
    );
}

#[test]
fn test_firmware_path_rejects_empty_absolute_traversal_or_unsafe_names() {
    let err = FirmwarePath::new("").unwrap_err().to_string();
    assert!(err.contains("firmware path must not be empty"));

    let err = FirmwarePath::new("/lib/firmware/amdgpu/foo.bin")
        .unwrap_err()
        .to_string();
    assert!(err.contains("relative to the firmware search path"));

    let err = FirmwarePath::new("C:/firmware/amdgpu/foo.bin")
        .unwrap_err()
        .to_string();
    assert!(err.contains("relative to the firmware search path"));

    let err = FirmwarePath::new("../firmware/foo.bin")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must not contain '..'"));

    let err = FirmwarePath::new(r"firmware\..\foo.bin")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must not contain '..'"));

    let err = FirmwarePath::new(".").unwrap_err().to_string();
    assert!(err.contains("must not resolve to the firmware search path root"));

    let err = FirmwarePath::new("foo firmware.bin")
        .unwrap_err()
        .to_string();
    assert!(err.contains("contains whitespace"));

    let err = FirmwarePath::new("foo\\bar.bin").unwrap_err().to_string();
    assert!(err.contains("invalid separator"));
}

#[test]
fn test_source_file_path_accepts_kernel_source_files() {
    assert_eq!(
        SourceFilePath::new("drivers/foo/remove.c")
            .unwrap()
            .as_str(),
        "drivers/foo/remove.c"
    );
    assert_eq!(
        SourceFilePath::new("./arch/x86//kernel/head_64.S")
            .unwrap()
            .as_path(),
        Path::new("arch/x86/kernel/head_64.S")
    );
    assert_eq!(
        SourceFilePath::new("rust/kernel/lib.rs").unwrap().as_str(),
        "rust/kernel/lib.rs"
    );
    assert!(SourceFilePath::matches_path(Path::new(
        "drivers/foo/remove.c"
    )));
}

#[test]
fn test_source_file_path_rejects_invalid_paths() {
    let err = SourceFilePath::new("").unwrap_err().to_string();
    assert!(err.contains("source file path must not be empty"));

    let err = SourceFilePath::new("/drivers/foo/remove.c")
        .unwrap_err()
        .to_string();
    assert!(err.contains("relative to the kernel tree"));

    let err = SourceFilePath::new("C:/drivers/foo/remove.c")
        .unwrap_err()
        .to_string();
    assert!(err.contains("relative to the kernel tree"));

    let err = SourceFilePath::new("drivers/../remove.c")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must not contain '..'"));

    let err = SourceFilePath::new(r"drivers\..\remove.c")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must not contain '..'"));

    let err = SourceFilePath::new("drivers/foo/remove.h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must end with .c, .S, or .rs"));

    let err = SourceFilePath::new("drivers/foo/remove source.c")
        .unwrap_err()
        .to_string();
    assert!(err.contains("contains whitespace"));

    let err = SourceFilePath::new("$(obj)/remove.c")
        .unwrap_err()
        .to_string();
    assert!(err.contains("unsupported syntax"));

    assert!(!SourceFilePath::matches_path(Path::new(
        "drivers/foo/remove.h"
    )));
}

#[test]
fn test_kbuild_object_accepts_object_and_directory_refs() {
    assert_eq!(
        KbuildObject::new("drivers/foo/remove.o").unwrap().as_str(),
        "drivers/foo/remove.o"
    );
    assert_eq!(
        KbuildObject::new("./drivers//foo/remove.o")
            .unwrap()
            .as_str(),
        "drivers/foo/remove.o"
    );
    let directory = KbuildObject::new("drivers/foo/remove/").unwrap();
    assert_eq!(directory.as_str(), "drivers/foo/remove/");
    assert!(directory.is_directory_ref());
}

#[test]
fn test_kbuild_object_rejects_invalid_paths_or_make_syntax() {
    let err = KbuildObject::new(" ").unwrap_err().to_string();
    assert!(err.contains("kbuild object must not be empty"));

    let err = KbuildObject::new("/drivers/foo/remove.o")
        .unwrap_err()
        .to_string();
    assert!(err.contains("relative to the kernel tree"));

    let err = KbuildObject::new("C:/drivers/foo/remove.o")
        .unwrap_err()
        .to_string();
    assert!(err.contains("relative to the kernel tree"));

    let err = KbuildObject::new("drivers/../remove.o")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must not contain '..'"));

    let err = KbuildObject::new(r"drivers\..\remove.o")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must not contain '..'"));

    let err = KbuildObject::new("drivers/foo/remove.c")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must end with .o or /"));

    let err = KbuildObject::new("$(obj)/remove.o")
        .unwrap_err()
        .to_string();
    assert!(err.contains("unsupported make syntax"));
}

#[test]
fn test_header_path_accepts_kernel_header_paths() {
    assert_eq!(
        HeaderPath::new("include/linux/foo.h").unwrap().as_str(),
        "include/linux/foo.h"
    );
    assert_eq!(
        HeaderPath::new("./drivers//foo/private.h")
            .unwrap()
            .as_str(),
        "drivers/foo/private.h"
    );
    assert_eq!(
        HeaderPath::new("arch/x86/include/generated/asm/offsets.h")
            .unwrap()
            .as_path(),
        Path::new("arch/x86/include/generated/asm/offsets.h")
    );
}

#[test]
fn test_header_path_rejects_invalid_paths() {
    let err = HeaderPath::new("").unwrap_err().to_string();
    assert!(err.contains("header path must not be empty"));

    let err = HeaderPath::new("/include/linux/foo.h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("relative to the kernel tree"));

    let err = HeaderPath::new("C:/include/linux/foo.h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("relative to the kernel tree"));

    let err = HeaderPath::new("include/../linux/foo.h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must not contain '..'"));

    let err = HeaderPath::new(r"include\..\linux/foo.h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must not contain '..'"));

    let err = HeaderPath::new("include/linux/foo.c")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must end with .h"));

    let err = HeaderPath::new("include/linux/foo header.h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("contains whitespace"));
}

#[test]
fn test_uapi_path_accepts_kernel_uapi_roots_and_children() {
    assert_eq!(
        UapiPath::new("include/uapi").unwrap().as_str(),
        "include/uapi"
    );
    assert_eq!(
        UapiPath::new("./include//generated/uapi/linux/foo.h")
            .unwrap()
            .as_str(),
        "include/generated/uapi/linux/foo.h"
    );
    assert_eq!(
        UapiPath::new("arch/x86/include/uapi/asm/foo.h")
            .unwrap()
            .as_path(),
        Path::new("arch/x86/include/uapi/asm/foo.h")
    );
    assert!(UapiPath::matches_path(Path::new(
        "arch/arm64/include/generated/uapi"
    )));
}

#[test]
fn test_uapi_path_rejects_non_uapi_or_unsafe_paths() {
    let err = UapiPath::new("").unwrap_err().to_string();
    assert!(err.contains("UAPI path must not be empty"));

    let err = UapiPath::new("/include/uapi/linux/foo.h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("relative to the kernel tree"));

    let err = UapiPath::new("C:/include/uapi/linux/foo.h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("relative to the kernel tree"));

    let err = UapiPath::new("include/uapi/../linux/foo.h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must not contain '..'"));

    let err = UapiPath::new(r"include\uapi\..\linux/foo.h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must not contain '..'"));

    let err = UapiPath::new("include/linux/foo.h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("UAPI path must be under"));

    let err = UapiPath::new("drivers/foo/uapi.h").unwrap_err().to_string();
    assert!(err.contains("UAPI path must be under"));

    assert!(!UapiPath::matches_path(Path::new("include/linux/foo.h")));
}

#[test]
fn test_generated_artifact_path_accepts_kernel_generated_roots_and_children() {
    assert_eq!(
        GeneratedArtifactPath::new("include/generated")
            .unwrap()
            .as_str(),
        "include/generated"
    );
    assert_eq!(
        GeneratedArtifactPath::new("./include//generated/autoconf.h")
            .unwrap()
            .as_str(),
        "include/generated/autoconf.h"
    );
    assert_eq!(
        GeneratedArtifactPath::new("arch/x86/include/generated/asm/offsets.h")
            .unwrap()
            .as_path(),
        Path::new("arch/x86/include/generated/asm/offsets.h")
    );
    assert_eq!(
        GeneratedArtifactPath::new("include/config/auto.conf")
            .unwrap()
            .as_str(),
        "include/config/auto.conf"
    );
    assert_eq!(
        GeneratedArtifactPath::new("modules.order")
            .unwrap()
            .as_str(),
        "modules.order"
    );
    assert!(GeneratedArtifactPath::matches_path(Path::new(
        "include/generated/utsrelease.h"
    )));
}

#[test]
fn test_generated_artifact_path_rejects_non_generated_uapi_or_unsafe_paths() {
    let err = GeneratedArtifactPath::new("").unwrap_err().to_string();
    assert!(err.contains("generated artifact path must not be empty"));

    let err = GeneratedArtifactPath::new("/include/generated/autoconf.h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("relative to the kernel tree"));

    let err = GeneratedArtifactPath::new("C:/include/generated/autoconf.h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("relative to the kernel tree"));

    let err = GeneratedArtifactPath::new("include/generated/../config/auto.conf")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must not contain '..'"));

    let err = GeneratedArtifactPath::new("include/generated/bad artifact.h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("contains whitespace"));

    let err = GeneratedArtifactPath::new("include/generated/uapi/linux/foo.h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("generated artifact path must be under"));

    let err = GeneratedArtifactPath::new("drivers/foo/generated.h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("generated artifact path must be under"));

    let err = GeneratedArtifactPath::new("include/generated/$(obj).h")
        .unwrap_err()
        .to_string();
    assert!(err.contains("unsupported syntax"));

    assert!(!GeneratedArtifactPath::matches_path(Path::new(
        "include/generated/uapi/linux/foo.h"
    )));
}

#[test]
fn test_documentation_path_accepts_documentation_roots_and_children() {
    assert_eq!(
        DocumentationPath::new("Documentation").unwrap().as_str(),
        "Documentation"
    );
    assert_eq!(
        DocumentationPath::new("./Documentation//networking/bluetooth.rst")
            .unwrap()
            .as_str(),
        "Documentation/networking/bluetooth.rst"
    );
    assert_eq!(
        DocumentationPath::new("Documentation/admin-guide")
            .unwrap()
            .as_path(),
        Path::new("Documentation/admin-guide")
    );
    assert!(DocumentationPath::matches_path(Path::new(
        "Documentation/driver-api/usb.rst"
    )));
}

#[test]
fn test_documentation_path_rejects_non_documentation_or_unsafe_paths() {
    let err = DocumentationPath::new("").unwrap_err().to_string();
    assert!(err.contains("documentation path must not be empty"));

    let err = DocumentationPath::new("/Documentation/index.rst")
        .unwrap_err()
        .to_string();
    assert!(err.contains("relative to the kernel tree"));

    let err = DocumentationPath::new("C:/Documentation/index.rst")
        .unwrap_err()
        .to_string();
    assert!(err.contains("relative to the kernel tree"));

    let err = DocumentationPath::new("Documentation/../README")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must not contain '..'"));

    let err = DocumentationPath::new("Documentation/bad doc.rst")
        .unwrap_err()
        .to_string();
    assert!(err.contains("contains whitespace"));

    let err = DocumentationPath::new("drivers/foo/README.rst")
        .unwrap_err()
        .to_string();
    assert!(err.contains("documentation path must be under Documentation"));

    let err = DocumentationPath::new("Documentation/$(obj).rst")
        .unwrap_err()
        .to_string();
    assert!(err.contains("unsupported syntax"));

    assert!(!DocumentationPath::matches_path(Path::new(
        "drivers/foo/README.rst"
    )));
}

#[test]
fn test_tool_path_accepts_tools_roots_and_children() {
    assert_eq!(ToolPath::new("tools").unwrap().as_str(), "tools");
    assert_eq!(
        ToolPath::new("./tools//perf/builtin-stat.c")
            .unwrap()
            .as_str(),
        "tools/perf/builtin-stat.c"
    );
    assert_eq!(
        ToolPath::new("tools/testing/selftests").unwrap().as_path(),
        Path::new("tools/testing/selftests")
    );
    assert!(ToolPath::matches_path(Path::new("tools/objtool/check.c")));
}

#[test]
fn test_tool_path_rejects_non_tools_or_unsafe_paths() {
    let err = ToolPath::new("").unwrap_err().to_string();
    assert!(err.contains("tool path must not be empty"));

    let err = ToolPath::new("/tools/perf").unwrap_err().to_string();
    assert!(err.contains("relative to the kernel tree"));

    let err = ToolPath::new("C:/tools/perf").unwrap_err().to_string();
    assert!(err.contains("relative to the kernel tree"));

    let err = ToolPath::new("tools/../scripts").unwrap_err().to_string();
    assert!(err.contains("must not contain '..'"));

    let err = ToolPath::new("tools/bad tool").unwrap_err().to_string();
    assert!(err.contains("contains whitespace"));

    let err = ToolPath::new("Documentation/tools.rst")
        .unwrap_err()
        .to_string();
    assert!(err.contains("tool path must be under tools"));

    let err = ToolPath::new("tools/$(obj)").unwrap_err().to_string();
    assert!(err.contains("unsupported syntax"));

    assert!(!ToolPath::matches_path(Path::new(
        "Documentation/tools.rst"
    )));
}

#[test]
fn test_sample_path_accepts_samples_roots_and_children() {
    assert_eq!(SamplePath::new("samples").unwrap().as_str(), "samples");
    assert_eq!(
        SamplePath::new("./samples//bpf/xdp1_kern.c")
            .unwrap()
            .as_str(),
        "samples/bpf/xdp1_kern.c"
    );
    assert_eq!(
        SamplePath::new("samples/kobject").unwrap().as_path(),
        Path::new("samples/kobject")
    );
    assert!(SamplePath::matches_path(Path::new(
        "samples/hidraw/hid-example.c"
    )));
}

#[test]
fn test_sample_path_rejects_non_samples_or_unsafe_paths() {
    let err = SamplePath::new("").unwrap_err().to_string();
    assert!(err.contains("sample path must not be empty"));

    let err = SamplePath::new("/samples/bpf").unwrap_err().to_string();
    assert!(err.contains("relative to the kernel tree"));

    let err = SamplePath::new("C:/samples/bpf").unwrap_err().to_string();
    assert!(err.contains("relative to the kernel tree"));

    let err = SamplePath::new("samples/../tools").unwrap_err().to_string();
    assert!(err.contains("must not contain '..'"));

    let err = SamplePath::new("samples/bad sample")
        .unwrap_err()
        .to_string();
    assert!(err.contains("contains whitespace"));

    let err = SamplePath::new("tools/perf").unwrap_err().to_string();
    assert!(err.contains("sample path must be under samples"));

    let err = SamplePath::new("samples/$(obj)").unwrap_err().to_string();
    assert!(err.contains("unsupported syntax"));

    assert!(!SamplePath::matches_path(Path::new("tools/perf")));
}

#[test]
fn test_kunit_suite_accepts_stable_suite_names() {
    assert_eq!(KunitSuite::new("net_test").unwrap().as_str(), "net_test");
    assert_eq!(
        KunitSuite::new("kunit-resource-test").unwrap().as_str(),
        "kunit-resource-test"
    );
    assert_eq!(
        KunitSuite::new("bpf.verifier").unwrap().as_str(),
        "bpf.verifier"
    );
}

#[test]
fn test_kunit_suite_rejects_unsafe_or_ambiguous_names() {
    let err = KunitSuite::new("").unwrap_err().to_string();
    assert!(err.contains("KUnit suite must not be empty"));

    let err = KunitSuite::new("bad suite").unwrap_err().to_string();
    assert!(err.contains("contains whitespace"));

    let err = KunitSuite::new("../bad").unwrap_err().to_string();
    assert!(err.contains("must not contain '..'"));

    let err = KunitSuite::new("suite/*").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));

    let err = KunitSuite::new("suite:case").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));
}

#[test]
fn test_kselftest_target_accepts_stable_target_names() {
    assert_eq!(KselftestTarget::new("net").unwrap().as_str(), "net");
    assert_eq!(KselftestTarget::new("bpf").unwrap().as_str(), "bpf");
    assert_eq!(
        KselftestTarget::new("./drivers//net/bonding")
            .unwrap()
            .as_str(),
        "drivers/net/bonding"
    );
    assert_eq!(
        KselftestTarget::new("tc-testing").unwrap().as_str(),
        "tc-testing"
    );
}

#[test]
fn test_kselftest_target_rejects_unsafe_or_ambiguous_names() {
    let err = KselftestTarget::new("").unwrap_err().to_string();
    assert!(err.contains("kselftest target must not be empty"));

    let err = KselftestTarget::new("/net").unwrap_err().to_string();
    assert!(err.contains("relative to the kselftest target set"));

    let err = KselftestTarget::new("C:/net").unwrap_err().to_string();
    assert!(err.contains("relative to the kselftest target set"));

    let err = KselftestTarget::new("../bad").unwrap_err().to_string();
    assert!(err.contains("must not contain '..'"));

    let err = KselftestTarget::new("bad target").unwrap_err().to_string();
    assert!(err.contains("contains whitespace"));

    let err = KselftestTarget::new("target/*").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));

    let err = KselftestTarget::new("target:case").unwrap_err().to_string();
    assert!(err.contains("invalid characters"));
}
