//! Argv process execution helpers.

use anyhow::{Context, Result};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::cleanup::ProcessCleanup;
use super::environment::EnvironmentAllowlist;
use super::log_capture::CapturedCommandOutput;
use super::timeout::ExecutionTimeout;

#[derive(Debug, Clone)]
pub(crate) struct CommandSpec {
    program: String,
    args: Vec<String>,
    current_dir: Option<String>,
    environment: EnvironmentAllowlist,
    timeout: ExecutionTimeout,
    cleanup: ProcessCleanup,
}

#[allow(dead_code)]
impl CommandSpec {
    pub(crate) fn new(cmd: &str, args: &[&str]) -> Self {
        Self {
            program: cmd.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            current_dir: None,
            environment: EnvironmentAllowlist::inherit_parent(),
            timeout: ExecutionTimeout::disabled(),
            cleanup: ProcessCleanup::kill_on_timeout(),
        }
    }

    pub(crate) fn current_dir(mut self, dir: &str) -> Self {
        self.current_dir = Some(dir.to_string());
        self
    }

    pub(crate) fn environment(mut self, environment: EnvironmentAllowlist) -> Self {
        self.environment = environment;
        self
    }

    pub(crate) fn timeout(mut self, timeout: ExecutionTimeout) -> Self {
        self.timeout = timeout;
        self
    }

    pub(crate) fn cleanup(mut self, cleanup: ProcessCleanup) -> Self {
        self.cleanup = cleanup;
        self
    }

    pub(crate) fn execute(&self) -> Result<CapturedCommandOutput> {
        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = &self.current_dir {
            command.current_dir(dir);
        }
        self.environment.apply_to(&mut command);

        let mut child = command
            .spawn()
            .with_context(|| format!("failed to run: {}", self.command_context()))?;

        if let Some(timeout) = self.timeout.duration() {
            let started = Instant::now();
            loop {
                if child.try_wait()?.is_some() {
                    let output = child.wait_with_output()?;
                    return Ok(CapturedCommandOutput::from_output(output));
                }
                if started.elapsed() >= timeout {
                    self.cleanup.cleanup_timed_out_child(&mut child);
                    let output = child.wait_with_output()?;
                    anyhow::bail!(
                        "{} timed out after {:?}: {}",
                        self.program,
                        timeout,
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                thread::sleep(Duration::from_millis(10));
            }
        }

        let output = child.wait_with_output()?;
        Ok(CapturedCommandOutput::from_output(output))
    }

    pub(crate) fn output_stdout_trimmed(&self) -> Result<String> {
        let output = self.execute().with_context(|| {
            if let Some(dir) = &self.current_dir {
                format!("failed to run: {} in {}", self.command_context(), dir)
            } else {
                format!("failed to run: {}", self.command_context())
            }
        })?;

        if !output.status.success() {
            anyhow::bail!(
                "{} failed: {}",
                self.command_context(),
                output.stderr_lossy()
            );
        }
        Ok(output.stdout_trimmed_lossy())
    }

    fn command_context(&self) -> String {
        if self.args.is_empty() {
            self.program.clone()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }
}

pub(crate) fn run(cmd: &str, args: &[&str]) -> Result<String> {
    CommandSpec::new(cmd, args).output_stdout_trimmed()
}

pub(crate) fn run_in_dir(dir: &str, cmd: &str, args: &[&str]) -> Result<String> {
    CommandSpec::new(cmd, args)
        .current_dir(dir)
        .output_stdout_trimmed()
}

#[allow(dead_code)]
pub(crate) fn run_quiet(cmd: &str, args: &[&str]) -> Result<()> {
    run(cmd, args)?;
    Ok(())
}
