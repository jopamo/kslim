use super::common::*;

#[test]
fn execution_module_owns_process_execution_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let execution_mod = production_source(&root.join("src/execution/mod.rs"));
    let argv = production_source(&root.join("src/execution/argv.rs"));
    let environment = production_source(&root.join("src/execution/environment.rs"));
    let timeout = production_source(&root.join("src/execution/timeout.rs"));
    let cleanup = production_source(&root.join("src/execution/cleanup.rs"));
    let log_capture = production_source(&root.join("src/execution/log_capture.rs"));
    let process = production_source(&root.join("src/process.rs"));

    assert!(
        main.contains("mod execution;") && main.contains("mod process;"),
        "main.rs should register owned execution module and legacy process facade"
    );

    for module in [
        "mod argv;",
        "mod cleanup;",
        "mod environment;",
        "mod log_capture;",
        "mod timeout;",
    ] {
        assert!(
            execution_mod.contains(module),
            "src/execution/mod.rs should register execution submodule {module}"
        );
    }
    for export in [
        "pub(crate) use argv::{run, run_in_dir, run_quiet, CommandSpec};",
        "pub(crate) use cleanup::ProcessCleanup;",
        "pub(crate) use environment::EnvironmentAllowlist;",
        "pub(crate) use log_capture::CapturedCommandOutput;",
        "pub(crate) use timeout::ExecutionTimeout;",
    ] {
        assert!(
            execution_mod.contains(export),
            "src/execution/mod.rs should re-export execution boundary item {export}"
        );
    }

    assert!(
        process.contains("pub(crate) use crate::execution::{run, run_in_dir};")
            && process.contains("pub(crate) use crate::execution::run_quiet;")
            && !process.contains("Command::new"),
        "src/process.rs should remain only a compatibility facade over src/execution/*"
    );

    for required in [
        "pub(crate) struct CommandSpec",
        "program: String",
        "args: Vec<String>",
        "current_dir: Option<String>",
        "Command::new(&self.program)",
        ".args(&self.args)",
        "command.current_dir(dir)",
        ".stdout(Stdio::piped())",
        ".stderr(Stdio::piped())",
        "child.try_wait()?",
        "child.wait_with_output()?",
        "pub(crate) fn run(cmd: &str, args: &[&str]) -> Result<String>",
        "pub(crate) fn run_in_dir(dir: &str, cmd: &str, args: &[&str]) -> Result<String>",
    ] {
        assert!(
            argv.contains(required),
            "src/execution/argv.rs should own argv execution item {required}"
        );
    }

    for required in [
        "pub(crate) enum EnvironmentAllowlist",
        "InheritParent",
        "AllowOnly(BTreeSet<String>)",
        "pub(crate) fn allow_only",
        "command.env_clear();",
        "std::env::var_os(name)",
    ] {
        assert!(
            environment.contains(required),
            "src/execution/environment.rs should own environment allowlist item {required}"
        );
    }

    for required in [
        "pub(crate) enum ExecutionTimeout",
        "Disabled",
        "After(Duration)",
        "pub(crate) fn after(duration: Duration)",
        "pub(crate) fn duration(self) -> Option<Duration>",
    ] {
        assert!(
            timeout.contains(required),
            "src/execution/timeout.rs should own timeout item {required}"
        );
    }

    for required in [
        "pub(crate) struct ProcessCleanup",
        "kill_on_timeout: bool",
        "pub(crate) fn cleanup_timed_out_child",
        "child.kill()",
    ] {
        assert!(
            cleanup.contains(required),
            "src/execution/cleanup.rs should own process cleanup item {required}"
        );
    }

    for required in [
        "pub(crate) struct CapturedCommandOutput",
        "pub(crate) status: ExitStatus",
        "pub(crate) stdout: Vec<u8>",
        "pub(crate) stderr: Vec<u8>",
        "pub(crate) fn from_output(output: Output) -> Self",
        "pub(crate) fn stdout_trimmed_lossy(&self) -> String",
        "pub(crate) fn stderr_lossy(&self) -> String",
    ] {
        assert!(
            log_capture.contains(required),
            "src/execution/log_capture.rs should own log capture item {required}"
        );
    }

    let execution_sources = [execution_mod, argv, environment, timeout, cleanup, log_capture].join("\n");
    for forbidden in [
        "crate::cli",
        "crate::commands",
        "crate::generate",
        "crate::reducer",
        "crate::publish",
        "crate::output_repo",
        "crate::kconfig",
        "crate::kbuild",
        "crate::feature",
        "crate::tree_index",
    ] {
        assert!(
            !execution_sources.contains(forbidden),
            "src/execution/* should not depend on CLI, command, lifecycle, output, or kernel-domain logic; found {forbidden}"
        );
    }
}
