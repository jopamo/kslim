use super::*;

fn removal_proofs_with_removed_paths(paths: &[&str]) -> HeaderRemovalProofs {
    removal_proofs_with_removed_headers(paths, &[])
}

fn removal_proofs_with_removed_headers(
    manifest_paths: &[&str],
    removed_header_paths: &[PathBuf],
) -> HeaderRemovalProofs {
    removal_proofs_with_removed_headers_and_abi_policy(
        manifest_paths,
        removed_header_paths,
        &crate::abi::AbiPolicyConfig::default(),
    )
}

fn removal_proofs_with_removed_headers_and_abi_policy(
    manifest_paths: &[&str],
    removed_header_paths: &[PathBuf],
    abi_policy: &crate::abi::AbiPolicyConfig,
) -> HeaderRemovalProofs {
    let manifest_paths = manifest_paths.iter().map(PathBuf::from).collect::<Vec<_>>();
    HeaderRemovalProofs::from_manifest_paths_with_abi_policy(
        &manifest_paths,
        removed_header_paths,
        abi_policy,
    )
}

fn allow_public_header_removal() -> crate::abi::AbiPolicyConfig {
    crate::abi::AbiPolicyConfig {
        allow_public_header_removal: true,
        allow_uapi_header_removal: false,
    }
}

fn allow_uapi_header_removal() -> crate::abi::AbiPolicyConfig {
    crate::abi::AbiPolicyConfig {
        allow_public_header_removal: false,
        allow_uapi_header_removal: true,
    }
}

#[path = "tests_index.rs"]
mod index;
#[path = "tests_cleanup.rs"]
mod cleanup;
#[path = "tests_private_header.rs"]
mod private_header;
#[path = "tests_public_header_policy.rs"]
mod public_header_policy;
