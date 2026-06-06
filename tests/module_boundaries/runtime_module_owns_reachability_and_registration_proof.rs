use super::common::*;

#[test]
fn runtime_module_owns_reachability_and_registration_proof() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let runtime_mod = production_source(&root.join("src/runtime/mod.rs"));
    let registration = production_source(&root.join("src/runtime/registration.rs"));
    let reachability = production_source(&root.join("src/runtime/reachability.rs"));
    let runtime_facade = production_source(&root.join("src/runtime_registrations.rs"));
    let architecture = production_source(&root.join("docs/architecture.md"));

    assert!(
        main.contains("mod runtime;") && main.contains("mod runtime_registrations;"),
        "main.rs should register runtime ownership and legacy runtime_registrations facade modules"
    );

    for required in [
        "//! Runtime reachability and entry-point proof gates.",
        "mod reachability;",
        "mod registration;",
        "pub(crate) use registration::{",
        "prove_removed_runtime_registrations_have_no_live_entry_points",
        "RuntimeRegistrationRemovalProof",
        "pub(crate) use reachability::{",
        "ModuleEntryPoint",
        "RuntimeCallbackName",
        "RuntimeReachabilityKind",
        "RuntimeReachabilitySubject",
    ] {
        assert!(
            runtime_mod.contains(required),
            "src/runtime/mod.rs should own runtime module declaration/export item {required}"
        );
    }

    for required in [
        "pub(crate) struct RuntimeRegistrationRemovalProof",
        "pub(crate) fn prove_removed_runtime_registrations_have_no_live_entry_points",
        "fn scan_runtime_registrations_in_content",
        "fn parse_registration_entry_points",
        "fn live_references_for_entry_points",
        "fn identifier_occurrence_lines",
        "fn mask_c_comments_and_literals",
    ] {
        assert!(
            registration.contains(required),
            "src/runtime/registration.rs should own runtime registration proof item {required}"
        );
    }

    for required in [
        "pub(crate) enum RuntimeReachabilityKind",
        "Initcall",
        "RuntimeRegistration",
        "Callback",
        "ModuleEntryPoint",
        "pub(crate) struct RuntimeCallbackName",
        "pub(crate) struct ModuleEntryPoint",
        "pub(crate) enum RuntimeReachabilitySubject",
        "pub(crate) fn stable_key(&self) -> String",
        "initcall",
        "runtime_registration",
        "callback",
        "module_entry_point",
    ] {
        assert!(
            reachability.contains(required),
            "src/runtime/reachability.rs should own runtime reachability item {required}"
        );
    }

    assert!(
        runtime_facade.contains("pub(crate) use crate::runtime::{")
            && !runtime_facade.contains("struct RuntimeRegistration"),
        "src/runtime_registrations.rs should be a compatibility facade over src/runtime"
    );

    assert!(
        architecture.contains("`runtime/*`")
            && architecture.contains("Initcall, registration, callback, module entry-point")
            && architecture.contains("runtime reachability proof")
            && architecture.contains("`runtime_registrations.rs` is only the compatibility facade"),
        "docs/architecture.md should document runtime ownership and runtime_registrations facade"
    );
}
