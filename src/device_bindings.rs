//! Compatibility facade for devicetree binding removal proof.
//!
//! Hardware matching and devicetree proof ownership lives in `crate::hardware`;
//! this module preserves existing `crate::hardware::*` call sites while
//! migration proceeds.

#[allow(unused_imports)]
pub(crate) use crate::hardware::{
    prove_removed_device_bindings_have_no_live_references, DeviceBindingRemovalProof,
};
