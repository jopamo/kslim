//! Proof-gated rewrite application for deterministic fixups.
//!
//! This module owns mutation of source files once planning has assembled
//! diagnostic, manifest, and read-only index proof.

use anyhow::Result;
use std::path::Path;

use crate::diagnostics::ClassifiedDiagnostic;
use crate::edit_reason::{
    sort_edit_records, write_verified_rewrite, DiagnosticClass, EditProofSource, EditReason,
    EditRecord, LineRange,
};
use crate::prune::RemovalAccounting;
use crate::tree_index::TreeIndex;

use super::{
    is_tree_index_truth, manifest_proven_removed_header_path, proof_matches_tree_index, FixupProof,
    FixupResult,
};

pub(in crate::fixups) fn validate_fixup_result(
    fixup_name: &str,
    index: &TreeIndex,
    diagnostic: &ClassifiedDiagnostic,
    result: &FixupResult,
) -> Result<()> {
    if result.edits.is_empty() {
        anyhow::bail!("fixup '{}' returned no edits", fixup_name);
    }
    crate::edit_reason::validate_edit_records(&result.edits)?;
    if !result
        .proof_sources
        .iter()
        .any(FixupProof::is_manifest_truth)
    {
        anyhow::bail!(
            "fixup '{}' returned edits without manifest truth proof",
            fixup_name
        );
    }
    if !result
        .proof_sources
        .iter()
        .any(|proof| is_tree_index_truth(proof) && proof_matches_tree_index(proof, index))
    {
        anyhow::bail!(
            "fixup '{}' returned edits without tree index truth proof",
            fixup_name
        );
    }
    if !result
        .proof_sources
        .iter()
        .any(|proof| proof.matches_diagnostic(diagnostic))
    {
        anyhow::bail!(
            "fixup '{}' returned edits without classified diagnostic truth proof",
            fixup_name
        );
    }
    Ok(())
}

pub(in crate::fixups) fn remove_missing_header_include(
    root: &Path,
    removal: &RemovalAccounting,
    source_file: &Path,
    header: &str,
    index: &TreeIndex,
    diagnostic: &ClassifiedDiagnostic,
    proof_sources: &[FixupProof],
) -> Result<Vec<EditRecord>> {
    let source_path = root.join(source_file);
    if !source_path.exists() {
        anyhow::bail!(
            "cannot apply missing-header fixup: source file missing: {}",
            source_file.display()
        );
    }

    let source_dir = source_path.parent().unwrap_or(root);
    if manifest_proven_removed_header_path(root, source_dir, header, removal).is_none() {
        anyhow::bail!(
            "cannot apply missing-header fixup for '{}' because the header is not proven removed",
            header
        );
    }

    let content = std::fs::read_to_string(&source_path)?;
    let mut out = String::with_capacity(content.len());
    let mut edits = Vec::new();
    let include_angle = format!("#include <{}>", header);
    let include_quote = format!("#include \"{}\"", header);

    for (idx, line) in content.lines().enumerate() {
        if line.trim() == include_angle || line.trim() == include_quote {
            edits.push(EditRecord::new(
                source_file.to_path_buf(),
                Some(LineRange {
                    start: idx + 1,
                    end: idx + 1,
                }),
                format!("{line}\n"),
                String::new(),
                EditReason::BuildDiagnostic {
                    class: crate::edit_reason::DiagnosticClass::MissingHeader,
                },
                EditProofSource::ClassifiedDiagnostic {
                    diagnostic_id: crate::edit_reason::DiagnosticClass::MissingHeader.into(),
                },
                "fixups.remove_missing_header_include",
            ));
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }

    if edits.is_empty() {
        anyhow::bail!(
            "cannot apply missing-header fixup for '{}' because no matching include line was found in {}",
            header,
            source_file.display()
        );
    }

    write_proven_fixup_rewrite(
        root,
        &source_path,
        &out,
        edits,
        proof_sources,
        "fixups.remove_missing_header_include",
        index,
        diagnostic,
    )
}

pub(in crate::fixups) fn remove_stale_kbuild_directory_reference(
    root: &Path,
    file: &Path,
    line: usize,
    assignment_lhs: &str,
    directory: &str,
    index: &TreeIndex,
    diagnostic: &ClassifiedDiagnostic,
    proof_sources: &[FixupProof],
) -> Result<Vec<EditRecord>> {
    remove_kbuild_assignment_token(
        root,
        file,
        line,
        assignment_lhs,
        directory,
        "fixups.remove_stale_kbuild_directory_ref",
        DiagnosticClass::StaleKbuildDirectoryRef,
        index,
        diagnostic,
        proof_sources,
    )
}

pub(in crate::fixups) fn remove_stale_kbuild_object_reference(
    root: &Path,
    file: &Path,
    line: usize,
    assignment_lhs: &str,
    object: &str,
    index: &TreeIndex,
    diagnostic: &ClassifiedDiagnostic,
    proof_sources: &[FixupProof],
) -> Result<Vec<EditRecord>> {
    remove_kbuild_assignment_token(
        root,
        file,
        line,
        assignment_lhs,
        object,
        "fixups.remove_stale_kbuild_object_ref",
        DiagnosticClass::StaleKbuildObjectRef,
        index,
        diagnostic,
        proof_sources,
    )
}

pub(in crate::fixups) fn remove_missing_kconfig_source(
    root: &Path,
    kconfig_file: &Path,
    line: usize,
    source: &str,
    index: &TreeIndex,
    diagnostic: &ClassifiedDiagnostic,
    proof_sources: &[FixupProof],
) -> Result<Vec<EditRecord>> {
    let path = root.join(kconfig_file);
    if !path.exists() {
        anyhow::bail!(
            "cannot apply missing-Kconfig-source fixup: file missing: {}",
            kconfig_file.display()
        );
    }

    let content = std::fs::read_to_string(&path)?;
    let mut out = String::with_capacity(content.len());
    let mut edits = Vec::new();
    let mut matched_line = false;

    for (idx, current_line) in content.lines().enumerate() {
        if idx + 1 != line {
            out.push_str(current_line);
            out.push('\n');
            continue;
        }

        matched_line = true;
        let parsed_source = crate::kconfig::parse_kconfig_source(current_line).ok_or_else(|| {
            anyhow::anyhow!(
                "cannot apply missing-Kconfig-source fixup in {}:{} because the line is no longer a supported Kconfig source directive",
                kconfig_file.display(),
                line
            )
        })?;
        if parsed_source.path != source {
            anyhow::bail!(
                "cannot apply missing-Kconfig-source fixup in {}:{} because source changed from '{}' to '{}'",
                kconfig_file.display(),
                line,
                source,
                parsed_source.path
            );
        }

        let replacement = removed_kconfig_source_replacement(current_line);
        edits.push(EditRecord::new(
            kconfig_file.to_path_buf(),
            Some(LineRange {
                start: line,
                end: line,
            }),
            format!("{current_line}\n"),
            replacement.clone(),
            EditReason::BuildDiagnostic {
                class: DiagnosticClass::MissingKconfigSource,
            },
            EditProofSource::ClassifiedDiagnostic {
                diagnostic_id: DiagnosticClass::MissingKconfigSource.into(),
            },
            "fixups.remove_missing_kconfig_source",
        ));
        out.push_str(&replacement);
    }

    if !matched_line {
        anyhow::bail!(
            "cannot apply missing-Kconfig-source fixup because {}:{} no longer exists",
            kconfig_file.display(),
            line
        );
    }

    write_proven_fixup_rewrite(
        root,
        &path,
        &out,
        edits,
        proof_sources,
        "fixups.remove_missing_kconfig_source",
        index,
        diagnostic,
    )
}

fn remove_kbuild_assignment_token(
    root: &Path,
    file: &Path,
    line: usize,
    assignment_lhs: &str,
    token_to_remove: &str,
    pass_name: &'static str,
    class: DiagnosticClass,
    index: &TreeIndex,
    diagnostic: &ClassifiedDiagnostic,
    proof_sources: &[FixupProof],
) -> Result<Vec<EditRecord>> {
    let path = root.join(file);
    if !path.exists() {
        anyhow::bail!(
            "cannot apply kbuild fixup: file missing: {}",
            file.display()
        );
    }

    let content = std::fs::read_to_string(&path)?;
    let mut out = String::with_capacity(content.len());
    let mut edits = Vec::new();
    let mut matched_line = false;

    for entry in crate::kbuild::logical_lines(&content) {
        if entry.start_line != line {
            for raw in entry.original {
                out.push_str(&raw);
                out.push('\n');
            }
            continue;
        }

        matched_line = true;
        let assignment = crate::kbuild::parse_kbuild_assignment(&entry.joined).ok_or_else(|| {
            anyhow::anyhow!(
                "cannot apply kbuild fixup in {}:{} because the logical line is no longer a supported kbuild assignment",
                file.display(),
                line
            )
        })?;
        if assignment.lhs != assignment_lhs {
            anyhow::bail!(
                "cannot apply kbuild fixup in {}:{} because assignment lhs changed from '{}' to '{}'",
                file.display(),
                line,
                assignment_lhs,
                assignment.lhs
            );
        }

        let mut kept = Vec::new();
        let mut removed = 0usize;
        for token in assignment.rhs.split_whitespace() {
            if token == token_to_remove {
                removed += 1;
            } else {
                kept.push(token.to_string());
            }
        }
        if removed == 0 {
            anyhow::bail!(
                "cannot apply kbuild fixup in {}:{} because token '{}' is no longer present",
                file.display(),
                line,
                token_to_remove
            );
        }

        let comment_suffix = trailing_comment_suffix(&entry.original);
        let after = if kept.is_empty() {
            format!(
                "# kslim: removed stale make refs from {}{}\n",
                assignment.lhs, comment_suffix
            )
        } else {
            format!(
                "{} {} {}{}\n",
                assignment.lhs,
                assignment.op,
                kept.join(" "),
                comment_suffix
            )
        };
        let before = logical_line_text(&entry.original);
        edits.push(EditRecord::new(
            file.to_path_buf(),
            Some(LineRange {
                start: line,
                end: line + entry.original.len().saturating_sub(1),
            }),
            before,
            after.clone(),
            EditReason::BuildDiagnostic {
                class: class.clone(),
            },
            EditProofSource::ClassifiedDiagnostic {
                diagnostic_id: class.clone().into(),
            },
            pass_name,
        ));
        out.push_str(&after);
    }

    if !matched_line {
        anyhow::bail!(
            "cannot apply kbuild fixup because {}:{} no longer exists",
            file.display(),
            line
        );
    }
    if edits.is_empty() {
        anyhow::bail!(
            "cannot apply kbuild fixup because no edit was produced for {}:{}",
            file.display(),
            line
        );
    }

    write_proven_fixup_rewrite(
        root,
        &path,
        &out,
        edits,
        proof_sources,
        pass_name,
        index,
        diagnostic,
    )
}

pub(in crate::fixups) fn write_proven_fixup_rewrite(
    root: &Path,
    path: &Path,
    content: &str,
    edits: Vec<EditRecord>,
    proof_sources: &[FixupProof],
    pass_name: &'static str,
    index: &TreeIndex,
    diagnostic: &ClassifiedDiagnostic,
) -> Result<Vec<EditRecord>> {
    let mut edits = edits;
    sort_edit_records(&mut edits);
    let result = FixupResult::new(edits, proof_sources.to_vec());
    validate_fixup_result(pass_name, index, diagnostic, &result)?;
    write_verified_rewrite(root, path, content, &result.edits, pass_name)?;
    Ok(result.edits)
}

fn logical_line_text(original: &[String]) -> String {
    let mut out = String::new();
    for raw in original {
        out.push_str(raw);
        out.push('\n');
    }
    out
}

fn trailing_comment_suffix(original: &[String]) -> String {
    let Some(last) = original.last() else {
        return String::new();
    };
    let Some(hash_idx) = last.find('#') else {
        return String::new();
    };
    let prefix_end = last[..hash_idx]
        .rfind(|c: char| !c.is_whitespace())
        .map_or(0, |idx| idx + 1);
    last[prefix_end..].to_string()
}

fn removed_kconfig_source_replacement(line: &str) -> String {
    let (directive_text, comment_suffix) = split_kconfig_trailing_comment(line);
    format!(
        "{}# kslim: removed {}{}\n",
        line_indentation_prefix(directive_text),
        directive_text.trim_start(),
        comment_suffix
    )
}

fn split_kconfig_trailing_comment(line: &str) -> (&str, &str) {
    let mut in_quotes = false;

    for (idx, ch) in line.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            '#' if !in_quotes => return (&line[..idx], &line[idx..]),
            _ => {}
        }
    }

    (line, "")
}

fn line_indentation_prefix(line: &str) -> &str {
    let end = line
        .char_indices()
        .find_map(|(idx, ch)| (!ch.is_whitespace()).then_some(idx))
        .unwrap_or(line.len());
    &line[..end]
}
