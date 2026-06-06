use super::common::*;

#[test]
fn filesystem_mutation_is_separate_from_proof_generation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let proof_modules = [
        "src/removal_manifest.rs",
        "src/removal_manifest/model.rs",
        "src/removal_manifest/parse.rs",
        "src/removal_manifest/match_rules.rs",
        "src/removal_manifest/validate.rs",
        "src/exported_symbols.rs",
        "src/hardware/devicetree.rs",
        "src/device_bindings.rs",
        "src/runtime/registration.rs",
        "src/runtime_registrations.rs",
    ];

    for relative in proof_modules {
        let source = production_source(&root.join(relative));
        assert_no_filesystem_mutation_api(relative, &source);
    }

    let prune = production_sources(
        &root,
        &["src/prune.rs", "src/prune/path.rs", "src/prune/report.rs"],
    );
    assert!(
        prune.contains("pub fn prune_tree_from_manifest(root: &str, manifest: &RemovalManifest)")
            && prune.contains("pub(crate) fn prune_declared_paths_from_manifest(")
            && prune.contains("EditProofSource::removal_manifest_path"),
        "prune modules should mutate only from normalized manifest proof, not raw profile intent"
    );

    for forbidden in [
        "SlimConfig",
        "from_slim_config_for_tree",
        "from_slim_config_with",
        "effective_removal_input",
        "derive_removed_exported_symbol_proofs",
        "derive_removed_device_binding_proofs",
        "derive_removed_runtime_registration_proofs",
        "prove_removed_exports_have_no_live_consumers",
        "prove_removed_device_bindings_have_no_live_references",
        "prove_removed_runtime_registrations_have_no_live_entry_points",
    ] {
        assert!(
            !prune.contains(forbidden),
            "prune modules should consume proof-bearing RemovalManifest values instead of generating proof while mutating; found {forbidden}"
        );
    }
}

fn assert_no_filesystem_mutation_api(path: &str, source: &str) {
    for forbidden in [
        "std::fs::write",
        "std::fs::remove_file",
        "std::fs::remove_dir",
        "std::fs::remove_dir_all",
        "std::fs::rename",
        "std::fs::copy",
        "std::fs::create_dir",
        "std::fs::create_dir_all",
        "std::fs::set_permissions",
        "File::create",
        "OpenOptions",
        ".write_all(",
    ] {
        assert!(
            !source.contains(forbidden),
            "{path} is a proof-generation module and must not mutate the filesystem; found {forbidden}"
        );
    }
}
