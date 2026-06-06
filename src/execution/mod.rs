//! Process execution boundaries.
//!
//! Execution owns argv construction, captured output, timeout handling,
//! process cleanup after timeout, and explicit environment inheritance or
//! allowlisting. Domain modules should call these narrow helpers instead of
//! constructing process policy inline.

#![allow(unused_imports)]

mod argv;
mod cleanup;
mod environment;
mod log_capture;
mod timeout;

pub(crate) use argv::{run, run_in_dir, run_quiet, CommandSpec};
pub(crate) use cleanup::ProcessCleanup;
pub(crate) use environment::EnvironmentAllowlist;
pub(crate) use log_capture::CapturedCommandOutput;
pub(crate) use timeout::ExecutionTimeout;
