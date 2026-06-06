use super::common::*;

#[test]
fn include_private_header_orphaning_lives_in_private_header_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let includes = production_source(&root.join("src/source_scan/includes/mod.rs"));
    let private_header = production_source(&root.join("src/source_scan/includes/private_header.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod private_header;",
        "pub(in crate::source_scan::includes) use private_header::manifest_removed_private_header_proof_path",
        "target_is_covered_by_removal_manifest",
        "target_is_gone_from_reduced_tree",
    ] {
        assert!(
            includes.contains(required),
            "src/source_scan/includes/mod.rs should expose private-header orphaning through {required}"
        );
    }

    for required in [
        "pub(crate) fn target_is_gone_from_reduced_tree(",
        "pub(crate) fn target_is_covered_by_removal_manifest(",
        "pub(crate) fn include_site_passes_preprocessor_or_local_rule_gate(",
        "pub(crate) fn local_removal_rule_applies(",
        "fn should_remove_manifest_removed_private_header(",
        "pub(in crate::source_scan::includes) fn manifest_removed_private_header_proof_path(",
        "target_is_explicitly_removed_public_header(root, removal_proofs, classified_targets)",
        "!is_public_preserved_header_path(&classified.target.path)",
        "!is_generated_header_target(&classified.target)",
        "proof_paths.sort();",
        "proof_paths.dedup();",
    ] {
        assert!(
            private_header.contains(required),
            "src/source_scan/includes/private_header.rs should own private-header orphaning item {required}"
        );
    }

    for forbidden in [
        "\npub(crate) fn target_is_gone_from_reduced_tree(",
        "\npub(crate) fn target_is_covered_by_removal_manifest(",
        "\npub(crate) fn include_site_passes_preprocessor_or_local_rule_gate(",
        "\npub(crate) fn local_removal_rule_applies(",
        "\nfn should_remove_manifest_removed_private_header(",
        "\npub(in crate::source_scan::includes) fn manifest_removed_private_header_proof_path(",
        "proof_paths.sort();",
        "proof_paths.dedup();",
    ] {
        assert!(
            !includes.contains(forbidden),
            "src/source_scan/includes/mod.rs should not retain extracted private-header orphaning implementation {forbidden}"
        );
    }

    for required in ["`src/source_scan/includes/private_header.rs`", "Private-header orphaning"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document private-header module ownership through {required}"
        );
    }
}
