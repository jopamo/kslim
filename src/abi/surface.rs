//! ABI-facing surface classification.
//!
//! The reducer treats these as policy-sensitive user/kernel or tooling
//! contracts. This module classifies names and paths only; it does not approve
//! removal.

use std::path::Path;

use crate::model::UapiPath;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AbiSurfaceKind {
    PublicHeader,
    UapiHeader,
    HeadersInstallTarget,
    SyscallInterface,
    IoctlInterface,
    SysfsInterface,
    ProcfsInterface,
    DebugfsInterface,
    NetlinkInterface,
    TracepointInterface,
}

#[allow(dead_code)]
impl AbiSurfaceKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::PublicHeader => "public_header",
            Self::UapiHeader => "uapi_header",
            Self::HeadersInstallTarget => "headers_install_target",
            Self::SyscallInterface => "syscall_interface",
            Self::IoctlInterface => "ioctl_interface",
            Self::SysfsInterface => "sysfs_interface",
            Self::ProcfsInterface => "procfs_interface",
            Self::DebugfsInterface => "debugfs_interface",
            Self::NetlinkInterface => "netlink_interface",
            Self::TracepointInterface => "tracepoint_interface",
        }
    }
}

pub(crate) fn classify_abi_header_path(path: &Path) -> Option<AbiSurfaceKind> {
    if !has_header_extension(path) {
        return None;
    }
    if UapiPath::matches_path(path) {
        return Some(AbiSurfaceKind::UapiHeader);
    }
    if path.starts_with("include/linux") || path.starts_with("include/net") {
        return Some(AbiSurfaceKind::PublicHeader);
    }
    None
}

#[allow(dead_code)]
pub(crate) fn is_headers_install_target(target: &str) -> bool {
    target == "headers_install"
}

pub(crate) fn has_header_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext == "h")
}
