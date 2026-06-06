use anyhow::{Context, Result};
use std::path::Path;

use crate::config::ProfileConfig;
use crate::diagnostics::{classify_selftest_failure, ClassifiedDiagnostic};
use crate::edit_reason::{
    ensure_edit_records_for_mutation, sort_edit_records, validate_edit_records_with_policy,
    EditRecord, EditValidationPolicy,
};
use crate::fixups::{FixupAttempt, SkippedFixup};
use crate::selftest::SelfTestFailure;

use super::ReducerStats;

pub(crate) fn audit_mutating_pass_edits(
    pass_name: &str,
    mutation_count: usize,
    edits: &[EditRecord],
    reducer_config: &crate::config::ReducerConfig,
) -> Result<()> {
    validate_reducer_edit_records(edits, reducer_config).with_context(|| {
        format!("invalid canonical proof source in mutating pass output '{pass_name}'")
    })?;

    ensure_edit_records_for_mutation(pass_name, mutation_count, edits)?;

    Ok(())
}

pub(crate) fn validate_reducer_edit_provenance(
    stats: &ReducerStats,
    reducer_config: &crate::config::ReducerConfig,
) -> Result<()> {
    validate_reducer_edit_records(&stats.edits, reducer_config)
}

fn validate_reducer_edit_records(
    edits: &[EditRecord],
    reducer_config: &crate::config::ReducerConfig,
) -> Result<()> {
    validate_edit_records_with_policy(edits, edit_validation_policy(reducer_config))
}

fn edit_validation_policy(
    reducer_config: &crate::config::ReducerConfig,
) -> EditValidationPolicy {
    EditValidationPolicy {
        reject_unreasoned_edits: reducer_config.reject_unreasoned_edits,
        reject_speculative_fallout_edits: reducer_config.reject_speculative_fallout_edits,
    }
}

pub fn apply_selftest_fixup(
    tree_path: &str,
    profile: &ProfileConfig,
    stats: &mut ReducerStats,
    failure: &SelfTestFailure,
) -> Result<bool> {
    if !stats.ran {
        return Ok(false);
    }

    let classified = classify_selftest_failure(Path::new(tree_path), failure);
    if classified.is_unknown_class() {
        log::warn!("reducer: stopping on unknown diagnostic: {}", failure);
        stats.skipped_fixups.push(SkippedFixup {
            fixer_name: None,
            diagnostic: classified,
            reason: String::from("unknown diagnostic"),
        });
        return Ok(false);
    }

    let attempt = match crate::fixups::apply_classified_fixup(
        Path::new(tree_path),
        &stats.removal,
        &classified,
    ) {
        Ok(attempt) => attempt,
        Err(err) => {
            log::warn!(
                "reducer: rejected deterministic fixup for class={}: {:#}",
                classified.class().stable_name(),
                err
            );
            stats.skipped_fixups.push(SkippedFixup {
                fixer_name: None,
                diagnostic: classified,
                reason: err.to_string(),
            });
            return Ok(false);
        }
    };
    match attempt {
        FixupAttempt::Applied(applied) => {
            let mut applied = applied;
            sort_edit_records(&mut applied.edits);
            audit_mutating_pass_edits(applied.fixer_name, 1, &applied.edits, &profile.reducer)?;
            stats.edits.extend(applied.edits.clone());
            if matches!(
                applied.diagnostic,
                ClassifiedDiagnostic::MissingKconfigSource { .. }
            ) {
                apply_additional_cpp_fold_after_config_truth_update(
                    tree_path,
                    stats,
                    &profile.reducer,
                )?;
            }
            sort_edit_records(&mut stats.edits);
            validate_reducer_edit_provenance(stats, &profile.reducer)?;
            stats.applied_fixups.push(applied);
            Ok(true)
        }
        FixupAttempt::Skipped(skipped) => {
            log::warn!(
                "reducer: skipped deterministic fixup for class={}: {}",
                skipped.diagnostic.class().stable_name(),
                skipped.reason
            );
            stats.skipped_fixups.push(skipped);
            Ok(false)
        }
    }
}

fn apply_additional_cpp_fold_after_config_truth_update(
    tree_path: &str,
    stats: &mut ReducerStats,
    reducer_config: &crate::config::ReducerConfig,
) -> Result<()> {
    if stats.removal.removed_config_symbols.is_empty() {
        return Ok(());
    }

    let report = crate::cpp::fold_removed_config_branches_report(
        Path::new(tree_path),
        &stats.removal.removed_config_symbols,
    )?;
    if report.edits.is_empty() {
        return Ok(());
    }
    audit_mutating_pass_edits(
        "cpp.fold_removed_config_branches",
        report
            .counts
            .branches_folded
            .max(report.counts.files_touched),
        &report.edits,
        reducer_config,
    )?;

    crate::cpp::apply_fold_report(Path::new(tree_path), &report)?;
    stats.cpp_report.branches_folded += report.counts.branches_folded;
    stats.cpp_report.files_touched += report.counts.files_touched;
    stats.cpp_report.skipped_nested_edge_cases += report.counts.skipped_nested_edge_cases;
    stats.edits.extend(report.edits);
    stats
        .unsupported_cpp_expressions
        .extend(report.unsupported_expressions);
    stats
        .skipped_cpp_nested_edge_cases
        .extend(report.skipped_nested_edge_cases);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit_reason::{EditProofSource, EditReason};
    use std::path::PathBuf;

    fn manifest_path_edit(path: &str) -> EditRecord {
        EditRecord::new(
            PathBuf::from(path),
            None,
            String::from("before\n"),
            String::new(),
            EditReason::ManifestPath {
                path: PathBuf::from(path),
            },
            EditProofSource::removal_manifest_path(PathBuf::from(path)),
            "test.pass",
        )
    }

    #[test]
    fn audit_mutating_pass_edits_rejects_noncanonical_proof_source() {
        let mut edit = manifest_path_edit("drivers/foo/old.c");
        edit.proof_source = EditProofSource::stale_kbuild_reference(String::from("foo.o"));

        let err = audit_mutating_pass_edits(
            "test.pass",
            1,
            &[edit],
            &crate::config::ReducerConfig::default(),
        )
        .unwrap_err();
        let err = format!("{err:#}");

        assert!(err.contains("mutating pass output 'test.pass'"));
        assert!(err.contains("multiple competing proof sources"));
    }

    #[test]
    fn audit_mutating_pass_edits_rejects_mutation_without_matching_edit_record() {
        let err = audit_mutating_pass_edits(
            "test.pass",
            1,
            &[],
            &crate::config::ReducerConfig::default(),
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("without edit records"));

        let edit = manifest_path_edit("drivers/foo/old.c");
        let err = audit_mutating_pass_edits(
            "test.pass",
            2,
            &[edit],
            &crate::config::ReducerConfig::default(),
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("2 mutation(s)"));
        assert!(err.contains("only 1 matching edit record"));

        audit_mutating_pass_edits(
            "test.pass",
            0,
            &[],
            &crate::config::ReducerConfig::default(),
        )
        .unwrap();
    }
}
