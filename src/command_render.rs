use anyhow::Result;
use std::path::Path;

use crate::config;
use crate::feature::{FeatureConflictReport, FeatureImpactReport};
use crate::generate::{self, GenerateOptions, GenerateStage};
use crate::paths::KernelSourceRoot;
use crate::reducer;

pub(crate) fn print_plan_summary(summary: &generate::GeneratePlanSummary) {
    println!("plan: {}", summary.plan_id);
    println!("fingerprint: {}", summary.fingerprint);
    println!("config hash: {}", summary.config_content_hash);
    println!("tool version: {}", summary.tool_version);
    println!("profile: {}", summary.profile);
    println!("base: {} -> {}", summary.base_ref, summary.base_commit);
    println!("output:");
    println!("  path: {}", summary.output_path);
    println!("  branch: {}", summary.output_branch);
    println!("  mode: {}", summary.mode);
    println!("inputs:");
    println!("  patch sources: {}", summary.patch_sources);
    println!("  patches: {}", summary.patch_count);
    println!("  integrations: {}", summary.integration_count);
    println!("features:");
    println!("  source: {}", summary.feature_source);
    println!("  remove paths: {}", summary.remove_path_count);
    println!("  remove configs: {}", summary.remove_config_count);
    println!("  preserve paths: {}", summary.preserve_path_count);
    println!("  preserve configs: {}", summary.preserve_config_count);
    println!("selftests:");
    println!("  enabled: {}", summary.selftests_enabled);
    println!("  kernel builds: {}", summary.kernel_build_count);
    println!("  commands: {}", summary.selftest_command_count);
}

pub(crate) fn print_reduce_tree_result(
    profile_name: &str,
    tree: &KernelSourceRoot,
    result: &reducer::ReducerResult,
) {
    println!(
        "reduce-tree: {}",
        if result.stats.ran { "done" } else { "no-op" }
    );
    println!("profile: {}", profile_name);
    println!("tree: {}", tree.as_path().display());
    println!("status: {}", result.status.stable_name());
    println!("publishable: {}", result.publishable);
    println!("convergence: {}", result.convergence.stable_name());
    println!("edits:");
    println!("  total: {}", result.edit_summary.total_edits);
    println!("  files removed: {}", result.edit_summary.files_removed);
    println!("  dirs removed: {}", result.edit_summary.dirs_removed);
    println!(
        "  configs disabled: {}",
        result.edit_summary.configs_disabled
    );
    println!(
        "  defaults overridden: {}",
        result.edit_summary.defaults_overridden
    );
    println!(
        "  kconfig refs removed: {}",
        result.edit_summary.kconfig_refs_removed
    );
    println!(
        "  makefile refs removed: {}",
        result.edit_summary.makefile_refs_removed
    );
    println!(
        "  cpp branches folded: {}",
        result.edit_summary.cpp_branches_folded
    );
    println!(
        "  include lines removed: {}",
        result.edit_summary.include_lines_removed
    );
    println!("diagnostics:");
    println!(
        "  unsupported kconfig expressions: {}",
        result.diagnostic_summary.unsupported_kconfig_expressions
    );
    println!(
        "  unsupported cpp expressions: {}",
        result.diagnostic_summary.unsupported_cpp_expressions
    );
    println!(
        "  skipped cpp nested edge cases: {}",
        result.diagnostic_summary.skipped_cpp_nested_edge_cases
    );
    println!(
        "  skipped makefile lines: {}",
        result.diagnostic_summary.skipped_makefile_lines
    );
    println!(
        "  skipped fixups: {}",
        result.diagnostic_summary.skipped_fixups
    );
    println!(
        "  unknown diagnostics: {}",
        result.diagnostic_summary.unknown_diagnostics
    );
}

pub(crate) fn print_feature_conflicts(report: &FeatureConflictReport) {
    println!("feature conflicts:");
    println!("  total: {}", report.len());
    println!("  blocking: {}", report.blocking_count());
    if report.is_empty() {
        println!("  (none)");
        return;
    }
    for conflict in report.conflicts() {
        println!("  - {}", conflict.stable_key());
        println!("    kind: {}", conflict.kind().stable_name());
        println!("    feature: {}", conflict.feature().as_str());
        println!("    subject: {}", conflict.subject().as_str());
        println!("    summary: {}", conflict.summary());
        println!("    action: {}", conflict.suggested_action());
        println!("    strict blocking: {}", conflict.strict_blocking());
    }
}

pub(crate) fn print_generate_result(
    opts: &GenerateOptions,
    result: generate::GenerateResult,
) -> Result<()> {
    if opts.dry_run || opts.deep_dry_run || opts.report_only {
        return Ok(());
    }
    let stage = render_generate_stage_for_ci_summary(result.stage);
    if result.committed {
        println!("Generated commit on branch '{}'", result.branch);
        println!("  stage: {}", stage);
        println!("  files: {}", result.file_count);
        println!("  bytes: {}", result.total_bytes);
        println!("  patches: {}", result.patch_count);
        if result.selftests_enabled {
            println!(
                "  selftests: {} built-in, {} custom",
                result.built_in_selftests, result.selftest_commands
            );
        }
    } else {
        println!(
            "No changes (idempotent generate) on branch '{}'",
            result.branch
        );
        println!("  stage: {}", stage);
    }
    Ok(())
}

pub(crate) fn print_frozen_plan_verification(path: &Path, inputs: &generate::FrozenPlanInputs) {
    println!("frozen-plan: verified");
    println!("  path: {}", path.display());
    println!("  plan: {}", inputs.plan_id);
    println!("  fingerprint: {}", inputs.fingerprint);
    println!(
        "  base: {} -> {}",
        inputs.resolved_base.r#ref, inputs.resolved_base.commit
    );
    println!("  output branch: {}", inputs.output_branch);
}

pub(crate) fn print_explain_feature_intent(
    feature: &str,
    mode: &str,
    intent: &config::FeatureIntentConfig,
    profile: &config::ProfileConfig,
) -> Result<()> {
    let impact = FeatureImpactReport::for_feature(profile, feature)?;
    println!("decision: {}", feature_decision(mode));
    println!("owner: profile features.{}.{}", mode, feature);
    println!(
        "proof source: profile feature intent features.{}.{}",
        mode, feature
    );
    println!(
        "kind: {}",
        intent.kind.as_deref().unwrap_or("(unspecified)")
    );
    println!("roots: {}", comma_list(&intent.roots));
    println!("configs: {}", comma_list(&intent.configs));
    println!("exported symbols: {}", comma_list(&intent.exported_symbols));
    println!("module names: {}", comma_list(&intent.module_names));
    println!("module aliases: {}", comma_list(&intent.module_aliases));
    println!(
        "device compatibles: {}",
        comma_list(&intent.device_compatibles)
    );
    println!("ACPI IDs: {}", comma_list(&intent.acpi_ids));
    println!("PCI IDs: {}", comma_list(&intent.pci_ids));
    println!("USB IDs: {}", comma_list(&intent.usb_ids));
    println!("firmware paths: {}", comma_list(&intent.firmware_paths));
    println!("initcalls: {}", comma_list(&intent.initcalls));
    println!(
        "runtime registrations: {}",
        comma_list(&intent.runtime_registrations)
    );
    println!("docs: {}", comma_list(&intent.docs));
    println!("tools: {}", comma_list(&intent.tools));
    println!("samples: {}", comma_list(&intent.samples));
    println!("KUnit suites: {}", comma_list(&intent.kunit_suites));
    println!(
        "kselftest targets: {}",
        comma_list(&intent.kselftest_targets)
    );
    println!("remove paths: {}", comma_list(&intent.remove_paths));
    println!("remove configs: {}", comma_list(&intent.remove_configs));
    println!(
        "remove exported symbols: {}",
        comma_list(&intent.remove_exported_symbols)
    );
    println!(
        "remove module names: {}",
        comma_list(&intent.remove_module_names)
    );
    println!(
        "remove module aliases: {}",
        comma_list(&intent.remove_module_aliases)
    );
    println!(
        "remove device compatibles: {}",
        comma_list(&intent.remove_device_compatibles)
    );
    println!("remove ACPI IDs: {}", comma_list(&intent.remove_acpi_ids));
    println!("remove PCI IDs: {}", comma_list(&intent.remove_pci_ids));
    println!("remove USB IDs: {}", comma_list(&intent.remove_usb_ids));
    println!(
        "remove firmware paths: {}",
        comma_list(&intent.remove_firmware_paths)
    );
    println!("remove initcalls: {}", comma_list(&intent.remove_initcalls));
    println!(
        "remove runtime registrations: {}",
        comma_list(&intent.remove_runtime_registrations)
    );
    println!("remove docs: {}", comma_list(&intent.remove_docs));
    println!("remove tools: {}", comma_list(&intent.remove_tools));
    println!("remove samples: {}", comma_list(&intent.remove_samples));
    println!(
        "remove KUnit suites: {}",
        comma_list(&intent.remove_kunit_suites)
    );
    println!(
        "remove kselftest targets: {}",
        comma_list(&intent.remove_kselftest_targets)
    );
    println!("arch scope: {}", comma_list(&intent.arch_scope));
    println!("safety: {}", intent.safety.unwrap_or_default().as_str());
    println!(
        "allow public headers: {}",
        intent.allow_public_header_removal
    );
    println!("allow uapi headers: {}", intent.allow_uapi_header_removal);
    println!("require clean boot: {}", intent.require_clean_boot);
    println!("report only: {}", intent.report_only);
    println!("effective impact:");
    println!("  remove paths: {}", impact.remove_paths());
    println!("  remove configs: {}", impact.remove_configs());
    println!("  default overrides: {}", impact.default_overrides());
    println!("  preserve paths: {}", impact.preserve_paths());
    println!("  preserve configs: {}", impact.preserve_configs());
    Ok(())
}

fn render_generate_stage_for_ci_summary(stage: GenerateStage) -> &'static str {
    stage.as_str()
}

fn feature_decision(mode: &str) -> &'static str {
    match mode {
        "remove" => "removed",
        "preserve" => "preserved",
        _ => "declared",
    }
}

fn comma_list(values: &[String]) -> String {
    if values.is_empty() {
        "(none)".to_string()
    } else {
        values.join(", ")
    }
}
