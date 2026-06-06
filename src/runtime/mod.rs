//! Runtime reachability and entry-point proof gates.
//!
//! This module owns runtime-facing subjects for initcalls, registrations,
//! callbacks, module entry points, and reachability. It records and proves
//! runtime entry-point safety only; reducer mutation, candidate state, and
//! report rendering live elsewhere. `crate::runtime_registrations` is only a
//! compatibility facade while older call sites migrate.

mod reachability;
mod registration;

pub(crate) use registration::{
    prove_removed_runtime_registrations_have_no_live_entry_points,
    RuntimeRegistrationRemovalProof,
};

#[allow(unused_imports)]
pub(crate) use reachability::{
    ModuleEntryPoint, RuntimeCallbackName, RuntimeReachabilityKind, RuntimeReachabilitySubject,
};
