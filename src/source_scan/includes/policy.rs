//! Public-header include policy.
//!
//! This module owns ABI-sensitive public-header include preservation and
//! explicit-removal decisions. It delegates the authority bit to
//! `abi_policy`, keeps generated headers out of public-header removal, and
//! fails closed unless the removal manifest carries exact header truth plus
//! matching ABI policy approval.

use std::path::Path;

use super::{
    candidate_include_root_target, preserve_subsystem_looking_include_when_resolved_header_exists,
    ClassifiedIncludeTarget, HeaderRemovalProofs, IncludeResolveRule, IncludeSite,
    IncludeTargetClassification, ResolvedIncludeTarget,
};

#[allow(dead_code)]
pub(in crate::source_scan::includes) fn target_is_explicitly_removed_public_header(
    root: &Path,
    removal_proofs: &HeaderRemovalProofs,
    classified_targets: &[ClassifiedIncludeTarget],
) -> bool {
    let [classified] = classified_targets else {
        return false;
    };

    classified.classification == IncludeTargetClassification::RemovedByManifest
        && is_public_preserved_header_path(&classified.target.path)
        && crate::abi::allows_public_header_removal(
            &classified.target.path,
            &removal_proofs.abi_policy,
        )
        && !root.join(&classified.target.path).exists()
        && removal_proofs
            .explicit_proof_path_for(&classified.target.path)
            .is_some()
        && !is_generated_header_target(&classified.target)
}

#[allow(dead_code)]
pub(in crate::source_scan::includes) fn should_report_conservatively_preserved_public_header(
    root: &Path,
    removal_proofs: &HeaderRemovalProofs,
    classified_targets: &[ClassifiedIncludeTarget],
) -> bool {
    let [classified] = classified_targets else {
        return false;
    };

    classified.classification == IncludeTargetClassification::RemovedByManifest
        && is_public_preserved_header_path(&classified.target.path)
        && !root.join(&classified.target.path).exists()
        && removal_proofs
            .proof_path_for(&classified.target.path)
            .is_some_and(|proof_path| {
                proof_path != classified.target.path
                    || !crate::abi::allows_public_header_removal(
                        &classified.target.path,
                        &removal_proofs.abi_policy,
                    )
            })
}

#[allow(dead_code)]
pub(in crate::source_scan::includes) fn is_surviving_public_header_site(
    classified_targets: &[ClassifiedIncludeTarget],
) -> bool {
    classified_targets.len() == 1
        && classified_targets[0].classification
            == IncludeTargetClassification::PublicPreservedHeader
}

#[allow(dead_code)]
pub(in crate::source_scan::includes) fn should_report_missing_public_header(
    site: &IncludeSite,
    classified_targets: &[ClassifiedIncludeTarget],
) -> bool {
    if !candidate_include_root_target(site)
        .as_deref()
        .is_some_and(is_public_preserved_header_path)
    {
        return false;
    }

    if preserve_subsystem_looking_include_when_resolved_header_exists(site, classified_targets) {
        return false;
    }

    classified_targets.is_empty()
        || classified_targets.iter().any(|classified| {
            classified.classification == IncludeTargetClassification::AbsentForUnknownReason
        })
}

#[allow(dead_code)]
pub(in crate::source_scan::includes) fn is_public_preserved_header_path(path: &Path) -> bool {
    crate::abi::is_public_header_path(path)
}

#[allow(dead_code)]
pub(in crate::source_scan::includes) fn is_generated_header_target(target: &ResolvedIncludeTarget) -> bool {
    target.rule == IncludeResolveRule::ConfiguredGeneratedRoot
        || target.path.starts_with("include/generated")
        || is_arch_generated_header_path(&target.path)
}

#[allow(dead_code)]
fn is_arch_generated_header_path(path: &Path) -> bool {
    let mut components = path.components();
    matches!(
        (
            components.next().and_then(|part| part.as_os_str().to_str()),
            components.next().and_then(|part| part.as_os_str().to_str()),
            components.next().and_then(|part| part.as_os_str().to_str()),
            components.next().and_then(|part| part.as_os_str().to_str()),
        ),
        (
            Some("arch"),
            Some(_arch),
            Some("include"),
            Some("generated")
        )
    )
}
