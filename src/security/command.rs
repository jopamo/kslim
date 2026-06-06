//! Command execution security policy.
//!
//! This module owns command trust-boundary decisions. Execution owns mechanics;
//! command policy owns which argv shapes are acceptable before mechanics run.

use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandPolicy {
    allow_shell: bool,
}

impl Default for CommandPolicy {
    fn default() -> Self {
        Self { allow_shell: false }
    }
}

#[allow(dead_code)]
impl CommandPolicy {
    pub(crate) fn fail_closed() -> Self {
        Self::default()
    }

    pub(crate) fn allow_shell_for_explicit_selftest() -> Self {
        Self { allow_shell: true }
    }

    pub(crate) fn validate_program(&self, program: &str) -> Result<()> {
        let program = program.trim();
        if program.is_empty() {
            anyhow::bail!("security command policy rejects empty command program");
        }
        if !self.allow_shell && matches!(program, "sh" | "bash" | "dash") {
            anyhow::bail!(
                "security command policy rejects shell execution without explicit compatibility mode"
            );
        }
        Ok(())
    }
}
