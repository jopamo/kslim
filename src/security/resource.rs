//! Resource-limit security policy.
//!
//! Resource policy is parsed in profiles but nondefault resource relaxation is
//! not implemented yet. This boundary names the fail-closed decision point.

use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResourcePolicy {
    allow_unbounded_execution: bool,
}

impl Default for ResourcePolicy {
    fn default() -> Self {
        Self {
            allow_unbounded_execution: false,
        }
    }
}

#[allow(dead_code)]
impl ResourcePolicy {
    pub(crate) fn fail_closed() -> Self {
        Self::default()
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.allow_unbounded_execution {
            anyhow::bail!(
                "security resource policy rejects unbounded execution until resource planning lands"
            );
        }
        Ok(())
    }
}
