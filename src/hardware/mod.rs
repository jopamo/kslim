//! Hardware identity matching and devicetree proof gates.
//!
//! This module owns hardware-facing match subjects for devicetree,
//! modalias/module matching, firmware paths, PCI, USB, ACPI, and platform
//! surfaces. It records and compares hardware identity only; reducer mutation,
//! candidate state, and report rendering live elsewhere. `crate::device_bindings`
//! is only a compatibility facade while older call sites migrate.

mod devicetree;
mod matching;

pub(crate) use devicetree::{
    prove_removed_device_bindings_have_no_live_references, DeviceBindingRemovalProof,
};

#[allow(unused_imports)]
pub(crate) use matching::{HardwareMatchKind, HardwareMatchSubject, PlatformMatchName};
