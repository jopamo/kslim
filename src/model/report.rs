//! Report and attempt-summary value models.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::path_policy::path_is_empty_like;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ReportPath(PathBuf);

impl ReportPath {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if path_is_empty_like(&path) {
            anyhow::bail!("failure report path is empty");
        }
        Ok(Self(path))
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.0.as_path()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReducerReportSummary {
    pub files_removed: usize,
    pub dirs_removed: usize,
    pub edit_records: usize,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelftestReportSummary {
    pub enabled: bool,
    pub built_in_checks: usize,
    pub kernel_builds_run: usize,
    pub commands_run: usize,
}
