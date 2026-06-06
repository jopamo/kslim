//! Selftest failure capture for diagnostic classification.
//!
//! This module owns the boundary between selftest result shapes and the
//! classifier's narrow diagnostic input model.

use crate::selftest::{CapturedCommandFailure, SelfTestFailure};

pub(in crate::diagnostics) enum CapturedDiagnostic<'a> {
    Command(CapturedCommandDiagnostic<'a>),
    BuiltIn(CapturedBuiltInDiagnostic<'a>),
}

pub(in crate::diagnostics) struct CapturedCommandDiagnostic<'a> {
    pub stderr: &'a str,
    pub target: Option<&'a str>,
    pub arch: Option<&'a str>,
    pub config: Option<&'a str>,
}

pub(in crate::diagnostics) struct CapturedBuiltInDiagnostic<'a> {
    pub check: &'a str,
    pub message: &'a str,
}

pub(in crate::diagnostics) fn capture_selftest_failure(
    failure: &SelfTestFailure,
) -> CapturedDiagnostic<'_> {
    match failure {
        SelfTestFailure::KernelBuild { details, .. } | SelfTestFailure::Command { details } => {
            CapturedDiagnostic::Command(capture_command_failure(details))
        }
        SelfTestFailure::BuiltIn { check, message } => {
            CapturedDiagnostic::BuiltIn(CapturedBuiltInDiagnostic { check, message })
        }
    }
}

fn capture_command_failure(details: &CapturedCommandFailure) -> CapturedCommandDiagnostic<'_> {
    CapturedCommandDiagnostic {
        stderr: &details.stderr,
        target: details.target.as_deref(),
        arch: details.arch.as_deref(),
        config: details.config.as_deref(),
    }
}
