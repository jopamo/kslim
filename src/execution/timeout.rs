//! Timeout model for process execution.

use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionTimeout {
    Disabled,
    After(Duration),
}

impl Default for ExecutionTimeout {
    fn default() -> Self {
        Self::Disabled
    }
}

#[allow(dead_code)]
impl ExecutionTimeout {
    pub(crate) fn disabled() -> Self {
        Self::Disabled
    }

    pub(crate) fn after(duration: Duration) -> Self {
        Self::After(duration)
    }

    pub(crate) fn duration(self) -> Option<Duration> {
        match self {
            Self::Disabled => None,
            Self::After(duration) => Some(duration),
        }
    }
}
