//! Compatibility facade for ABI-sensitive removal policy.
//!
//! ABI surface taxonomy and fail-closed removal policy live in `crate::abi`;
//! this module preserves existing `crate::abi_policy::*` call sites while
//! migration proceeds.

#[allow(unused_imports)]
pub(crate) use crate::abi::{
    allows_public_header_removal, is_public_header_path, is_uapi_header_path, is_uapi_path,
    validate_declared_removal, validate_public_header_removal, validate_uapi_removal,
};
#[allow(unused_imports)]
pub use crate::abi::AbiPolicyConfig;
