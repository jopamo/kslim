use super::common::*;

#[test]
fn include_cleanup_rewrite_lives_in_cleanup_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let includes = production_source(&root.join("src/source_scan/includes/mod.rs"));
    let cleanup = production_source(&root.join("src/source_scan/includes/cleanup.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod cleanup;",
        "apply_include_rewrite_report",
        "rewrite_removed_header_includes_report_with_removed_configs",
        "IncludeReportCounts",
        "IncludeRewriteReport",
        "ManualIncludeHandlingSite",
    ] {
        assert!(
            includes.contains(required),
            "src/source_scan/includes/mod.rs should expose include cleanup rewrite/reporting through {required}"
        );
    }

    for required in [
        "pub(crate) struct IncludeReportCounts",
        "pub(crate) struct IncludeRewriteReport",
        "pub(crate) enum ManualIncludeHandlingKind",
        "pub(crate) struct ManualIncludeHandlingSite",
        "enum IncludeRemovalProof",
        "ManifestHeader",
        "DeadBranch",
        "pub(crate) fn rewrite_removed_header_includes(",
        "pub(crate) fn rewrite_removed_header_includes_report(",
        "pub(crate) fn rewrite_removed_header_includes_report_with_removed_configs(",
        "pub(crate) fn apply_include_rewrite_report(",
        "proven_dead_include_line_proofs_by_file",
        "record_manual_include_site",
        "index_include_sites(root)?",
        "resolve_include_targets_for_removed_headers(root, &site, removal_proofs)",
        "sort_edit_records(&mut report.edits)",
        "write_verified_rewrite(",
        "ensure_edit_records_for_mutation(",
    ] {
        assert!(
            cleanup.contains(required),
            "src/source_scan/includes/cleanup.rs should own include cleanup rewrite/reporting item {required}"
        );
    }

    for forbidden in [
        "\npub(crate) struct IncludeReportCounts",
        "\npub(crate) struct IncludeRewriteReport",
        "\npub(crate) enum ManualIncludeHandlingKind",
        "\npub(crate) struct ManualIncludeHandlingSite",
        "\nenum IncludeRemovalProof",
        "\npub(crate) fn rewrite_removed_header_includes(",
        "\npub(crate) fn rewrite_removed_header_includes_report(",
        "\npub(crate) fn rewrite_removed_header_includes_report_with_removed_configs(",
        "\npub(crate) fn apply_include_rewrite_report(",
        "\nfn proven_dead_include_line_proofs_by_file(",
        "\nfn record_manual_include_site(",
        "write_verified_rewrite(",
    ] {
        assert!(
            !includes.contains(forbidden),
            "src/source_scan/includes/mod.rs should not retain extracted include cleanup implementation {forbidden}"
        );
    }

    for required in ["`src/source_scan/includes/cleanup.rs`", "Include cleanup rewrite"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document include cleanup module ownership through {required}"
        );
    }
}
