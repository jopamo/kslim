use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::edit_reason::{
    proof_source_kind_for_reason_key, validate_reported_no_speculative_fallout_edit,
    validate_reported_proof_source_payload_for_reason,
};

use super::super::plan::GeneratePlan;
use super::super::state::CandidateTreeState;
use super::metadata::CandidateMetadataSummary;
use super::report::{
    read_reducer_diagnostics, read_reducer_edit_summary, read_reducer_report,
    ReducerClassifiedDiagnostic, ReducerEditRecord, ReducerEditTruth,
    ReducerSkippedFixupDiagnostic,
};

pub(super) fn verify_reducer_success(
    plan: &GeneratePlan,
    candidate: &CandidateTreeState,
    metadata: &CandidateMetadataSummary,
) -> Result<bool> {
    if !super::plan_requires_reducer(plan) {
        return Ok(true);
    }
    if !candidate.reduced {
        anyhow::bail!("verification failed: reducer is enabled but candidate state is not reduced");
    }

    if !metadata.reduced || !metadata.reducer_ran {
        anyhow::bail!(
            "verification failed: reducer is enabled but candidate metadata does not record reducer success"
        );
    }
    Ok(true)
}

pub(super) fn verify_no_unreasoned_edits(
    plan: &GeneratePlan,
    candidate: &CandidateTreeState,
    metadata: &CandidateMetadataSummary,
) -> Result<()> {
    if !super::reducer_artifacts_required(plan, metadata) {
        return Ok(());
    }

    let reducer_report = read_reducer_report(candidate.metadata_dir.as_path())?;
    let edit_summary = read_reducer_edit_summary(candidate.metadata_dir.as_path())?;
    if reducer_report.summary.edit_records != edit_summary.edit_records {
        anyhow::bail!(
            "verification failed: reducer edit summary count {} does not match reducer report count {}",
            edit_summary.edit_records,
            reducer_report.summary.edit_records
        );
    }
    if edit_summary.edit_records != edit_summary.edit_record_details.len() {
        anyhow::bail!(
            "verification failed: reducer edit summary declares {} edit record(s) but includes {} detail record(s)",
            edit_summary.edit_records,
            edit_summary.edit_record_details.len()
        );
    }
    for edit in &edit_summary.edit_record_details {
        verify_edit_record_byte_evidence(edit)?;
        verify_edit_record_has_single_proof_source(edit)?;
    }
    if !plan.resolved.reducer_plan.reject_unreasoned_edits {
        return Ok(());
    }
    for edit in &edit_summary.edit_record_details {
        verify_reasoned_edit_record(edit)?;
    }
    Ok(())
}

pub(super) fn verify_no_unknown_diagnostics_in_strict_mode(
    plan: &GeneratePlan,
    candidate: &CandidateTreeState,
    metadata: &CandidateMetadataSummary,
) -> Result<()> {
    if !plan.resolved.reducer_plan.fail_on_unknown_diagnostics
        || !super::reducer_artifacts_required(plan, metadata)
    {
        return Ok(());
    }

    let diagnostics = read_reducer_diagnostics(candidate.metadata_dir.as_path())?;
    if !diagnostics.unknown_diagnostics.is_empty() {
        anyhow::bail!(
            "verification failed: unknown diagnostic in strict mode: diagnostics report declares {} unknown diagnostic(s)",
            diagnostics.unknown_diagnostics.len()
        );
    }
    for diagnostic in &diagnostics.classified_diagnostics {
        if diagnostic_is_unknown(diagnostic) {
            anyhow::bail!(
                "verification failed: unknown diagnostic in strict mode: classified diagnostic"
            );
        }
    }
    for consumed in &diagnostics.consumed_diagnostics {
        if diagnostic_is_unknown(&consumed.diagnostic) {
            anyhow::bail!(
                "verification failed: unknown diagnostic in strict mode: consumed diagnostic"
            );
        }
    }
    for skipped in &diagnostics.skipped_diagnostics {
        if skipped_is_unknown_diagnostic(skipped) {
            anyhow::bail!(
                "verification failed: unknown diagnostic in strict mode: {}",
                skipped.reason
            );
        }
    }
    for skipped in &diagnostics.skipped_fixup_diagnostics {
        if skipped_is_unknown_diagnostic(skipped) {
            anyhow::bail!(
                "verification failed: unknown diagnostic in strict mode: {}",
                skipped.reason
            );
        }
    }
    Ok(())
}

fn skipped_is_unknown_diagnostic(skipped: &ReducerSkippedFixupDiagnostic) -> bool {
    diagnostic_is_unknown(&skipped.diagnostic) || skipped.reason == "unknown diagnostic"
}

fn diagnostic_is_unknown(diagnostic: &ReducerClassifiedDiagnostic) -> bool {
    diagnostic.class == "Unknown"
}

pub(super) fn verify_no_broad_speculative_fallout_edits(
    plan: &GeneratePlan,
    candidate: &CandidateTreeState,
    metadata: &CandidateMetadataSummary,
) -> Result<()> {
    if !plan.resolved.reducer_plan.reject_speculative_fallout_edits
        || !super::reducer_artifacts_required(plan, metadata)
    {
        return Ok(());
    }

    let edit_summary = read_reducer_edit_summary(candidate.metadata_dir.as_path())?;
    for edit in &edit_summary.edit_record_details {
        validate_reported_no_speculative_fallout_edit(
            &edit.edit_reason.kind,
            &edit.edit_reason.payload,
            &edit.proof_source.kind,
            &edit.proof_source.payload,
        )
        .with_context(|| {
            format!(
                "verification failed: reducer edit in {} for pass {} is broad speculative fallout",
                edit.file, edit.pass_name
            )
        })?;
    }
    Ok(())
}

pub(super) fn verify_no_unsupported_syntax_in_strict_mode(
    plan: &GeneratePlan,
    candidate: &CandidateTreeState,
    metadata: &CandidateMetadataSummary,
) -> Result<()> {
    if !plan.resolved.reducer_plan.report_unsupported_expressions
        || !super::reducer_artifacts_required(plan, metadata)
    {
        return Ok(());
    }

    let report = read_reducer_report(candidate.metadata_dir.as_path())?;
    if report.unsupported_fallout.unsupported_kconfig_expressions > 0 {
        anyhow::bail!(
            "verification failed: unsupported Kconfig syntax in strict mode: {} site(s)",
            report.unsupported_fallout.unsupported_kconfig_expressions
        );
    }
    if report.unsupported_fallout.unsupported_cpp_expressions > 0 {
        anyhow::bail!(
            "verification failed: unsupported preprocessor syntax in strict mode: {} site(s)",
            report.unsupported_fallout.unsupported_cpp_expressions
        );
    }

    let diagnostics = read_reducer_diagnostics(candidate.metadata_dir.as_path())?;
    if let Some(site) = diagnostics.unsupported_kconfig_expressions.first() {
        anyhow::bail!(
            "verification failed: unsupported Kconfig syntax in strict mode at {}:{} {} {} {} ({})",
            site.kind,
            site.file,
            site.line,
            site.directive,
            site.expression,
            site.reason
        );
    }
    if let Some(site) = diagnostics.unsupported_cpp_expressions.first() {
        anyhow::bail!(
            "verification failed: unsupported preprocessor syntax in strict mode at {}:{} {} {} {} ({})",
            site.kind,
            site.file,
            site.line,
            site.directive,
            site.expression,
            site.reason
        );
    }
    Ok(())
}

pub(super) fn verify_selftest_policy(
    plan: &GeneratePlan,
    candidate: &CandidateTreeState,
    metadata: &CandidateMetadataSummary,
) -> Result<bool> {
    let selftests_required =
        plan.resolved.selftest_plan.enabled && plan.requested.cli_overrides.run_selftests;
    if !selftests_required {
        if candidate.selftested && !metadata.selftested {
            anyhow::bail!(
                "verification failed: candidate state records selftests but candidate metadata does not"
            );
        }
        return Ok(true);
    }

    if !candidate.selftested {
        anyhow::bail!(
            "verification failed: selected selftest/build matrix is enabled but candidate state is not selftested"
        );
    }
    if !metadata.selftested {
        anyhow::bail!(
            "verification failed: selected selftest/build matrix is enabled but candidate metadata does not record success"
        );
    }
    Ok(true)
}

fn verify_reasoned_edit_record(edit: &ReducerEditRecord) -> Result<()> {
    verify_non_empty_edit_field("file", &edit.file)?;
    verify_non_empty_edit_field("pass_name", &edit.pass_name)?;
    verify_known_edit_kind(&edit.edit_kind)?;
    verify_line_range(edit)?;
    verify_edit_record_byte_evidence(edit)?;
    if edit.old.logical_item == edit.new_value.logical_item {
        anyhow::bail!(
            "verification failed: reducer edit in {} for pass {} has no effective change",
            edit.file,
            edit.pass_name
        );
    }
    verify_non_empty_edit_field("idempotence_marker", &edit.idempotence_marker)?;
    verify_reasoned_edit_truth("edit reason", &edit.edit_reason)?;
    verify_reasoned_edit_truth("proof source", &edit.proof_source)?;
    verify_edit_record_has_single_proof_source(edit)?;
    Ok(())
}

fn verify_edit_record_has_single_proof_source(edit: &ReducerEditRecord) -> Result<()> {
    let expected_proof_kind = proof_kind_for_reason_kind(&edit.edit_reason.kind).ok_or_else(|| {
        anyhow::anyhow!(
            "verification failed: reducer edit in {} for pass {} has unsupported edit reason kind {}",
            edit.file,
            edit.pass_name,
            edit.edit_reason.kind
        )
    })?;
    if edit.proof_source.kind != expected_proof_kind {
        anyhow::bail!(
            "verification failed: reducer edit in {} for pass {} has competing proof sources: reason {} requires {} but proof source is {}",
            edit.file,
            edit.pass_name,
            edit.edit_reason.kind,
            expected_proof_kind,
            edit.proof_source.kind
        );
    }
    if payload_has_non_empty_values(&edit.edit_reason.payload)
        && payload_has_non_empty_values(&edit.proof_source.payload)
    {
        validate_reported_proof_source_payload_for_reason(
            &edit.edit_reason.kind,
            &edit.edit_reason.payload,
            &edit.proof_source.payload,
        )
        .with_context(|| {
            format!(
                "verification failed: reducer edit in {} for pass {} has competing proof sources",
                edit.file, edit.pass_name
            )
        })?;
    }
    Ok(())
}

fn verify_edit_record_byte_evidence(edit: &ReducerEditRecord) -> Result<()> {
    verify_byte_evidence(
        "old",
        &edit.file,
        &edit.pass_name,
        &edit.old.logical_item,
        edit.old.byte_len,
        &edit.old.sha256,
    )?;
    verify_byte_evidence(
        "new",
        &edit.file,
        &edit.pass_name,
        &edit.new_value.logical_item,
        edit.new_value.byte_len,
        &edit.new_value.sha256,
    )
}

fn verify_byte_evidence(
    label: &str,
    file: &str,
    pass_name: &str,
    logical_item: &str,
    byte_len: usize,
    sha256: &str,
) -> Result<()> {
    let actual_len = logical_item.as_bytes().len();
    if byte_len != actual_len {
        anyhow::bail!(
            "verification failed: reducer edit in {} for pass {} has invalid {} byte length {}; expected {}",
            file,
            pass_name,
            label,
            byte_len,
            actual_len
        );
    }
    verify_non_empty_edit_field(&format!("{} sha256", label), sha256)?;
    let actual_sha256 = hex::encode(Sha256::digest(logical_item.as_bytes()));
    if sha256 != actual_sha256 {
        anyhow::bail!(
            "verification failed: reducer edit in {} for pass {} has invalid {} sha256",
            file,
            pass_name,
            label
        );
    }
    Ok(())
}

fn verify_non_empty_edit_field(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        anyhow::bail!("verification failed: reducer edit has empty {}", label);
    }
    Ok(())
}

fn verify_known_edit_kind(kind: &str) -> Result<()> {
    match kind {
        "remove_path" | "remove_line" | "remove_block" | "rewrite_line" | "rewrite_block" => Ok(()),
        _ => anyhow::bail!(
            "verification failed: reducer edit has unsupported kind {}",
            kind
        ),
    }
}

fn verify_line_range(edit: &ReducerEditRecord) -> Result<()> {
    match (edit.old.line_start, edit.old.line_end) {
        (Some(start), Some(end)) if start >= 1 && end >= start => Ok(()),
        (None, None) if edit.edit_kind == "remove_path" => Ok(()),
        (Some(_), Some(_)) => anyhow::bail!(
            "verification failed: reducer edit in {} for pass {} has invalid line range",
            edit.file,
            edit.pass_name
        ),
        _ => anyhow::bail!(
            "verification failed: reducer edit in {} for pass {} has incomplete line range",
            edit.file,
            edit.pass_name
        ),
    }
}

fn verify_reasoned_edit_truth(label: &str, truth: &ReducerEditTruth) -> Result<()> {
    verify_non_empty_edit_field(&format!("{} kind", label), &truth.kind)?;
    verify_non_empty_edit_field(&format!("{} payload", label), &truth.payload)?;
    if !payload_has_non_empty_values(&truth.payload) {
        anyhow::bail!(
            "verification failed: reducer edit has unreasoned {} payload {}",
            label,
            truth.payload
        );
    }
    if truth.kind == "build_diagnostic" || truth.kind == "classified_build_diagnostic" {
        if truth
            .payload
            .split_whitespace()
            .any(|part| part == "class=Unknown")
        {
            anyhow::bail!(
                "verification failed: reducer edit has unreasoned {} payload {}",
                label,
                truth.payload
            );
        }
    }
    Ok(())
}

fn payload_has_non_empty_values(payload: &str) -> bool {
    let payload = payload.trim();
    if payload.is_empty() {
        return false;
    }
    let mut saw_key_value = false;
    for part in payload.split_whitespace() {
        let Some((_, value)) = part.split_once('=') else {
            continue;
        };
        saw_key_value = true;
        if value.trim().is_empty() {
            return false;
        }
    }
    saw_key_value
}

fn proof_kind_for_reason_kind(reason_kind: &str) -> Option<&'static str> {
    proof_source_kind_for_reason_key(reason_kind).map(|kind| kind.json_key())
}
