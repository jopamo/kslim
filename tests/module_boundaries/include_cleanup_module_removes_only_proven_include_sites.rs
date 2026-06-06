use super::common::*;

#[test]
fn include_cleanup_module_removes_only_proven_include_sites() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let includes = production_sources(
        &root,
        &[
            "src/source_scan/includes/mod.rs",
            "src/source_scan/includes/cleanup.rs",
            "src/source_scan/includes/index.rs",
            "src/source_scan/includes/private_header.rs",
        ],
    );

    for required in [
        "enum IncludeRemovalProof",
        "ManifestHeader",
        "DeadBranch",
        "proven_dead_include_line_proofs_by_file",
        "visible_cpp_directive_lines",
        "site_live_after_preprocessor",
        "manifest_removed_private_header_proof_path",
        "EditReason::RemovedDeadBranchInclude",
        "EditProofSource::removal_manifest_header",
        "EditProofSource::removal_manifest_config",
    ] {
        assert!(
            includes.contains(required),
            "include cleanup modules should remove includes only with manifest or dead-branch proof through \
             {required}"
        );
    }

    for forbidden in ["content.replace(", "line.replace(", "std::fs::write(&path"] {
        assert!(
            !includes.contains(forbidden),
            "include cleanup must not use broad whole-file replacement patterns; found {forbidden}"
        );
    }
}
