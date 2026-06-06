//! Captured stdout/stderr and exit status for process execution.

use std::process::{ExitStatus, Output};

#[derive(Debug)]
pub(crate) struct CapturedCommandOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

impl CapturedCommandOutput {
    pub(crate) fn from_output(output: Output) -> Self {
        Self {
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }

    pub(crate) fn stdout_trimmed_lossy(&self) -> String {
        String::from_utf8_lossy(&self.stdout).trim().to_string()
    }

    pub(crate) fn stderr_lossy(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}
