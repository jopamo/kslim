use super::common::*;

#[test]
fn abi_module_owns_abi_surface_policy() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let abi_mod = production_source(&root.join("src/abi/mod.rs"));
    let abi_policy = production_source(&root.join("src/abi/policy.rs"));
    let abi_surface = production_source(&root.join("src/abi/surface.rs"));
    let abi_facade = production_source(&root.join("src/abi_policy.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        main.contains("mod abi;"),
        "main.rs should register the ABI ownership module"
    );

    for required in [
        "//! ABI-sensitive surface taxonomy and removal policy.",
        "mod policy;",
        "mod surface;",
        "pub(crate) use policy::{",
        "pub use policy::AbiPolicyConfig;",
        "pub(crate) use surface::{",
        "pub use surface::AbiSurfaceKind;",
    ] {
        assert!(
            abi_mod.contains(required),
            "src/abi/mod.rs should declare and expose ABI module item {required}"
        );
    }

    for required in [
        "pub struct AbiPolicyConfig",
        "pub fn is_fail_closed(&self) -> bool",
        "pub(crate) fn validate_declared_removal",
        "pub(crate) fn validate_uapi_removal",
        "pub(crate) fn validate_public_header_removal",
        "explicit ABI policy approval",
        "allow_public_header_removal",
        "allow_uapi_header_removal",
    ] {
        assert!(
            abi_policy.contains(required),
            "src/abi/policy.rs should own fail-closed ABI policy item {required}"
        );
    }

    for required in [
        "pub enum AbiSurfaceKind",
        "PublicHeader",
        "UapiHeader",
        "HeadersInstallTarget",
        "SyscallInterface",
        "IoctlInterface",
        "SysfsInterface",
        "ProcfsInterface",
        "DebugfsInterface",
        "NetlinkInterface",
        "TracepointInterface",
        "headers_install_target",
        "syscall_interface",
        "ioctl_interface",
        "sysfs_interface",
        "procfs_interface",
        "debugfs_interface",
        "netlink_interface",
        "tracepoint_interface",
        "pub(crate) fn classify_abi_header_path",
        "pub(crate) fn is_headers_install_target",
    ] {
        assert!(
            abi_surface.contains(required),
            "src/abi/surface.rs should own ABI surface taxonomy item {required}"
        );
    }

    assert!(
        abi_facade.contains("pub use crate::abi::AbiPolicyConfig;")
            && abi_facade.contains("pub(crate) use crate::abi::{")
            && !abi_facade.contains("pub struct AbiPolicyConfig"),
        "src/abi_policy.rs should be only a compatibility facade over src/abi"
    );

    assert!(
        architecture.contains("`abi/*`")
            && architecture.contains("UAPI/public-header removal policy")
            && architecture.contains("headers_install, syscall, ioctl, sysfs, procfs, debugfs, netlink, and tracepoint")
            && architecture.contains("`abi_policy.rs` is only the compatibility facade"),
        "docs/architecture.md should document ABI module ownership and facade"
    );
}
