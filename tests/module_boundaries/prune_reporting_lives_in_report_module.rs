use super::common::*;

#[test]
fn prune_reporting_lives_in_report_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let prune = production_source(&root.join("src/prune.rs"));
    let report = production_source(&root.join("src/prune/report.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod report;",
        "pub use report::{prune_tree_from_manifest, PruneStats}",
        "pub(crate) use report::continue_prune_after_kconfig",
    ] {
        assert!(
            prune.contains(required),
            "src/prune.rs should expose prune reporting through {required}"
        );
    }

    for required in [
        "pub struct PruneStats",
        "pub fn prune_tree_from_manifest(",
        "pub(crate) fn continue_prune_from_declared(",
        "pub(crate) fn continue_prune_after_kconfig(",
        "rewrite_build_graph(",
        "sort_edit_records(&mut edits)",
        "PruneResult {",
        "kconfig_refs_removed: kconfig_stage.kconfig_report.removed_sources",
        "makefile_refs_removed: rewrite_stats.makefile_refs_removed",
        "skipped_makefile_lines: rewrite_stats.skipped_makefile_lines",
    ] {
        assert!(
            report.contains(required),
            "src/prune/report.rs should own prune reporting item {required}"
        );
    }

    for forbidden in [
        "\npub struct PruneStats",
        "\npub(crate) fn continue_prune_from_declared(",
        "\npub(crate) fn continue_prune_after_kconfig(",
        "sort_edit_records(&mut edits)",
        "kconfig_refs_removed: kconfig_stage.kconfig_report.removed_sources",
        "makefile_refs_removed: rewrite_stats.makefile_refs_removed",
    ] {
        assert!(
            !prune.contains(forbidden),
            "src/prune.rs should not retain extracted prune reporting implementation {forbidden}"
        );
    }

    for required in ["`src/prune/report.rs`", "Prune reporting"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document prune reporting module ownership through {required}"
        );
    }
}
