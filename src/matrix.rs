use std::collections::BTreeMap;

use anyhow::Result;

use crate::config;

pub(crate) fn normalize_cli_matrix(matrix: &str) -> Result<String> {
    let normalized = matrix.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" => anyhow::bail!("cli --matrix must not be empty"),
        "default" | "extended" | "hardening" | "runtime" => Ok(normalized),
        _ => anyhow::bail!(
            "cli --matrix is invalid: expected default, extended, hardening, or runtime"
        ),
    }
}

pub(crate) fn apply_cli_matrix_override(
    profile: &mut config::ProfileConfig,
    matrix: Option<&str>,
) -> Result<()> {
    let Some(matrix) = matrix else {
        return Ok(());
    };
    match normalize_cli_matrix(matrix)?.as_str() {
        "default" => {}
        "extended" => {
            profile.selftests.enabled = true;
        }
        "hardening" => {
            profile.selftests.enabled = true;
            profile.selftests.check_kconfig_sources = true;
            profile.selftests.check_makefiles = true;
        }
        "runtime" => {
            profile.selftests.enabled = true;
            profile.selftests.check_kconfig_sources = false;
            profile.selftests.check_makefiles = false;
            profile.selftests.kernel_builds.clear();
        }
        _ => unreachable!("normalize_cli_matrix accepts only supported matrix names"),
    }
    Ok(())
}

pub(crate) fn print_summary(
    profile_name: &str,
    profile: &config::ProfileConfig,
    matrix: Option<&str>,
) {
    println!("matrix");
    println!("profile: {}", profile_name);
    println!("selected matrix: {}", matrix.unwrap_or("profile"));
    println!("effective source: selftests");
    println!("future build matrix:");
    println!("  enabled: {}", profile.build_matrix.enabled);
    println!("  presets: {}", comma_list(&profile.build_matrix.presets));
    println!("  arches: {}", comma_list(&profile.build_matrix.arches));
    println!(
        "  config targets: {}",
        comma_list(&profile.build_matrix.config_targets)
    );
    println!("  targets: {}", comma_list(&profile.build_matrix.targets));
    println!(
        "  randconfig seed: {}",
        profile
            .build_matrix
            .randconfig_seed
            .as_deref()
            .unwrap_or("(none)")
    );
    println!("  jobs: {}", optional_usize(profile.build_matrix.jobs));
    println!("  fail on error: {}", profile.build_matrix.fail_on_error);
    println!("future runtime matrix:");
    println!("  enabled: {}", profile.runtime_matrix.enabled);
    println!(
        "  boot arches: {}",
        comma_list(&profile.runtime_matrix.boot_arches)
    );
    println!(
        "  qemu machines: {}",
        comma_list(&profile.runtime_matrix.qemu_machines)
    );
    println!(
        "  kunit suites: {}",
        comma_list(&profile.runtime_matrix.kunit_suites)
    );
    println!(
        "  kselftest targets: {}",
        comma_list(&profile.runtime_matrix.kselftest_targets)
    );
    println!("  module smoke: {}", profile.runtime_matrix.module_smoke);
    println!(
        "  require clean dmesg: {}",
        profile.runtime_matrix.require_clean_dmesg
    );
    println!(
        "  boot timeout seconds: {}",
        optional_u64(profile.runtime_matrix.boot_timeout_seconds)
    );
    println!("  fail on error: {}", profile.runtime_matrix.fail_on_error);
    print_selftest_matrix(&profile.selftests);
}

fn print_selftest_matrix(selftests: &config::SelfTestConfig) {
    println!("selected selftests:");
    println!("  enabled: {}", selftests.enabled);
    println!(
        "  check kconfig sources: {}",
        selftests.check_kconfig_sources
    );
    println!("  check makefiles: {}", selftests.check_makefiles);
    println!("  kernel builds: {}", selftests.kernel_builds.len());
    for (idx, build) in selftests.kernel_builds.iter().enumerate() {
        print_kernel_build_matrix_entry(idx + 1, build);
    }
    println!("  commands: {}", selftests.commands.len());
    for command in &selftests.commands {
        println!("    - {}", command);
    }
}

fn print_kernel_build_matrix_entry(index: usize, build: &config::KernelBuildConfig) {
    println!(
        "    - {}",
        build
            .name
            .as_deref()
            .map(str::to_string)
            .unwrap_or_else(|| format!("#{index}"))
    );
    println!(
        "      arch: {}",
        build
            .env
            .get("ARCH")
            .map(String::as_str)
            .unwrap_or("(none)")
    );
    println!(
        "      config target: {}",
        build.config_target.as_deref().unwrap_or("(none)")
    );
    println!("      targets: {}", comma_list(&build.targets));
    println!(
        "      output dir: {}",
        build.output_dir.as_deref().unwrap_or("(none)")
    );
    println!("      jobs: {}", optional_usize(build.jobs));
    println!("      clean: {}", build.clean);
    println!(
        "      make program: {}",
        build.make_program.as_deref().unwrap_or("(default)")
    );
    println!("      make args: {}", comma_list(&build.make_args));
    println!("      env: {}", env_list(&build.env));
}

fn comma_list(values: &[String]) -> String {
    if values.is_empty() {
        "(none)".to_string()
    } else {
        values.join(", ")
    }
}

fn optional_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "(none)".to_string())
}

fn optional_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "(none)".to_string())
}

fn env_list(values: &BTreeMap<String, String>) -> String {
    if values.is_empty() {
        return "(none)".to_string();
    }
    values
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(", ")
}
