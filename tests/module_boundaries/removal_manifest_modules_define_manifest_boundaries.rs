use super::common::*;

#[test]
fn removal_manifest_modules_define_manifest_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let facade = production_source(&root.join("src/removal_manifest.rs"));
    let model = production_source(&root.join("src/removal_manifest/model.rs"));
    let parse = production_source(&root.join("src/removal_manifest/parse.rs"));
    let validate = production_source(&root.join("src/removal_manifest/validate.rs"));
    let match_rules = production_source(&root.join("src/removal_manifest/match_rules.rs"));
    let generated_policy = production_source(&root.join("src/generated/policy.rs"));

    for required in [
        "mod model;",
        "mod parse;",
        "mod validate;",
        "mod match_rules;",
    ] {
        assert!(
            facade.contains(required),
            "removal_manifest.rs should register split manifest module {required}"
        );
    }
    for required in [
        "pub struct RemovalManifest",
        "pub enum RemovalKey",
        "pub enum RemovalReason",
        "removed_exported_symbols",
        "removed_device_bindings",
        "removed_runtime_registrations",
    ] {
        assert!(
            model.contains(required),
            "removal_manifest/model.rs should own manifest data shape item {required}"
        );
    }
    for required in [
        "from_slim_config_with_root",
        "normalize_declared_path",
        "derive_removed_headers",
        "derive_removed_kconfig_sources",
        "derive_removed_kbuild_objects",
    ] {
        assert!(
            parse.contains(required),
            "removal_manifest/parse.rs should own slim-input assembly item {required}"
        );
    }
    for required in [
        "validate_declared_abi_removal_policy",
        "abi_sensitive_path_requires_own_manifest_truth",
        "derive_removed_path_categories",
    ] {
        assert!(
            validate.contains(required),
            "removal_manifest/validate.rs should own manifest validation item {required}"
        );
    }
    for required in [
        "normalize_generated_include_roots",
        "is_generated_include_header_path",
        "generated include roots must not contain '..'",
    ] {
        assert!(
            generated_policy.contains(required),
            "src/generated/policy.rs should own generated include policy item {required}"
        );
    }

    for required in [
        "derive_removed_exported_symbol_proofs",
        "derive_removed_device_binding_proofs",
        "derive_removed_runtime_registration_proofs",
        "removed_composite_kbuild_object_targets",
        "removed_path_is_derivable_header",
    ] {
        assert!(
            match_rules.contains(required),
            "removal_manifest/match_rules.rs should own derived manifest match rule {required}"
        );
    }
}
