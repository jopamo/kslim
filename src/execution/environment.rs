//! Environment inheritance and allowlist handling for process execution.

use std::collections::BTreeSet;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EnvironmentAllowlist {
    InheritParent,
    AllowOnly(BTreeSet<String>),
}

impl Default for EnvironmentAllowlist {
    fn default() -> Self {
        Self::InheritParent
    }
}

#[allow(dead_code)]
impl EnvironmentAllowlist {
    pub(crate) fn inherit_parent() -> Self {
        Self::InheritParent
    }

    pub(crate) fn allow_only(names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::AllowOnly(names.into_iter().map(Into::into).collect())
    }

    pub(crate) fn apply_to(&self, command: &mut Command) {
        match self {
            Self::InheritParent => {}
            Self::AllowOnly(names) => {
                command.env_clear();
                for name in names {
                    if let Some(value) = std::env::var_os(name) {
                        command.env(name, value);
                    }
                }
            }
        }
    }
}
