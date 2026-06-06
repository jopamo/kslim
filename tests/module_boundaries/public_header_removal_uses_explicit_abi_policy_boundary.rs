use super::common::*;

#[test]
fn public_header_removal_uses_explicit_abi_policy_boundary() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let abi_policy = production_source(&root.join("src/abi/policy.rs"));
    let config = production_source(&root.join("src/config/model.rs"));
    let manifest_validate = production_source(&root.join("src/removal_manifest/validate.rs"));
    let prune = production_sources(&root, &["src/prune.rs", "src/prune/path.rs"]);
    let includes = production_sources(&root, &["src/source_scan/includes/mod.rs", "src/source_scan/includes/policy.rs"]);

    for required in [
        "pub struct AbiPolicyConfig",
        "fn is_uapi_path",
        "fn validate_uapi_removal",
        "fn validate_declared_removal",
        "UAPI removal requires explicit ABI policy approval",
        "allow_public_header_removal",
        "allow_uapi_header_removal",
        "explicit ABI policy approval",
        "abi.allow_public_header_removal",
        "abi.allow_uapi_header_removal",
    ] {
        assert!(
            abi_policy.contains(required),
            "src/abi/policy.rs should expose explicit ABI policy through {required}"
        );
    }
    assert!(
        config.contains("pub abi: AbiPolicyConfig"),
        "ProfileConfig should carry the ABI policy selected by profile input"
    );

    for required in [
        "validate_declared_abi_removal_policy",
        "abi::validate_declared_removal",
        "abi_sensitive_path_requires_own_manifest_truth",
    ] {
        assert!(
            manifest_validate.contains(required),
            "removal_manifest/validate.rs should centralize public-header policy through {required}"
        );
    }

    for required in [
        "crate::abi::validate_declared_removal",
        "abi_sensitive_path_requires_exact_manifest_truth",
        "crate::abi::is_uapi_path",
    ] {
        assert!(
            prune.contains(required),
            "prune modules should require exact UAPI manifest truth and ABI policy through {required}"
        );
    }

    assert!(
        includes.contains("crate::abi::allows_public_header_removal")
            && includes.contains("crate::abi::is_public_header_path")
            && includes.contains("removal_proofs.abi_policy"),
        "include cleanup should preserve public headers unless HeaderRemovalProofs carries ABI policy"
    );
}
