use super::common::*;

#[test]
fn runtime_registration_removal_requires_no_live_entry_point_proof() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let runtime_registrations = production_source(&root.join("src/runtime/registration.rs"));
    let runtime_registrations_facade = production_source(&root.join("src/runtime_registrations.rs"));
    let manifest = format!(
        "{}\n{}",
        production_source(&root.join("src/removal_manifest/model.rs")),
        production_source(&root.join("src/removal_manifest/match_rules.rs"))
    );
    let reducer_report = production_sources(
        &root,
        &[
            "src/reducer/report/json.rs",
            "src/reducer/report/json/schema.rs",
            "src/reducer/report/json/serializer.rs",
            "src/reducer/report/json/escaping.rs",
            "src/reducer/report/json/canonical.rs",
        ],
    );

    assert!(
        main.contains("mod runtime;") && main.contains("mod runtime_registrations;"),
        "main.rs should register the runtime owner and runtime-registration facade modules"
    );

    for required in [
        "RuntimeRegistrationRemovalProof",
        "prove_removed_runtime_registrations_have_no_live_entry_points",
        "runtime registration removal requires proof",
        "live entry point",
        "RuntimeRegistrationSurface::is_known_registration_macro",
        "mask_c_comments_and_literals",
    ] {
        assert!(
            runtime_registrations.contains(required),
            "src/runtime/registration.rs should prove removed runtime registrations have no live entry points through {required}"
        );
    }

    assert!(
        runtime_registrations_facade.contains("pub(crate) use crate::runtime::{")
            && runtime_registrations_facade.contains("RuntimeRegistrationRemovalProof"),
        "src/runtime_registrations.rs should be a compatibility facade over src/runtime"
    );

    for required in [
        "removed_runtime_registrations",
        "derive_removed_runtime_registration_proofs",
        "prove_removed_runtime_registrations_have_no_live_entry_points",
    ] {
        assert!(
            manifest.contains(required),
            "removal manifest modules should carry runtime-registration removal proof through {required}"
        );
    }

    for required in [
        "removed_runtime_registration_count",
        "removed_runtime_registrations",
        "render_removed_runtime_registrations_json",
    ] {
        assert!(
            reducer_report.contains(required),
            "reducer reports should expose runtime-registration proof through {required}"
        );
    }
}
