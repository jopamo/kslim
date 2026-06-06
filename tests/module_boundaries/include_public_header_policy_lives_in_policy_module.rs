use super::common::*;

#[test]
fn include_public_header_policy_lives_in_policy_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let includes = production_source(&root.join("src/source_scan/includes/mod.rs"));
    let policy = production_source(&root.join("src/source_scan/includes/policy.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    for required in [
        "mod policy;",
        "is_public_preserved_header_path",
        "target_is_explicitly_removed_public_header",
        "should_report_conservatively_preserved_public_header",
        "should_report_missing_public_header",
        "is_generated_header_target",
    ] {
        assert!(
            includes.contains(required),
            "src/source_scan/includes/mod.rs should expose public-header policy through {required}"
        );
    }

    for required in [
        "pub(in crate::source_scan::includes) fn target_is_explicitly_removed_public_header(",
        "pub(in crate::source_scan::includes) fn should_report_conservatively_preserved_public_header(",
        "pub(in crate::source_scan::includes) fn is_surviving_public_header_site(",
        "pub(in crate::source_scan::includes) fn should_report_missing_public_header(",
        "pub(in crate::source_scan::includes) fn is_public_preserved_header_path(",
        "pub(in crate::source_scan::includes) fn is_generated_header_target(",
        "fn is_arch_generated_header_path(",
        "crate::abi::allows_public_header_removal(",
        "crate::abi::is_public_header_path(path)",
        "removal_proofs.abi_policy",
        "explicit_proof_path_for(&classified.target.path)",
        "proof_path != classified.target.path",
        "candidate_include_root_target(site)",
        "preserve_subsystem_looking_include_when_resolved_header_exists",
        "!is_generated_header_target(&classified.target)",
    ] {
        assert!(
            policy.contains(required),
            "src/source_scan/includes/policy.rs should own public-header policy item {required}"
        );
    }

    for forbidden in [
        "\npub(in crate::source_scan::includes) fn target_is_explicitly_removed_public_header(",
        "\npub(in crate::source_scan::includes) fn should_report_conservatively_preserved_public_header(",
        "\npub(in crate::source_scan::includes) fn is_surviving_public_header_site(",
        "\npub(in crate::source_scan::includes) fn should_report_missing_public_header(",
        "\npub(in crate::source_scan::includes) fn is_public_preserved_header_path(",
        "\npub(in crate::source_scan::includes) fn is_generated_header_target(",
        "\nfn is_arch_generated_header_path(",
        "crate::abi::allows_public_header_removal(",
        "crate::abi::is_public_header_path(path)",
        "proof_path != classified.target.path",
    ] {
        assert!(
            !includes.contains(forbidden),
            "src/source_scan/includes/mod.rs should not retain extracted public-header policy implementation {forbidden}"
        );
    }

    for required in ["`src/source_scan/includes/policy.rs`", "Public-header policy"] {
        assert!(
            architecture.contains(required),
            "docs/architecture.md should document public-header policy ownership through {required}"
        );
    }
}
