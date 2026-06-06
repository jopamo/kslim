//! Compatibility facade for runtime-registration removal proof.
//!
//! Runtime reachability and registration proof ownership lives in
//! `crate::runtime`; this module preserves existing
//! `crate::runtime_registrations::*` call sites while migration proceeds.

#[allow(unused_imports)]
pub(crate) use crate::runtime::{
    prove_removed_runtime_registrations_have_no_live_entry_points,
    RuntimeRegistrationRemovalProof,
};
