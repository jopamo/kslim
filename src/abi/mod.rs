//! ABI-sensitive surface taxonomy and removal policy.
//!
//! This module owns fail-closed ABI removal policy and ABI-facing surface
//! classification. Read-only tree indexes may record facts about these
//! surfaces, but policy decisions live here. `crate::abi_policy` is only a
//! compatibility facade while older call sites migrate.

mod policy;
mod surface;

#[allow(unused_imports)]
pub(crate) use policy::{
    allows_public_header_removal, is_public_header_path, is_uapi_header_path, is_uapi_path,
    validate_declared_removal, validate_public_header_removal, validate_uapi_removal,
};
pub use policy::AbiPolicyConfig;

#[allow(unused_imports)]
pub(crate) use surface::{
    classify_abi_header_path, has_header_extension, is_headers_install_target,
};
pub use surface::AbiSurfaceKind;
