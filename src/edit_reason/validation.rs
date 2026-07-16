//! Edit validation.
//!
//! This module owns edit-record field validation, pass/reason compatibility,
//! reasoned-proof strict policy checks, mutation/edit-count checks, and
//! verified text rewrite authorization.

use anyhow::{Context, Result};
use std::path::Path;

use super::{
    DiagnosticClass, EditIdempotenceMarker, EditKind, EditRecord, EditReason, LineRange,
    MAX_EDIT_CONTENT_BYTES,
};

impl EditRecord {
    pub fn validate_no_competing_proof_sources(&self) -> Result<()> {
        let reason_source_kind = self.reason.proof_source_kind();
        let explicit_source_kind = self.proof_source.kind();
        if reason_source_kind == explicit_source_kind
            && self.proof_source.matches_reason(&self.reason)
        {
            return Ok(());
        }

        anyhow::bail!(
            "edit in {} for pass {} has multiple competing proof sources: EditReason {} ({}) implies {} ({}) but explicit proof source is {} ({})",
            self.file.display(),
            self.pass_name,
            self.reason.json_key(),
            self.reason.payload_label(),
            reason_source_kind.json_key(),
            reason_source_kind.report_label(),
            explicit_source_kind.json_key(),
            self.proof_source.payload_label()
        )
    }

    pub fn validate_required_fields(&self) -> Result<()> {
        validate_stable_audit_aliases(self)?;
        validate_relative_edit_path(&self.file)?;
        if self.pass_name.trim().is_empty() {
            anyhow::bail!("edit for {} has empty pass name", self.file.display());
        }
        if let Some(range) = self.line_range {
            if !range.is_valid() {
                anyhow::bail!(
                    "edit in {} for pass {} has invalid line range {}..{}",
                    self.file.display(),
                    self.pass_name,
                    range.start,
                    range.end
                );
            }
        }
        validate_reason_for_pass(self.pass_name, &self.reason)?;
        validate_span_for_edit_kind(
            &self.file,
            self.pass_name,
            self.line_range,
            self.edit_kind,
            &self.after,
        )?;
        validate_audit_content("before", &self.before)?;
        validate_audit_content("after", &self.after)?;
        if self.before == self.after {
            anyhow::bail!(
                "edit in {} for pass {} has no effective change",
                self.file.display(),
                self.pass_name
            );
        }

        let expected_kind = EditKind::from_change(self.line_range, &self.after);
        if self.edit_kind != expected_kind {
            anyhow::bail!(
                "edit in {} for pass {} has unstable edit kind {}; expected {}",
                self.file.display(),
                self.pass_name,
                self.edit_kind.json_key(),
                expected_kind.json_key()
            );
        }

        let expected = EditIdempotenceMarker::for_record_parts(
            &self.file,
            self.line_range,
            &self.before,
            &self.after,
            &self.reason,
            &self.proof_source,
            self.edit_kind,
            self.pass_name,
        );
        if self.idempotence_marker != expected {
            anyhow::bail!(
                "edit in {} for pass {} has invalid idempotence marker",
                self.file.display(),
                self.pass_name
            );
        }

        Ok(())
    }

    pub fn validate_reasoned(&self) -> Result<()> {
        self.validate_no_competing_proof_sources()?;
        self.reason.validate_reasoned_payload().with_context(|| {
            format!(
                "edit in {} for pass {} has unreasoned EditReason",
                self.file.display(),
                self.pass_name
            )
        })?;
        self.proof_source
            .validate_reasoned_payload()
            .with_context(|| {
                format!(
                    "edit in {} for pass {} has unreasoned proof source",
                    self.file.display(),
                    self.pass_name
                )
            })?;
        Ok(())
    }

    pub fn validate_not_speculative_fallout(&self) -> Result<()> {
        if self.reason.is_broad_speculative_fallout()
            || self.proof_source.is_broad_speculative_fallout()
        {
            anyhow::bail!(
                "edit in {} for pass {} is a broad speculative fallout edit; broad speculative fallout edits are forbidden",
                self.file.display(),
                self.pass_name
            );
        }
        Ok(())
    }
}

fn validate_stable_audit_aliases(edit: &EditRecord) -> Result<()> {
    if edit.path != edit.file {
        anyhow::bail!(
            "edit in {} for pass {} has unstable path alias {}",
            edit.file.display(),
            edit.pass_name,
            edit.path.display()
        );
    }
    if edit.pass != edit.pass_name {
        anyhow::bail!(
            "edit in {} for pass {} has unstable pass alias {}",
            edit.file.display(),
            edit.pass_name,
            edit.pass
        );
    }
    if edit.span != edit.line_range {
        anyhow::bail!(
            "edit in {} for pass {} has unstable span alias",
            edit.file.display(),
            edit.pass_name
        );
    }
    if edit.old.as_ref() != Some(&edit.before) {
        anyhow::bail!(
            "edit in {} for pass {} has unstable old logical item",
            edit.file.display(),
            edit.pass_name
        );
    }
    if edit.new.as_ref() != Some(&edit.after) {
        anyhow::bail!(
            "edit in {} for pass {} has unstable new logical item",
            edit.file.display(),
            edit.pass_name
        );
    }
    Ok(())
}

fn validate_span_for_edit_kind(
    file: &Path,
    pass_name: &str,
    line_range: Option<LineRange>,
    edit_kind: EditKind,
    after: &str,
) -> Result<()> {
    match (line_range, edit_kind) {
        (None, EditKind::RemovePath) if after.is_empty() => Ok(()),
        (None, EditKind::RemovePath) => anyhow::bail!(
            "edit in {} for pass {} is a whole-path deletion with replacement content",
            file.display(),
            pass_name
        ),
        (None, _) => anyhow::bail!(
            "edit in {} for pass {} is a text rewrite without a span",
            file.display(),
            pass_name
        ),
        (Some(_), EditKind::RemovePath) => anyhow::bail!(
            "edit in {} for pass {} is a whole-path deletion with a text span",
            file.display(),
            pass_name
        ),
        (Some(_), _) => Ok(()),
    }
}

fn validate_reason_for_pass(pass_name: &str, reason: &EditReason) -> Result<()> {
    if reason_allowed_for_pass(pass_name, reason) {
        return Ok(());
    }

    anyhow::bail!(
        "EditReason {} is not valid for pass {}",
        reason.json_key(),
        pass_name
    )
}

fn reason_allowed_for_pass(pass_name: &str, reason: &EditReason) -> bool {
    if pass_name.starts_with("test.") {
        return true;
    }

    match pass_name {
        "prune.remove_path" | "prune.cleanup_empty_parents" => {
            matches!(
                reason,
                EditReason::DeclaredPathPruned | EditReason::ManifestPath { .. }
            )
        }
        "prune.prune_configs" | "prune.rewrite_kconfig_defaults" => {
            matches!(reason, EditReason::ManifestConfig { .. })
        }
        "kconfig.rewrite_relations" => matches!(
            reason,
            EditReason::RemovedKconfigSymbolEdge
                | EditReason::SimplifiedKconfigExpression
                | EditReason::ManifestConfig { .. }
                | EditReason::SimplifiedTristateExpr { .. }
        ),
        "kconfig.rewrite_dead_symbol_definitions" => {
            matches!(reason, EditReason::RemovedDeadKconfigSymbolDefinition { .. })
        },
        "kconfig.rewrite_empty_menus" => {
            matches!(reason, EditReason::RemovedEmptyKconfigMenu { .. })
        },
        "prune.rewrite_kconfig_sources" => matches!(reason, EditReason::RemovedKconfigSource),
        "prune.rewrite_removed_kconfig_helpers" => {
            matches!(reason, EditReason::ManifestPath { .. })
        }
        "prune.rewrite_makefiles" => matches!(
            reason,
            EditReason::RemovedKbuildDirectoryRef
                | EditReason::RemovedKbuildObjectRef
                | EditReason::RemovedKbuildConfigGatedRef
                | EditReason::RemovedKbuildIncludePath
                | EditReason::RemovedKbuildRef { .. }
        ),
        "cpp.fold_removed_config_branches" => matches!(
            reason,
            EditReason::FoldedDeadPreprocessorBranch | EditReason::ManifestConfig { .. }
        ),
        "includes.rewrite_removed_headers" => matches!(
            reason,
            EditReason::RemovedManifestBackedInclude
                | EditReason::RemovedDeadBranchInclude { .. }
                | EditReason::RemovedHeader { .. }
        ),
        "fixups.remove_missing_header_include" => matches!(
            reason,
            EditReason::DiagnosticMissingHeaderFixup
                | EditReason::BuildDiagnostic {
                    class: DiagnosticClass::MissingHeader
                }
        ),
        "fixups.remove_stale_kbuild_directory_ref" => matches!(
            reason,
            EditReason::DiagnosticStaleKbuildDirFixup
                | EditReason::BuildDiagnostic {
                    class: DiagnosticClass::StaleKbuildDirectoryRef
                }
        ),
        "fixups.remove_stale_kbuild_object_ref" => matches!(
            reason,
            EditReason::DiagnosticStaleKbuildObjectFixup
                | EditReason::BuildDiagnostic {
                    class: DiagnosticClass::StaleKbuildObjectRef
                }
        ),
        "fixups.remove_missing_kconfig_source" => matches!(
            reason,
            EditReason::DiagnosticMissingKconfigSourceFixup
                | EditReason::BuildDiagnostic {
                    class: DiagnosticClass::MissingKconfigSource
                }
        ),
        "fixups.refold_preprocessor" => matches!(
            reason,
            EditReason::DiagnosticPreprocessorRefoldFixup
                | EditReason::BuildDiagnostic {
                    class: DiagnosticClass::DeadConfigGatedCodePath
                        | DiagnosticClass::RemovedConfigSymbolUse
                        | DiagnosticClass::RemovedHeaderSymbolUse
                }
        ),
        _ => true,
    }
}

fn validate_audit_content(label: &str, content: &str) -> Result<()> {
    if content.len() <= MAX_EDIT_CONTENT_BYTES || content.starts_with("<kslim: content elided ") {
        return Ok(());
    }

    anyhow::bail!(
        "edit {label} content exceeds {} bytes without elision",
        MAX_EDIT_CONTENT_BYTES
    )
}

pub(in crate::edit_reason) fn validate_relative_edit_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        anyhow::bail!("edit record has empty file path");
    }

    for component in path.components() {
        match component {
            std::path::Component::Normal(_) => {}
            _ => {
                anyhow::bail!(
                    "edit record path must be normalized and relative: {}",
                    path.display()
                );
            }
        }
    }

    Ok(())
}

pub(in crate::edit_reason) fn validate_non_empty_payload(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        anyhow::bail!("{label} is empty");
    }
    Ok(())
}

pub(in crate::edit_reason) fn validate_non_empty_payload_path(label: &str, path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        anyhow::bail!("{label} is empty");
    }
    Ok(())
}

pub fn validate_edit_records(edits: &[EditRecord]) -> Result<()> {
    for edit in edits {
        edit.validate_no_competing_proof_sources()?;
        edit.validate_required_fields()?;
    }
    Ok(())
}

pub fn validate_reasoned_edit_records(edits: &[EditRecord]) -> Result<()> {
    validate_edit_records(edits)?;
    for edit in edits {
        edit.validate_reasoned()?;
    }
    Ok(())
}

pub fn validate_no_speculative_fallout_edit_records(edits: &[EditRecord]) -> Result<()> {
    for edit in edits {
        edit.validate_not_speculative_fallout()?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EditValidationPolicy {
    pub reject_unreasoned_edits: bool,
    pub reject_speculative_fallout_edits: bool,
}

pub fn validate_edit_records_with_policy(
    edits: &[EditRecord],
    policy: EditValidationPolicy,
) -> Result<()> {
    if policy.reject_unreasoned_edits {
        validate_reasoned_edit_records(edits)?;
    } else {
        validate_edit_records(edits)?;
    }

    if policy.reject_speculative_fallout_edits {
        validate_no_speculative_fallout_edit_records(edits)?;
    }

    Ok(())
}

pub fn ensure_edit_records_for_mutation(
    pass_name: &str,
    mutation_count: usize,
    edits: &[EditRecord],
) -> Result<()> {
    validate_edit_records(edits)?;

    if mutation_count == 0 {
        return Ok(());
    }

    let matching_edit_records = edits
        .iter()
        .filter(|edit| edit.pass_name == pass_name)
        .count();

    if matching_edit_records >= mutation_count {
        return Ok(());
    }

    if matching_edit_records == 0 {
        anyhow::bail!(
            "mutating pass '{}' reported {} mutation(s) without edit records",
            pass_name,
            mutation_count
        );
    }

    anyhow::bail!(
        "mutating pass '{}' reported {} mutation(s) with only {} matching edit record(s)",
        pass_name,
        mutation_count,
        matching_edit_records
    )
}

pub fn write_verified_rewrite(
    root: &Path,
    path: &Path,
    content: &str,
    edits: &[EditRecord],
    pass_name: &'static str,
) -> Result<()> {
    validate_edit_records(edits)?;

    let relative = path.strip_prefix(root).unwrap_or(path);
    let matching: Vec<&EditRecord> = edits
        .iter()
        .filter(|edit| edit.file == relative && edit.pass_name == pass_name)
        .collect();

    if matching.is_empty() {
        anyhow::bail!(
            "refusing unproven rewrite for {} in pass {}",
            relative.display(),
            pass_name
        );
    }

    let original = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read rewrite target: {}", path.display()))?;
    verify_rewrite_is_fully_recorded(relative, pass_name, &original, content, &matching)?;

    std::fs::write(path, content)
        .with_context(|| format!("failed to write verified rewrite: {}", path.display()))?;
    Ok(())
}

#[derive(Debug, Clone)]
struct TextReplacement {
    range: LineRange,
    before: String,
    after: String,
}

fn verify_rewrite_is_fully_recorded(
    relative: &Path,
    pass_name: &str,
    original: &str,
    content: &str,
    matching: &[&EditRecord],
) -> Result<()> {
    let replacements = unique_text_replacements(relative, pass_name, matching)?;
    if replacements.is_empty() {
        anyhow::bail!(
            "refusing rewrite for {} in pass {} without text edit records",
            relative.display(),
            pass_name
        );
    }

    validate_text_replacement_before_spans(relative, pass_name, original, &replacements)?;
    let effective_replacements = outermost_text_replacements(relative, pass_name, &replacements)?;
    let expected = apply_text_replacements(relative, pass_name, original, &effective_replacements)?;
    if expected != content {
        anyhow::bail!(
            "rewrite for {} in pass {} contains unrecorded mutation(s)",
            relative.display(),
            pass_name
        );
    }

    Ok(())
}

fn unique_text_replacements(
    relative: &Path,
    pass_name: &str,
    matching: &[&EditRecord],
) -> Result<Vec<TextReplacement>> {
    let mut replacements: Vec<TextReplacement> = Vec::new();
    for edit in matching {
        let Some(range) = edit.line_range else {
            anyhow::bail!(
                "whole-path edit record cannot authorize text rewrite for {} in pass {}",
                relative.display(),
                pass_name
            );
        };
        if edit.before.starts_with("<kslim: content elided ")
            || edit.after.starts_with("<kslim: content elided ")
        {
            anyhow::bail!(
                "elided edit record content cannot authorize text rewrite for {} in pass {}",
                relative.display(),
                pass_name
            );
        }

        if replacements.iter().any(|candidate| {
            candidate.range == range
                && candidate.before == edit.before
                && candidate.after == edit.after
        }) {
            continue;
        }
        if replacements.iter().any(|candidate| candidate.range == range) {
            anyhow::bail!(
                "conflicting edit records for {}:{}..{} in pass {}",
                relative.display(),
                range.start,
                range.end,
                pass_name
            );
        }

        replacements.push(TextReplacement {
            range,
            before: edit.before.clone(),
            after: edit.after.clone(),
        });
    }

    replacements.sort_by(|left, right| {
        left.range
            .start
            .cmp(&right.range.start)
            .then(right.range.end.cmp(&left.range.end))
    });

    Ok(replacements)
}

fn outermost_text_replacements(
    relative: &Path,
    pass_name: &str,
    replacements: &[TextReplacement],
) -> Result<Vec<TextReplacement>> {
    for (left_index, left) in replacements.iter().enumerate() {
        for right in replacements.iter().skip(left_index + 1) {
            if !line_ranges_overlap(left.range, right.range) {
                continue;
            }
            if line_range_contains(left.range, right.range)
                || line_range_contains(right.range, left.range)
            {
                continue;
            }
            anyhow::bail!(
                "overlapping edit records for {} in pass {}",
                relative.display(),
                pass_name
            );
        }
    }

    let mut effective = Vec::new();
    for replacement in replacements {
        let nested_inside_other = replacements.iter().any(|candidate| {
            candidate.range != replacement.range
                && line_range_contains(candidate.range, replacement.range)
        });
        if !nested_inside_other {
            effective.push(replacement.clone());
        }
    }

    effective.sort_by(|left, right| {
        left.range
            .start
            .cmp(&right.range.start)
            .then(left.range.end.cmp(&right.range.end))
    });
    Ok(effective)
}

fn line_ranges_overlap(left: LineRange, right: LineRange) -> bool {
    left.start <= right.end && right.start <= left.end
}

fn line_range_contains(container: LineRange, contained: LineRange) -> bool {
    container.start <= contained.start && container.end >= contained.end
}

fn validate_text_replacement_before_spans(
    relative: &Path,
    pass_name: &str,
    original: &str,
    replacements: &[TextReplacement],
) -> Result<()> {
    let lines = split_lines_preserving_endings(original);
    for replacement in replacements {
        let start = replacement.range.start - 1;
        let end = replacement.range.end;
        if end > lines.len() {
            anyhow::bail!(
                "edit record span {}..{} exceeds {} line(s) in {} for pass {}",
                replacement.range.start,
                replacement.range.end,
                lines.len(),
                relative.display(),
                pass_name
            );
        }

        let actual_before = lines[start..end].concat();
        if actual_before != replacement.before {
            anyhow::bail!(
                "edit record before content does not match {}:{}..{} for pass {}",
                relative.display(),
                replacement.range.start,
                replacement.range.end,
                pass_name
            );
        }
    }
    Ok(())
}

fn apply_text_replacements(
    relative: &Path,
    pass_name: &str,
    original: &str,
    replacements: &[TextReplacement],
) -> Result<String> {
    let lines = split_lines_preserving_endings(original);
    let mut out = String::new();
    let mut cursor = 0usize;

    for replacement in replacements {
        let start = replacement.range.start - 1;
        let end = replacement.range.end;
        if end > lines.len() {
            anyhow::bail!(
                "edit record span {}..{} exceeds {} line(s) in {} for pass {}",
                replacement.range.start,
                replacement.range.end,
                lines.len(),
                relative.display(),
                pass_name
            );
        }
        if start < cursor {
            anyhow::bail!(
                "overlapping edit records for {} in pass {}",
                relative.display(),
                pass_name
            );
        }

        let actual_before = lines[start..end].concat();
        if actual_before != replacement.before {
            anyhow::bail!(
                "edit record before content does not match {}:{}..{} for pass {}",
                relative.display(),
                replacement.range.start,
                replacement.range.end,
                pass_name
            );
        }

        for line in &lines[cursor..start] {
            out.push_str(line);
        }
        out.push_str(&replacement.after);
        cursor = end;
    }

    for line in &lines[cursor..] {
        out.push_str(line);
    }

    Ok(out)
}

fn split_lines_preserving_endings(content: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut start = 0usize;
    for (idx, byte) in content.bytes().enumerate() {
        if byte == b'\n' {
            lines.push(content[start..=idx].to_string());
            start = idx + 1;
        }
    }
    if start < content.len() {
        lines.push(content[start..].to_string());
    }
    lines
}
