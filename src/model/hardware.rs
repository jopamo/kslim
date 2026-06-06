//! Hardware identifier and firmware path value models.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::path::{Path, PathBuf};

use super::validation::{
    is_acpi_id_char, is_device_compatible_char, is_upper_hex_digit, non_empty_model_value,
    normalized_relative_model_path_parts_against,
};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct DeviceCompatible(String);

#[allow(dead_code)]
impl DeviceCompatible {
    pub fn new(compatible: impl Into<String>) -> Result<Self> {
        let compatible = non_empty_model_value("device compatible", compatible)?;
        let comma_count = compatible.chars().filter(|ch| *ch == ',').count();
        if comma_count != 1 {
            anyhow::bail!(
                "device compatible must use vendor,device form: {}",
                compatible
            );
        }
        let (vendor, device) = compatible
            .split_once(',')
            .expect("device compatible comma count should be one");
        if vendor.is_empty() || device.is_empty() {
            anyhow::bail!(
                "device compatible must include nonempty vendor and device: {}",
                compatible
            );
        }
        if !compatible.chars().all(is_device_compatible_char) {
            anyhow::bail!(
                "device compatible contains invalid characters: {}",
                compatible
            );
        }
        Ok(Self(compatible))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for DeviceCompatible {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct AcpiId(String);

#[allow(dead_code)]
impl AcpiId {
    pub fn new(id: impl Into<String>) -> Result<Self> {
        let id = non_empty_model_value("ACPI ID", id)?;
        if !(4..=16).contains(&id.len()) {
            anyhow::bail!("ACPI ID must be 4 to 16 ASCII characters: {}", id);
        }
        if !id.chars().all(is_acpi_id_char) {
            anyhow::bail!(
                "ACPI ID must use uppercase ASCII letters and digits only: {}",
                id
            );
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for AcpiId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct PciId(String);

#[allow(dead_code)]
impl PciId {
    pub fn new(id: impl Into<String>) -> Result<Self> {
        let id = non_empty_model_value("PCI ID", id)?;
        let Some((vendor, device)) = id.split_once(':') else {
            anyhow::bail!(
                "PCI ID must use VVVV:DDDD hexadecimal vendor/device form: {}",
                id
            );
        };
        if vendor.len() != 4 || device.len() != 4 || device.contains(':') {
            anyhow::bail!(
                "PCI ID must use VVVV:DDDD hexadecimal vendor/device form: {}",
                id
            );
        }
        if !vendor.chars().all(is_upper_hex_digit) || !device.chars().all(is_upper_hex_digit) {
            anyhow::bail!("PCI ID must use uppercase hexadecimal digits: {}", id);
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for PciId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct UsbId(String);

#[allow(dead_code)]
impl UsbId {
    pub fn new(id: impl Into<String>) -> Result<Self> {
        let id = non_empty_model_value("USB ID", id)?;
        let Some((vendor, product)) = id.split_once(':') else {
            anyhow::bail!(
                "USB ID must use VVVV:PPPP hexadecimal vendor/product form: {}",
                id
            );
        };
        if vendor.len() != 4 || product.len() != 4 || product.contains(':') {
            anyhow::bail!(
                "USB ID must use VVVV:PPPP hexadecimal vendor/product form: {}",
                id
            );
        }
        if !vendor.chars().all(is_upper_hex_digit) || !product.chars().all(is_upper_hex_digit) {
            anyhow::bail!("USB ID must use uppercase hexadecimal digits: {}", id);
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for UsbId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct FirmwarePath(String);

#[allow(dead_code)]
impl FirmwarePath {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let parts = normalized_relative_model_path_parts_against(
            "firmware path",
            &path,
            "firmware search path",
        )?;
        Ok(Self(parts.join("/")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl Borrow<str> for FirmwarePath {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
