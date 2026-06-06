use super::common::*;

#[test]
fn core_module_owns_shared_foundations() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let core_mod = production_source(&root.join("src/core/mod.rs"));
    let core_error = production_source(&root.join("src/core/error.rs"));
    let core_fingerprint = production_source(&root.join("src/core/fingerprint.rs"));
    let core_result = production_source(&root.join("src/core/result.rs"));
    let core_serialization = production_source(&root.join("src/core/serialization.rs"));
    let core_stable_id = production_source(&root.join("src/core/stable_id.rs"));
    let legacy_error = production_source(&root.join("src/error.rs"));
    let generate_plan = plan_source(root);
    let generate_state = state_source(root);
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        main.contains("mod core;"),
        "src/main.rs should register the shared core module"
    );
    assert!(
        !root.join("src/core.rs").exists(),
        "core should be a subsystem directory, not a new top-level core.rs catch-all"
    );

    for required in [
        "mod error;",
        "mod fingerprint;",
        "mod result;",
        "mod serialization;",
        "mod stable_id;",
        "pub(crate) use error::KslimError;",
        "pub(crate) use fingerprint::{prefixed_sha256_hex, sha256_hex};",
        "pub(crate) use result::{KslimResult, StdResult};",
        "append_stable_key_value_line",
        "pub(crate) use stable_id::stable_id;",
    ] {
        assert!(
            core_mod.contains(required),
            "src/core/mod.rs should register and re-export shared foundation item {required}"
        );
    }
    assert!(
        core_error.contains("pub(crate) enum KslimError")
            && legacy_error.contains("pub(crate) use crate::core::KslimError;"),
        "KslimError should live in core while src/error.rs stays as a compatibility facade"
    );
    assert!(
        core_result.contains("pub(crate) type KslimResult<T> = anyhow::Result<T>;")
            && core_result.contains("pub(crate) type StdResult<T, E>"),
        "src/core/result.rs should own result aliases"
    );
    assert!(
        core_fingerprint.contains("pub(crate) fn sha256_hex")
            && core_fingerprint.contains("pub(crate) fn prefixed_sha256_hex"),
        "src/core/fingerprint.rs should own digest helpers"
    );
    assert!(
        core_serialization.contains("pub(crate) fn append_stable_key_value_line")
            && core_serialization.contains("pub(crate) fn escape_stable_value")
            && core_serialization.contains("pub(crate) fn bool_token"),
        "src/core/serialization.rs should own stable serialization helpers"
    );
    assert!(
        core_stable_id.contains("pub(crate) fn stable_id")
            && core_stable_id.contains("value.len().to_string()"),
        "src/core/stable_id.rs should own length-delimited stable ID construction"
    );
    assert!(
        generate_plan.contains("append_stable_key_value_line")
            && generate_plan.contains("sha256_hex as core_sha256_hex")
            && generate_state.contains("crate::core::stable_id(kind, fields)"),
        "existing fingerprint and stable-id callsites should consume core helpers"
    );

    let core_sources = [
        core_mod,
        core_error,
        core_fingerprint,
        core_result,
        core_serialization,
        core_stable_id,
    ]
    .join("\n");
    for forbidden in [
        "crate::config",
        "crate::generate",
        "crate::kconfig",
        "crate::kbuild",
        "crate::output_repo",
        "crate::reducer",
        "crate::tree_index",
        "crate::paths",
    ] {
        assert!(
            !core_sources.contains(forbidden),
            "src/core/* must not depend on product subsystems; found {forbidden}"
        );
    }
    assert!(
        architecture.contains("`core/*`")
            && architecture.contains("shared errors, result aliases, stable IDs")
            && architecture.contains("`core/*` has no product-subsystem dependencies"),
        "docs/architecture.md should document core ownership and dependency direction"
    );
}
