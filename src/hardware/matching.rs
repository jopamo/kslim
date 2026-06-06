//! Hardware match-subject taxonomy.
//!
//! Feature resolution and proof modules use these typed subjects to keep
//! devicetree, modalias, firmware, PCI, USB, ACPI, and platform matching in one
//! domain instead of scattering string categories through reducer paths.

use anyhow::Result;

use crate::model::{AcpiId, DeviceCompatible, FirmwarePath, ModuleAlias, PciId, UsbId};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum HardwareMatchKind {
    DevicetreeCompatible,
    Modalias,
    FirmwarePath,
    PciId,
    UsbId,
    AcpiId,
    Platform,
}

#[allow(dead_code)]
impl HardwareMatchKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::DevicetreeCompatible => "devicetree_compatible",
            Self::Modalias => "modalias",
            Self::FirmwarePath => "firmware_path",
            Self::PciId => "pci_id",
            Self::UsbId => "usb_id",
            Self::AcpiId => "acpi_id",
            Self::Platform => "platform",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PlatformMatchName(String);

#[allow(dead_code)]
impl PlatformMatchName {
    pub(crate) fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        if name.trim().is_empty() {
            anyhow::bail!("platform match name must not be empty");
        }
        if !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
        {
            anyhow::bail!("platform match name contains invalid characters: {name}");
        }
        Ok(Self(name))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum HardwareMatchSubject {
    DevicetreeCompatible(DeviceCompatible),
    Modalias(ModuleAlias),
    FirmwarePath(FirmwarePath),
    PciId(PciId),
    UsbId(UsbId),
    AcpiId(AcpiId),
    Platform(PlatformMatchName),
}

#[allow(dead_code)]
impl HardwareMatchSubject {
    pub(crate) fn kind(&self) -> HardwareMatchKind {
        match self {
            Self::DevicetreeCompatible(_) => HardwareMatchKind::DevicetreeCompatible,
            Self::Modalias(_) => HardwareMatchKind::Modalias,
            Self::FirmwarePath(_) => HardwareMatchKind::FirmwarePath,
            Self::PciId(_) => HardwareMatchKind::PciId,
            Self::UsbId(_) => HardwareMatchKind::UsbId,
            Self::AcpiId(_) => HardwareMatchKind::AcpiId,
            Self::Platform(_) => HardwareMatchKind::Platform,
        }
    }

    pub(crate) fn value(&self) -> &str {
        match self {
            Self::DevicetreeCompatible(value) => value.as_str(),
            Self::Modalias(value) => value.as_str(),
            Self::FirmwarePath(value) => value.as_str(),
            Self::PciId(value) => value.as_str(),
            Self::UsbId(value) => value.as_str(),
            Self::AcpiId(value) => value.as_str(),
            Self::Platform(value) => value.as_str(),
        }
    }

    pub(crate) fn stable_key(&self) -> String {
        format!("{}:{}", self.kind().stable_name(), self.value())
    }
}
