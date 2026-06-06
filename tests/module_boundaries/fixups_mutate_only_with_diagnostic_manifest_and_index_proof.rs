use super::common::*;

#[test]
fn fixups_mutate_only_with_diagnostic_manifest_and_index_proof() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let production = production_sources(
        root,
        &[
            "src/fixups.rs",
            "src/fixups/application.rs",
            "src/fixups/classification.rs",
            "src/fixups/planning.rs",
            "src/fixups/reindex.rs",
            "src/fixups/report.rs",
        ],
    );

    for required in [
        "pub fn apply_classified_fixup",
        "FixupProof::ManifestPath",
        "FixupProof::TreeIndexIncludeSite",
        "FixupProof::TreeIndexKbuildDirectoryRef",
        "FixupProof::TreeIndexKbuildObjectRef",
        "FixupProof::TreeIndexKconfigSourceRef",
        "classified_diagnostic_proof(diagnostic)",
        "fn write_proven_fixup_rewrite",
        "validate_fixup_result(pass_name, index, diagnostic, &result)?;",
    ] {
        assert!(
            production.contains(required),
            "fixups subsystem should require classified diagnostic plus manifest/index proof; missing {required}"
        );
    }

    assert_eq!(
        production.matches("write_verified_rewrite(").count(),
        1,
        "fixups subsystem should funnel actual writes through one proof-gated rewrite helper"
    );
    assert!(
        production.find("validate_fixup_result(pass_name, index, diagnostic, &result)?;")
            < production.find("write_verified_rewrite("),
        "fixups subsystem must validate manifest/index/diagnostic proof before any rewrite"
    );

    let forbidden_direct_mutation_or_policy = [
        "std::fs::write(",
        "std::fs::remove",
        "std::fs::rename",
        "std::fs::copy",
        "File::create",
        "OpenOptions",
        "crate::config",
        "crate::generate",
        "crate::output_repo",
        "crate::reducer",
        "crate::selftest",
        "crate::upstream",
    ];

    for forbidden in forbidden_direct_mutation_or_policy {
        assert!(
            !production.contains(forbidden),
            "fixups subsystem must not bypass classified-diagnostic plus manifest/index proof; found forbidden token {forbidden}"
        );
    }
}
