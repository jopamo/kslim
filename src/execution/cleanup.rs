//! Process cleanup after failed or timed-out execution.

use std::process::Child;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessCleanup {
    kill_on_timeout: bool,
}

impl Default for ProcessCleanup {
    fn default() -> Self {
        Self {
            kill_on_timeout: true,
        }
    }
}

#[allow(dead_code)]
impl ProcessCleanup {
    pub(crate) fn kill_on_timeout() -> Self {
        Self {
            kill_on_timeout: true,
        }
    }

    pub(crate) fn leave_child_for_debugging() -> Self {
        Self {
            kill_on_timeout: false,
        }
    }

    pub(crate) fn cleanup_timed_out_child(&self, child: &mut Child) {
        if self.kill_on_timeout {
            let _ = child.kill();
        }
    }
}
