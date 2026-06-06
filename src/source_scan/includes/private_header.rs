//! Private-header include orphaning decisions.
//!
//! This module owns the gates that decide when a resolved include target is a
//! removed private header orphan and can be dropped from the including source.
//! Public-header ABI policy remains separate and is queried only to avoid
//! treating ABI-preserved public headers as private orphan cleanup.

use std::path::{Path, PathBuf};

use super::{
    is_generated_header_target, is_public_preserved_header_path,
    target_is_explicitly_removed_public_header, ClassifiedIncludeTarget, HeaderRemovalProofs,
    IncludeResolveRule, IncludeTargetClassification,
};

#[allow(dead_code)]
pub(crate) fn target_is_gone_from_reduced_tree(
    classified_targets: &[ClassifiedIncludeTarget],
) -> bool {
    !classified_targets.is_empty()
        && classified_targets.iter().all(|classified| {
            matches!(
                classified.classification,
                IncludeTargetClassification::RemovedByManifest
                    | IncludeTargetClassification::AbsentForUnknownReason
            )
        })
}

#[allow(dead_code)]
pub(crate) fn target_is_covered_by_removal_manifest(
    classified_targets: &[ClassifiedIncludeTarget],
) -> bool {
    !classified_targets.is_empty()
        && classified_targets.iter().all(|classified| {
            classified.classification == IncludeTargetClassification::RemovedByManifest
        })
}

#[allow(dead_code)]
pub(crate) fn include_site_passes_preprocessor_or_local_rule_gate(
    classified_targets: &[ClassifiedIncludeTarget],
    site_live_after_preprocessor: bool,
) -> bool {
    if !site_live_after_preprocessor {
        return !classified_targets.is_empty();
    }

    local_removal_rule_applies(classified_targets)
}

#[allow(dead_code)]
pub(crate) fn local_removal_rule_applies(classified_targets: &[ClassifiedIncludeTarget]) -> bool {
    !classified_targets.is_empty()
        && classified_targets.iter().all(|classified| {
            matches!(
                classified.target.rule,
                IncludeResolveRule::LocalDirectory | IncludeResolveRule::FileRelativeQuoted
            )
        })
}

#[allow(dead_code)]
fn should_remove_manifest_removed_private_header(
    classified_targets: &[ClassifiedIncludeTarget],
    site_live_after_preprocessor: bool,
) -> bool {
    target_is_gone_from_reduced_tree(classified_targets)
        && target_is_covered_by_removal_manifest(classified_targets)
        && include_site_passes_preprocessor_or_local_rule_gate(
            classified_targets,
            site_live_after_preprocessor,
        )
        && classified_targets.iter().all(|classified| {
            !is_public_preserved_header_path(&classified.target.path)
                && !is_generated_header_target(&classified.target)
        })
}

#[allow(dead_code)]
pub(in crate::source_scan::includes) fn manifest_removed_private_header_proof_path(
    root: &Path,
    removal_proofs: &HeaderRemovalProofs,
    classified_targets: &[ClassifiedIncludeTarget],
    site_live_after_preprocessor: bool,
) -> Option<PathBuf> {
    if target_is_explicitly_removed_public_header(root, removal_proofs, classified_targets) {
        return removal_proofs.explicit_proof_path_for(&classified_targets[0].target.path);
    }

    if !should_remove_manifest_removed_private_header(
        classified_targets,
        site_live_after_preprocessor,
    ) {
        return None;
    }

    let mut proof_paths = Vec::new();
    for classified in classified_targets {
        if root.join(&classified.target.path).exists() {
            return None;
        }
        proof_paths.push(removal_proofs.proof_path_for(&classified.target.path)?);
    }

    proof_paths.sort();
    proof_paths.dedup();
    proof_paths.into_iter().next()
}
