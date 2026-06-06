//! Include-site resolution and cleanup for headers actually removed by pruning.
//!
//! This module will report or rewrite include sites only when the resolved
//! header target was removed by the reducer.

use std::path::{Path, PathBuf};

#[cfg(test)]
use crate::edit_reason::{EditProofSource, EditReason, EditRecord};

mod cleanup;
mod index;
mod private_header;
mod policy;

#[allow(unused_imports)]
pub(crate) use cleanup::{
    apply_include_rewrite_report, rewrite_removed_header_includes,
    rewrite_removed_header_includes_report,
    rewrite_removed_header_includes_report_with_removed_configs, IncludeReportCounts,
    IncludeRewriteReport, ManualIncludeHandlingKind, ManualIncludeHandlingSite,
};

pub(in crate::source_scan::includes) use index::{c_family_files, relative_to_root_path};
pub(in crate::source_scan::includes) use private_header::manifest_removed_private_header_proof_path;
pub(in crate::source_scan::includes) use policy::{
    is_generated_header_target, is_public_preserved_header_path, is_surviving_public_header_site,
    should_report_conservatively_preserved_public_header, should_report_missing_public_header,
    target_is_explicitly_removed_public_header,
};
#[allow(unused_imports)]
pub(crate) use private_header::{
    include_site_passes_preprocessor_or_local_rule_gate, local_removal_rule_applies,
    target_is_covered_by_removal_manifest, target_is_gone_from_reduced_tree,
};
#[cfg(test)]
pub(in crate::source_scan::includes) use index::parse_include_site;
pub(crate) use index::{index_include_sites, IncludeKind, IncludeSite};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum IncludeResolveRule {
    LocalDirectory,
    FileRelativeQuoted,
    IncludeRoot,
    ArchIncludeRoot,
    ConfiguredGeneratedRoot,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ResolvedIncludeTarget {
    pub path: PathBuf,
    pub rule: IncludeResolveRule,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum IncludeTargetClassification {
    Exists,
    RemovedByManifest,
    AbsentForUnknownReason,
    PublicPreservedHeader,
    GeneratedHeader,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ClassifiedIncludeTarget {
    pub target: ResolvedIncludeTarget,
    pub classification: IncludeTargetClassification,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct HeaderRemovalProofs {
    manifest_paths: Vec<PathBuf>,
    removed_header_paths: Vec<PathBuf>,
    abi_policy: crate::abi::AbiPolicyConfig,
}

impl HeaderRemovalProofs {
    #[allow(dead_code)]
    pub(crate) fn from_manifest_paths(
        manifest_paths: &[PathBuf],
        removed_header_paths: &[PathBuf],
    ) -> Self {
        Self::from_manifest_paths_with_abi_policy(
            manifest_paths,
            removed_header_paths,
            &crate::abi::AbiPolicyConfig::default(),
        )
    }

    pub(crate) fn from_manifest_paths_with_abi_policy(
        manifest_paths: &[PathBuf],
        removed_header_paths: &[PathBuf],
        abi_policy: &crate::abi::AbiPolicyConfig,
    ) -> Self {
        let mut manifest_paths = manifest_paths.to_vec();
        manifest_paths.sort();
        manifest_paths.dedup();
        let mut removed_header_paths = removed_header_paths.to_vec();
        removed_header_paths.sort();
        removed_header_paths.dedup();
        Self {
            manifest_paths,
            removed_header_paths,
            abi_policy: abi_policy.clone(),
        }
    }

    pub(in crate::source_scan::includes) fn proof_path_for(&self, path: &Path) -> Option<PathBuf> {
        self.manifest_paths
            .iter()
            .find(|removed| path == removed.as_path() || path.starts_with(removed.as_path()))
            .cloned()
    }

    pub(in crate::source_scan::includes) fn explicit_proof_path_for(&self, path: &Path) -> Option<PathBuf> {
        self.manifest_paths
            .iter()
            .find(|removed| path == removed.as_path())
            .cloned()
    }

    fn proof_path_for_removed_header(&self, path: &Path) -> Option<PathBuf> {
        self.contains_removed_header(path)
            .then(|| self.proof_path_for(path))
            .flatten()
    }

    fn contains_removed_header(&self, path: &Path) -> bool {
        self.removed_header_paths
            .iter()
            .any(|removed| removed == path)
    }

    #[cfg(test)]
    fn with_removed_header_paths(&self, removed_header_paths: &[PathBuf]) -> Self {
        Self::from_manifest_paths_with_abi_policy(
            &self.manifest_paths,
            removed_header_paths,
            &self.abi_policy,
        )
    }
}

#[allow(dead_code)]
pub(crate) fn resolve_include_targets(
    root: &Path,
    site: &IncludeSite,
) -> Vec<ResolvedIncludeTarget> {
    resolve_include_targets_with_generated_roots(root, site, &[])
}

#[allow(dead_code)]
pub(crate) fn classify_include_targets(
    root: &Path,
    targets: &[ResolvedIncludeTarget],
) -> Vec<ClassifiedIncludeTarget> {
    classify_include_targets_with_removal_proofs(root, targets, None)
}

#[allow(dead_code)]
pub(crate) fn classify_include_targets_with_removal_proofs(
    root: &Path,
    targets: &[ResolvedIncludeTarget],
    removal_proofs: Option<&HeaderRemovalProofs>,
) -> Vec<ClassifiedIncludeTarget> {
    targets
        .iter()
        .filter_map(|target| classify_include_target(root, target, removal_proofs))
        .collect()
}

#[allow(dead_code)]
pub(crate) fn report_live_include_site_needing_manual_handling(
    site: &IncludeSite,
    classified_targets: &[ClassifiedIncludeTarget],
    site_live_after_preprocessor: bool,
) -> Option<ManualIncludeHandlingSite> {
    if !site_live_after_preprocessor {
        return None;
    }

    if preserve_subsystem_looking_include_when_resolved_header_exists(site, classified_targets) {
        return None;
    }

    if classified_targets.is_empty()
        || classified_targets.iter().any(|classified| {
            classified.classification == IncludeTargetClassification::AbsentForUnknownReason
        })
    {
        return Some(ManualIncludeHandlingSite {
            site: site.clone(),
            kind: ManualIncludeHandlingKind::LiveMissingInclude,
            classified_targets: classified_targets.to_vec(),
        });
    }

    if classified_targets.len() > 1 {
        return Some(ManualIncludeHandlingSite {
            site: site.clone(),
            kind: ManualIncludeHandlingKind::AmbiguousInclude,
            classified_targets: classified_targets.to_vec(),
        });
    }

    None
}

#[allow(dead_code)]
pub(crate) fn preserve_subsystem_looking_include_when_resolved_header_exists(
    site: &IncludeSite,
    classified_targets: &[ClassifiedIncludeTarget],
) -> bool {
    is_subsystem_looking_include(site)
        && classified_targets.len() == 1
        && classified_targets[0].classification == IncludeTargetClassification::Exists
        && matches!(
            classified_targets[0].target.rule,
            IncludeResolveRule::LocalDirectory | IncludeResolveRule::FileRelativeQuoted
        )
}

#[allow(dead_code)]
fn is_subsystem_looking_include(site: &IncludeSite) -> bool {
    site.header.contains('/')
}

#[allow(dead_code)]
fn should_report_ambiguous_include(
    site: &IncludeSite,
    classified_targets: &[ClassifiedIncludeTarget],
) -> bool {
    matches!(
        report_live_include_site_needing_manual_handling(site, classified_targets, true),
        Some(ManualIncludeHandlingSite {
            kind: ManualIncludeHandlingKind::AmbiguousInclude,
            ..
        })
    )
}

#[allow(dead_code)]
pub(crate) fn resolve_include_targets_with_generated_roots(
    root: &Path,
    site: &IncludeSite,
    generated_roots: &[PathBuf],
) -> Vec<ResolvedIncludeTarget> {
    let mut targets = Vec::new();

    push_existing_target(
        root,
        candidate_local_directory_target(site),
        IncludeResolveRule::LocalDirectory,
        &mut targets,
    );
    push_existing_target(
        root,
        candidate_file_relative_quoted_target(site),
        IncludeResolveRule::FileRelativeQuoted,
        &mut targets,
    );
    push_existing_target(
        root,
        candidate_include_root_target(site),
        IncludeResolveRule::IncludeRoot,
        &mut targets,
    );
    push_existing_target(
        root,
        candidate_arch_include_root_target(site),
        IncludeResolveRule::ArchIncludeRoot,
        &mut targets,
    );

    for generated_root in generated_roots {
        push_existing_target(
            root,
            candidate_configured_generated_root_target(site, generated_root),
            IncludeResolveRule::ConfiguredGeneratedRoot,
            &mut targets,
        );
    }

    targets
}

#[allow(dead_code)]
pub(in crate::source_scan::includes) fn resolve_include_targets_for_removed_headers(
    root: &Path,
    site: &IncludeSite,
    removal_proofs: &HeaderRemovalProofs,
) -> Vec<ResolvedIncludeTarget> {
    let mut targets = Vec::new();

    push_existing_or_proven_removed_target(
        root,
        removal_proofs,
        candidate_local_directory_target(site),
        IncludeResolveRule::LocalDirectory,
        &mut targets,
    );
    push_existing_or_proven_removed_target(
        root,
        removal_proofs,
        candidate_file_relative_quoted_target(site),
        IncludeResolveRule::FileRelativeQuoted,
        &mut targets,
    );
    push_existing_or_proven_removed_target(
        root,
        removal_proofs,
        candidate_include_root_target(site),
        IncludeResolveRule::IncludeRoot,
        &mut targets,
    );
    push_existing_or_proven_removed_target(
        root,
        removal_proofs,
        candidate_arch_include_root_target(site),
        IncludeResolveRule::ArchIncludeRoot,
        &mut targets,
    );

    targets
}

#[allow(dead_code)]
fn classify_include_target(
    root: &Path,
    target: &ResolvedIncludeTarget,
    removal_proofs: Option<&HeaderRemovalProofs>,
) -> Option<ClassifiedIncludeTarget> {
    let target_path = root.join(&target.path);
    if target_path.is_file() {
        let classification = if is_generated_header_target(target) {
            IncludeTargetClassification::GeneratedHeader
        } else if is_public_preserved_header_path(&target.path) {
            IncludeTargetClassification::PublicPreservedHeader
        } else {
            IncludeTargetClassification::Exists
        };
        return Some(ClassifiedIncludeTarget {
            target: target.clone(),
            classification,
        });
    }

    if removal_proofs.is_some_and(|removal_proofs| {
        removal_proofs
            .proof_path_for_removed_header(&target.path)
            .is_some()
    }) {
        return Some(ClassifiedIncludeTarget {
            target: target.clone(),
            classification: IncludeTargetClassification::RemovedByManifest,
        });
    }

    Some(ClassifiedIncludeTarget {
        target: target.clone(),
        classification: IncludeTargetClassification::AbsentForUnknownReason,
    })
}

#[allow(dead_code)]
fn resolve_local_directory_target(root: &Path, site: &IncludeSite) -> Option<PathBuf> {
    push_resolved_candidate(root, candidate_local_directory_target(site))
}

#[allow(dead_code)]
fn candidate_local_directory_target(site: &IncludeSite) -> Option<PathBuf> {
    if include_header_has_relative_segments(&site.header) {
        return None;
    }
    candidate_from_source_directory(site)
}

#[allow(dead_code)]
fn resolve_file_relative_quoted_target(root: &Path, site: &IncludeSite) -> Option<PathBuf> {
    push_resolved_candidate(root, candidate_file_relative_quoted_target(site))
}

#[allow(dead_code)]
fn candidate_file_relative_quoted_target(site: &IncludeSite) -> Option<PathBuf> {
    if site.kind != IncludeKind::Quoted || !include_header_has_relative_segments(&site.header) {
        return None;
    }
    candidate_from_source_directory(site)
}

#[allow(dead_code)]
fn resolve_include_root_target(root: &Path, site: &IncludeSite) -> Option<PathBuf> {
    push_resolved_candidate(root, candidate_include_root_target(site))
}

#[allow(dead_code)]
fn candidate_include_root_target(site: &IncludeSite) -> Option<PathBuf> {
    if include_header_has_relative_segments(&site.header) {
        return None;
    }
    normalize_root_relative_join(Path::new("include"), Path::new(&site.header))
}

#[allow(dead_code)]
fn resolve_arch_include_root_target(root: &Path, site: &IncludeSite) -> Option<PathBuf> {
    push_resolved_candidate(root, candidate_arch_include_root_target(site))
}

#[allow(dead_code)]
fn candidate_arch_include_root_target(site: &IncludeSite) -> Option<PathBuf> {
    if include_header_has_relative_segments(&site.header) {
        return None;
    }

    let arch = inferred_source_arch(site)?;
    normalize_root_relative_join(
        Path::new("arch").join(arch).join("include").as_path(),
        Path::new(&site.header),
    )
}

#[allow(dead_code)]
fn resolve_configured_generated_root_target(
    root: &Path,
    site: &IncludeSite,
    generated_root: &Path,
) -> Option<PathBuf> {
    push_resolved_candidate(
        root,
        candidate_configured_generated_root_target(site, generated_root),
    )
}

#[allow(dead_code)]
fn candidate_configured_generated_root_target(
    site: &IncludeSite,
    generated_root: &Path,
) -> Option<PathBuf> {
    if include_header_has_relative_segments(&site.header) {
        return None;
    }

    let generated_root = normalize_root_relative_join(Path::new(""), generated_root)?;
    if generated_root.as_os_str().is_empty() {
        return None;
    }

    normalize_root_relative_join(&generated_root, Path::new(&site.header))
}

#[allow(dead_code)]
fn resolve_from_source_directory(root: &Path, site: &IncludeSite) -> Option<PathBuf> {
    push_resolved_candidate(root, candidate_from_source_directory(site))
}

#[allow(dead_code)]
fn candidate_from_source_directory(site: &IncludeSite) -> Option<PathBuf> {
    let source_dir = site.file.parent().unwrap_or_else(|| Path::new(""));
    normalize_root_relative_join(source_dir, Path::new(&site.header))
}

#[allow(dead_code)]
fn push_existing_target(
    root: &Path,
    candidate: Option<PathBuf>,
    rule: IncludeResolveRule,
    targets: &mut Vec<ResolvedIncludeTarget>,
) {
    if let Some(path) = push_resolved_candidate(root, candidate) {
        targets.push(ResolvedIncludeTarget { path, rule });
    }
}

#[allow(dead_code)]
fn push_existing_or_proven_removed_target(
    root: &Path,
    removal_proofs: &HeaderRemovalProofs,
    candidate: Option<PathBuf>,
    rule: IncludeResolveRule,
    targets: &mut Vec<ResolvedIncludeTarget>,
) {
    let Some(path) = candidate else {
        return;
    };

    if root.join(&path).is_file()
        || removal_proofs
            .proof_path_for_removed_header(&path)
            .is_some()
    {
        targets.push(ResolvedIncludeTarget { path, rule });
    }
}

#[allow(dead_code)]
fn push_resolved_candidate(root: &Path, candidate: Option<PathBuf>) -> Option<PathBuf> {
    let path = candidate?;
    root.join(&path).is_file().then_some(path)
}

#[allow(dead_code)]
fn inferred_source_arch(site: &IncludeSite) -> Option<&str> {
    let mut components = site.file.components();
    match (
        components.next()?.as_os_str().to_str()?,
        components.next()?.as_os_str().to_str()?,
    ) {
        ("arch", arch) if !arch.is_empty() => Some(arch),
        _ => None,
    }
}

#[allow(dead_code)]
fn include_header_has_relative_segments(header: &str) -> bool {
    Path::new(header).components().any(|component| {
        matches!(
            component,
            std::path::Component::CurDir | std::path::Component::ParentDir
        )
    })
}

#[allow(dead_code)]
fn normalize_root_relative_join(base: &Path, suffix: &Path) -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::path::Component;

    let mut out = Vec::<OsString>::new();

    for component in base.components().chain(suffix.components()) {
        match component {
            Component::CurDir => {}
            Component::Normal(seg) => out.push(seg.to_os_string()),
            Component::ParentDir => {
                out.pop()?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    let mut path = PathBuf::new();
    for segment in out {
        path.push(segment);
    }
    Some(path)
}

#[cfg(test)]
mod tests;
