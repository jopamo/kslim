use super::common::*;

#[test]
fn security_policy_is_separate_from_implementation_details() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let security_mod = production_source(&root.join("src/security/mod.rs"));
    let security_policy = production_source(&root.join("src/security/policy.rs"));
    let security_filesystem = production_source(&root.join("src/security/filesystem.rs"));
    let security_network = production_source(&root.join("src/security/network.rs"));
    let security_command = production_source(&root.join("src/security/command.rs"));
    let security_resource = production_source(&root.join("src/security/resource.rs"));
    let security_report_safety = production_source(&root.join("src/security/report_safety.rs"));
    let abi_policy = production_source(&root.join("src/abi/policy.rs"));
    let abi_surface = production_source(&root.join("src/abi/surface.rs"));
    let abi_policy_facade = production_source(&root.join("src/abi_policy.rs"));
    let network_policy = production_source(&root.join("src/network_policy.rs"));
    let path_policy = production_source(&root.join("src/path_policy.rs"));
    let output_metadata = production_source(&root.join("src/output_repo/metadata.rs"));
    let config_validate = production_source(&root.join("src/config/validate.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));
    let architecture_flat = architecture.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(
        main.contains("mod security;")
            && security_mod.contains("mod policy;")
            && security_mod.contains("mod filesystem;")
            && security_mod.contains("mod network;")
            && security_mod.contains("mod command;")
            && security_mod.contains("mod resource;")
            && security_mod.contains("mod report_safety;")
            && security_mod.contains("pub(crate) use policy::validate_security_config;"),
        "main/security module boundary should expose security policy through src/security/*"
    );
    assert!(
        config_validate.contains("crate::security::validate_security_config(&profile.security)?"),
        "config validation should call centralized security policy instead of embedding it"
    );
    assert!(
        network_policy.contains("pub(crate) use crate::security::{")
            && network_policy.contains("require_local_upstream_url")
            && !network_policy.contains("enum EndpointKind"),
        "network_policy.rs should be a compatibility facade over src/security/network.rs"
    );
    assert!(
        path_policy.contains("pub(crate) use crate::security::{")
            && path_policy.contains("path_contains_parent_traversal")
            && !path_policy.contains("Component::ParentDir"),
        "path_policy.rs should be a compatibility facade over src/security/filesystem.rs"
    );

    for required in [
        "enum EndpointKind",
        "pub(crate) fn configure_cli",
        "pub(crate) fn require_local_upstream_url",
        "pub(crate) fn require_cli_no_network_endpoint",
        "fn endpoint_kind",
        "fn looks_like_scp_remote",
    ] {
        assert!(
            security_network.contains(required),
            "src/security/network.rs should own network policy item {required}"
        );
    }
    for required in [
        "pub(crate) fn path_is_empty_like",
        "pub(crate) fn path_is_absolute_like",
        "pub(crate) fn path_contains_parent_traversal",
        "pub(crate) fn normalized_relative_path_covers",
        "Component::ParentDir",
        "is_windows_absolute_path_like",
    ] {
        assert!(
            security_filesystem.contains(required),
            "src/security/filesystem.rs should own filesystem path policy item {required}"
        );
    }
    for required in [
        "pub(crate) struct CommandPolicy",
        "allow_shell: bool",
        "pub(crate) fn validate_program",
        "rejects shell execution without explicit compatibility mode",
    ] {
        assert!(
            security_command.contains(required),
            "src/security/command.rs should own command policy item {required}"
        );
    }
    for required in [
        "pub(crate) struct ResourcePolicy",
        "allow_unbounded_execution: bool",
        "pub(crate) fn validate",
        "rejects unbounded execution",
    ] {
        assert!(
            security_resource.contains(required),
            "src/security/resource.rs should own resource policy item {required}"
        );
    }
    for required in [
        "pub(crate) fn validate_report_text_has_no_temporary_paths",
        "pub(crate) fn temporary_path_markers",
        "pub(crate) fn validate_report_text_has_no_host_absolute_paths",
        "pub(crate) fn is_host_specific_absolute_path",
        "pub(crate) fn validate_report_text_has_no_raw_logs",
        "pub(crate) fn raw_log_marker",
        "pub(crate) fn validate_reproducible_timestamp",
        "pub(crate) fn is_reproducible_timestamp",
        "pub(crate) fn timestamp_markers",
    ] {
        assert!(
            security_report_safety.contains(required),
            "src/security/report_safety.rs should own report-safety policy item {required}"
        );
    }
    assert!(
        output_metadata.contains("crate::security::temporary_path_markers")
            && output_metadata.contains("crate::security::raw_log_marker")
            && output_metadata.contains("crate::security::timestamp_markers")
            && output_metadata.contains("crate::security::is_host_specific_absolute_path")
            && output_metadata.contains("crate::security::validate_reproducible_timestamp"),
        "output metadata should enforce committed report safety through src/security/report_safety.rs policy helpers"
    );

    for (path, source) in [
        ("src/security/policy.rs", security_policy.as_str()),
        ("src/security/filesystem.rs", security_filesystem.as_str()),
        ("src/security/network.rs", security_network.as_str()),
        ("src/security/command.rs", security_command.as_str()),
        ("src/security/resource.rs", security_resource.as_str()),
        (
            "src/security/report_safety.rs",
            security_report_safety.as_str(),
        ),
        ("src/abi/policy.rs", abi_policy.as_str()),
        ("src/abi/surface.rs", abi_surface.as_str()),
        ("src/abi_policy.rs", abi_policy_facade.as_str()),
        ("src/network_policy.rs", network_policy.as_str()),
        ("src/path_policy.rs", path_policy.as_str()),
    ] {
        for forbidden in [
            "crate::generate",
            "crate::output_repo",
            "crate::publish",
            "crate::reducer",
            "crate::prune",
            "crate::kconfig",
            "crate::kbuild",
            "crate::includes",
            "crate::fixups",
            "std::fs::write",
            "std::fs::remove_",
            "std::process::Command",
            "commit_if_changed",
        ] {
            assert!(
                !source.contains(forbidden),
                "{path} should own policy decisions without depending on implementation mechanics; found {forbidden}"
            );
        }
    }

    for required in [
        "`security/*` | Security policy decisions",
        "`abi/*` | ABI-sensitive UAPI",
        "`abi_policy.rs` is only the compatibility facade",
        "`network_policy.rs` | Compatibility facade",
        "`path_policy.rs` | Compatibility facade",
        "Security policy modules own trust-boundary decisions",
        "`security/network.rs` owns CLI network/offline endpoint classification",
        "`security/report_safety.rs` owns committed report/metadata rejection",
        "must not depend on generate, output, publish, reducer, or rewrite implementation modules",
    ] {
        assert!(
            architecture_flat.contains(required),
            "docs/architecture.md should describe centralized security policy boundary {required}"
        );
    }
}
