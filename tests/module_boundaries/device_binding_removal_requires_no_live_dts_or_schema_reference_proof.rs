use super::common::*;

#[test]
fn device_binding_removal_requires_no_live_dts_or_schema_reference_proof() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let main = production_source(&root.join("src/main.rs"));
    let device_bindings = production_source(&root.join("src/hardware/devicetree.rs"));
    let device_bindings_facade = production_source(&root.join("src/device_bindings.rs"));
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
        main.contains("mod hardware;") && main.contains("mod device_bindings;"),
        "main.rs should register the hardware owner and device-binding facade modules"
    );

    for required in [
        "DeviceBindingRemovalProof",
        "prove_removed_device_bindings_have_no_live_references",
        "device binding removal requires proof",
        "DTS/DTSI/schema reference",
        "DeviceCompatible",
        "compatible_strings_in_content",
        "schema_reference_tokens",
    ] {
        assert!(
            device_bindings.contains(required),
            "src/hardware/devicetree.rs should prove removed bindings have no live references through {required}"
        );
    }

    assert!(
        device_bindings_facade.contains("pub(crate) use crate::hardware::{")
            && device_bindings_facade.contains("DeviceBindingRemovalProof"),
        "src/device_bindings.rs should be a compatibility facade over src/hardware"
    );

    for required in [
        "removed_device_bindings",
        "derive_removed_device_binding_proofs",
        "prove_removed_device_bindings_have_no_live_references",
    ] {
        assert!(
            manifest.contains(required),
            "removal manifest modules should carry device-binding removal proof through {required}"
        );
    }

    for required in [
        "removed_device_binding_count",
        "removed_device_bindings",
        "render_removed_device_bindings_json",
    ] {
        assert!(
            reducer_report.contains(required),
            "reducer reports should expose device-binding proof through {required}"
        );
    }
}
