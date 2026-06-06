//! Include cleanup rewrite planning, reporting, and application.
//!
//! This module owns proof-gated include-line removal reports and verified
//! rewrite application. Include-site discovery, target resolution, private
//! header rules, and public-header policy stay in their owned slices.

use anyhow::Result;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::edit_reason::{
    ensure_edit_records_for_mutation, sort_edit_records, write_verified_rewrite, EditProofSource,
    EditReason, EditRecord, LineRange,
};

use super::{
    c_family_files, classify_include_targets_with_removal_proofs, index_include_sites,
    is_surviving_public_header_site, manifest_removed_private_header_proof_path,
    relative_to_root_path, report_live_include_site_needing_manual_handling,
    resolve_include_targets_for_removed_headers,
    should_report_conservatively_preserved_public_header, should_report_missing_public_header,
    ClassifiedIncludeTarget, HeaderRemovalProofs, IncludeSite,
};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ManualIncludeHandlingKind {
    LiveMissingInclude,
    AmbiguousInclude,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ManualIncludeHandlingSite {
    pub site: IncludeSite,
    pub kind: ManualIncludeHandlingKind,
    pub classified_targets: Vec<ClassifiedIncludeTarget>,
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IncludeReportCounts {
    pub removed_include_lines: usize,
    pub live_missing_includes: usize,
    pub public_headers_preserved: usize,
    pub ambiguous_includes: usize,
}

#[allow(dead_code)]
#[derive(Debug, Default)]
pub(crate) struct IncludeRewriteReport {
    pub counts: IncludeReportCounts,
    pub edits: Vec<EditRecord>,
    pub manual_sites: Vec<ManualIncludeHandlingSite>,
    rewrites: Vec<PendingIncludeRewrite>,
}

#[derive(Debug)]
struct PendingIncludeRewrite {
    path: PathBuf,
    content: String,
    edits: Vec<EditRecord>,
}

#[derive(Debug, Clone)]
struct IncludeRemovalSite {
    site: IncludeSite,
    proof: IncludeRemovalProof,
}

#[derive(Debug, Clone)]
enum IncludeRemovalProof {
    ManifestHeader { manifest_path: PathBuf },
    DeadBranch { symbol: String },
}

#[allow(dead_code)]
pub(crate) fn rewrite_removed_header_includes(
    root: &Path,
    removal_proofs: &HeaderRemovalProofs,
) -> Result<Vec<EditRecord>> {
    let report = rewrite_removed_header_includes_report(root, removal_proofs)?;
    apply_include_rewrite_report(root, &report)?;
    Ok(report.edits)
}

#[allow(dead_code)]
pub(crate) fn rewrite_removed_header_includes_report(
    root: &Path,
    removal_proofs: &HeaderRemovalProofs,
) -> Result<IncludeRewriteReport> {
    rewrite_removed_header_includes_report_with_removed_configs(root, removal_proofs, &[])
}

#[allow(dead_code)]
pub(crate) fn rewrite_removed_header_includes_report_with_removed_configs(
    root: &Path,
    removal_proofs: &HeaderRemovalProofs,
    removed_configs: &[String],
) -> Result<IncludeRewriteReport> {
    let mut report = IncludeRewriteReport::default();
    let mut removal_sites_by_file = BTreeMap::<PathBuf, Vec<IncludeRemovalSite>>::new();
    let dead_branch_proofs_by_file =
        proven_dead_include_line_proofs_by_file(root, removed_configs)?;

    for site in index_include_sites(root)? {
        let targets = resolve_include_targets_for_removed_headers(root, &site, removal_proofs);
        let classified =
            classify_include_targets_with_removal_proofs(root, &targets, Some(removal_proofs));
        let dead_branch_proof = dead_branch_proofs_by_file
            .get(&site.file)
            .and_then(|proofs| proofs.get(&site.line));
        let site_live_after_preprocessor = dead_branch_proof.is_none();
        record_manual_include_site(
            &mut report,
            root,
            removal_proofs,
            &site,
            &classified,
            site_live_after_preprocessor,
        );
        if is_surviving_public_header_site(&classified) {
            report.counts.public_headers_preserved += 1;
        }
        if let Some(manifest_path) = manifest_removed_private_header_proof_path(
            root,
            removal_proofs,
            &classified,
            site_live_after_preprocessor,
        ) {
            removal_sites_by_file
                .entry(site.file.clone())
                .or_default()
                .push(IncludeRemovalSite {
                    site,
                    proof: IncludeRemovalProof::ManifestHeader { manifest_path },
                });
        } else if let Some(dead_branch_proof) = dead_branch_proof {
            removal_sites_by_file
                .entry(site.file.clone())
                .or_default()
                .push(IncludeRemovalSite {
                    site,
                    proof: IncludeRemovalProof::DeadBranch {
                        symbol: dead_branch_proof.symbol.clone(),
                    },
                });
        }
    }

    for (relative_file, removal_sites) in removal_sites_by_file {
        let source_path = root.join(&relative_file);
        let content = std::fs::read_to_string(&source_path)?;
        let line_removals = removal_sites
            .into_iter()
            .map(|removal| (removal.site.line, removal))
            .collect::<BTreeMap<_, _>>();
        let mut rewritten = String::with_capacity(content.len());
        let mut file_edits = Vec::new();

        for (idx, line) in content.split_inclusive('\n').enumerate() {
            let line_number = idx + 1;
            let Some(site) = line_removals.get(&line_number) else {
                rewritten.push_str(line);
                continue;
            };
            let (reason, proof_source) = include_removal_reason_and_proof(site);

            file_edits.push(EditRecord::new(
                relative_file.clone(),
                Some(LineRange {
                    start: line_number,
                    end: line_number,
                }),
                line.to_string(),
                String::new(),
                reason,
                proof_source,
                "includes.rewrite_removed_headers",
            ));
        }

        if !file_edits.is_empty() {
            report.edits.extend(file_edits.clone());
            report.rewrites.push(PendingIncludeRewrite {
                path: source_path,
                content: rewritten,
                edits: file_edits,
            });
        }
    }

    report.counts.removed_include_lines = report.edits.len();
    ensure_edit_records_for_mutation(
        "includes.rewrite_removed_headers",
        report
            .counts
            .removed_include_lines
            .max(report.rewrites.len()),
        &report.edits,
    )?;
    canonicalize_include_rewrite_report(&mut report);
    Ok(report)
}

fn canonicalize_include_rewrite_report(report: &mut IncludeRewriteReport) {
    sort_edit_records(&mut report.edits);
    for site in &mut report.manual_sites {
        site.classified_targets.sort();
        site.classified_targets.dedup();
    }
    report.manual_sites.sort();
    report.rewrites.sort_by(|left, right| left.path.cmp(&right.path));
    for rewrite in &mut report.rewrites {
        sort_edit_records(&mut rewrite.edits);
    }
}

fn include_removal_reason_and_proof(removal: &IncludeRemovalSite) -> (EditReason, EditProofSource) {
    match &removal.proof {
        IncludeRemovalProof::ManifestHeader { manifest_path } => (
            EditReason::RemovedHeader {
                header: removal.site.header.clone(),
            },
            EditProofSource::removal_manifest_header(
                removal.site.header.clone(),
                manifest_path.clone(),
            ),
        ),
        IncludeRemovalProof::DeadBranch { symbol } => (
            EditReason::RemovedDeadBranchInclude {
                header: removal.site.header.clone(),
                symbol: symbol.clone(),
            },
            EditProofSource::removal_manifest_config(symbol.clone()),
        ),
    }
}

fn proven_dead_include_line_proofs_by_file(
    root: &Path,
    removed_configs: &[String],
) -> Result<BTreeMap<PathBuf, BTreeMap<usize, crate::source_scan::cpp::DeadCppBranchProof>>> {
    let mut proofs_by_file = BTreeMap::new();
    if removed_configs.is_empty() {
        return Ok(proofs_by_file);
    }

    for path in c_family_files(root) {
        let content = std::fs::read_to_string(&path)?;
        let lines = content.lines().collect::<Vec<_>>();
        let proofs = crate::source_scan::cpp::proven_dead_cpp_branch_lines(&lines, removed_configs);
        if !proofs.is_empty() {
            proofs_by_file.insert(relative_to_root_path(root, &path), proofs);
        }
    }

    Ok(proofs_by_file)
}

fn record_manual_include_site(
    report: &mut IncludeRewriteReport,
    root: &Path,
    removal_proofs: &HeaderRemovalProofs,
    site: &IncludeSite,
    classified: &[ClassifiedIncludeTarget],
    site_live_after_preprocessor: bool,
) {
    if let Some(manual) = report_live_include_site_needing_manual_handling(
        site,
        classified,
        site_live_after_preprocessor,
    ) {
        match manual.kind {
            ManualIncludeHandlingKind::AmbiguousInclude => {
                report.counts.ambiguous_includes += 1;
                report.manual_sites.push(manual);
            }
            ManualIncludeHandlingKind::LiveMissingInclude
                if should_report_missing_public_header(site, classified) =>
            {
                report.counts.live_missing_includes += 1;
                report.manual_sites.push(manual);
            }
            ManualIncludeHandlingKind::LiveMissingInclude => {}
        }
        return;
    }

    if site_live_after_preprocessor
        && should_report_conservatively_preserved_public_header(root, removal_proofs, classified)
    {
        report.counts.live_missing_includes += 1;
        report.manual_sites.push(ManualIncludeHandlingSite {
            site: site.clone(),
            kind: ManualIncludeHandlingKind::LiveMissingInclude,
            classified_targets: classified.to_vec(),
        });
    }
}

#[allow(dead_code)]
pub(crate) fn apply_include_rewrite_report(
    root: &Path,
    report: &IncludeRewriteReport,
) -> Result<()> {
    ensure_edit_records_for_mutation(
        "includes.rewrite_removed_headers",
        report
            .counts
            .removed_include_lines
            .max(report.rewrites.len()),
        &report.edits,
    )?;

    for rewrite in &report.rewrites {
        write_verified_rewrite(
            root,
            &rewrite.path,
            &rewrite.content,
            &rewrite.edits,
            "includes.rewrite_removed_headers",
        )?;
    }

    Ok(())
}
