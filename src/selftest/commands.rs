use std::path::Path;
use std::process::Command;
use std::time::Instant;

use super::{CapturedCommandFailure, SelfTestFailure};

pub(super) fn run_command(root: &Path, command: &str) -> std::result::Result<(), SelfTestFailure> {
    let started = Instant::now();
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(root)
        .output()
        .map_err(|err| SelfTestFailure::Command {
            details: CapturedCommandFailure {
                command: command.to_string(),
                target: None,
                arch: None,
                config: None,
                stdout: String::new(),
                stderr: format!("failed to run selftest command: {}", err),
                exit_status: None,
                elapsed: started.elapsed(),
            },
        })?;

    if !output.status.success() {
        return Err(SelfTestFailure::Command {
            details: CapturedCommandFailure {
                command: command.to_string(),
                target: None,
                arch: None,
                config: None,
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                exit_status: output.status.code(),
                elapsed: started.elapsed(),
            },
        });
    }

    Ok(())
}
