//! Exported-symbol and runtime registration value models.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;

use super::validation::{
    is_c_identifier, is_c_identifier_continue, is_c_identifier_start, non_empty_model_value,
};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct ExportedSymbol(String);

#[allow(dead_code)]
impl ExportedSymbol {
    pub fn new(symbol: impl Into<String>) -> Result<Self> {
        let symbol = non_empty_model_value("exported symbol", symbol)?;
        let mut chars = symbol.chars();
        let Some(first) = chars.next() else {
            anyhow::bail!("exported symbol must not be empty");
        };
        if !is_c_identifier_start(first) || !chars.all(is_c_identifier_continue) {
            anyhow::bail!("exported symbol contains invalid characters: {}", symbol);
        }
        Ok(Self(symbol))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for ExportedSymbol {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct Initcall(String);

#[allow(dead_code)]
impl Initcall {
    pub fn new(initcall: impl Into<String>) -> Result<Self> {
        let initcall = non_empty_model_value("initcall", initcall)?;
        let mut chars = initcall.chars();
        let Some(first) = chars.next() else {
            anyhow::bail!("initcall must not be empty");
        };
        if !is_c_identifier_start(first) || !chars.all(is_c_identifier_continue) {
            anyhow::bail!("initcall contains invalid characters: {}", initcall);
        }
        Ok(Self(initcall))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for Initcall {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct RuntimeRegistrationSurface(String);

#[allow(dead_code)]
impl RuntimeRegistrationSurface {
    pub fn new(surface: impl Into<String>) -> Result<Self> {
        let surface = non_empty_model_value("runtime registration surface", surface)?;
        let Some((registration_macro, entry_point)) = surface.split_once(':') else {
            anyhow::bail!(
                "runtime registration surface must use registration_macro:entry_point form: {}",
                surface
            );
        };
        if registration_macro.is_empty() || entry_point.is_empty() || entry_point.contains(':') {
            anyhow::bail!(
                "runtime registration surface must use registration_macro:entry_point form: {}",
                surface
            );
        }
        if !is_c_identifier(registration_macro) {
            anyhow::bail!(
                "runtime registration macro contains invalid characters: {}",
                registration_macro
            );
        }
        if !Self::is_known_registration_macro(registration_macro) {
            anyhow::bail!(
                "unsupported runtime registration macro: {}",
                registration_macro
            );
        }
        if !is_c_identifier(entry_point) {
            anyhow::bail!(
                "runtime registration entry point contains invalid characters: {}",
                entry_point
            );
        }
        Ok(Self(format!("{registration_macro}:{entry_point}")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn registration_macro(&self) -> &str {
        self.0
            .split_once(':')
            .expect("runtime registration surface stores macro:entry_point")
            .0
    }

    pub fn entry_point(&self) -> &str {
        self.0
            .split_once(':')
            .expect("runtime registration surface stores macro:entry_point")
            .1
    }

    pub(crate) fn is_known_registration_macro(registration_macro: &str) -> bool {
        matches!(
            registration_macro,
            "module_init"
                | "module_exit"
                | "early_initcall"
                | "pure_initcall"
                | "core_initcall"
                | "core_initcall_sync"
                | "postcore_initcall"
                | "postcore_initcall_sync"
                | "arch_initcall"
                | "arch_initcall_sync"
                | "subsys_initcall"
                | "subsys_initcall_sync"
                | "fs_initcall"
                | "fs_initcall_sync"
                | "rootfs_initcall"
                | "device_initcall"
                | "device_initcall_sync"
                | "late_initcall"
                | "late_initcall_sync"
                | "console_initcall"
                | "security_initcall"
                | "module_platform_driver"
                | "builtin_platform_driver"
                | "module_platform_driver_probe"
                | "platform_driver_register"
                | "platform_driver_probe"
                | "module_i2c_driver"
                | "builtin_i2c_driver"
                | "i2c_add_driver"
                | "module_spi_driver"
                | "spi_register_driver"
                | "module_pci_driver"
                | "pci_register_driver"
                | "module_usb_driver"
                | "usb_register"
                | "module_serdev_device_driver"
                | "module_acpi_driver"
                | "module_amba_driver"
                | "module_mdio_driver"
                | "module_phy_driver"
                | "module_misc_device"
                | "misc_register"
                | "register_netdev"
                | "register_netdevice"
        )
    }
}

impl Borrow<str> for RuntimeRegistrationSurface {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
