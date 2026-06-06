//! Generate plan-report and dry-run rendering helpers.
//!
//! This module owns read-only plan/candidate summaries for dry-run, deep
//! dry-run, and report-only modes. It does not materialize candidates, verify
//! candidate metadata, commit output, or publish state.

use anyhow::Result;
use std::path::Path;

use crate::config::ConfigSourceMap;

use super::plan::{self, GeneratePlanSourceMaps};
use super::{
    ensure_non_authoritative_attempt_path, project_attempt_metadata_dir,
    project_failure_report_path, remove_optional_dir, GenerateResult, GenerateStage,
    GeneratedArtifacts, VerifiedGeneratedOutput,
};

pub(super) fn dry_run_result_from_plan(generate_plan: &plan::GeneratePlan) -> GenerateResult {
    let resolved = &generate_plan.resolved;
    println!();
    println!("  upstream access:       direct read-only");
    println!("  would verify upstream: {}", resolved.base.url);
    println!(
        "  would resolve base:   {} -> {}",
        resolved.base.r#ref, resolved.base.commit
    );
    println!("  upstream commit date: {}", resolved.base.resolved_at);
    println!("  would materialize tree: yes");
    println!(
        "  output path:           {}",
        resolved.output_plan.output_path.as_path().display()
    );
    if resolved.patch_plan.sources.is_empty() {
        println!("  would apply patches:   none");
    } else {
        println!(
            "  would apply patches:   {} commit(s) from {} source(s)",
            resolved.patch_plan.total_patch_count,
            resolved.patch_plan.sources.len()
        );
        for source in &resolved.patch_plan.sources {
            println!(
                "    - {} ({}) [{} commit(s)]",
                source.branch, source.worktree_path, source.patch_count
            );
        }
    }
    println!();
    println!("  target branch: {}", resolved.output_plan.branch);

    GenerateResult {
        committed: false,
        stage: GenerateStage::Resolve,
        branch: resolved.output_plan.branch.clone(),
        tag: None,
        output_commit: None,
        file_count: 0,
        total_bytes: 0,
        patch_count: resolved.patch_plan.total_patch_count,
        selftests_enabled: generate_plan.requested.cli_overrides.run_selftests,
        built_in_selftests: 0,
        selftest_commands: 0,
    }
}

pub(super) fn deep_dry_run_result_from_candidate(
    generate_plan: &plan::GeneratePlan,
    generated: &GeneratedArtifacts,
    verified: &VerifiedGeneratedOutput,
) -> GenerateResult {
    let resolved = &generate_plan.resolved;
    let (selftests_enabled, built_in_selftests, selftest_commands) = verified
        .selftests()
        .map(|result| (result.enabled, result.built_in_checks, result.commands_run))
        .unwrap_or((false, 0, 0));
    println!();
    println!("  deep dry-run:         candidate verified without publishing");
    println!(
        "  would publish path:   {}",
        resolved.output_plan.output_path.as_path().display()
    );
    println!("  target branch:        {}", resolved.output_plan.branch);
    println!("  files:                {}", generated.file_count);
    println!("  bytes:                {}", generated.total_bytes);
    println!(
        "  patches:              {}",
        resolved.patch_plan.total_patch_count
    );
    if selftests_enabled {
        println!(
            "  selftests:            {} built-in, {} custom",
            built_in_selftests, selftest_commands
        );
    }

    GenerateResult {
        committed: false,
        stage: GenerateStage::Metadata,
        branch: resolved.output_plan.branch.clone(),
        tag: None,
        output_commit: None,
        file_count: generated.file_count,
        total_bytes: generated.total_bytes,
        patch_count: resolved.patch_plan.total_patch_count,
        selftests_enabled,
        built_in_selftests,
        selftest_commands,
    }
}

pub(super) fn report_only_result_from_plan(
    generate_plan: &plan::GeneratePlan,
    project_root: Option<&Path>,
) -> Result<GenerateResult> {
    let resolved = &generate_plan.resolved;
    if let Some(project_root) = project_root {
        write_report_only_plan_report(project_root, generate_plan)?;
    }
    println!();
    println!("  report-only plan:     {}", generate_plan.plan_id.as_str());
    if let Some(project_root) = project_root {
        println!(
            "  report path:          {}",
            project_failure_report_path(project_root).display()
        );
    }
    println!(
        "  base:                 {} -> {}",
        resolved.base.r#ref, resolved.base.commit
    );
    println!("  target branch:        {}", resolved.output_plan.branch);

    Ok(GenerateResult {
        committed: false,
        stage: GenerateStage::Resolve,
        branch: resolved.output_plan.branch.clone(),
        tag: None,
        output_commit: None,
        file_count: 0,
        total_bytes: 0,
        patch_count: resolved.patch_plan.total_patch_count,
        selftests_enabled: generate_plan.requested.cli_overrides.run_selftests,
        built_in_selftests: 0,
        selftest_commands: 0,
    })
}

fn write_report_only_plan_report(
    project_root: &Path,
    generate_plan: &plan::GeneratePlan,
) -> Result<()> {
    let attempt_dir = project_attempt_metadata_dir(project_root);
    ensure_non_authoritative_attempt_path(project_root, &attempt_dir)?;
    remove_optional_dir(&attempt_dir)?;
    crate::fsutil::ensure_dir(&attempt_dir)?;

    let report_path = project_failure_report_path(project_root);
    ensure_non_authoritative_attempt_path(project_root, &report_path)?;
    std::fs::write(report_path, render_report_only_plan_report(generate_plan))?;
    Ok(())
}

fn render_report_only_plan_report(generate_plan: &plan::GeneratePlan) -> String {
    let resolved = &generate_plan.resolved;
    let stage = GenerateStage::Resolve;
    let source_map_section =
        render_report_only_source_map_section(generate_plan.source_maps.as_ref());
    let patch_section = if resolved.patch_plan.sources.is_empty() {
        String::from("Patches: none\n")
    } else {
        let mut section = format!(
            "Patches:\n  Sources: {}\n  Total count: {}\n",
            resolved.patch_plan.sources.len(),
            resolved.patch_plan.total_patch_count
        );
        for source in &resolved.patch_plan.sources {
            section.push_str(&format!(
                "  - {} ({}) [{} commit(s)]\n",
                source.branch, source.worktree_path, source.patch_count
            ));
        }
        section
    };
    format!(
        concat!(
            "kslim report\n",
            "============\n\n",
            "Status: report-only\n",
            "Authoritative: false\n",
            "Metadata scope: non-authoritative-attempt\n",
            "Plan ID: {}\n",
            "Plan fingerprint: {}\n",
            "Config content hash: {}\n",
            "Tool version: {}\n",
            "Profile: {}\n",
            "Mode: {}\n",
            "Upstream: {}\n",
            "Base ref: {}\n",
            "Base commit: {}\n",
            "Stage: {}\n",
            "Output path: {}\n",
            "Target branch: {}\n\n",
            "{}",
            "{}",
        ),
        generate_plan.plan_id.as_str(),
        generate_plan.fingerprint.as_str(),
        generate_plan.config_content_hash.as_str(),
        generate_plan.created_with.as_str(),
        generate_plan.requested.selected_profile.as_str(),
        resolved.output_plan.mode,
        resolved.base.url,
        resolved.base.r#ref,
        resolved.base.commit,
        render_generate_stage_for_report(stage),
        resolved.output_plan.output_path.as_path().display(),
        resolved.output_plan.branch,
        source_map_section,
        patch_section,
    )
}

fn render_report_only_source_map_section(
    source_maps: Option<&GeneratePlanSourceMaps>,
) -> String {
    let Some(source_maps) = source_maps else {
        return String::from("Source map: unavailable\n\n");
    };
    if source_maps.is_empty() {
        return String::from("Source map: empty\n\n");
    }

    let mut section = String::from("Source map:\n");
    render_source_map_group(&mut section, "Config", &source_maps.config);
    render_source_map_group(&mut section, "Profile", &source_maps.profile);
    render_source_map_group(&mut section, "Overrides", &source_maps.overrides);
    section.push('\n');
    section
}

fn render_source_map_group(section: &mut String, label: &str, source_map: &ConfigSourceMap) {
    section.push_str(&format!("  {label}:\n"));
    if source_map.is_empty() {
        section.push_str("    <empty>\n");
        return;
    }
    for (path, source) in source_map.iter() {
        section.push_str(&format!(
            "    {}: {} ({})\n",
            render_source_map_report_value(path),
            source.kind.as_str(),
            render_source_map_report_value(&source.source)
        ));
    }
}

fn render_source_map_report_value(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

fn render_generate_stage_for_report(stage: GenerateStage) -> &'static str {
    stage.as_str()
}

