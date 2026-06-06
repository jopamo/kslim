//! Fail-closed ABI-sensitive removal policy.
//!
//! Public headers and UAPI surfaces are ABI-facing. Reducer inputs must opt in
//! before exact public-header or UAPI removal is accepted. Surface taxonomy for
//! headers_install, syscall, ioctl, sysfs, procfs, debugfs, netlink, and
//! tracepoint policy lives beside this file in `surface.rs`.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::model::UapiPath;

use super::surface::has_header_extension;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AbiPolicyConfig {
    #[serde(default)]
    pub allow_public_header_removal: bool,
    #[serde(default)]
    pub allow_uapi_header_removal: bool,
}

impl AbiPolicyConfig {
    pub fn is_fail_closed(&self) -> bool {
        !self.allow_public_header_removal && !self.allow_uapi_header_removal
    }
}

pub(crate) fn is_uapi_path(path: &Path) -> bool {
    UapiPath::matches_path(path)
}

pub(crate) fn is_uapi_header_path(path: &Path) -> bool {
    is_uapi_path(path) && has_header_extension(path)
}

pub(crate) fn is_public_header_path(path: &Path) -> bool {
    path.starts_with("include/linux") || is_uapi_path(path) || path.starts_with("include/net")
}

pub(crate) fn allows_public_header_removal(path: &Path, policy: &AbiPolicyConfig) -> bool {
    if !is_public_header_path(path) {
        return true;
    }

    if policy.is_fail_closed() {
        return false;
    }

    if is_uapi_path(path) {
        policy.allow_uapi_header_removal
    } else {
        policy.allow_public_header_removal
    }
}

pub(crate) fn validate_declared_removal(path: &Path, policy: &AbiPolicyConfig) -> Result<()> {
    validate_uapi_removal(path, policy)?;
    validate_public_header_removal(path, policy)
}

pub(crate) fn validate_uapi_removal(path: &Path, policy: &AbiPolicyConfig) -> Result<()> {
    let Ok(uapi_path) = UapiPath::new(path.to_path_buf()) else {
        return Ok(());
    };
    if policy.allow_uapi_header_removal {
        return Ok(());
    }

    anyhow::bail!(
        "UAPI removal requires explicit ABI policy approval for '{}'; set abi.allow_uapi_header_removal = true",
        uapi_path.as_str(),
    )
}

pub(crate) fn validate_public_header_removal(path: &Path, policy: &AbiPolicyConfig) -> Result<()> {
    if !has_header_extension(path)
        || !is_public_header_path(path)
        || allows_public_header_removal(path, policy)
    {
        return Ok(());
    }

    let policy_key = if is_uapi_path(path) {
        "abi.allow_uapi_header_removal"
    } else {
        "abi.allow_public_header_removal"
    };
    anyhow::bail!(
        "public header removal requires explicit ABI policy approval for '{}'; set {} = true",
        path.display(),
        policy_key,
    )
}

