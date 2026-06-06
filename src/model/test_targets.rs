//! KUnit and kselftest target value models.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::path::Path;

use crate::path_policy::contains_parent_traversal;

use super::validation::{
    is_kselftest_target_char, is_kunit_suite_char, non_empty_model_value,
    normalized_relative_model_path_parts_against,
};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct KunitSuite(String);

#[allow(dead_code)]
impl KunitSuite {
    pub fn new(suite: impl Into<String>) -> Result<Self> {
        let suite = non_empty_model_value("KUnit suite", suite)?;
        if suite.chars().any(char::is_whitespace) {
            anyhow::bail!("KUnit suite contains whitespace: {}", suite);
        }
        if contains_parent_traversal(&suite) {
            anyhow::bail!("KUnit suite must not contain '..': {}", suite);
        }
        if !suite.chars().all(is_kunit_suite_char) {
            anyhow::bail!("KUnit suite contains invalid characters: {}", suite);
        }
        Ok(Self(suite))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for KunitSuite {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct KselftestTarget(String);

#[allow(dead_code)]
impl KselftestTarget {
    pub fn new(target: impl Into<String>) -> Result<Self> {
        let target = target.into();
        let parts = normalized_relative_model_path_parts_against(
            "kselftest target",
            Path::new(&target),
            "kselftest target set",
        )?;
        let normalized = parts.join("/");
        if !normalized.chars().all(is_kselftest_target_char) {
            anyhow::bail!("kselftest target contains invalid characters: {}", target);
        }
        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for KselftestTarget {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
