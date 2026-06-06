use super::common::*;

#[test]
fn kconfig_rewrite_removes_dead_source_lines_only_with_manifest_index_proof() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kconfig = production_sources(&root, &["src/kconfig/mod.rs", "src/kconfig/rewrite.rs"]);
    let kconfig_with_tests = production_sources(
        &root,
        &[
            "src/kconfig/tests.rs",
            "src/kconfig/tests_report.rs",
            "src/kconfig/tests_rewrite.rs",
            "src/kconfig/tests_root_facade.rs",
            "src/kconfig/tests_solver.rs",
        ],
    );
    let prune = production_sources(&root, &["src/prune.rs", "src/prune/stale_reference.rs"]);
    let edit_reason = production_sources(
        &root,
        &[
            "src/edit_reason.rs",
            "src/edit_reason/reason.rs",
            "src/edit_reason/proof_source.rs",
            "src/edit_reason/render.rs",
        ],
    );
    let architecture =
        std::fs::read_to_string(root.join("docs/architecture.md")).expect("failed to read docs");
    let kernel_build = kernel_build_iteration_docs(&root);

    for required in [
        "pub(crate) struct KconfigSourceRemovalProof",
        "source_removal_proofs: &[KconfigSourceRemovalProof]",
        "fn kconfig_source_removal_proof_for_line",
        "if proof.optional || proof.source.contains('$')",
        "!source.optional",
        "resolve_kconfig_source(root, current_dir, &source).is_none()",
        "proof.removed_target.clone()",
        "EditProofSource::removal_manifest_kconfig_source",
    ] {
        assert!(
            kconfig.contains(required),
            "src/kconfig/mod.rs should remove dead source lines only with explicit proof; missing {required}"
        );
    }

    for required in [
        "TreeIndex::build(root, manifest)?",
        "kconfig_source_removal_proofs(root, manifest, &index)",
        "manifest.removed_kconfig_sources_vec()",
        "index.kconfig_sources",
        "if source_ref.optional || source_ref.source.contains('$')",
        "manifest_removed_kconfig_source_target(",
    ] {
        assert!(
            prune.contains(required),
            "prune.rs should derive source-removal proofs from manifest plus tree index; missing {required}"
        );
    }

    for required in [
        "KconfigSource(PathBuf)",
        "removal_manifest_kconfig_source",
        "EditReason::RemovedKconfigSource",
        "EditProofSourceKind::RemovalManifestEntry",
    ] {
        assert!(
            edit_reason.contains(required),
            "edit provenance should record removed source lines as manifest-backed proof; missing {required}"
        );
    }

    for required in [
        "test_rewrite_kconfig_sources_removes_dead_source_and_preserves_live_source",
        "source \\\"drivers/dead/Kconfig\\\"",
        "source \\\"drivers/live/Kconfig\\\"",
        "source \\\"drivers/unproven-dead/Kconfig\\\"",
        "EditReason::RemovedKconfigSource",
        "EditProofSource::removal_manifest_kconfig_source(PathBuf::from(",
        "test_rewrite_kconfig_sources_preserves_optional_missing_sources",
        "osource \\\"drivers/optional/Kconfig\\\"",
        "orsource \\\"drivers/optional-relative/Kconfig\\\"",
        "optional: true",
        "relative: true",
        "test_rewrite_kconfig_sources_requires_manifest_index_proof",
        "file: PathBuf::from(\"Kconfig\")",
        "line: 2,",
        "source: String::from(\"drivers/missing/Kconfig\")",
        "removed_target: PathBuf::from(\"drivers/missing/Kconfig\")",
    ] {
        assert!(
            kconfig_with_tests.contains(required),
            "unit tests should pin exact manifest/index proof gating; missing {required}"
        );
    }

    for docs in [architecture, kernel_build] {
        assert!(
            docs.contains("Kconfig `source` removal is manifest/index proof-gated"),
            "docs should describe manifest/index proof-gated Kconfig source removal"
        );
        assert!(
            docs.contains("Dead Kconfig source references")
                && docs.contains("rewritten only when the source target is absent"),
            "docs should describe dead Kconfig source removal"
        );
        assert!(
            docs.contains("Optional Kconfig sources")
                && docs.contains("are preserved even when missing"),
            "docs should describe optional Kconfig source preservation"
        );
        assert!(
            docs.contains("unresolved sources without both proofs are preserved"),
            "docs should say unresolved sources without both proofs are preserved"
        );
    }

    assert!(
        !kconfig.contains("pub(crate) fn rewrite_kconfig_sources(root: &Path)"),
        "source rewriting should not keep the old unproven missing-target API"
    );
    assert!(
        !kconfig.contains("EditProofSource::stale_kconfig_source(PathBuf::from(&source.path))"),
        "source-line deletion must not be justified by a stale-reference proof alone"
    );
}
