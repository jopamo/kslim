//! Edit reason serialization helpers.
//!
//! This module owns stable serialized reason/proof lookup helpers and
//! parsing/checking for reported edit-truth payload strings read back from
//! reducer artifacts.

use anyhow::Result;

use super::EditProofSourceKind;

pub fn proof_source_kind_for_reason_key(reason_key: &str) -> Option<EditProofSourceKind> {
    match reason_key {
        "declared_path_pruned"
        | "simplified_kconfig_expression"
        | "folded_dead_preprocessor_branch"
        | "removed_manifest_backed_include"
        | "removed_dead_branch_include"
        | "manifest_path"
        | "manifest_config"
        | "removed_kconfig_source"
        | "removed_header"
        | "simplified_tristate_expr" => Some(EditProofSourceKind::RemovalManifestEntry),
        "reported_live_missing_include" => Some(EditProofSourceKind::TreeIndexEntry),
        "removed_dead_kconfig_symbol_definition" | "removed_empty_kconfig_menu" => {
            Some(EditProofSourceKind::KconfigSolverProof)
        }
        "removed_kconfig_symbol_edge"
        | "removed_kbuild_directory_ref"
        | "removed_kbuild_object_ref"
        | "removed_kbuild_config_gated_ref"
        | "removed_kbuild_include_path"
        | "removed_kbuild_ref" => Some(EditProofSourceKind::StaleReference),
        "diagnostic_missing_header_fixup"
        | "diagnostic_stale_kbuild_dir_fixup"
        | "diagnostic_stale_kbuild_object_fixup"
        | "diagnostic_missing_kconfig_source_fixup"
        | "diagnostic_preprocessor_refold_fixup"
        | "build_diagnostic" => Some(EditProofSourceKind::ClassifiedDiagnostic),
        _ => None,
    }
}

pub fn validate_reported_proof_source_payload_for_reason(
    reason_kind: &str,
    reason_payload: &str,
    proof_payload: &str,
) -> Result<()> {
    match reason_kind {
        "declared_path_pruned" => {
            require_reported_payload_value("proof", proof_payload, "path")?;
        }
        "simplified_kconfig_expression" | "folded_dead_preprocessor_branch" => {
            require_reported_payload_value("proof", proof_payload, "symbol")?;
        }
        "removed_manifest_backed_include" => {
            require_reported_payload_value("proof", proof_payload, "header")?;
        }
        "removed_dead_branch_include" => {
            validate_reported_payload_values_match(
                reason_payload,
                "symbol",
                proof_payload,
                "symbol",
            )?;
        }
        "reported_live_missing_include" => {
            require_reported_payload_value("proof", proof_payload, "index_kind")?;
            require_reported_payload_value("proof", proof_payload, "key")?;
        }
        "removed_kconfig_source" => {
            require_reported_payload_value("proof", proof_payload, "kconfig_source")?;
        }
        "removed_dead_kconfig_symbol_definition" => {
            validate_reported_payload_value(
                "proof",
                proof_payload,
                "solver",
                "unreachable_symbol_definition",
            )?;
            validate_reported_payload_values_match(
                reason_payload,
                "symbol",
                proof_payload,
                "symbol",
            )?;
        }
        "removed_empty_kconfig_menu" => {
            validate_reported_payload_value(
                "proof",
                proof_payload,
                "solver",
                "empty_menu",
            )?;
            validate_reported_payload_values_match(
                reason_payload,
                "prompt",
                proof_payload,
                "prompt",
            )?;
        }
        "removed_kconfig_symbol_edge" => {
            validate_reported_payload_value(
                "proof",
                proof_payload,
                "reference_kind",
                "kconfig_symbol_edge",
            )?;
        }
        "removed_kbuild_directory_ref" => {
            validate_reported_payload_value(
                "proof",
                proof_payload,
                "reference_kind",
                "kbuild_directory_ref",
            )?;
        }
        "removed_kbuild_object_ref" => {
            validate_reported_payload_value(
                "proof",
                proof_payload,
                "reference_kind",
                "kbuild_object_ref",
            )?;
        }
        "removed_kbuild_config_gated_ref" => {
            validate_reported_payload_value(
                "proof",
                proof_payload,
                "reference_kind",
                "kbuild_config_gated_ref",
            )?;
        }
        "removed_kbuild_include_path" => {
            validate_reported_payload_value(
                "proof",
                proof_payload,
                "reference_kind",
                "kbuild_include_path",
            )?;
        }
        "diagnostic_missing_header_fixup" => {
            validate_reported_payload_value("proof", proof_payload, "class", "MissingHeader")?;
        }
        "diagnostic_stale_kbuild_dir_fixup" => {
            validate_reported_payload_value(
                "proof",
                proof_payload,
                "class",
                "StaleKbuildDirectoryRef",
            )?;
        }
        "diagnostic_stale_kbuild_object_fixup" => {
            validate_reported_payload_value(
                "proof",
                proof_payload,
                "class",
                "StaleKbuildObjectRef",
            )?;
        }
        "diagnostic_missing_kconfig_source_fixup" => {
            validate_reported_payload_value(
                "proof",
                proof_payload,
                "class",
                "MissingKconfigSource",
            )?;
        }
        "diagnostic_preprocessor_refold_fixup" => {
            validate_reported_payload_value_in(
                "proof",
                proof_payload,
                "class",
                &[
                    "DeadConfigGatedCodePath",
                    "RemovedConfigSymbolUse",
                    "RemovedHeaderSymbolUse",
                ],
            )?;
        }
        "manifest_path" => {
            validate_reported_payload_values_match(reason_payload, "path", proof_payload, "path")?;
        }
        "manifest_config" | "simplified_tristate_expr" => {
            validate_reported_payload_values_match(
                reason_payload,
                "symbol",
                proof_payload,
                "symbol",
            )?;
        }
        "removed_header" => {
            validate_reported_payload_values_match(
                reason_payload,
                "header",
                proof_payload,
                "header",
            )?;
        }
        "removed_kbuild_ref" => {
            validate_reported_payload_values_match(
                reason_payload,
                "reference",
                proof_payload,
                "key",
            )?;
        }
        "build_diagnostic" => {
            validate_reported_payload_values_match(
                reason_payload,
                "class",
                proof_payload,
                "class",
            )?;
        }
        _ => {}
    }
    Ok(())
}

pub fn validate_reported_no_speculative_fallout_edit(
    reason_kind: &str,
    reason_payload: &str,
    proof_kind: &str,
    proof_payload: &str,
) -> Result<()> {
    if reported_truth_is_broad_speculative_fallout(reason_kind, reason_payload)
        || reported_truth_is_broad_speculative_fallout(proof_kind, proof_payload)
    {
        anyhow::bail!(
            "broad speculative fallout edits are forbidden: reason {} ({}) proof {} ({})",
            reason_kind,
            reason_payload,
            proof_kind,
            proof_payload
        );
    }
    Ok(())
}

fn reported_truth_is_broad_speculative_fallout(kind: &str, payload: &str) -> bool {
    matches!(kind, "build_diagnostic" | "classified_build_diagnostic")
        && reported_payload_value(payload, "class") == Some("UndefinedReference")
}

fn validate_reported_payload_values_match(
    reason_payload: &str,
    reason_key: &str,
    proof_payload: &str,
    proof_key: &str,
) -> Result<()> {
    let reason_value = require_reported_payload_value("reason", reason_payload, reason_key)?;
    let proof_value = require_reported_payload_value("proof", proof_payload, proof_key)?;
    if reason_value != proof_value {
        anyhow::bail!(
            "reason payload {reason_key}={reason_value} conflicts with proof payload {proof_key}={proof_value}"
        );
    }
    Ok(())
}

fn validate_reported_payload_value(
    label: &str,
    payload: &str,
    key: &str,
    expected: &str,
) -> Result<()> {
    let value = require_reported_payload_value(label, payload, key)?;
    if value != expected {
        anyhow::bail!("{label} payload {key}={value} does not match expected {key}={expected}");
    }
    Ok(())
}

fn validate_reported_payload_value_in(
    label: &str,
    payload: &str,
    key: &str,
    expected: &[&str],
) -> Result<()> {
    let value = require_reported_payload_value(label, payload, key)?;
    if !expected.iter().any(|candidate| *candidate == value) {
        anyhow::bail!("{label} payload {key}={value} does not match any expected {key} value");
    }
    Ok(())
}

fn require_reported_payload_value<'a>(label: &str, payload: &'a str, key: &str) -> Result<&'a str> {
    let value = reported_payload_value(payload, key)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("{label} payload is missing non-empty {key}"))?;
    Ok(value)
}

fn reported_payload_value<'a>(payload: &'a str, key: &str) -> Option<&'a str> {
    payload.split_whitespace().find_map(|part| {
        let (candidate_key, value) = part.split_once('=')?;
        (candidate_key == key).then_some(value)
    })
}
