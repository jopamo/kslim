use anyhow::Result;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config::SelfTestConfig;

mod built_in;
mod commands;
mod kernel_build;

#[derive(Debug, Clone)]
pub struct SelfTestResult {
    pub enabled: bool,
    pub built_in_checks: usize,
    pub kernel_builds_run: usize,
    pub commands_run: usize,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct SelfTestExecutionCounts {
    built_in_checks: usize,
    kernel_builds_run: usize,
    commands_run: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedCommandFailure {
    pub command: String,
    pub target: Option<String>,
    pub arch: Option<String>,
    pub config: Option<String>,
    pub stdout: String,
    pub stderr: String,
    pub exit_status: Option<i32>,
    pub elapsed: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfTestFailure {
    BuiltIn {
        check: &'static str,
        message: String,
    },
    KernelBuild {
        label: String,
        output_dir: PathBuf,
        details: CapturedCommandFailure,
    },
    Command {
        details: CapturedCommandFailure,
    },
}

impl fmt::Display for SelfTestFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BuiltIn { message, .. } => f.write_str(message),
            Self::KernelBuild {
                label,
                output_dir,
                details,
            } => write!(
                f,
                "kernel build selftest '{}' failed\nprogram: {}\noutput_dir: {}\ntargets: {}\nstdout:\n{}\nstderr:\n{}",
                label,
                details.command,
                output_dir.display(),
                details.target.as_deref().unwrap_or(""),
                details.stdout.trim_end(),
                details.stderr.trim_end(),
            ),
            Self::Command { details } => write!(
                f,
                "selftest command failed: {}\nstdout:\n{}\nstderr:\n{}",
                details.command,
                details.stdout.trim_end(),
                details.stderr.trim_end(),
            ),
        }
    }
}

impl std::error::Error for SelfTestFailure {}

#[allow(dead_code)]
pub fn run(root: &str, config: &SelfTestConfig) -> Result<SelfTestResult> {
    run_capture(root, config).map_err(|err| anyhow::anyhow!("{}", err))
}

pub fn run_capture(
    root: &str,
    config: &SelfTestConfig,
) -> std::result::Result<SelfTestResult, SelfTestFailure> {
    let root = Path::new(root);

    if !config.enabled {
        return Ok(SelfTestResult {
            enabled: false,
            built_in_checks: 0,
            kernel_builds_run: 0,
            commands_run: 0,
        });
    }

    let counts = run_enabled_selftests(root, config)?;

    Ok(SelfTestResult {
        enabled: true,
        built_in_checks: counts.built_in_checks,
        kernel_builds_run: counts.kernel_builds_run,
        commands_run: counts.commands_run,
    })
}

fn run_enabled_selftests(
    root: &Path,
    config: &SelfTestConfig,
) -> std::result::Result<SelfTestExecutionCounts, SelfTestFailure> {
    let mut counts = SelfTestExecutionCounts::default();

    if config.check_kconfig_sources {
        built_in::validate_kconfig_sources(root)?;
        counts.built_in_checks += 1;
    }

    if config.check_makefiles {
        built_in::validate_makefiles(root)?;
        counts.built_in_checks += 1;
    }

    for (idx, build) in config.kernel_builds.iter().enumerate() {
        kernel_build::run_kernel_build(root, build, idx)?;
        counts.built_in_checks += 1;
        counts.kernel_builds_run += 1;
    }

    for command in &config.commands {
        commands::run_command(root, command)?;
        counts.commands_run += 1;
    }

    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::built_in::{validate_kconfig_sources, validate_makefiles};
    use super::*;
    use crate::config::{KernelBuildConfig, SelfTestConfig};

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn test_parse_kconfig_source() {
        let source =
            crate::kconfig::parse_kconfig_source(r#"source "drivers/gpu/drm/amd/amdgpu/Kconfig""#)
                .expect("should parse source");
        assert_eq!(source.path, "drivers/gpu/drm/amd/amdgpu/Kconfig");
        assert!(!source.optional);
        assert!(!source.relative);
    }

    #[test]
    fn test_make_selftest_allows_composite_objects() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("Makefile"), "foo-y := a.o b.o\nobj-y += foo.o\n").unwrap();
        std::fs::write(root.join("a.c"), "int a;\n").unwrap();
        std::fs::write(root.join("b.c"), "int b;\n").unwrap();

        validate_makefiles(root).unwrap();
    }

    #[test]
    fn test_make_selftest_allows_root_relative_arch_dir_refs() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("arch/x86/lib")).unwrap();
        std::fs::write(root.join("arch/x86/Makefile"), "libs-y += arch/x86/lib/\n").unwrap();

        validate_makefiles(root).unwrap();
    }

    #[test]
    fn test_make_selftest_ignores_non_build_graph_assignments() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(
            root.join("Makefile"),
            "ARCH_PROCESSED := $(shell echo $(ARCH) | sed -e s/i.86/x86/)\nobj-y += keep/\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("keep")).unwrap();

        validate_makefiles(root).unwrap();
    }

    #[test]
    fn test_make_selftest_ignores_recipe_and_define_assignment_bodies() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(
            root.join("Makefile"),
            concat!(
                "define macro_body\n",
                "obj-y += missing-from-define.o\n",
                "endef\n",
                "all:\n",
                "\tobj-y += missing-from-recipe.o\n",
                "obj-y += keep/\n",
            ),
        )
        .unwrap();
        std::fs::create_dir_all(root.join("keep")).unwrap();

        validate_makefiles(root).unwrap();
    }

    #[test]
    fn test_make_selftest_rejects_missing_object_provider() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("Makefile"), "obj-y += missing.o\n").unwrap();

        let err = validate_makefiles(root).unwrap_err().to_string();
        assert!(err.contains("missing.o"), "unexpected error: {}", err);
    }

    #[test]
    fn test_kconfig_selftest_rejects_missing_source() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(
            root.join("Kconfig"),
            "source \"drivers/gpu/drm/amd/amdgpu/Kconfig\"\n",
        )
        .unwrap();

        let err = validate_kconfig_sources(root).unwrap_err().to_string();
        assert!(
            err.contains("missing Kconfig source"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_kernel_build_selftest_runs_config_and_targets() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("Makefile"), "all:\n\t@:\n").unwrap();
        let script = root.join("fake-make.sh");
        let log = root.join("make.log");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nexit 0\n",
                log.display()
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            let mut perms = std::fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script, perms).unwrap();
        }

        let config = SelfTestConfig {
            enabled: true,
            check_kconfig_sources: false,
            check_makefiles: false,
            kernel_builds: vec![KernelBuildConfig {
                name: Some("probe".to_string()),
                config_target: Some("defconfig".to_string()),
                targets: vec!["drivers/gpu/drm/".to_string(), "modules".to_string()],
                output_dir: Some("out/probe".to_string()),
                jobs: Some(4),
                clean: true,
                make_program: Some("./fake-make.sh".to_string()),
                make_args: vec!["V=1".to_string()],
                env: Default::default(),
            }],
            commands: Vec::new(),
        };

        let result = run(root.to_str().unwrap(), &config).unwrap();
        assert_eq!(result.built_in_checks, 1);
        assert_eq!(result.kernel_builds_run, 1);
        assert_eq!(result.commands_run, 0);

        let output = std::fs::read_to_string(log).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("O="));
        assert!(lines[0].contains("-j4"));
        assert!(lines[0].contains("V=1"));
        assert!(lines[0].contains("defconfig"));
        assert!(lines[1].contains("drivers/gpu/drm/"));
        assert!(lines[1].contains("modules"));
        assert!(root.join("out/probe").exists());
    }

    #[test]
    fn test_kernel_build_selftest_reports_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("Makefile"), "all:\n\t@:\n").unwrap();
        let script = root.join("fake-make.sh");
        std::fs::write(
            &script,
            "#!/bin/sh\ncase \"$*\" in\n  *defconfig*) exit 7 ;;\nesac\nexit 0\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            let mut perms = std::fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script, perms).unwrap();
        }

        let config = SelfTestConfig {
            enabled: true,
            check_kconfig_sources: false,
            check_makefiles: false,
            kernel_builds: vec![KernelBuildConfig {
                name: Some("broken".to_string()),
                config_target: Some("defconfig".to_string()),
                targets: Vec::new(),
                output_dir: None,
                jobs: None,
                clean: true,
                make_program: Some("./fake-make.sh".to_string()),
                make_args: Vec::new(),
                env: Default::default(),
            }],
            commands: Vec::new(),
        };

        let err = run(root.to_str().unwrap(), &config)
            .unwrap_err()
            .to_string();
        assert!(err.contains("kernel build selftest 'broken' failed"));
        assert!(err.contains("defconfig"));
    }

    #[test]
    fn test_kernel_build_selftest_rejects_source_root_output_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("Makefile"), "all:\n\t@:\n").unwrap();

        let err = run(
            root.to_str().unwrap(),
            &SelfTestConfig {
                enabled: true,
                check_kconfig_sources: false,
                check_makefiles: false,
                kernel_builds: vec![KernelBuildConfig {
                    name: Some("dangerous".to_string()),
                    config_target: Some("defconfig".to_string()),
                    targets: Vec::new(),
                    output_dir: Some(".".to_string()),
                    jobs: None,
                    clean: true,
                    make_program: None,
                    make_args: Vec::new(),
                    env: Default::default(),
                }],
                commands: Vec::new(),
            },
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("invalid kernel build output dir"));
        assert!(err.contains("must not be the source root"));
        assert!(root.join("Makefile").exists());
    }

    #[test]
    fn test_kernel_build_selftest_rejects_invalid_arch_before_clean() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let output_dir = root.join("build");
        std::fs::create_dir_all(&output_dir).unwrap();
        std::fs::write(root.join("Makefile"), "all:\n\t@:\n").unwrap();
        std::fs::write(output_dir.join("marker"), "keep\n").unwrap();

        let err = run(
            root.to_str().unwrap(),
            &SelfTestConfig {
                enabled: true,
                check_kconfig_sources: false,
                check_makefiles: false,
                kernel_builds: vec![KernelBuildConfig {
                    name: Some("bad-arch".to_string()),
                    config_target: Some("defconfig".to_string()),
                    targets: Vec::new(),
                    output_dir: Some("build".to_string()),
                    jobs: None,
                    clean: true,
                    make_program: None,
                    make_args: Vec::new(),
                    env: [("ARCH".to_string(), "x86/../../host".to_string())]
                        .into_iter()
                        .collect(),
                }],
                commands: Vec::new(),
            },
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("invalid kernel build ARCH"));
        assert!(err.contains("invalid characters"));
        assert!(output_dir.join("marker").exists());
    }

    #[test]
    fn test_run_capture_returns_structured_command_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let err = run_capture(
            root.to_str().unwrap(),
            &SelfTestConfig {
                enabled: true,
                check_kconfig_sources: false,
                check_makefiles: false,
                kernel_builds: Vec::new(),
                commands: vec!["printf 'hi'; printf 'boom' >&2; exit 7".to_string()],
            },
        )
        .unwrap_err();

        match err {
            SelfTestFailure::Command { details } => {
                assert_eq!(details.command, "printf 'hi'; printf 'boom' >&2; exit 7");
                assert_eq!(details.exit_status, Some(7));
                assert_eq!(details.stdout, "hi");
                assert_eq!(details.stderr, "boom");
                assert!(details.elapsed >= Duration::ZERO);
            }
            other => panic!("expected command failure, got {other:?}"),
        }
    }

    #[test]
    fn test_run_capture_returns_structured_kernel_build_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("Makefile"), "all:\n\t@:\n").unwrap();
        let script = root.join("fake-make.sh");
        std::fs::write(
            &script,
            "#!/bin/sh\nprintf '%s' \"$*\"\nprintf 'bad build' >&2\nexit 9\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            let mut perms = std::fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script, perms).unwrap();
        }

        let err = run_capture(
            root.to_str().unwrap(),
            &SelfTestConfig {
                enabled: true,
                check_kconfig_sources: false,
                check_makefiles: false,
                kernel_builds: vec![KernelBuildConfig {
                    name: Some("broken".to_string()),
                    config_target: Some("defconfig".to_string()),
                    targets: vec!["modules".to_string()],
                    output_dir: None,
                    jobs: None,
                    clean: true,
                    make_program: Some("./fake-make.sh".to_string()),
                    make_args: Vec::new(),
                    env: [("ARCH".to_string(), "arm64".to_string())]
                        .into_iter()
                        .collect(),
                }],
                commands: Vec::new(),
            },
        )
        .unwrap_err();

        match err {
            SelfTestFailure::KernelBuild {
                label,
                output_dir: _,
                details,
            } => {
                assert_eq!(label, "broken");
                assert!(details.command.ends_with("fake-make.sh"));
                assert_eq!(details.arch.as_deref(), Some("arm64"));
                assert_eq!(details.config.as_deref(), Some("defconfig"));
                assert_eq!(details.exit_status, Some(9));
                assert!(details
                    .target
                    .as_deref()
                    .is_some_and(|t| t.contains("defconfig")));
                assert!(details.stderr.contains("bad build"));
                assert!(details.elapsed >= Duration::ZERO);
            }
            other => panic!("expected kernel build failure, got {other:?}"),
        }
    }

    #[test]
    fn test_selftest_run_executes_custom_commands_and_counts_them() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let marker = root.join("command-ran");

        let config = SelfTestConfig {
            enabled: true,
            check_kconfig_sources: false,
            check_makefiles: false,
            kernel_builds: Vec::new(),
            commands: vec![format!("touch {}", marker.display())],
        };

        let result = run(root.to_str().unwrap(), &config).unwrap();

        assert_eq!(result.built_in_checks, 0);
        assert_eq!(result.kernel_builds_run, 0);
        assert_eq!(result.commands_run, 1);
        assert!(marker.exists());
    }

    #[test]
    fn test_selftest_run_preserves_disabled_noop_behavior() {
        let tmp = tempfile::tempdir().unwrap();
        let result = run(
            tmp.path().to_str().unwrap(),
            &SelfTestConfig {
                enabled: false,
                check_kconfig_sources: true,
                check_makefiles: true,
                kernel_builds: vec![KernelBuildConfig {
                    name: None,
                    config_target: None,
                    targets: Vec::new(),
                    output_dir: None,
                    jobs: None,
                    clean: true,
                    make_program: None,
                    make_args: Vec::new(),
                    env: Default::default(),
                }],
                commands: vec!["false".to_string()],
            },
        )
        .unwrap();

        assert!(!result.enabled);
        assert_eq!(result.built_in_checks, 0);
        assert_eq!(result.kernel_builds_run, 0);
        assert_eq!(result.commands_run, 0);
    }
}
