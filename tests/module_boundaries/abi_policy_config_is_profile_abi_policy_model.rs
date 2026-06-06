use super::common::*;

fn section_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let (_, rest) = source
        .split_once(start)
        .unwrap_or_else(|| panic!("missing section start marker {start:?}"));
    let (section, _) = rest
        .split_once(end)
        .unwrap_or_else(|| panic!("missing section end marker {end:?}"));
    section
}

#[test]
fn abi_policy_config_is_profile_abi_policy_model() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let abi_policy = production_source(&root.join("src/abi/policy.rs"));
    let abi_facade = production_source(&root.join("src/abi_policy.rs"));
    let config_model = production_source(&root.join("src/config/model.rs"));
    let config_mod = production_source(&root.join("src/config/mod.rs"));
    let config_templates = production_source(&root.join("src/config/templates.rs"));
    let config_validate = production_source(&root.join("src/config/validate.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let kernel_build_guide = kernel_build_iteration_docs(&root);
    let architecture_flat = architecture.replace("\n  ", " ");

    assert!(
        abi_policy.contains("pub struct AbiPolicyConfig"),
        "src/abi/policy.rs should define the [abi] profile policy model"
    );
    let abi_config = section_between(
        &abi_policy,
        "pub struct AbiPolicyConfig",
        "impl AbiPolicyConfig",
    );
    for required in [
        "pub allow_public_header_removal: bool,",
        "pub allow_uapi_header_removal: bool,",
    ] {
        assert!(
            abi_config.contains(required),
            "AbiPolicyConfig should own explicit policy flag {required}"
        );
    }
    assert_eq!(
        abi_config.matches("#[serde(default)]").count(),
        2,
        "AbiPolicyConfig flags should default to fail-closed false values"
    );
    assert!(
        abi_policy.contains("pub fn is_fail_closed(&self) -> bool")
            && abi_policy
                .contains("!self.allow_public_header_removal && !self.allow_uapi_header_removal"),
        "AbiPolicyConfig should expose fail-closed default detection"
    );

    for forbidden in [
        "FeatureConfig",
        "ReducerConfig",
        "SlimConfig",
        "RemovalManifest",
        "PrunePlan",
        "GeneratePlan",
        "CandidateTreeState",
        "PublishedSnapshotState",
        "LockfilePath",
    ] {
        assert!(
            !abi_config.contains(forbidden),
            "AbiPolicyConfig must stay narrow policy input, not resolved plan/state {forbidden}"
        );
    }

    assert!(
        abi_policy.contains("pub(crate) fn validate_declared_removal")
            && abi_policy.contains("validate_uapi_removal(path, policy)?;")
            && abi_policy.contains("validate_public_header_removal(path, policy)")
            && abi_policy.contains("if is_uapi_path(path)")
            && abi_policy.contains("policy.allow_uapi_header_removal")
            && abi_policy.contains("policy.allow_public_header_removal")
            && abi_policy.contains("explicit ABI policy approval"),
        "src/abi/policy.rs should centralize ABI-sensitive policy and keep UAPI approval explicit"
    );
    assert!(
        config_model.contains("use crate::abi::AbiPolicyConfig;")
            && config_model.contains("pub abi: AbiPolicyConfig,")
            && config_model.contains("pub fn effective_abi_policy(&self) -> AbiPolicyConfig")
            && config_model
                .contains("policy.allow_public_header_removal |= intent.allow_public_header_removal")
            && config_model
                .contains("policy.allow_uapi_header_removal |= intent.allow_uapi_header_removal"),
        "ProfileConfig should carry AbiPolicyConfig selected by profile input and resolve scoped feature ABI approvals"
    );
    assert!(
        config_mod.contains("pub use crate::abi::AbiPolicyConfig;"),
        "config/mod.rs should re-export AbiPolicyConfig with other profile config models"
    );
    assert!(
        abi_facade.contains("pub use crate::abi::AbiPolicyConfig;")
            && abi_facade.contains("pub(crate) use crate::abi::{"),
        "src/abi_policy.rs should be a compatibility facade over src/abi"
    );
    assert!(
        config_templates.contains("abi: AbiPolicyConfig::default()")
            && config_templates.contains("[abi]")
            && config_templates.contains("allow_public_header_removal = false")
            && config_templates.contains("allow_uapi_header_removal = false")
            && config_templates.contains("Public-header approval does not approve UAPI removal"),
        "profile templates should show fail-closed, separate ABI policy flags"
    );
    assert!(
        config_validate.contains("profile.effective_removal_input()")
            && config_validate.contains("profile.effective_preservation_input()")
            && config_validate.contains("validate_scoped_abi_policy(profile)?")
            && config_validate.contains("profile.effective_abi_policy()")
            && config_validate.contains("from_slim_config_with_abi_policy_and_preservation"),
        "profile validation should pass effective AbiPolicyConfig into normalized removal-manifest policy"
    );
    assert!(
        architecture_flat.contains("`AbiPolicyConfig` is the `[abi]` profile policy model")
            && architecture_flat.contains("Defaults fail closed")
            && architecture_flat.contains("public-header approval does not imply UAPI approval"),
        "architecture docs should describe AbiPolicyConfig ownership and fail-closed behavior"
    );
    assert!(
        kernel_build_guide.contains("[abi]")
            && kernel_build_guide.contains("ABI-sensitive removals fail closed")
            && kernel_build_guide.contains("Public-header approval does not approve UAPI removal"),
        "kernel build guide should document explicit, separate ABI policy flags"
    );
}
