//! Kernel module name and alias value models.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;

use super::validation::{
    is_module_alias_char, is_module_name_continue, is_module_name_start, non_empty_model_value,
};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct ModuleName(String);

#[allow(dead_code)]
impl ModuleName {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = non_empty_model_value("module name", name)?;
        if name.ends_with(".ko") {
            anyhow::bail!("module name must omit .ko suffix: {}", name);
        }

        let mut chars = name.chars();
        let Some(first) = chars.next() else {
            anyhow::bail!("module name must not be empty");
        };
        if !is_module_name_start(first) || !chars.all(is_module_name_continue) {
            anyhow::bail!("module name contains invalid characters: {}", name);
        }

        Ok(Self(name.replace('-', "_")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for ModuleName {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct ModuleAlias(String);

#[allow(dead_code)]
impl ModuleAlias {
    pub fn new(alias: impl Into<String>) -> Result<Self> {
        let alias = non_empty_model_value("module alias", alias)?;
        if alias.ends_with(".ko") {
            anyhow::bail!("module alias must not be a module file name: {}", alias);
        }
        if alias.chars().any(|ch| ch.is_ascii_whitespace()) {
            anyhow::bail!("module alias must not contain whitespace: {}", alias);
        }
        if !alias.chars().all(is_module_alias_char) {
            anyhow::bail!("module alias contains invalid characters: {}", alias);
        }
        Ok(Self(alias))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for ModuleAlias {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
