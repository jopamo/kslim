use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use crate::config::KernelBuildConfig;
use crate::model::ArchName;
use crate::paths::{KernelBuildDir, KernelSourceRoot};

use super::{CapturedCommandFailure, SelfTestFailure};

pub(super) fn run_kernel_build(
    root: &Path,
    build: &KernelBuildConfig,
    index: usize,
) -> std::result::Result<(), SelfTestFailure> {
    let label = kernel_build_label(build, index);
    let arch = kernel_build_arch(build).map_err(|err| SelfTestFailure::BuiltIn {
        check: "kernel-build-arch",
        message: format!(
            "invalid kernel build ARCH for selftest '{}': {:#}",
            label, err
        ),
    })?;
    let source_root = KernelSourceRoot::new(root).map_err(|err| SelfTestFailure::BuiltIn {
        check: "kernel-build-source-root",
        message: format!(
            "invalid kernel source root for selftest '{}': {:#}",
            label, err
        ),
    })?;
    let output_dir = kernel_build_output_dir(&source_root, build, index).map_err(|err| {
        SelfTestFailure::BuiltIn {
            check: "kernel-build-output-dir",
            message: format!(
                "invalid kernel build output dir for selftest '{}': {:#}",
                label, err
            ),
        }
    })?;

    if build.clean && output_dir.as_path().exists() {
        std::fs::remove_dir_all(output_dir.as_path()).map_err(|err| SelfTestFailure::BuiltIn {
            check: "kernel-build-output-clean",
            message: format!(
                "failed to clean kernel build output dir for selftest '{}': {}: {}",
                label,
                output_dir.as_path().display(),
                err
            ),
        })?;
    }
    std::fs::create_dir_all(output_dir.as_path()).map_err(|err| SelfTestFailure::BuiltIn {
        check: "kernel-build-output-create",
        message: format!(
            "failed to create kernel build output dir for selftest '{}': {}: {}",
            label,
            output_dir.as_path().display(),
            err
        ),
    })?;

    if let Some(config_target) = build.config_target.as_deref() {
        run_make(
            root,
            build,
            &label,
            arch.as_ref(),
            &output_dir,
            &[config_target],
        )?;
    }

    if !build.targets.is_empty() {
        let targets: Vec<&str> = build.targets.iter().map(String::as_str).collect();
        run_make(root, build, &label, arch.as_ref(), &output_dir, &targets)?;
    }

    Ok(())
}

fn run_make(
    root: &Path,
    build: &KernelBuildConfig,
    label: &str,
    arch: Option<&ArchName>,
    output_dir: &KernelBuildDir,
    targets: &[&str],
) -> std::result::Result<(), SelfTestFailure> {
    let program = build
        .make_program
        .as_deref()
        .map(|program| resolve_program(root, program))
        .unwrap_or_else(|| OsString::from("make"));

    let mut command = Command::new(&program);
    command.current_dir(root);
    command.arg(format!("O={}", output_dir.as_path().display()));

    if let Some(jobs) = build.jobs {
        command.arg(format!("-j{}", jobs));
    }

    for (key, value) in &build.env {
        command.env(key, value);
    }

    command.args(&build.make_args);
    command.args(targets);
    let started = Instant::now();

    let output = command
        .output()
        .map_err(|err| SelfTestFailure::KernelBuild {
            label: label.to_string(),
            output_dir: output_dir.as_path().to_path_buf(),
            details: CapturedCommandFailure {
                command: Path::new(&program).display().to_string(),
                target: Some(targets.join(" ")),
                arch: arch.map(|arch| arch.as_str().to_string()),
                config: build.config_target.clone(),
                stdout: String::new(),
                stderr: format!(
                    "failed to run kernel build selftest '{}' with program '{}': {}",
                    label,
                    Path::new(&program).display(),
                    err
                ),
                exit_status: None,
                elapsed: started.elapsed(),
            },
        })?;

    if !output.status.success() {
        return Err(SelfTestFailure::KernelBuild {
            label: label.to_string(),
            output_dir: output_dir.as_path().to_path_buf(),
            details: CapturedCommandFailure {
                command: Path::new(&program).display().to_string(),
                target: Some(targets.join(" ")),
                arch: arch.map(|arch| arch.as_str().to_string()),
                config: build.config_target.clone(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                exit_status: output.status.code(),
                elapsed: started.elapsed(),
            },
        });
    }

    Ok(())
}

fn kernel_build_arch(build: &KernelBuildConfig) -> anyhow::Result<Option<ArchName>> {
    build
        .env
        .get("ARCH")
        .map(|arch| ArchName::new(arch.as_str()))
        .transpose()
}

fn kernel_build_label(build: &KernelBuildConfig, index: usize) -> String {
    build
        .name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("kernel-build-{}", index + 1))
}

fn kernel_build_output_dir(
    source_root: &KernelSourceRoot,
    build: &KernelBuildConfig,
    index: usize,
) -> anyhow::Result<KernelBuildDir> {
    let Some(configured) = build
        .output_dir
        .as_deref()
        .filter(|dir| !dir.trim().is_empty())
    else {
        return Ok(KernelBuildDir::default_for_source_root(source_root, index));
    };
    KernelBuildDir::new_for_source_root(source_root, configured)
}

fn resolve_program(root: &Path, program: &str) -> OsString {
    let path = Path::new(program);
    if path.is_absolute() || path.components().count() > 1 {
        root.join(path).into_os_string()
    } else {
        OsString::from(program)
    }
}
